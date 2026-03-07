use super::ir::{self, Command, Definition, Immediate, IrGen, Output, Value};
use super::lexer::TypeKind;
use crate::executable::{self, Bytecode, Executable, Fn};
use crate::vm::CommandType as CmdType;
use std::collections::{BTreeMap, HashMap, HashSet};

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
    GreaterThan,
    LessThan,
    Eq,
    Exit,
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
    pub params: Vec<(String, TypeKind, usize)>,
    pub symbols: Vec<(String, TypeKind, usize)>,
    pub blocks: Vec<Vec<Inst>>,

    virt_locs: BTreeMap<usize, RegLoc>,
    phys_owners: BTreeMap<PhysReg, usize>,
    dirty_regs: HashSet<PhysReg>,
    stack_bytes: usize,
}

impl Function {
    fn new(
        name: String,
        params: Vec<(String, TypeKind, usize)>,
        symbols: Vec<(String, TypeKind, usize)>,
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
    pub input: IrGen,
    pub functions: Vec<Function>,
    pub logs: Vec<String>,
    loc: (usize, usize),
    registers: Vec<ir::Register>,
    current_block_usage: HashMap<usize, usize>,
    current_var_scopes: HashMap<usize, HashSet<usize>>,

    // LRU Tracking
    register_lru: HashMap<PhysReg, usize>,
    time_step: usize,
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
            logs: Vec::new(),
            loc: (0, 0),
            registers,
            current_block_usage: HashMap::new(),
            current_var_scopes: HashMap::new(),
            register_lru: HashMap::new(),
            time_step: 0,
        }
    }

    pub fn select_instructions(&mut self) {
        let input_funcs = self.input.functions.clone();
        for func_def in input_funcs {
            self.current_var_scopes.clear();
            for (b_idx, block) in func_def.body.iter().enumerate() {
                for cmd in block {
                    Self::collect_reads(cmd, |id| {
                        self.current_var_scopes.entry(id).or_default().insert(b_idx);
                    });
                }
            }

            let mut symbols = Vec::new();
            let mut params = Vec::new();
            for sym in &func_def.symbols {
                if let Definition::Var(ty) = &sym.body {
                    if symbols.len() <= sym.id {
                        symbols.resize(sym.id + 1, ("".into(), TypeKind::Void, 0));
                    }
                    symbols[sym.id] = (sym.name.clone(), ty.clone(), sym.size.unwrap());
                } else if let Definition::Parameter(ty) = &sym.body {
                    if params.len() <= sym.id {
                        params.resize(sym.id + 1, ("".into(), TypeKind::Void, 0));
                    }
                    params[sym.id] = (sym.name.clone(), ty.clone(), sym.size.unwrap())
                }
            }

            self.functions
                .push(Function::new(func_def.name, params, symbols));
            self.loc = (self.functions.len() - 1, 0);

            for (i, block) in func_def.body.iter().enumerate() {
                self.functions[self.loc.0].blocks.push(vec![]);
                self.loc.1 = i;
                self.reset_phys_regs();
                self.current_block_usage.clear();
                self.register_lru.clear();
                self.time_step = 0;

                for cmd in block {
                    Self::collect_reads(cmd, |id| {
                        *self.current_block_usage.entry(id).or_default() += 1;
                    });
                }

                for command in block {
                    self.process_command(command);
                }
                self.flush_registers();
            }
        }
    }

    fn touch_reg(&mut self, reg: PhysReg) {
        self.time_step += 1;
        self.register_lru.insert(reg, self.time_step);
        match reg {
            PhysReg::EX1 => {
                self.register_lru.insert(PhysReg::R2, self.time_step);
                self.register_lru.insert(PhysReg::R3, self.time_step);
            }
            PhysReg::EX2 => {
                self.register_lru.insert(PhysReg::R4, self.time_step);
                self.register_lru.insert(PhysReg::R5, self.time_step);
            }
            PhysReg::R2 | PhysReg::R3 => {
                self.register_lru.insert(PhysReg::EX1, self.time_step);
            }
            PhysReg::R4 | PhysReg::R5 => {
                self.register_lru.insert(PhysReg::EX2, self.time_step);
            }
            _ => {}
        }
    }

    fn collect_reads<F>(cmd: &Command, mut callback: F)
    where
        F: FnMut(usize),
    {
        let mut check_val = |v: &Value| {
            if let Value::Register(r) = v {
                callback(r.id as usize);
            }
        };

        match cmd {
            Command::Add(a, b, _)
            | Command::Sub(a, b, _)
            | Command::Mul(a, b, _)
            | Command::Div(a, b, _)
            | Command::Mod(a, b, _)
            | Command::And(a, b, _)
            | Command::Or(a, b, _)
            | Command::Xor(a, b, _)
            | Command::Shl(a, b, _)
            | Command::Shr(a, b, _)
            | Command::Eq(a, b, _)
            | Command::Gt(a, b, _)
            | Command::Lt(a, b, _) => {
                check_val(a);
                check_val(b);
            }
            Command::Not(a, _) | Command::Move(a, _) | Command::Push(a) | Command::Load(a, _) => {
                check_val(a);
            }
            Command::Store(val, ptr) => {
                check_val(val);
                check_val(ptr);
            }
            Command::JumpTrue(_, cond) | Command::JumpFalse(_, cond) => {
                check_val(cond);
            }
            Command::Call(loc, _) => {
                check_val(loc);
            }
            Command::Ret(Some(val)) => {
                check_val(val);
            }
            Command::Pop(_) | Command::Jump(_) | Command::Ret(None) => {}
        }
    }

    fn decrement_usage(&mut self, virt_id: usize) {
        let is_zero = if let Some(count) = self.current_block_usage.get_mut(&virt_id) {
            *count -= 1;
            *count == 0
        } else {
            false
        };
        if is_zero {
            let is_local = self
                .current_var_scopes
                .get(&virt_id)
                .map(|s| s.len() == 1)
                .unwrap_or(false);
            if is_local {
                self.release_reg(virt_id);
            }
        }
    }

    fn release_reg(&mut self, virt_id: usize) {
        let func_idx = self.loc.0;
        if let Some(RegLoc::Physical(p)) = self.functions[func_idx].virt_locs.get(&virt_id).cloned()
        {
            if let Some(owner) = self.functions[func_idx].phys_owners.get(&p) {
                if *owner == virt_id {
                    self.functions[func_idx].phys_owners.remove(&p);
                }
            }
            self.functions[func_idx].dirty_regs.remove(&p);
            self.functions[func_idx].virt_locs.remove(&virt_id);
        }
    }

    fn resolve_and_free(&mut self, val: &Value) -> Inst {
        let inst = self.resolve_operand(val);
        if let Value::Register(r) = val {
            self.decrement_usage(r.id as usize);
        }
        inst
    }

    fn resolve_keep(&mut self, val: &Value) -> Inst {
        self.resolve_operand(val)
    }

    fn free_op(&mut self, val: &Value) {
        if let Value::Register(r) = val {
            self.decrement_usage(r.id as usize);
        }
    }

    fn log(&mut self, msg: String) {
        self.logs.push(msg);
    }

    fn validate_operand(&mut self, inst: Inst, source_val: &Value) -> Inst {
        if let Inst::PhysReg(reg) = inst {
            if let Value::Register(virt) = source_val {
                let func_idx = self.loc.0;
                let current_loc = self.functions[func_idx].virt_locs.get(&(virt.id as usize));

                match current_loc {
                    Some(RegLoc::Stack(_)) => return self.resolve_operand(source_val),
                    Some(RegLoc::Physical(p)) => {
                        if *p != reg {
                            return Inst::PhysReg(*p);
                        }
                    }
                    _ => return self.resolve_operand(source_val),
                }
            }
        }
        inst
    }

    fn process_command(&mut self, cmd: &Command) {
        self.log(format!("Processing Command: {:?}", cmd));
        let l = self.functions[self.loc.0].blocks[self.loc.1].len();
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
                let op_a = self.resolve_keep(a);
                let ty = self.get_cmd_type(&out.ty);
                let target = match ty {
                    CommandType::I32 | CommandType::U32 => PhysReg::EX1,
                    _ => PhysReg::R1,
                };
                if !self.is_reg_free(target) {
                    self.spill(target);
                }
                let valid_a = self.validate_operand(op_a, a);
                self.claim_reg(target, out.id as usize);
                self.emit(Inst::OpCode(OpCode::Logic(LogicOp::Not, ty)));
                self.emit(valid_a);
                self.free_op(a);
            }
            Command::Move(val, out) => {
                let src = self.resolve_keep(val);
                let dest = self.allocate_output(out);
                let ty = self.get_cmd_type(&out.ty);
                let valid_src = self.validate_operand(src, val);

                self.emit(Inst::OpCode(OpCode::Move(ty)));
                self.emit(valid_src);
                self.emit(Inst::PhysReg(dest));
                self.free_op(val);
            }
            Command::Load(ptr, dest) => {
                let p_ptr = self.resolve_keep(ptr);
                let p_dest = self.allocate_output(dest);
                let valid_ptr = self.validate_operand(p_ptr, ptr);
                let ty = self.get_cmd_type(&dest.ty);
                self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Load, ty)));
                self.emit(valid_ptr);
                self.emit(Inst::PhysReg(p_dest));
                self.free_op(ptr);
            }
            Command::Store(val, ptr) => {
                let val_ty = self.get_val_ty(val);
                let ty = self.get_cmd_type(&val_ty);

                let v_op = self.resolve_keep(val);
                // ensure_reg() uses EX1 scratch. If v_op in EX1, EX1 protection must run.
                let p_op = self.resolve_keep(ptr);

                let valid_v = self.validate_operand(v_op, val);
                let valid_p = self.validate_operand(p_op, ptr);

                self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
                self.emit(valid_p); // Address
                self.emit(valid_v); // Value

                self.free_op(val);
                self.free_op(ptr);
            }
            Command::Jump(loc) => {
                self.flush_registers();
                self.emit(Inst::OpCode(OpCode::Jump));
                self.emit(Inst::Location(loc.clone()));
            }
            Command::JumpTrue(loc, cond) => {
                let op_c = self.resolve_and_free(cond);
                self.flush_registers();
                self.emit(Inst::OpCode(OpCode::JumpNotZero));
                self.emit(Inst::Location(loc.clone()));
                self.emit(op_c);
            }
            Command::JumpFalse(loc, cond) => {
                let op_c = self.resolve_and_free(cond);
                self.flush_registers();
                self.emit(Inst::OpCode(OpCode::JumpZero));
                self.emit(Inst::Location(loc.clone()));
                self.emit(op_c);
            }
            Command::Call(loc, _) => {
                self.flush_registers();
                let loc_op = self.resolve_and_free(loc);
                self.emit(Inst::OpCode(OpCode::Call));
                self.emit(loc_op);
            }
            Command::Ret(val_opt) => {
                if self.functions[self.loc.0].name == "main" {
                    self.emit(Inst::OpCode(OpCode::Exit));
                } else {
                    let count = if let Some(val) = val_opt {
                        let ty = match val {
                            Value::Register(r) => r.ty.clone(),
                            Value::Immediate(i) => i.ty.clone(),
                            _ => TypeKind::Void,
                        };
                        let op = self.resolve_and_free(val);
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
            }
            Command::Eq(a, b, c) => self.emit_cmp(OpCode::Eq, a, b, c),
            Command::Gt(a, b, c) => self.emit_cmp(OpCode::GreaterThan, a, b, c),
            Command::Lt(a, b, c) => self.emit_cmp(OpCode::LessThan, a, b, c),
            Command::Push(a) => {
                let ty = self.get_cmd_type(&self.get_val_ty(a));
                let op_a = self.resolve_and_free(a);
                self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, ty)));
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
        self.log(format!(
            "Emitted: {:?}",
            &self.functions[self.loc.0].blocks[self.loc.1][l..]
        ));
    }

    fn emit(&mut self, inst: Inst) {
        self.functions[self.loc.0].blocks[self.loc.1].push(inst);
    }

    fn check_and_reload(
        &mut self,
        operand: Inst,
        original_val: &Value,
        spilled_reg: PhysReg,
    ) -> Inst {
        if let Inst::PhysReg(r) = operand {
            if r == spilled_reg || (self.is_ex1_alias(spilled_reg) && self.is_ex1_alias(r)) {
                return self.resolve_operand(original_val);
            }
        }
        operand
    }

    fn emit_math(&mut self, op: ArithmeticOp, a: &Value, b: &Value, c: &Output) {
        let op_a = self.resolve_keep(a);
        let op_b = self.resolve_keep(b);
        let ty = self.get_cmd_type(&c.ty);

        let hardcoded_dest = match ty {
            CommandType::I16 | CommandType::U16 => PhysReg::R1,
            CommandType::F32 => PhysReg::F1,
            CommandType::I32 | CommandType::U32 => PhysReg::EX1,
        };

        if !self.is_reg_free(hardcoded_dest) {
            self.spill(hardcoded_dest);
        }

        let valid_a = self.validate_operand(op_a, a);
        let valid_b = self.validate_operand(op_b, b);

        self.claim_reg(hardcoded_dest, c.id as usize);
        self.functions[self.loc.0].dirty_regs.insert(hardcoded_dest);

        self.emit(Inst::OpCode(OpCode::Arithmetic(op, ty)));
        self.emit(valid_a);
        self.emit(valid_b);

        self.free_op(a);
        self.free_op(b);
    }

    fn emit_logic(&mut self, op: LogicOp, a: &Value, b: &Value, c: &Output) {
        let op_a = self.resolve_keep(a);
        let op_b = self.resolve_keep(b);
        let ty = self.get_cmd_type(&c.ty);

        let hardcoded_dest = match ty {
            CommandType::I32 | CommandType::U32 => PhysReg::EX1,
            _ => PhysReg::R1,
        };

        if !self.is_reg_free(hardcoded_dest) {
            self.spill(hardcoded_dest);
        }

        let valid_a = self.validate_operand(op_a, a);
        let valid_b = self.validate_operand(op_b, b);

        self.claim_reg(hardcoded_dest, c.id as usize);
        self.functions[self.loc.0].dirty_regs.insert(hardcoded_dest);

        self.emit(Inst::OpCode(OpCode::Logic(op, ty)));
        self.emit(valid_a);
        self.emit(valid_b);

        self.free_op(a);
        self.free_op(b);
    }

    fn emit_cmp(&mut self, op_code: OpCode, a: &Value, b: &Value, c: &Output) {
        let op_a = self.resolve_keep(a);
        let op_b = self.resolve_keep(b);

        if !self.is_reg_free(PhysReg::R1) {
            self.spill(PhysReg::R1);
        }

        let valid_a = self.validate_operand(op_a, a);
        let valid_b = self.validate_operand(op_b, b);

        self.claim_reg(PhysReg::R1, c.id as usize);
        self.functions[self.loc.0].dirty_regs.insert(PhysReg::R1);

        self.emit(Inst::OpCode(op_code));
        self.emit(valid_a);
        self.emit(valid_b);

        self.free_op(a);
        self.free_op(b);
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
        if let Some(rloc_ptr) = self.functions[func_idx].virt_locs.get(&virt_id) {
            let rloc = rloc_ptr.clone();
            if let RegLoc::Physical(p) = rloc {
                let phys = p.clone();
                self.touch_reg(phys);
                return phys;
            }

            let ty = self.registers[virt_id].ty.clone();
            let phys = self.allocate_reg(&ty, virt_id);

            self.log(format!("RegLoc of vreg{}: {:?}", virt_id, rloc));
            if let RegLoc::Stack(offset) = rloc {
                self.log(format!(
                    "Reloading vreg{} from stack offset {} into {:?}",
                    virt_id, offset, phys
                ));
                let cmd_ty = self.get_cmd_type(&ty);

                // IMPORTANT: Removed redundant "spill aliases" loop here.
                // allocate_reg has already guaranteed 'phys' is free to use.
                // We only need to check EX1 for scratchpad safety.

                let ex1_busy = !self.is_reg_free(PhysReg::EX1) && phys != PhysReg::EX1;

                if ex1_busy {
                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                }

                self.emit(Inst::OpCode(OpCode::Arithmetic(
                    ArithmeticOp::Add,
                    CommandType::I32,
                )));
                self.emit(Inst::ARP);
                self.emit(Inst::StackOffset(offset));
                self.emit(Inst::OpCode(OpCode::Arithmetic(
                    ArithmeticOp::Add,
                    CommandType::I32,
                )));
                self.emit(Inst::PhysReg(PhysReg::EX1));
                self.emit(Inst::SymbolSecLen);

                self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Load, cmd_ty)));
                self.emit(Inst::PhysReg(PhysReg::EX1));
                self.emit(Inst::PhysReg(phys));

                if ex1_busy {
                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                }

                self.functions[func_idx].dirty_regs.remove(&phys);
            }
            phys
        } else {
            let ty = self.registers[virt_id].ty.clone();
            let phys = self.allocate_reg(&ty, virt_id);
            self.functions[func_idx].dirty_regs.insert(phys);
            phys
        }
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

        let victim = *candidates
            .iter()
            .min_by_key(|r| self.register_lru.get(r).unwrap_or(&0))
            .unwrap_or(&candidates[0]);
        self.spill(victim);
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

    fn spill(&mut self, reg: PhysReg) {
        let func_idx = self.loc.0;
        let owner_opt = self.functions[func_idx].phys_owners.get(&reg).cloned();

        if let Some(owner_id) = owner_opt {
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
                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX2));
                    self.emit(Inst::OpCode(OpCode::Move(ty)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                    self.emit(Inst::PhysReg(PhysReg::EX2));
                    self.emit(Inst::OpCode(OpCode::Arithmetic(
                        ArithmeticOp::Add,
                        CommandType::I32,
                    )));
                    self.emit(Inst::ARP);
                    self.emit(Inst::StackOffset(offset));
                    self.emit(Inst::OpCode(OpCode::Arithmetic(
                        ArithmeticOp::Add,
                        CommandType::I32,
                    )));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                    self.emit(Inst::SymbolSecLen);
                    let temp_reg = match reg {
                        PhysReg::EX1 => PhysReg::EX2,
                        PhysReg::R2 => PhysReg::R4,
                        PhysReg::R3 => PhysReg::R5,
                        _ => unreachable!(),
                    };
                    self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
                    self.emit(Inst::PhysReg(PhysReg::EX1)); // Address
                    self.emit(Inst::PhysReg(temp_reg)); // Value
                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX2));
                } else {
                    let ex1_busy = !self.is_reg_free(PhysReg::EX1);
                    if ex1_busy {
                        self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
                        self.emit(Inst::PhysReg(PhysReg::EX1));
                    }

                    self.emit(Inst::OpCode(OpCode::Arithmetic(
                        ArithmeticOp::Add,
                        CommandType::I32,
                    )));
                    self.emit(Inst::ARP);
                    self.emit(Inst::StackOffset(offset));
                    self.emit(Inst::OpCode(OpCode::Arithmetic(
                        ArithmeticOp::Add,
                        CommandType::I32,
                    )));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                    self.emit(Inst::SymbolSecLen);
                    self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                    self.emit(Inst::PhysReg(reg));

                    if ex1_busy {
                        self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
                        self.emit(Inst::PhysReg(PhysReg::EX1));
                    }
                }
            }
            self.log(format!(
                "Spilling vreg{} into stack offset {} from phys {:?}",
                owner_id, offset, reg
            ));
            self.functions[func_idx]
                .virt_locs
                .insert(owner_id, RegLoc::Stack(offset));
            self.functions[func_idx].phys_owners.remove(&reg);
            self.functions[func_idx].dirty_regs.remove(&reg);
        } else {
            // Recursive spill for aliases only
            if reg == PhysReg::EX1 {
                self.spill(PhysReg::R2);
                self.spill(PhysReg::R3);
            } else if reg == PhysReg::EX2 {
                self.spill(PhysReg::R4);
                self.spill(PhysReg::R5);
            } else if reg == PhysReg::R2 || reg == PhysReg::R3 {
                self.spill(PhysReg::EX1);
            } else if reg == PhysReg::R4 || reg == PhysReg::R5 {
                self.spill(PhysReg::EX2);
            }
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
            _ => CommandType::I16,
        }
    }

    fn claim_reg(&mut self, reg: PhysReg, virt_id: usize) {
        let func = &mut self.functions[self.loc.0];
        func.phys_owners.insert(reg, virt_id);
        func.virt_locs.insert(virt_id, RegLoc::Physical(reg));
        self.touch_reg(reg);
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
            self.spill(r);
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
                            .map(|instr| match instr {
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
                                    TypeKind::Float32 => Bytecode::Float(im.value as f32),
                                    TypeKind::Int32 | TypeKind::Uint32 | TypeKind::Pointer(_) => {
                                        Bytecode::Int32(im.value as i32)
                                    }
                                    _ => Bytecode::Int(im.value as i16),
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
                                    OpCode::Exit => Bytecode::Command(CmdType::Exit),
                                    OpCode::Call => Bytecode::Command(CmdType::Call),
                                    OpCode::Return => Bytecode::Command(CmdType::Return),
                                    OpCode::Jump => Bytecode::Command(CmdType::Jump),
                                    OpCode::JumpNotZero => Bytecode::Command(CmdType::JumpNotZero),
                                    OpCode::JumpZero => Bytecode::Command(CmdType::JumpZero),
                                    OpCode::Eq => Bytecode::Command(CmdType::Equals),
                                    OpCode::GreaterThan => Bytecode::Command(CmdType::Greater),
                                    OpCode::LessThan => Bytecode::Command(CmdType::LessThan),
                                    OpCode::Move(_) => Bytecode::Command(CmdType::Mov),
                                    OpCode::Stack(sop, ty) => Bytecode::Command(match sop {
                                        StackOp::Push => match ty {
                                            CommandType::F32 => CmdType::Pushf,
                                            CommandType::I16 | CommandType::U16 => CmdType::Push,
                                            CommandType::I32 | CommandType::U32 => CmdType::PushEx,
                                        },
                                        StackOp::Pop => CmdType::Pop,
                                    }),
                                    OpCode::Memory(mop, ty) => Bytecode::Command(match mop {
                                        MemoryOp::Load => match ty {
                                            CommandType::F32 => CmdType::Loadf,
                                            CommandType::I32 | CommandType::U32 => CmdType::LoadEx,
                                            _ => CmdType::Load,
                                        },
                                        MemoryOp::Store => match ty {
                                            CommandType::F32 => CmdType::Storef,
                                            CommandType::I32 | CommandType::U32 => CmdType::StoreEx,
                                            _ => CmdType::Store,
                                        },
                                    }),
                                    OpCode::Arithmetic(aop, ty) => Bytecode::Command(match aop {
                                        ArithmeticOp::Add => match ty {
                                            CommandType::I16 => CmdType::Add,
                                            CommandType::U16 => CmdType::AddU,
                                            CommandType::I32 => CmdType::AddEx,
                                            CommandType::U32 => CmdType::AddExU,
                                            CommandType::F32 => CmdType::Addf,
                                        },
                                        ArithmeticOp::Sub => match ty {
                                            CommandType::I16 => CmdType::Sub,
                                            CommandType::U16 => CmdType::SubU,
                                            CommandType::I32 => CmdType::SubEx,
                                            CommandType::U32 => CmdType::SubExU,
                                            CommandType::F32 => CmdType::Subf,
                                        },
                                        ArithmeticOp::Mul => match ty {
                                            CommandType::I16 => CmdType::Mul,
                                            CommandType::U16 => CmdType::MulU,
                                            CommandType::I32 => CmdType::MulEx,
                                            CommandType::U32 => CmdType::MulExU,
                                            CommandType::F32 => CmdType::Mulf,
                                        },
                                        ArithmeticOp::Div => match ty {
                                            CommandType::I16 => CmdType::Div,
                                            CommandType::U16 => CmdType::DivU,
                                            CommandType::I32 => CmdType::DivEx,
                                            CommandType::U32 => CmdType::DivExU,
                                            CommandType::F32 => CmdType::Divf,
                                        },
                                        ArithmeticOp::Mod => CmdType::Mod,
                                    }),
                                    OpCode::Logic(lop, ty) => Bytecode::Command(match lop {
                                        LogicOp::And => match ty {
                                            CommandType::I32
                                            | CommandType::U32
                                            | CommandType::F32 => CmdType::AndEx,
                                            _ => CmdType::And,
                                        },
                                        LogicOp::Or => match ty {
                                            CommandType::I32
                                            | CommandType::U32
                                            | CommandType::F32 => CmdType::OrEx,
                                            _ => CmdType::Or,
                                        },
                                        LogicOp::Xor => match ty {
                                            CommandType::I32
                                            | CommandType::U32
                                            | CommandType::F32 => CmdType::XorEx,
                                            _ => CmdType::Xor,
                                        },
                                        LogicOp::Not => match ty {
                                            CommandType::I32
                                            | CommandType::U32
                                            | CommandType::F32 => CmdType::NotEx,
                                            _ => CmdType::Not,
                                        },
                                        LogicOp::Shl => match ty {
                                            CommandType::I32
                                            | CommandType::U32
                                            | CommandType::F32 => CmdType::ShlEx,
                                            _ => CmdType::Shl,
                                        },
                                        LogicOp::Shr => match ty {
                                            CommandType::I32
                                            | CommandType::U32
                                            | CommandType::F32 => CmdType::ShrEx,
                                            _ => CmdType::Shr,
                                        },
                                    }),
                                },
                            })
                            .collect()
                    })
                    .collect::<Vec<Vec<Bytecode>>>(),
            );
            for (name, _, size) in fun.symbols.iter() {
                bytefn.add_symbol(name, *size);
            }
            bytefn.add_symbol(
                "__internal_reg_save_463653961935601537679876958223",
                fun.stack_bytes,
            );
            exe.add_fn(bytefn);
        }
        exe
    }
}
