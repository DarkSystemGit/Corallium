use super::ir::{
    self, Command, Definition, Immediate, ImplicitParam, ImplicitParamType, IrGen, Output, Value,
};
use super::lexer::TypeKind;
use crate::executable::{Bytecode, Executable, Fn, Library};
use crate::vm::CommandType as CmdType;
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
//DISCLAIMER:
//The register allocation code in this file is mostly ai-generated(gemini&claude), due to my lack of knowlage on the subject.
// I regert using AI, as it prob mad me spend more time than I need to debugging, but oh well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhysReg {
    R1,
    R2,
    R3,
    R4,
    R5,
    EX1,
    EX2,
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
    PhysReg(PhysReg),
    Immediate(Immediate),
    Location(ir::Location),
    StackOffset(usize),
    ARP,
    SymbolSecLen,
    ArgCount,
    RestoreOffset,
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

// ---------------------------------------------------------------------------
// Function / Backend structures
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, TypeKind, usize)>,
    pub symbols: Vec<(String, TypeKind, usize)>,
    pub blocks: Vec<Vec<Inst>>,
    virt_locs: IndexMap<usize, RegLoc>,
    phys_owners: IndexMap<PhysReg, usize>,
    dirty_regs: HashSet<PhysReg>,
    stack_bytes: usize,
    stack_home: HashMap<usize, usize>,
    permanent_stack_slots: HashMap<usize, usize>,
    pub sret: bool,
    pub compile: bool,
}

impl Function {
    fn new(
        name: String,
        params: Vec<(String, TypeKind, usize)>,
        symbols: Vec<(String, TypeKind, usize)>,
        sret: bool,
        compile: bool,
    ) -> Self {
        Self {
            name,
            params,
            symbols,
            blocks: Vec::new(),
            virt_locs: IndexMap::new(),
            phys_owners: IndexMap::new(),
            dirty_regs: HashSet::new(),
            stack_bytes: 0,
            stack_home: HashMap::new(),
            permanent_stack_slots: HashMap::new(),
            sret,
            compile,
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
    def_blocks: HashMap<usize, usize>,

    register_lru: HashMap<PhysReg, usize>,
    time_step: usize,

    next_use_map: HashMap<usize, Vec<usize>>,
    current_cmd_idx: usize,
}

// ---------------------------------------------------------------------------
// Backend — public interface
// ---------------------------------------------------------------------------

impl Backend {
    fn alias_group(reg: PhysReg) -> &'static [PhysReg] {
        match reg {
            PhysReg::EX1 | PhysReg::R2 | PhysReg::R3 => &[PhysReg::EX1, PhysReg::R2, PhysReg::R3],
            PhysReg::EX2 | PhysReg::R4 | PhysReg::R5 => &[PhysReg::EX2, PhysReg::R4, PhysReg::R5],
            _ => &[PhysReg::R1], // placeholder, callers handle non-aliased regs directly
        }
    }

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
            def_blocks: HashMap::new(),
            register_lru: HashMap::new(),
            time_step: 0,
            next_use_map: HashMap::new(),
            current_cmd_idx: 0,
        }
    }

    pub fn select_instructions(&mut self) {
        let input_funcs = self.input.functions.clone();
        for func_def in input_funcs {
            self.current_var_scopes.clear();
            self.def_blocks.clear();

            for (b_idx, block) in func_def.body.iter().enumerate() {
                for cmd in block {
                    Self::collect_reads(cmd, |id| {
                        self.current_var_scopes.entry(id).or_default().insert(b_idx);
                    });
                    Self::collect_writes(cmd, |id| {
                        self.def_blocks.insert(id, b_idx);
                    });
                }
            }

            let mut symbols: Vec<(String, TypeKind, usize)> = Vec::new();
            let mut params: Vec<(String, TypeKind, usize)> = func_def
                .implict_params
                .iter()
                .map(|x| (x.name.clone().unwrap(), x.ty.clone(), 2))
                .collect();
            for sym in &func_def.symbols {
                let slot = match &sym.body {
                    Definition::Var(_) => {
                        symbols.resize(sym.id + 1, ("".into(), TypeKind::Void, 0));
                        &mut symbols[sym.id]
                    }
                    Definition::Parameter(_) => {
                        let id = sym.id + func_def.implict_params.len();
                        params.resize(id + 1, ("".into(), TypeKind::Void, 0));
                        &mut params[id]
                    }
                    _ => continue,
                };
                let ty = match &sym.body {
                    Definition::Var(t) | Definition::Parameter(t) => t.clone(),
                    _ => unreachable!(),
                };
                *slot = (sym.name.clone(), ty, sym.size.unwrap());
            }

            self.log(format!("Emitting fn {}:", func_def.name));
            self.log(format!("Params: {:?}", params));
            self.log(format!("Symbols: {:?}", symbols));
            self.functions.push(Function::new(
                func_def.name,
                params,
                symbols,
                func_def
                    .implict_params
                    .iter()
                    .filter(|x| x.param_ty == ImplicitParamType::ReturnPassthorugh)
                    .collect::<Vec<&ImplicitParam>>()
                    .len()
                    > 0,
                func_def.compile,
            ));
            self.loc = (self.functions.len() - 1, 0);

            for (i, block) in func_def.body.iter().enumerate() {
                self.log(format!("Emitting block {}:", i));
                self.functions[self.loc.0].blocks.push(vec![]);
                self.loc.1 = i;
                self.reset_phys_regs();
                self.current_block_usage.clear();
                self.register_lru.clear();
                self.time_step = 0;

                self.next_use_map = Self::build_next_use(block);
                self.current_cmd_idx = 0;

                for cmd in block {
                    Self::collect_reads(cmd, |id| {
                        *self.current_block_usage.entry(id).or_default() += 1;
                    });
                }
                if func_def.compile {
                    for command in block {
                        self.process_command(command);
                        self.current_cmd_idx += 1;
                    }
                }
                self.flush_registers();
            }
        }
    }

    pub fn emit_bytecode(&mut self) -> Vec<Fn> {
        let mut fns = vec![];
        for fun in &self.functions {
            if fun.compile {
                let blocks = fun
                    .blocks
                    .iter()
                    .map(|block| {
                        block
                            .iter()
                            .map(|inst| Self::lower_inst(inst, fun, &self.functions))
                            .collect()
                    })
                    .collect::<Vec<Vec<Bytecode>>>();

                let mut bytefn = Fn::new_with_blocks(
                    fun.name.clone(),
                    fun.params.iter().map(|x| x.2).collect(),
                    blocks,
                );
                for (name, _, size) in &fun.symbols {
                    bytefn.add_symbol(name, *size);
                }
                bytefn.add_symbol(
                    "__internal_reg_save_463653961935601537679876958223",
                    fun.stack_bytes,
                );
                fns.push(bytefn);
            }
        }
        fns
    }
}

// ---------------------------------------------------------------------------
// Next-use & Liveness helpers
// ---------------------------------------------------------------------------

impl Backend {
    fn is_used_in_current_cmd(&self, virt_id: usize) -> bool {
        self.next_use_map.get(&virt_id).map_or(false, |uses| {
            uses.iter().any(|&u| u == self.current_cmd_idx)
        })
    }

    fn build_next_use(block: &[Command]) -> HashMap<usize, Vec<usize>> {
        let mut map: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, cmd) in block.iter().enumerate() {
            Self::collect_reads(cmd, |id| {
                map.entry(id).or_default().push(i);
            });
        }
        map
    }

    fn next_use_distance(&self, virt_id: usize) -> usize {
        let after = self.current_cmd_idx;
        self.next_use_map
            .get(&virt_id)
            .and_then(|uses| uses.iter().find(|&&u| u > after).copied())
            .unwrap_or(usize::MAX)
    }

    fn phys_next_use_distance(&self, reg: PhysReg) -> usize {
        let func_idx = self.loc.0;
        let get_dist = |r| {
            if let Some(&owner) = self.functions[func_idx].phys_owners.get(&r) {
                self.next_use_distance(owner)
            } else {
                usize::MAX
            }
        };

        let mut dist = get_dist(reg);
        match reg {
            PhysReg::EX1 => {
                dist = dist.min(get_dist(PhysReg::R2)).min(get_dist(PhysReg::R3));
            }
            PhysReg::EX2 => {
                dist = dist.min(get_dist(PhysReg::R4)).min(get_dist(PhysReg::R5));
            }
            PhysReg::R2 | PhysReg::R3 => dist = dist.min(get_dist(PhysReg::EX1)),
            PhysReg::R4 | PhysReg::R5 => dist = dist.min(get_dist(PhysReg::EX2)),
            _ => {}
        }
        dist
    }

    fn is_strictly_local(&self, virt_id: usize) -> bool {
        let b_idx = self.loc.1;
        let defined_here = self.def_blocks.get(&virt_id) == Some(&b_idx);
        let only_used_here = self
            .current_var_scopes
            .get(&virt_id)
            .map_or(true, |s| s.len() == 1 && s.contains(&b_idx));
        defined_here && only_used_here
    }

    fn can_clobber(&self, reg: PhysReg) -> bool {
        let func_idx = self.loc.0;
        let check = |r: PhysReg| -> bool {
            if let Some(&owner) = self.functions[func_idx].phys_owners.get(&r) {
                self.next_use_distance(owner) == usize::MAX
                    && self.is_strictly_local(owner)
                    && !self.is_used_in_current_cmd(owner)
            } else {
                true
            }
        };
        match reg {
            PhysReg::EX1 => check(PhysReg::EX1) && check(PhysReg::R2) && check(PhysReg::R3),
            PhysReg::EX2 => check(PhysReg::EX2) && check(PhysReg::R4) && check(PhysReg::R5),
            PhysReg::R2 | PhysReg::R3 => check(reg) && check(PhysReg::EX1),
            PhysReg::R4 | PhysReg::R5 => check(reg) && check(PhysReg::EX2),
            _ => check(reg),
        }
    }
}

// ---------------------------------------------------------------------------
// Instruction selection
// ---------------------------------------------------------------------------

impl Backend {
    fn process_command(&mut self, cmd: &Command) {
        self.log(format!("Processing Command: {:?}", cmd));
        let block_start = self.functions[self.loc.0].blocks[self.loc.1].len();

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

            Command::Eq(a, b, c) => self.emit_cmp(OpCode::Eq, a, b, c),
            Command::Gt(a, b, c) => self.emit_cmp(OpCode::GreaterThan, a, b, c),
            Command::Lt(a, b, c) => self.emit_cmp(OpCode::LessThan, a, b, c),

            Command::Not(a, out) => {
                let op_a = self.resolve_operand(a);
                let ty = self.get_cmd_type(&out.ty);
                let dest = if matches!(ty, CommandType::I32 | CommandType::U32) {
                    PhysReg::EX1
                } else {
                    PhysReg::R1
                };
                if !self.is_reg_free(dest) && !self.can_clobber(dest) {
                    self.spill(dest);
                }
                let valid_a = self.validate_operand(op_a, a);
                self.claim_reg(dest, out.id as usize);
                self.mark_phys_dirty(dest);
                self.emit(Inst::OpCode(OpCode::Logic(LogicOp::Not, ty)));
                self.emit(valid_a);
                self.free_op(a);
            }

            Command::Move(val, out) => {
                let mut coalesced = false;

                // Move Coalescing (Eliminate redundant moves in favor of register renaming)
                if let Value::Register(r) = val {
                    let r_id = r.id as usize;
                    let out_id = out.id as usize;

                    if self.next_use_distance(r_id) == usize::MAX
                        && self.is_strictly_local(r_id)
                        && self.get_cmd_type(&r.ty) == self.get_cmd_type(&out.ty)
                    {
                        let phys = self.ensure_reg(r_id);
                        self.functions[self.loc.0].phys_owners.remove(&phys);
                        self.functions[self.loc.0].virt_locs.remove(&r_id);
                        self.functions[self.loc.0].dirty_regs.remove(&phys);

                        self.claim_reg(phys, out_id);
                        self.mark_phys_dirty(phys);

                        self.free_op(val);
                        coalesced = true;
                    }
                }

                if !coalesced {
                    let src = self.resolve_operand(val);
                    let dest = self.allocate_output(out);
                    let ty = self.get_cmd_type(&out.ty);
                    let valid_src = self.validate_operand(src, val);

                    self.emit(Inst::OpCode(OpCode::Move(ty)));
                    self.emit(valid_src);
                    self.emit(Inst::PhysReg(dest));
                    self.free_op(val);
                }
            }

            Command::Load(ptr, dest) => {
                let p_ptr = self.resolve_operand(ptr);
                let p_dest = self.allocate_output(dest);
                let valid_ptr = self.validate_operand(p_ptr, ptr);
                let ty = self.get_cmd_type(&dest.ty);
                self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Load, ty)));
                self.emit(valid_ptr);
                self.emit(Inst::PhysReg(p_dest));
                self.free_op(ptr);
            }

            Command::Store(val, ptr) => {
                let ty = self.get_cmd_type(&self.get_val_ty(val));
                let v_op = self.resolve_operand(val);
                let p_op = self.resolve_operand(ptr);
                let valid_v = self.validate_operand(v_op, val);
                let valid_p = self.validate_operand(p_op, ptr);
                self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
                self.emit(valid_p);
                self.emit(valid_v);
                self.free_op(val);
                self.free_op(ptr);
            }

            Command::Jump(loc) => {
                self.flush_registers();
                self.emit(Inst::OpCode(OpCode::Jump));
                self.emit(Inst::Location(loc.clone()));
            }
            Command::JumpTrue(loc, cond) => {
                self.flush_registers();
                let op_c = self.resolve_and_free(cond);
                self.emit(Inst::OpCode(OpCode::JumpNotZero));
                self.emit(Inst::Location(loc.clone()));
                self.emit(op_c);
            }
            Command::JumpFalse(loc, cond) => {
                self.flush_registers();
                let op_c = self.resolve_and_free(cond);
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
                    let has_val = if let Some(val) = val_opt {
                        let ty = self.get_cmd_type(&self.get_val_ty(val));
                        let op = self.resolve_and_free(val);
                        self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, ty)));
                        self.emit(op);
                        true
                    } else {
                        false
                    };
                    self.emit(Inst::OpCode(OpCode::Return));
                    self.emit(Inst::Immediate(Immediate {
                        value: has_val as u8 as f64,
                        ty: TypeKind::Uint16,
                    }));
                    self.emit(Inst::SymbolSecLen);
                    self.emit(Inst::ArgCount);
                }
            }

            Command::Push(a) => {
                let ty = self.get_cmd_type(&self.get_val_ty(a));
                let op_a = self.resolve_and_free(a);
                self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, ty)));
                self.emit(op_a);
            }
            Command::Pop(a) => {
                let reg_a = self.allocate_output(a);
                let ty = self.get_cmd_type(&a.ty);
                self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, ty)));
                self.emit(Inst::PhysReg(reg_a));
            }
        }

        self.log(format!(
            "Emitted: {:?}",
            &self.functions[self.loc.0].blocks[self.loc.1][block_start..]
        ));
    }

    fn emit_math(&mut self, op: ArithmeticOp, a: &Value, b: &Value, c: &Output) {
        let ty = self.get_cmd_type(&c.ty);
        let dest = match ty {
            CommandType::I16 | CommandType::U16 => PhysReg::R1,
            CommandType::F32 => PhysReg::F1,
            CommandType::I32 | CommandType::U32 => PhysReg::EX1,
        };
        let is_32bit_mod =
            matches!(op, ArithmeticOp::Mod) && matches!(ty, CommandType::I32 | CommandType::U32);
        if is_32bit_mod {
            let op_a = self.resolve_operand(a);
            let op_b = self.resolve_operand(b);

            if !self.is_reg_free(PhysReg::R1) && !self.can_clobber(PhysReg::R1) {
                self.spill(PhysReg::R1);
                self.mark_phys_dirty(PhysReg::R1);
            }
            let valid_a = self.validate_operand(op_a, a);
            let valid_b = self.validate_operand(op_b, b);
            self.emit(Inst::OpCode(OpCode::Arithmetic(op, ty)));
            self.emit(valid_a);
            self.emit(valid_b);
            self.free_op(a);
            self.free_op(b);
            if !self.is_reg_free(dest) && !self.can_clobber(dest) {
                self.spill(dest);
            }
            self.mark_phys_dirty(dest);
            self.emit(Inst::OpCode(OpCode::Move(ty)));
            self.emit(Inst::PhysReg(PhysReg::R1));
            self.emit(Inst::PhysReg(dest));
            self.claim_reg(dest, c.id as usize);
        } else {
            self.emit_binary(OpCode::Arithmetic(op, ty), dest, a, b, c.id as usize);
        }
    }

    fn emit_logic(&mut self, op: LogicOp, a: &Value, b: &Value, c: &Output) {
        let ty = self.get_cmd_type(&c.ty);
        let dest = if matches!(ty, CommandType::I32 | CommandType::U32) {
            PhysReg::EX1
        } else {
            PhysReg::R1
        };
        self.emit_binary(OpCode::Logic(op, ty), dest, a, b, c.id as usize);
    }

    fn emit_cmp(&mut self, op_code: OpCode, a: &Value, b: &Value, c: &Output) {
        self.emit_binary(op_code, PhysReg::R1, a, b, c.id as usize);
    }

    fn emit_binary(&mut self, op_code: OpCode, dest: PhysReg, a: &Value, b: &Value, out_id: usize) {
        let op_a = self.resolve_operand(a);
        let op_b = self.resolve_operand(b);

        if !self.is_reg_free(dest)
            && !self.can_clobber(dest)
            && !self.can_reuse_dest_owner(dest, a, b)
        {
            self.spill(dest);
        }

        let valid_a = self.validate_operand(op_a, a);
        let valid_b = self.validate_operand(op_b, b);

        self.claim_reg(dest, out_id);
        self.mark_phys_dirty(dest);
        self.emit(Inst::OpCode(op_code));
        self.emit(valid_a);
        self.emit(valid_b);
        self.free_op(a);
        self.free_op(b);
    }

    fn can_reuse_dest_owner(&self, dest: PhysReg, a: &Value, b: &Value) -> bool {
        let func_idx = self.loc.0;
        let Some(owner) = self.functions[func_idx].phys_owners.get(&dest).copied() else {
            return false;
        };
        let used_by_input = matches!(a, Value::Register(r) if r.id as usize == owner)
            || matches!(b, Value::Register(r) if r.id as usize == owner);
        used_by_input && self.next_use_distance(owner) == usize::MAX
    }
}

// ---------------------------------------------------------------------------
// Register allocation
// ---------------------------------------------------------------------------

impl Backend {
    fn ensure_reg(&mut self, virt_id: usize) -> PhysReg {
        let func_idx = self.loc.0;
        if let Some(RegLoc::Physical(p)) = self.functions[func_idx].virt_locs.get(&virt_id).copied()
        {
            if self.functions[func_idx].phys_owners.get(&p).copied() == Some(virt_id) {
                self.touch_reg(p);
                return p;
            }
            if let Some((&fixed_p, _)) = self.functions[func_idx]
                .phys_owners
                .iter()
                .find(|(_, owner)| **owner == virt_id)
            {
                self.functions[func_idx]
                    .virt_locs
                    .insert(virt_id, RegLoc::Physical(fixed_p));
                self.touch_reg(fixed_p);
                return fixed_p;
            }
            self.functions[func_idx].virt_locs.remove(&virt_id);
        }
        match self.functions[func_idx].virt_locs.get(&virt_id).copied() {
            Some(RegLoc::Stack(offset)) => {
                let ty = self.registers[virt_id].ty.clone();
                let phys = self.allocate_reg(&ty, virt_id);
                let cmd_ty = self.get_cmd_type(&ty);
                self.log(format!(
                    "Reloading vreg{} from stack offset {} into {:?}",
                    virt_id, offset, phys
                ));
                // emit_stack_addr uses EX1 as address scratch. If destination aliases EX1
                // (R2/R3/EX1), restoring EX1 would overwrite the just-loaded value.
                let save_ex1 = !self.is_reg_free(PhysReg::EX1) && !self.is_ex1_alias(phys);
                if save_ex1 {
                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                }
                self.emit_stack_addr(offset);
                self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Load, cmd_ty)));
                self.emit(Inst::PhysReg(PhysReg::EX1));
                self.emit(Inst::PhysReg(phys));
                if save_ex1 {
                    self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
                    self.emit(Inst::PhysReg(PhysReg::EX1));
                }
                self.functions[func_idx].dirty_regs.remove(&phys);
                self.functions[func_idx].stack_home.insert(virt_id, offset);
                phys
            }
            Some(RegLoc::Physical(_)) => unreachable!(),
            None => {
                let ty = self.registers[virt_id].ty.clone();
                let phys = self.allocate_reg(&ty, virt_id);
                self.mark_phys_dirty(phys);
                phys
            }
        }
    }

    fn allocate_reg(&mut self, ty: &TypeKind, owner_id: usize) -> PhysReg {
        let candidates: &[PhysReg] = match self.get_cmd_type(ty) {
            CommandType::I16 | CommandType::U16 => &[
                PhysReg::R1,
                PhysReg::R2,
                PhysReg::R3,
                PhysReg::R4,
                PhysReg::R5,
            ],
            CommandType::I32 | CommandType::U32 => &[PhysReg::EX1, PhysReg::EX2],
            CommandType::F32 => &[PhysReg::F1, PhysReg::F2],
        };

        if let Some(&free) = candidates.iter().find(|&&r| self.is_reg_free(r)) {
            self.claim_reg(free, owner_id);
            return free;
        }

        if let Some(&clobberable) = candidates.iter().find(|&&r| self.can_clobber(r)) {
            self.claim_reg(clobberable, owner_id);
            return clobberable;
        }

        let candidate_pool: Vec<PhysReg> = candidates
            .iter()
            .copied()
            .filter(|r| !self.reg_has_current_use(*r))
            .collect();
        let pool: &[PhysReg] = if candidate_pool.is_empty() {
            candidates
        } else {
            &candidate_pool
        };

        let &victim = pool
            .iter()
            .max_by_key(|&&r| {
                let dist = self.phys_next_use_distance(r);
                let lru = self.register_lru.get(&r).copied().unwrap_or(0);
                (dist, usize::MAX - lru)
            })
            .unwrap_or(&pool[0]);

        self.spill(victim);
        self.claim_reg(victim, owner_id);
        victim
    }

    fn allocate_output(&mut self, out: &Output) -> PhysReg {
        let phys = self.allocate_reg(&out.ty.clone(), out.id as usize);
        self.mark_phys_dirty(phys);
        phys
    }

    fn claim_reg(&mut self, reg: PhysReg, virt_id: usize) {
        let func = &mut self.functions[self.loc.0];
        let aliases: Vec<PhysReg> = match reg {
            PhysReg::EX1 | PhysReg::R2 | PhysReg::R3 => Self::alias_group(reg).to_vec(),
            PhysReg::EX2 | PhysReg::R4 | PhysReg::R5 => Self::alias_group(reg).to_vec(),
            _ => vec![reg],
        };
        for alias in aliases {
            if let Some(prev_owner) = func.phys_owners.get(&alias).copied() {
                if prev_owner != virt_id {
                    if let Some(RegLoc::Physical(p)) = func.virt_locs.get(&prev_owner).copied() {
                        if match reg {
                            PhysReg::EX1 | PhysReg::R2 | PhysReg::R3 => {
                                Self::alias_group(reg).contains(&p)
                            }
                            PhysReg::EX2 | PhysReg::R4 | PhysReg::R5 => {
                                Self::alias_group(reg).contains(&p)
                            }
                            _ => p == alias,
                        } {
                            if let Some(&perm) = func.permanent_stack_slots.get(&prev_owner) {
                                func.virt_locs.insert(prev_owner, RegLoc::Stack(perm));
                            } else if let Some(&home) = func.stack_home.get(&prev_owner) {
                                func.permanent_stack_slots.insert(prev_owner, home);
                                func.virt_locs.insert(prev_owner, RegLoc::Stack(home));
                            } else {
                                func.virt_locs.remove(&prev_owner);
                            }
                        }
                    }
                }
            }
            func.phys_owners.remove(&alias);
        }
        if let Some(RegLoc::Physical(old_phys)) = func.virt_locs.get(&virt_id).copied() {
            if old_phys != reg {
                let old_aliases: Vec<PhysReg> = match old_phys {
                    PhysReg::EX1 | PhysReg::R2 | PhysReg::R3 => {
                        Self::alias_group(old_phys).to_vec()
                    }
                    PhysReg::EX2 | PhysReg::R4 | PhysReg::R5 => {
                        Self::alias_group(old_phys).to_vec()
                    }
                    _ => vec![old_phys],
                };
                for a in old_aliases {
                    if func.phys_owners.get(&a).copied() == Some(virt_id) {
                        func.phys_owners.remove(&a);
                    }
                }
            }
        }
        func.phys_owners.insert(reg, virt_id);
        func.virt_locs.insert(virt_id, RegLoc::Physical(reg));
        self.touch_reg(reg);
    }

    fn reg_has_current_use(&self, reg: PhysReg) -> bool {
        let func_idx = self.loc.0;
        let owner_used = |r: PhysReg| {
            self.functions[func_idx]
                .phys_owners
                .get(&r)
                .map_or(false, |owner| self.is_used_in_current_cmd(*owner))
        };
        match reg {
            PhysReg::EX1 => {
                owner_used(PhysReg::EX1) || owner_used(PhysReg::R2) || owner_used(PhysReg::R3)
            }
            PhysReg::EX2 => {
                owner_used(PhysReg::EX2) || owner_used(PhysReg::R4) || owner_used(PhysReg::R5)
            }
            PhysReg::R2 | PhysReg::R3 => owner_used(reg) || owner_used(PhysReg::EX1),
            PhysReg::R4 | PhysReg::R5 => owner_used(reg) || owner_used(PhysReg::EX2),
            _ => owner_used(reg),
        }
    }

    fn release_reg(&mut self, virt_id: usize) {
        let func_idx = self.loc.0;
        if let Some(RegLoc::Physical(p)) = self.functions[func_idx].virt_locs.get(&virt_id).copied()
        {
            let aliases: Vec<PhysReg> = match p {
                PhysReg::EX1 | PhysReg::R2 | PhysReg::R3 => Self::alias_group(p).to_vec(),
                PhysReg::EX2 | PhysReg::R4 | PhysReg::R5 => Self::alias_group(p).to_vec(),
                _ => vec![p],
            };
            for a in aliases {
                if self.functions[func_idx].phys_owners.get(&a).copied() == Some(virt_id) {
                    self.functions[func_idx].phys_owners.remove(&a);
                    self.functions[func_idx].dirty_regs.remove(&a);
                }
            }
            self.functions[func_idx].virt_locs.remove(&virt_id);
        }
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
        let Some(owner_id) = self.functions[func_idx].phys_owners.get(&reg).copied() else {
            match reg {
                PhysReg::EX1 => {
                    self.spill(PhysReg::R2);
                    self.spill(PhysReg::R3);
                }
                PhysReg::EX2 => {
                    self.spill(PhysReg::R4);
                    self.spill(PhysReg::R5);
                }
                PhysReg::R2 | PhysReg::R3 => self.spill(PhysReg::EX1),
                PhysReg::R4 | PhysReg::R5 => self.spill(PhysReg::EX2),
                _ => {}
            }
            return;
        };

        let is_dirty = self.functions[func_idx].dirty_regs.contains(&reg);

        let offset = match self.functions[func_idx].virt_locs.get(&owner_id).copied() {
            Some(RegLoc::Stack(o)) => o,
            _ => {
                if let Some(&perm) = self.functions[func_idx]
                    .permanent_stack_slots
                    .get(&owner_id)
                {
                    perm
                } else if let Some(&home) = self.functions[func_idx].stack_home.get(&owner_id) {
                    self.functions[func_idx]
                        .permanent_stack_slots
                        .insert(owner_id, home);
                    home
                } else {
                    let o = self.functions[func_idx].stack_bytes;
                    self.functions[func_idx].stack_bytes += self.size_of_phys_reg(reg);
                    self.functions[func_idx]
                        .permanent_stack_slots
                        .insert(owner_id, o);
                    o
                }
            }
        };

        if is_dirty {
            let ty = self.type_of_phys_reg(reg);
            if self.is_ex1_alias(reg) {
                self.emit_spill_ex1_alias(reg, ty, offset);
            } else {
                self.emit_spill_general(reg, ty, offset);
            }
        }

        self.log(format!(
            "Spilling vreg{} into stack offset {} from phys {:?}",
            owner_id, offset, reg
        ));
        self.functions[func_idx]
            .virt_locs
            .insert(owner_id, RegLoc::Stack(offset));
        let aliases: Vec<PhysReg> = match reg {
            PhysReg::EX1 | PhysReg::R2 | PhysReg::R3 => Self::alias_group(reg).to_vec(),
            PhysReg::EX2 | PhysReg::R4 | PhysReg::R5 => Self::alias_group(reg).to_vec(),
            _ => vec![reg],
        };
        for a in aliases {
            if self.functions[func_idx].phys_owners.get(&a).copied() == Some(owner_id) {
                self.functions[func_idx].phys_owners.remove(&a);
                self.functions[func_idx].dirty_regs.remove(&a);
            }
        }
    }

    fn emit_stack_addr(&mut self, offset: usize) {
        //offset is realtive to regsave
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
        self.emit(Inst::RestoreOffset);
    }

    fn emit_spill_ex1_alias(&mut self, reg: PhysReg, ty: CommandType, offset: usize) {
        self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
        self.emit(Inst::PhysReg(PhysReg::EX2));
        self.emit(Inst::OpCode(OpCode::Move(CommandType::I32)));
        self.emit(Inst::PhysReg(PhysReg::EX1));
        self.emit(Inst::PhysReg(PhysReg::EX2));
        self.emit_stack_addr(offset);
        let value_reg = match reg {
            PhysReg::EX1 => PhysReg::EX2,
            PhysReg::R2 => PhysReg::R4,
            PhysReg::R3 => PhysReg::R5,
            _ => unreachable!(),
        };
        self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
        self.emit(Inst::PhysReg(PhysReg::EX1));
        self.emit(Inst::PhysReg(value_reg));
        // Restore only when spilling EX1 itself. For R2/R3, EX1 contains address scratch,
        // and restoring it here can clobber live pointer values.
        if reg == PhysReg::EX1 {
            self.emit(Inst::OpCode(OpCode::Move(CommandType::I32)));
            self.emit(Inst::PhysReg(PhysReg::EX2));
            self.emit(Inst::PhysReg(PhysReg::EX1));
        }
        self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
        self.emit(Inst::PhysReg(PhysReg::EX2));
    }

    fn emit_spill_general(&mut self, reg: PhysReg, ty: CommandType, offset: usize) {
        let save_ex1 = !self.is_reg_free(PhysReg::EX1) && reg != PhysReg::EX1;
        if save_ex1 {
            self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
            self.emit(Inst::PhysReg(PhysReg::EX1));
        }
        self.emit_stack_addr(offset);
        self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
        self.emit(Inst::PhysReg(PhysReg::EX1));
        self.emit(Inst::PhysReg(reg));
        if save_ex1 {
            self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
            self.emit(Inst::PhysReg(PhysReg::EX1));
        }
    }

    fn flush_registers(&mut self) {
        let active: HashSet<PhysReg> = self.functions[self.loc.0]
            .phys_owners
            .keys()
            .copied()
            .collect();
        let to_flush: Vec<PhysReg> = active
            .iter()
            .filter(|&&reg| match reg {
                PhysReg::R2 | PhysReg::R3 => !active.contains(&PhysReg::EX1),
                PhysReg::R4 | PhysReg::R5 => !active.contains(&PhysReg::EX2),
                _ => true,
            })
            .copied()
            .collect();
        for r in to_flush {
            self.spill(r);
        }
    }

    fn reset_phys_regs(&mut self) {
        let func = &mut self.functions[self.loc.0];
        func.phys_owners.clear();
        func.dirty_regs.clear();
        func.stack_home.clear();
        func.virt_locs
            .retain(|_, loc| matches!(loc, RegLoc::Stack(_)));
    }

    fn mark_phys_dirty(&mut self, phys: PhysReg) {
        let func_idx = self.loc.0;
        self.functions[func_idx].dirty_regs.insert(phys);
        if let Some(&owner) = self.functions[func_idx].phys_owners.get(&phys) {
            self.functions[func_idx].stack_home.remove(&owner);
        }
    }

    fn touch_reg(&mut self, reg: PhysReg) {
        self.time_step += 1;
        let ts = self.time_step;
        self.register_lru.insert(reg, ts);
        match reg {
            PhysReg::EX1 => {
                self.register_lru.insert(PhysReg::R2, ts);
                self.register_lru.insert(PhysReg::R3, ts);
            }
            PhysReg::EX2 => {
                self.register_lru.insert(PhysReg::R4, ts);
                self.register_lru.insert(PhysReg::R5, ts);
            }
            PhysReg::R2 | PhysReg::R3 => {
                self.register_lru.insert(PhysReg::EX1, ts);
            }
            PhysReg::R4 | PhysReg::R5 => {
                self.register_lru.insert(PhysReg::EX2, ts);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Operand resolution helpers
// ---------------------------------------------------------------------------

impl Backend {
    fn resolve_operand(&mut self, val: &Value) -> Inst {
        match val {
            Value::Register(r) => Inst::PhysReg(self.ensure_reg(r.id as usize)),
            Value::Immediate(i) => Inst::Immediate(i.clone()),
            Value::Location(l) => Inst::Location(l.clone()),
            Value::ARP => Inst::ARP,
        }
    }

    fn resolve_and_free(&mut self, val: &Value) -> Inst {
        let inst = self.resolve_operand(val);
        self.free_op(val);
        inst
    }

    fn free_op(&mut self, val: &Value) {
        if let Value::Register(r) = val {
            self.decrement_usage(r.id as usize);
        }
    }

    fn validate_operand(&mut self, inst: Inst, source_val: &Value) -> Inst {
        if let (Inst::PhysReg(reg), Value::Register(virt)) = (&inst, source_val) {
            let func_idx = self.loc.0;
            match self.functions[func_idx]
                .virt_locs
                .get(&(virt.id as usize))
                .copied()
            {
                Some(RegLoc::Stack(_)) => return self.resolve_operand(source_val),
                Some(RegLoc::Physical(p)) if p != *reg => return Inst::PhysReg(p),
                None => return self.resolve_operand(source_val),
                _ => {}
            }
        }
        inst
    }

    fn decrement_usage(&mut self, virt_id: usize) {
        let exhausted = if let Some(count) = self.current_block_usage.get_mut(&virt_id) {
            *count -= 1;
            *count == 0
        } else {
            false
        };
        if exhausted && self.is_strictly_local(virt_id) {
            self.release_reg(virt_id);
        }
    }

    fn collect_reads<F>(cmd: &Command, mut cb: F)
    where
        F: FnMut(usize),
    {
        let mut reg = |v: &Value| {
            if let Value::Register(r) = v {
                cb(r.id as usize);
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
                reg(a);
                reg(b);
            }
            Command::Not(a, _) | Command::Move(a, _) | Command::Push(a) | Command::Load(a, _) => {
                reg(a)
            }
            Command::Store(val, ptr) => {
                reg(val);
                reg(ptr);
            }
            Command::JumpTrue(_, c) | Command::JumpFalse(_, c) => reg(c),
            Command::Call(loc, _) => reg(loc),
            Command::Ret(Some(v)) => reg(v),
            Command::Pop(_) | Command::Jump(_) | Command::Ret(None) => {}
        }
    }

    fn collect_writes<F>(cmd: &Command, mut cb: F)
    where
        F: FnMut(usize),
    {
        let mut reg = |out: &Output| {
            cb(out.id as usize);
        };
        match cmd {
            Command::Add(_, _, out)
            | Command::Sub(_, _, out)
            | Command::Mul(_, _, out)
            | Command::Div(_, _, out)
            | Command::Mod(_, _, out)
            | Command::And(_, _, out)
            | Command::Or(_, _, out)
            | Command::Xor(_, _, out)
            | Command::Shl(_, _, out)
            | Command::Shr(_, _, out)
            | Command::Eq(_, _, out)
            | Command::Gt(_, _, out)
            | Command::Lt(_, _, out)
            | Command::Not(_, out)
            | Command::Move(_, out)
            | Command::Load(_, out)
            | Command::Pop(out) => reg(out),
            Command::Store(_, _)
            | Command::Jump(_)
            | Command::JumpTrue(_, _)
            | Command::JumpFalse(_, _)
            | Command::Call(_, _)
            | Command::Ret(_)
            | Command::Push(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Type helpers
// ---------------------------------------------------------------------------

impl Backend {
    fn get_val_ty(&self, v: &Value) -> TypeKind {
        match v {
            Value::ARP => TypeKind::Uint32,
            Value::Immediate(i) => i.ty.clone(),
            Value::Location(_) => TypeKind::Uint32,
            Value::Register(r) => r.ty.clone(),
        }
    }

    fn get_cmd_type(&self, ty: &TypeKind) -> CommandType {
        match ty {
            TypeKind::Int16 | TypeKind::Bool => CommandType::I16,
            TypeKind::Uint16 | TypeKind::Enum(_) | TypeKind::Char => CommandType::U16,
            TypeKind::Int32 | TypeKind::Pointer(_) | TypeKind::Optional(_) => CommandType::I32,
            TypeKind::Uint32 => CommandType::U32,
            TypeKind::Float32 => CommandType::F32,
            _ => CommandType::U32,
        }
    }

    fn is_ex1_alias(&self, reg: PhysReg) -> bool {
        matches!(reg, PhysReg::EX1 | PhysReg::R2 | PhysReg::R3)
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

    fn emit(&mut self, inst: Inst) {
        self.functions[self.loc.0].blocks[self.loc.1].push(inst);
    }

    fn log(&mut self, msg: String) {
        self.logs.push(msg);
    }
}

// ---------------------------------------------------------------------------
// Lowering
// ---------------------------------------------------------------------------

impl Backend {
    fn lower_inst(inst: &Inst, fun: &Function, all_fns: &[Function]) -> Bytecode {
        match inst {
            Inst::ARP => Bytecode::Register(CmdType::ARP),
            Inst::ArgCount => Bytecode::ArgCount(),
            Inst::SymbolSecLen => Bytecode::SymbolSectionLen(),
            Inst::RestoreOffset => Bytecode::Symbol(
                "__internal_reg_save_463653961935601537679876958223".to_string(),
                0,
            ),
            Inst::Location(loc) => match loc {
                ir::Location::Argument(name) => {
                    Bytecode::Argument(fun.params.iter().position(|x| x.0 == *name).unwrap())
                }
                ir::Location::Symbol(id, offset) => {
                    Bytecode::Symbol(fun.symbols[*id].0.clone(), *offset as i32)
                }
                ir::Location::Block(b) => Bytecode::BlockLoc(*b as isize),
                ir::Location::Function(fnid) => Bytecode::FunctionRef(all_fns[*fnid].name.clone()),
                ir::Location::None => unreachable!(),
            },
            Inst::Immediate(im) => match im.ty {
                TypeKind::Float32 => Bytecode::Float(im.value as f32),
                TypeKind::Int32
                | TypeKind::Uint32
                | TypeKind::Pointer(_)
                | TypeKind::Optional(_) => Bytecode::Int32(im.value as i32),
                _ => Bytecode::Int(im.value as i16),
            },
            Inst::StackOffset(off) => Bytecode::Int(*off as i16),
            Inst::PhysReg(r) => Bytecode::Register(match r {
                PhysReg::R1 => CmdType::R1,
                PhysReg::R2 => CmdType::R2,
                PhysReg::R3 => CmdType::R3,
                PhysReg::R4 => CmdType::R4,
                PhysReg::R5 => CmdType::R5,
                PhysReg::EX1 => CmdType::EX1,
                PhysReg::EX2 => CmdType::EX2,
                PhysReg::F1 => CmdType::F1,
                PhysReg::F2 => CmdType::F2,
            }),
            Inst::OpCode(op) => Bytecode::Command(match op {
                OpCode::Exit => CmdType::Exit,
                OpCode::Call => CmdType::Call,
                OpCode::Return => CmdType::Return,
                OpCode::Jump => CmdType::Jump,
                OpCode::JumpNotZero => CmdType::JumpNotZero,
                OpCode::JumpZero => CmdType::JumpZero,
                OpCode::Eq => CmdType::Equals,
                OpCode::GreaterThan => CmdType::Greater,
                OpCode::LessThan => CmdType::LessThan,
                OpCode::Move(_) => CmdType::Mov,
                OpCode::Stack(sop, ty) => match sop {
                    StackOp::Push => match ty {
                        CommandType::F32 => CmdType::Pushf,
                        CommandType::I32 | CommandType::U32 => CmdType::PushEx,
                        _ => CmdType::Push,
                    },
                    StackOp::Pop => CmdType::Pop,
                },
                OpCode::Memory(mop, ty) => match mop {
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
                },
                OpCode::Arithmetic(aop, ty) => match aop {
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
                },
                OpCode::Logic(lop, ty) => {
                    let wide = matches!(ty, CommandType::I32 | CommandType::U32 | CommandType::F32);
                    match lop {
                        LogicOp::And => {
                            if wide {
                                CmdType::AndEx
                            } else {
                                CmdType::And
                            }
                        }
                        LogicOp::Or => {
                            if wide {
                                CmdType::OrEx
                            } else {
                                CmdType::Or
                            }
                        }
                        LogicOp::Xor => {
                            if wide {
                                CmdType::XorEx
                            } else {
                                CmdType::Xor
                            }
                        }
                        LogicOp::Not => {
                            if wide {
                                CmdType::NotEx
                            } else {
                                CmdType::Not
                            }
                        }
                        LogicOp::Shl => {
                            if wide {
                                CmdType::ShlEx
                            } else {
                                CmdType::Shl
                            }
                        }
                        LogicOp::Shr => {
                            if wide {
                                CmdType::ShrEx
                            } else {
                                CmdType::Shr
                            }
                        }
                    }
                }
            }),
        }
    }
}
