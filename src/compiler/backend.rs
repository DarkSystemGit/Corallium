use super::ir::{self, Command, Definition, Immediate, IrGen, Output, Value};
use super::lexer::TypeKind;
use crate::executable::{Bytecode, Executable, Fn};
use crate::vm::CommandType as CmdType;
use std::collections::{BTreeMap, HashMap, HashSet};

// ---------------------------------------------------------------------------
// Physical register file
// ---------------------------------------------------------------------------
// R1–R5  : 16-bit general purpose
// EX1    : 32-bit, aliases R2 + R3
// EX2    : 32-bit, aliases R4 + R5
// F1, F2 : 32-bit float

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

// ---------------------------------------------------------------------------
// Instruction representation
// ---------------------------------------------------------------------------

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

    virt_locs: BTreeMap<usize, RegLoc>,
    phys_owners: BTreeMap<PhysReg, usize>,
    dirty_regs: HashSet<PhysReg>,
    stack_bytes: usize,
    /// For a virtual register currently in a physical register but NOT dirty,
    /// records the stack slot that holds a valid copy. Cleared when the register
    /// is written (marked dirty). Lets spill() reuse the existing slot instead
    /// of allocating a new one and leaving it with garbage.
    stack_home: HashMap<usize, usize>,
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
            blocks: Vec::new(),
            virt_locs: BTreeMap::new(),
            phys_owners: BTreeMap::new(),
            dirty_regs: HashSet::new(),
            stack_bytes: 0,
            stack_home: HashMap::new(),
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

    register_lru: HashMap<PhysReg, usize>,
    time_step: usize,
}

// ---------------------------------------------------------------------------
// Backend — public interface
// ---------------------------------------------------------------------------

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
            // Build a map of which blocks each virtual register is used in.
            self.current_var_scopes.clear();
            for (b_idx, block) in func_def.body.iter().enumerate() {
                for cmd in block {
                    Self::collect_reads(cmd, |id| {
                        self.current_var_scopes.entry(id).or_default().insert(b_idx);
                    });
                }
            }

            // Separate symbols and parameters by their definition kind.
            let mut symbols: Vec<(String, TypeKind, usize)> = Vec::new();
            let mut params: Vec<(String, TypeKind, usize)> = Vec::new();
            for sym in &func_def.symbols {
                let slot = match &sym.body {
                    Definition::Var(ty) => {
                        symbols.resize(sym.id + 1, ("".into(), TypeKind::Void, 0));
                        &mut symbols[sym.id]
                    }
                    Definition::Parameter(ty) => {
                        params.resize(sym.id + 1, ("".into(), TypeKind::Void, 0));
                        &mut params[sym.id]
                    }
                    _ => continue,
                };
                let ty = match &sym.body {
                    Definition::Var(t) | Definition::Parameter(t) => t.clone(),
                    _ => unreachable!(),
                };
                *slot = (sym.name.clone(), ty, sym.size.unwrap());
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

    pub fn emit_bytecode(&mut self) -> Executable {
        let mut exe = Executable::new();
        for fun in &self.functions {
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

            let mut bytefn = Fn::new_with_blocks(fun.name.clone(), fun.params.len(), blocks);
            for (name, _, size) in &fun.symbols {
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

// ---------------------------------------------------------------------------
// Backend — instruction selection
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
                if !self.is_reg_free(dest) {
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
                let src = self.resolve_operand(val);
                let dest = self.allocate_output(out);
                let ty = self.get_cmd_type(&out.ty);
                let valid_src = self.validate_operand(src, val);
                self.emit(Inst::OpCode(OpCode::Move(ty)));
                self.emit(valid_src);
                self.emit(Inst::PhysReg(dest));
                self.free_op(val);
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

    // Arithmetic/logic/comparison helpers — the dest register is always hardcoded
    // to a specific physical register for simplicity.

    fn emit_math(&mut self, op: ArithmeticOp, a: &Value, b: &Value, c: &Output) {
        let ty = self.get_cmd_type(&c.ty);
        let dest = match ty {
            CommandType::I16 | CommandType::U16 => PhysReg::R1,
            CommandType::F32 => PhysReg::F1,
            CommandType::I32 | CommandType::U32 => PhysReg::EX1,
        };
        self.emit_binary(OpCode::Arithmetic(op, ty), dest, a, b, c.id as usize);
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

    /// Shared core for all binary ops: resolve operands, spill dest if occupied,
    /// claim dest, emit the instruction.
    fn emit_binary(&mut self, op_code: OpCode, dest: PhysReg, a: &Value, b: &Value, out_id: usize) {
        let op_a = self.resolve_operand(a);
        let op_b = self.resolve_operand(b);

        if !self.is_reg_free(dest) {
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
}

// ---------------------------------------------------------------------------
// Backend — register allocation
// ---------------------------------------------------------------------------

impl Backend {
    /// Returns the physical register currently holding `virt_id`, reloading
    /// from the stack if necessary.
    fn ensure_reg(&mut self, virt_id: usize) -> PhysReg {
        let func_idx = self.loc.0;

        match self.functions[func_idx].virt_locs.get(&virt_id).copied() {
            Some(RegLoc::Physical(p)) => {
                self.touch_reg(p);
                return p;
            }
            Some(RegLoc::Stack(offset)) => {
                let ty = self.registers[virt_id].ty.clone();
                let phys = self.allocate_reg(&ty, virt_id);
                let cmd_ty = self.get_cmd_type(&ty);

                self.log(format!(
                    "Reloading vreg{} from stack offset {} into {:?}",
                    virt_id, offset, phys
                ));

                // EX1 is used as a scratch register to compute the stack address.
                // Preserve it only when it isn't the reload destination itself.
                let save_ex1 = !self.is_reg_free(PhysReg::EX1) && phys != PhysReg::EX1;
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

                // A freshly reloaded register is clean (matches its stack copy).
                // Record the home slot so spill() can reuse it without emitting
                // a Store if this register is never written before being evicted.
                self.functions[func_idx].dirty_regs.remove(&phys);
                self.functions[func_idx].stack_home.insert(virt_id, offset);
                phys
            }
            None => {
                // No location recorded yet — allocate a fresh register.
                let ty = self.registers[virt_id].ty.clone();
                let phys = self.allocate_reg(&ty, virt_id);
                // Owner already set by claim_reg inside allocate_reg; mark dirty
                // and clear any stale home slot.
                self.mark_phys_dirty(phys);
                phys
            }
        }
    }

    /// Find a free register for `owner_id`, spilling the LRU candidate if needed.
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

        let &victim = candidates
            .iter()
            .min_by_key(|r| self.register_lru.get(r).copied().unwrap_or(0))
            .unwrap_or(&candidates[0]);
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
        func.phys_owners.insert(reg, virt_id);
        func.virt_locs.insert(virt_id, RegLoc::Physical(reg));
        self.touch_reg(reg);
    }

    fn release_reg(&mut self, virt_id: usize) {
        let func_idx = self.loc.0;
        if let Some(RegLoc::Physical(p)) = self.functions[func_idx].virt_locs.get(&virt_id).copied()
        {
            // Only touch the physical register's state if this virtual register
            // still owns it.  After a spill-and-reclaim sequence a different
            // virtual register may have taken ownership; unconditionally clearing
            // the dirty flag here would silently discard that register's value.
            if self.functions[func_idx].phys_owners.get(&p).copied() == Some(virt_id) {
                self.functions[func_idx].phys_owners.remove(&p);
                self.functions[func_idx].dirty_regs.remove(&p);
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
            // No direct owner — recurse into aliases.
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

        // Reuse an existing stack slot if one was already assigned.
        // Crucially, also check stack_home: if this register was reloaded from
        // the stack but never written (not dirty), virt_locs now shows
        // Physical(reg) not Stack(_), but the home slot still has a valid copy.
        let offset = match self.functions[func_idx].virt_locs.get(&owner_id).copied() {
            Some(RegLoc::Stack(o)) => o,
            _ => {
                if let Some(&home) = self.functions[func_idx].stack_home.get(&owner_id) {
                    home
                } else {
                    let o = self.functions[func_idx].stack_bytes;
                    self.functions[func_idx].stack_bytes += self.size_of_phys_reg(reg);
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
        self.functions[func_idx].phys_owners.remove(&reg);
        self.functions[func_idx].dirty_regs.remove(&reg);
    }

    /// Emit the two-add sequence that loads ARP + offset + SymbolSecLen into EX1,
    /// leaving EX1 pointing at the spill slot. Used by both spill and reload paths.
    fn emit_stack_addr(&mut self, offset: usize) {
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

    /// Emit instructions to save an EX1-aliased register (EX1, R2, or R3) to
    /// the stack. EX2 is used as a temporary to hold the value while EX1 is
    /// repurposed to compute the destination address.
    fn emit_spill_ex1_alias(&mut self, reg: PhysReg, ty: CommandType, offset: usize) {
        // Save EX2 so we can use it as a scratch value holder.
        self.emit(Inst::OpCode(OpCode::Stack(StackOp::Push, CommandType::I32)));
        self.emit(Inst::PhysReg(PhysReg::EX2));

        // Copy all 32 bits of EX1 into EX2 before EX1 is clobbered by the address
        // computation. Using I32 (not `ty`) ensures both narrow halves are captured:
        // R4 = original R2, R5 = original R3. This is correct even when spilling
        // only R2 or R3 individually, because the I32 restore below puts both
        // halves back, preserving the peer register's value if it is still live.
        self.emit(Inst::OpCode(OpCode::Move(CommandType::I32)));
        self.emit(Inst::PhysReg(PhysReg::EX1));
        self.emit(Inst::PhysReg(PhysReg::EX2));

        // Compute the stack address in EX1.
        self.emit_stack_addr(offset);

        // Store the saved value (in EX2 or its narrow alias) to the stack address.
        let value_reg = match reg {
            PhysReg::EX1 => PhysReg::EX2,
            PhysReg::R2 => PhysReg::R4,
            PhysReg::R3 => PhysReg::R5,
            _ => unreachable!(),
        };
        self.emit(Inst::OpCode(OpCode::Memory(MemoryOp::Store, ty)));
        self.emit(Inst::PhysReg(PhysReg::EX1));
        self.emit(Inst::PhysReg(value_reg));

        // Restore EX1 from EX2, then restore EX2.
        self.emit(Inst::OpCode(OpCode::Move(CommandType::I32)));
        self.emit(Inst::PhysReg(PhysReg::EX2));
        self.emit(Inst::PhysReg(PhysReg::EX1));
        self.emit(Inst::OpCode(OpCode::Stack(StackOp::Pop, CommandType::I32)));
        self.emit(Inst::PhysReg(PhysReg::EX2));
    }

    /// Emit instructions to save a non-EX1-aliased register to the stack.
    /// EX1 is used as a scratch register for the address and is preserved
    /// via push/pop if it was already in use.
    fn emit_spill_general(&mut self, reg: PhysReg, ty: CommandType, offset: usize) {
        let save_ex1 = !self.is_reg_free(PhysReg::EX1);
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

    /// Flush all currently live registers to the stack (e.g. before a branch).
    /// Normalises alias groups so we never try to spill both EX1 and R2/R3.
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

    /// Clear physical-register state at the start of a new block.
    /// Stack locations remain valid across blocks; physical ones do not.
    fn reset_phys_regs(&mut self) {
        let func = &mut self.functions[self.loc.0];
        func.phys_owners.clear();
        func.dirty_regs.clear();
        func.stack_home.clear();
        func.virt_locs
            .retain(|_, loc| matches!(loc, RegLoc::Stack(_)));
    }

    /// Mark a physical register as dirty (its value differs from any stack copy)
    /// and invalidate the stack_home record for whoever owns it.
    fn mark_phys_dirty(&mut self, phys: PhysReg) {
        let func_idx = self.loc.0;
        self.functions[func_idx].dirty_regs.insert(phys);
        if let Some(&owner) = self.functions[func_idx].phys_owners.get(&phys) {
            self.functions[func_idx].stack_home.remove(&owner);
        }
    }

    // -----------------------------------------------------------------------
    // LRU tracking
    // -----------------------------------------------------------------------

    fn touch_reg(&mut self, reg: PhysReg) {
        self.time_step += 1;
        let ts = self.time_step;
        self.register_lru.insert(reg, ts);
        // Keep alias groups in sync so LRU picks are coherent.
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
// Backend — operand resolution helpers
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

    /// Re-resolve `inst` if the value has been moved since `inst` was computed.
    /// This handles the case where a spill occurred between resolve and use.
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
        if exhausted {
            let is_local = self
                .current_var_scopes
                .get(&virt_id)
                .map_or(false, |s| s.len() == 1);
            if is_local {
                self.release_reg(virt_id);
            }
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
}

// ---------------------------------------------------------------------------
// Backend — type helpers
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
            TypeKind::Int32 => CommandType::I32,
            TypeKind::Uint32 => CommandType::U32,
            TypeKind::Pointer(_) => CommandType::I32,
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

impl Backend {
    fn lower_inst(inst: &Inst, fun: &Function, all_fns: &[Function]) -> Bytecode {
        match inst {
            Inst::ARP => Bytecode::Register(CmdType::ARP),
            Inst::ArgCount => Bytecode::ArgCount(),
            Inst::SymbolSecLen => Bytecode::SymbolSectionLen(),
            Inst::RestoreOffset => Bytecode::Int32(
                fun.symbols
                    .iter()
                    .filter(|x| x.0 != "__internal_reg_save_463653961935601537679876958223")
                    .map(|x| x.2)
                    .sum::<usize>() as i32,
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
                TypeKind::Int32 | TypeKind::Uint32 | TypeKind::Pointer(_) => {
                    Bytecode::Int32(im.value as i32)
                }
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
