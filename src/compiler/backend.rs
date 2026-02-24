use super::ir::{self, Command, Definition, Immediate, IrGen, Output, Value};
use super::lexer::TypeKind;
use crate::executable::{self, Bytecode, Executable, Fn};
use crate::vm::CommandType as CmdType;
use std::collections::{BTreeMap, HashSet};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhysReg {
    // 16-bit General Purpose
    R1,
    R2,
    R3,
    R4,
    R5,
    // 32-bit Extended (Aliased: EX1=R2+R3, EX2=R4+R5)
    EX1,
    EX2,
    // 32-bit Float
    F1,
    F2,
}

#[derive(Debug, Clone, Copy)]
enum RegLoc {
    Stack(usize),
    Physical(PhysReg),
}

#[derive(Debug)]
pub enum Inst {
    OpCode(OpCode),
    // Operands
    PhysReg(PhysReg),
    Immediate(Immediate),
    Location(ir::Location),
    StackOffset(usize),
    ARP,
    SymbolSecLen,
    ArgCount,
}

#[derive(Debug)]
pub enum OpCode {
    Arithmetic(ArithmeticOp, CommandType),
    Stack(StackOp, CommandType),
    Memory(MemoryOp, CommandType),
    Logic(LogicOp, CommandType),
    Move(CommandType),
    Call,
    Return,
    Jump,
    JumpNotZero,
    JumpZero,
    GreaterThan(CommandType),
    LessThan(CommandType),
    Eq(CommandType),
}

#[derive(Debug)]
pub enum LogicOp {
    And,
    Or,
    Not,
    Xor,
    Shl,
    Shr,
}

#[derive(Debug)]
pub enum MemoryOp {
    Load,
    Store,
}

#[derive(Debug)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug)]
pub enum StackOp {
    Push,
    Pop,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandType {
    I16,
    I32,
    F32,
    U16,
    U32,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, TypeKind)>,
    pub symbols: Vec<(String, TypeKind)>,
    pub blocks: Vec<Vec<Inst>>,

    virt_locs: BTreeMap<usize, RegLoc>,
    phys_owners: BTreeMap<PhysReg, usize>,
    dirty_regs: HashSet<PhysReg>,
    stack_bytes: usize,
}

impl Function {
    fn new(
        name: String,
        params: Vec<(String, TypeKind)>,
        symbols: Vec<(String, TypeKind)>,
    ) -> Self {
        Self {
            name,
            params,
            symbols,
            blocks: vec![],
            virt_locs: BTreeMap::new(),
            phys_owners: BTreeMap::new(),
            dirty_regs: HashSet::new(),
            stack_bytes: 0,
        }
    }
}

#[derive(Debug)]
pub struct Backend {
    input: IrGen,
    pub functions: Vec<Function>,
    loc: (usize, usize),
    registers: Vec<ir::Register>,
}

impl Backend {
    pub fn new(input: &str, filename: &str) -> Self {
        let mut ir_gen = IrGen::new(filename, input.to_string());
        ir_gen.compile();
        let mut registers = ir_gen.registers.clone();
        registers.sort_by_key(|x| x.id);

        Backend {
            input: ir_gen,
            functions: Vec::new(),
            loc: (0, 0),
            registers,
        }
    }

    pub fn select_instructions(&mut self) {
        let input_funcs = self.input.functions.clone();
        for func_def in input_funcs {
            let mut symbols = Vec::new();
            let mut params = Vec::new();
            for sym in &func_def.symbols {
                if let Definition::Var(ty) = &sym.body {
                    if symbols.len() <= sym.id {
                        symbols.resize(sym.id + 1, ("".into(), TypeKind::Void));
                    }
                    symbols[sym.id] = (sym.name.clone(), ty.clone());
                } else if let Definition::Parameter(ty) = &sym.body {
                    if params.len() <= sym.id {
                        params.resize(sym.id + 1, ("".into(), TypeKind::Void));
                    }
                    params[sym.id] = (sym.name.clone(), ty.clone())
                }
            }

            self.functions
                .push(Function::new(func_def.name, params, symbols));
            self.loc = (self.functions.len() - 1, 0);
            for (i, block) in func_def.body.iter().enumerate() {
                self.functions[self.loc.0].blocks.push(vec![]);
                self.loc.1 = i;
                self.reset_phys_regs();
                for command in block {
                    self.process_command(command);
                }
                self.flush_registers();
            }
        }
    }

    fn process_command(&mut self, cmd: &Command) {
        match cmd {
            Command::Add(a, b, c) => self.emit_math(ArithmeticOp::Add, a, b, c),
            Command::Sub(a, b, c) => self.emit_math(ArithmeticOp::Sub, a, b, c),
            Command::Mul(a, b, c) => self.emit_math(ArithmeticOp::Mul, a, b, c),
            Command::Div(a, b, c) => self.emit_math(ArithmeticOp::Div, a, b, c),
            Command::Mod(a, b, c) => self.emit_math(ArithmeticOp::Mod, a, b, c),

            Command::And(a, b, c) => self.emit_logic(LogicOp::And, a, b, c),
            Command::Or(a, b, c) => self.emit_logic(LogicOp::Or, a, b, c),
            Command::Xor(a, b, c) => self.emit_logic(LogicOp::Xor, a, b, c),
            Command::Shl(a, b, c) => self.emit_logic(LogicOp::Shl, a, b, c),
            Command::Shr(a, b, c) => self.emit_logic(LogicOp::Shr, a, b, c),

            Command::Not(a, out) => {
                let op_a = self.resolve_operand(a);
                let rout = self.kidnap_reg(PhysReg::R1, out.id as usize);
                let ty = self.get_cmd_type(&out.ty);
                self.emit(Inst::OpCode(OpCode::Logic(LogicOp::Not, ty)));
                self.emit(op_a);
                self.emit(Inst::PhysReg(rout));
            }
            Command::Move(val, out) => {
                let src = self.resolve_operand(val);
                let dest = self.allocate_output(out);
                let ty = self.get_cmd_type(&out.ty);
                self.emit(Inst::OpCode(OpCode::Move(ty)));
                self.emit(src);
                self.emit(Inst::PhysReg(dest));
            }

            Command::Load(ptr, dest) => {
                let p_ptr = self.resolve_operand(ptr);
                let p_dest = self.allocate_output(dest);
                let ty = self.get_cmd_type(&dest.ty);
                self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Load, ty)));
                self.emit(p_ptr); // Address
                self.emit(Inst::PhysReg(p_dest)); // Destination
            }

            Command::Store(val, ptr) => {
                let v_op = self.resolve_operand(val);
                let p_op = self.resolve_operand(ptr);
                // Type comes from the value being stored
                let val_ty = self.get_val_ty(val);
                let ty = self.get_cmd_type(&val_ty);
                self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
                self.emit(v_op); // Value
                self.emit(p_op); // Address
            }
            Command::Jump(loc) => {
                self.flush_registers();
                self.emit(Inst::OpCode(OpCode::Jump));
                self.emit(Inst::Location(loc.clone()));
            }
            Command::JumpTrue(loc, cond) => {
                let op_c = self.resolve_operand(cond);
                self.flush_registers();
                self.emit(Inst::OpCode(OpCode::JumpNotZero));
                self.emit(Inst::Location(loc.clone()));
                self.emit(op_c);
            }
            Command::JumpFalse(loc, cond) => {
                let op_c = self.resolve_operand(cond);
                self.flush_registers();
                self.emit(Inst::OpCode(OpCode::JumpZero));
                self.emit(Inst::Location(loc.clone()));
                self.emit(op_c);
            }
            Command::Call(loc, count) => {
                let loc_op = self.resolve_operand(loc);
                self.emit(Inst::OpCode(OpCode::Call));
                self.emit(loc_op);
                self.emit(Inst::Immediate(Immediate {
                    value: *count as f64,
                    ty: TypeKind::Uint16,
                }));
            }
            Command::Ret(val_opt) => {
                let count = if let Some(val) = val_opt {
                    let op = self.resolve_operand(val);
                    let ty = match val {
                        Value::Register(r) => r.ty.clone(),
                        Value::Immediate(i) => i.ty.clone(),
                        _ => TypeKind::Void,
                    };
                    let cmd_ty = self.get_cmd_type(&ty);
                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, cmd_ty)));
                    self.emit(op);
                    1
                } else {
                    0
                };
                self.emit(Inst::OpCode(OpCode::Return));
                self.emit(Inst::Immediate(Immediate {
                    value: count as f64,
                    ty: TypeKind::Uint16,
                }));
                self.emit(Inst::SymbolSecLen);
                self.emit(Inst::ArgCount);
            }
            Command::Eq(a, b, c) => self.emit_cmp(OpCode::Eq(CommandType::I32), a, b, c),
            Command::Gt(a, b, c) => self.emit_cmp(OpCode::GreaterThan(CommandType::I32), a, b, c),
            Command::Lt(a, b, c) => self.emit_cmp(OpCode::LessThan(CommandType::I32), a, b, c),
            Command::Push(a) => {
                let op_a = self.resolve_operand(a);
                self.emit(Inst::OpCode(OpCode::Stack(
                    StackOp::Push,
                    self.get_cmd_type(&self.get_val_ty(a)),
                )));
                self.emit(op_a);
            }
            Command::Pop(a) => {
                let reg_a = self.allocate_output(a);
                self.emit(Inst::OpCode(OpCode::Stack(
                    StackOp::Pop,
                    self.get_cmd_type(&a.ty),
                )));
                self.emit(Inst::PhysReg(reg_a));
            }
        }
    }
    fn emit(&mut self, inst: Inst) {
        self.functions[self.loc.0].blocks[self.loc.1].push(inst);
    }

    fn emit_math(&mut self, op: ArithmeticOp, a: &Value, b: &Value, c: &Output) {
        let op_a = self.resolve_operand(a);
        let op_b = self.resolve_operand(b);
        let ty = self.get_cmd_type(&c.ty);
        let out = self.kidnap_reg(
            match ty {
                CommandType::I16 => PhysReg::R1,
                CommandType::F32 => PhysReg::F1,
                CommandType::I32 => PhysReg::EX1,
                CommandType::U16 => PhysReg::R1,
                CommandType::U32 => PhysReg::EX1,
            },
            c.id as usize,
        );
        self.emit(Inst::OpCode(OpCode::Arithmetic(op, ty)));
        self.emit(op_a);
        self.emit(op_b);
        //self.emit(Inst::PhysReg(out));
    }

    fn emit_logic(&mut self, op: LogicOp, a: &Value, b: &Value, c: &Output) {
        let op_a = self.resolve_operand(a);
        let op_b = self.resolve_operand(b);
        let ty = self.get_cmd_type(&c.ty);
        let out = self.kidnap_reg(
            match ty {
                CommandType::I16 => PhysReg::R1,
                CommandType::F32 => PhysReg::F1,
                CommandType::I32 => PhysReg::EX1,
                CommandType::U16 => PhysReg::R1,
                CommandType::U32 => PhysReg::EX1,
            },
            c.id as usize,
        );
        self.emit(Inst::OpCode(OpCode::Logic(op, ty)));
        self.emit(op_a);
        self.emit(op_b);
        //self.emit(Inst::PhysReg(out));
    }

    fn emit_cmp(&mut self, op_code: OpCode, a: &Value, b: &Value, c: &Output) {
        let op_a = self.resolve_operand(a);
        let op_b = self.resolve_operand(b);
        let reg_c = self.kidnap_reg(PhysReg::R1, c.id as usize);

        self.emit(Inst::OpCode(op_code));
        self.emit(op_a);
        self.emit(op_b);
        //self.emit(Inst::PhysReg(reg_c));
    }

    fn resolve_operand(&mut self, val: &Value) -> Inst {
        match val {
            Value::Register(r) => {
                let phys = self.ensure_reg(r.id as usize);
                Inst::PhysReg(phys)
            }
            Value::Immediate(imm) => Inst::Immediate(imm.clone()),
            Value::Location(loc) => Inst::Location(loc.clone()),
            Value::ARP => Inst::ARP,
        }
    }

    fn allocate_output(&mut self, out: &Output) -> PhysReg {
        let ty = out.ty.clone();
        let phys = self.allocate_reg(&ty, out.id as usize);
        self.functions[self.loc.0].dirty_regs.insert(phys);
        phys
    }
    fn is_ex1_alias(&self, reg: PhysReg) -> bool {
        match reg {
            PhysReg::EX1 | PhysReg::R2 | PhysReg::R3 => true,
            _ => false,
        }
    }

    fn ensure_reg(&mut self, virt_id: usize) -> PhysReg {
        let func_idx = self.loc.0;

        if let Some(RegLoc::Physical(p)) = self.functions[func_idx].virt_locs.get(&virt_id) {
            return *p;
        }

        let ty = self.registers[virt_id].ty.clone();
        let phys = self.allocate_reg(&ty, virt_id);

        if let Some(RegLoc::Stack(offset)) =
            self.functions[func_idx].virt_locs.get(&virt_id).cloned()
        {
            let cmd_ty = self.get_cmd_type(&ty);
            let needs_restore = !self.is_ex1_alias(phys);

            if needs_restore {
                self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
                self.emit(Inst::PhysReg(PhysReg::EX1));
            } else {
                self.evict_if_busy(PhysReg::EX1);
            }
            self.emit(Inst::OpCode(OpCode::Arithmetic(
                ArithmeticOp::Add,
                CommandType::U32,
            )));
            self.emit(Inst::ARP);
            self.emit(Inst::Immediate(Immediate {
                value: offset as f64,
                ty: TypeKind::Uint32,
            }));
            self.emit(Inst::PhysReg(PhysReg::EX1));
            self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Load, cmd_ty)));
            self.emit(Inst::PhysReg(PhysReg::EX1));
            self.emit(Inst::PhysReg(phys));

            if needs_restore {
                self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
                self.emit(Inst::PhysReg(PhysReg::EX1));
            }

            self.functions[func_idx].dirty_regs.remove(&phys);
        } else {
            self.functions[func_idx].dirty_regs.insert(phys);
        }

        phys
    }

    fn allocate_reg(&mut self, ty: &TypeKind, owner_id: usize) -> PhysReg {
        let candidates = match self.get_cmd_type(ty) {
            CommandType::I16 | CommandType::U16 => vec![
                PhysReg::R1,
                PhysReg::R2,
                PhysReg::R3,
                PhysReg::R4,
                PhysReg::R5,
            ],
            CommandType::I32 | CommandType::U32 => vec![PhysReg::EX1, PhysReg::EX2],
            CommandType::F32 => vec![PhysReg::F1, PhysReg::F2],
        };

        for &reg in &candidates {
            if self.is_reg_free(reg) {
                self.claim_reg(reg, owner_id);
                return reg;
            }
        }

        let victim = candidates[0];

        // Handle Aliasing evictions
        if victim == PhysReg::EX1 {
            self.evict_if_busy(PhysReg::R2);
            self.evict_if_busy(PhysReg::R3);
            self.evict_if_busy(PhysReg::EX1);
        } else if victim == PhysReg::EX2 {
            self.evict_if_busy(PhysReg::R4);
            self.evict_if_busy(PhysReg::R5);
            self.evict_if_busy(PhysReg::EX2);
        } else {
            match victim {
                PhysReg::R2 | PhysReg::R3 => self.evict_if_busy(PhysReg::EX1),
                PhysReg::R4 | PhysReg::R5 => self.evict_if_busy(PhysReg::EX2),
                _ => {}
            }
            self.evict_if_busy(victim);
        }

        self.claim_reg(victim, owner_id);
        victim
    }
    fn kidnap_reg(&mut self, victim: PhysReg, owner_id: usize) -> PhysReg {
        // Handle Aliasing evictions
        if victim == PhysReg::EX1 {
            self.evict_if_busy(PhysReg::R2);
            self.evict_if_busy(PhysReg::R3);
            self.evict_if_busy(PhysReg::EX1);
        } else if victim == PhysReg::EX2 {
            self.evict_if_busy(PhysReg::R4);
            self.evict_if_busy(PhysReg::R5);
            self.evict_if_busy(PhysReg::EX2);
        } else {
            match victim {
                PhysReg::R2 | PhysReg::R3 => self.evict_if_busy(PhysReg::EX1),
                PhysReg::R4 | PhysReg::R5 => self.evict_if_busy(PhysReg::EX2),
                _ => {}
            }
            self.evict_if_busy(victim);
        }

        self.claim_reg(victim, owner_id);
        victim
    }
    fn is_reg_free(&self, reg: PhysReg) -> bool {
        let owners = &self.functions[self.loc.0].phys_owners;
        if owners.contains_key(&reg) {
            return false;
        }

        match reg {
            PhysReg::EX1 => {
                !owners.contains_key(&PhysReg::R2) && !owners.contains_key(&PhysReg::R3)
            }
            PhysReg::EX2 => {
                !owners.contains_key(&PhysReg::R4) && !owners.contains_key(&PhysReg::R5)
            }
            PhysReg::R2 | PhysReg::R3 => !owners.contains_key(&PhysReg::EX1),
            PhysReg::R4 | PhysReg::R5 => !owners.contains_key(&PhysReg::EX2),
            _ => true,
        }
    }

    fn evict_if_busy(&mut self, reg: PhysReg) {
        let func_idx = self.loc.0;
        if let Some(&owner_id) = self.functions[func_idx].phys_owners.get(&reg) {
            let is_dirty = self.functions[func_idx].dirty_regs.contains(&reg);
            let offset = match self.functions[func_idx].virt_locs.get(&owner_id).cloned() {
                Some(RegLoc::Stack(o)) => o,
                _ => {
                    let o = self.functions[func_idx].stack_bytes;
                    self.functions[func_idx].stack_bytes += self.size_of_phys_reg(reg);
                    o
                }
            };
            if is_dirty {
                let ty = self.type_of_phys_reg(reg);

                if self.is_ex1_alias(reg) {
                    self.evict_if_busy(PhysReg::EX2);
                    self.emit(Inst::OpCode(OpCode::Move(CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                    self.emit(Inst::PhysReg(PhysReg::EX2));
                    self.emit(Inst::OpCode(OpCode::Arithmetic(
                        ArithmeticOp::Add,
                        CommandType::U32,
                    )));
                    self.emit(Inst::ARP);
                    self.emit(Inst::Immediate(Immediate {
                        value: offset as f64,
                        ty: TypeKind::Uint32,
                    }));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                    self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
                    self.emit(Inst::PhysReg(PhysReg::EX2));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                } else {
                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));

                    self.emit(Inst::OpCode(OpCode::Arithmetic(
                        ArithmeticOp::Add,
                        CommandType::U32,
                    )));
                    self.emit(Inst::ARP);
                    self.emit(Inst::Immediate(Immediate {
                        value: offset as f64,
                        ty: TypeKind::Uint32,
                    }));
                    self.emit(Inst::PhysReg(PhysReg::EX1));

                    self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
                    self.emit(Inst::PhysReg(reg));
                    self.emit(Inst::PhysReg(PhysReg::EX1));

                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                }
            }
            self.functions[func_idx]
                .virt_locs
                .insert(owner_id, RegLoc::Stack(offset));
            self.functions[func_idx].phys_owners.remove(&reg);
            self.functions[func_idx].dirty_regs.remove(&reg);
        }
    }

    fn size_of_phys_reg(&self, r: PhysReg) -> usize {
        match r {
            PhysReg::EX1 | PhysReg::EX2 | PhysReg::F1 | PhysReg::F2 => 2,
            _ => 1,
        }
    }
    fn type_of_phys_reg(&self, r: PhysReg) -> CommandType {
        match r {
            PhysReg::EX1 | PhysReg::EX2 => CommandType::I32,
            PhysReg::F1 | PhysReg::F2 => CommandType::F32,
            PhysReg::R1 | PhysReg::R2 | PhysReg::R3 | PhysReg::R4 | PhysReg::R5 => CommandType::I16,
        }
    }
    fn claim_reg(&mut self, reg: PhysReg, virt_id: usize) {
        let func = &mut self.functions[self.loc.0];
        func.phys_owners.insert(reg, virt_id);
        func.virt_locs.insert(virt_id, RegLoc::Physical(reg));
    }

    fn reset_phys_regs(&mut self) {
        self.functions[self.loc.0].phys_owners.clear();
        self.functions[self.loc.0].dirty_regs.clear();
    }

    fn flush_registers(&mut self) {
        let active: Vec<PhysReg> = self.functions[self.loc.0]
            .phys_owners
            .keys()
            .cloned()
            .collect();
        for r in active {
            self.evict_if_busy(r);
        }
    }

    fn get_val_ty(&self, v: &Value) -> TypeKind {
        match v {
            Value::ARP => TypeKind::Uint32,
            Value::Immediate(im) => im.ty.clone(),
            Value::Location(_) => TypeKind::Uint32,
            Value::Register(r) => r.ty.clone(),
        }
    }

    fn get_cmd_type(&self, ty: &TypeKind) -> CommandType {
        match ty {
            TypeKind::Int16 | TypeKind::Bool => CommandType::I16,
            TypeKind::Uint16 | TypeKind::Enum(_) | TypeKind::Char => CommandType::U16,
            TypeKind::Int32 => CommandType::I32,
            TypeKind::Uint32 | TypeKind::Pointer(_) => CommandType::U32,
            TypeKind::Float32 => CommandType::F32,
            _ => CommandType::U32,
        }
    }
    pub fn display_ir(&self) {
        for fun in &self.functions {
            println!("fn {}({:?})->{{", fun.name, fun.params);
            for (i, block) in fun.blocks.iter().enumerate() {
                println!("  block {}:", i);
                println!("  {:#?}", block);
            }
            println!("}}");
        }
    }
    pub fn emit_bytecode(&mut self) -> Executable {
        let mut exe = Executable::new();
        for fun in &self.functions {
            let mut bytefn = Fn::new_with_blocks(
                fun.name.clone(),
                fun.params.len(),
                fun.blocks
                    .iter()
                    .map(|block| {
                        block
                            .iter()
                            .map(|instr| {
                                match instr {
                                Inst::ARP => Bytecode::Register(CmdType::ARP),
                                Inst::ArgCount => Bytecode::ArgCount(),
                                Inst::SymbolSecLen => Bytecode::SymbolSectionLen(),
                                Inst::Location(loc) => match &loc {
                                    &ir::Location::Argument(name) => Bytecode::Argument(
                                        fun.params.iter().position(|x| x.0 == *name).unwrap(),
                                    ),
                                    &ir::Location::Symbol(id, offset) => {
                                        Bytecode::Symbol(fun.symbols[*id].0.clone(), *offset as i32)
                                    }
                                    &ir::Location::Block(b) => Bytecode::BlockLoc(*b as isize),
                                    &ir::Location::Function(fnid) => {
                                        Bytecode::FunctionRef(self.functions[*fnid].name.clone())
                                    }
                                    &ir::Location::None => unreachable!(),
                                },
                                Inst::Immediate(im) => match im.ty {
                                    TypeKind::Int16 => Bytecode::Int(im.value as i16),
                                    TypeKind::Int32 => Bytecode::Int32(im.value as i32),
                                    TypeKind::Bool => Bytecode::Int(im.value as i16),
                                    TypeKind::Enum(_) => Bytecode::Int(im.value as i16),
                                    TypeKind::Pointer(_) => Bytecode::Int32(im.value as i32),
                                    TypeKind::Float32 => Bytecode::Float(im.value as f32),
                                    TypeKind::Char => Bytecode::Int(im.value as i16),
                                    TypeKind::Uint16 => Bytecode::Int(im.value as i16),
                                    TypeKind::Uint32 => Bytecode::Int32(im.value as i32),
                                    _ => unreachable!(),
                                },
                                Inst::StackOffset(off) => Bytecode::Int(*off as i16),
                                Inst::PhysReg(r) => match r {
                                    PhysReg::R1 => Bytecode::Register(CmdType::R1),
                                    PhysReg::R2 => Bytecode::Register(CmdType::R2),
                                    PhysReg::R3 => Bytecode::Register(CmdType::R3),
                                    PhysReg::R4 => Bytecode::Register(CmdType::R4),
                                    PhysReg::R5 => Bytecode::Register(CmdType::R5),
                                    PhysReg::EX1 => Bytecode::Register(CmdType::EX1),
                                    PhysReg::EX2 => Bytecode::Register(CmdType::EX2),
                                    PhysReg::F1 => Bytecode::Register(CmdType::F1),
                                    PhysReg::F2 => Bytecode::Register(CmdType::F2),
                                },
                                Inst::OpCode(op) => match op {
                                    OpCode::Call => Bytecode::Command(CmdType::Call),
                                    OpCode::Return=>Bytecode::Command(CmdType::Return),
                                    OpCode::Jump=>Bytecode::Command(CmdType::Jump),
                                    OpCode::JumpNotZero=>Bytecode::Command(CmdType::JumpNotZero),
                                    OpCode::JumpZero=>Bytecode::Command(CmdType::JumpZero),
                                    OpCode::Eq(CmdType)=>Bytecode::Command(CmdType::),
                                },
                            }
                            })
                            .collect()
                    })
                    .collect::<Vec<Vec<Bytecode>>>(),
            );
            for (name, ty) in fun.symbols.iter() {
                bytefn.add_symbol(name, 0);
            }
            exe.add_fn(bytefn);
        }
        exe
    }
}
