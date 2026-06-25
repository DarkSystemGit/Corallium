use indexmap::IndexMap;

use crate::assembler::parser::{Function, Statement, StatementKind};
use crate::executable::{Bytecode, Data, Executable, Fn, Library};
use crate::vm::CommandType;

use super::lexer::RegisterType;
use super::parser::{Block, GlobalDef, Value};
pub struct CodeGen {
    error: bool,
    input: Vec<Statement>,
    emitted: Vec<(Fn, Vec<String>, usize)>,
    current_fn: usize,
    globals: IndexMap<String, Vec<Data>>,
    block_map: IndexMap<String, usize>,
    file_name: String,
    pub imports: Vec<String>,
}
#[derive(Debug, Clone)]
pub enum Object {
    Exe(Executable),
    Lib(Library),
}
impl CodeGen {
    pub fn new(file_name: String) -> CodeGen {
        CodeGen {
            error: false,
            input: vec![],
            emitted: vec![],
            current_fn: 0,
            globals: IndexMap::new(),
            block_map: IndexMap::new(),
            file_name,
            imports: vec![],
        }
    }
    fn genValue(&mut self, val: Value) -> Option<Vec<Data>> {
        match val {
            Value::Array(vals) => {
                let mut out = vec![];
                for value in vals {
                    let mut data = self.genValue(value)?;
                    out.append(&mut data);
                }
                Some(out)
            }
            Value::Float(f) => Some(vec![Data::Float(f)]),
            Value::Int16(i) => Some(vec![Data::Int(i)]),
            Value::Int32(i) => Some(vec![Data::Int32(i)]),
            _ => None,
        }
    }
    pub fn genBytecode(&mut self, input: Vec<Statement>, lib: bool) -> Option<Object> {
        self.error = false;
        self.input = input;
        self.emitted.clear();
        self.globals.clear();

        let input = self.input.clone();
        self.collectGlobals(&input);
        for stmt in input {
            match stmt.kind {
                StatementKind::Function(fun) => {
                    self.emitFn(fun);
                }
                StatementKind::Import(str) => self.imports.push(str),
                StatementKind::GlobalDef(_) => {}
                _ => self.emitError(
                    "Only globals and function definitions are allowwed in the top-level",
                ),
            }
        }

        if self.error {
            return None;
        }
        if lib != true {
            let mut exe = Executable::new();
            for (_, data) in &self.globals {
                exe.add_constant(data.clone());
            }
            for (func, _, _) in self.emitted.clone() {
                exe.add_fn(func);
            }
            Some(Object::Exe(exe))
        } else {
            let mut exe = Library::new(self.file_name.clone());
            for (_, data) in &self.globals {
                exe.add_constant(data.clone());
            }
            for (func, _, _) in self.emitted.clone() {
                exe.add_fn(func);
            }
            Some(Object::Lib(exe))
        }
    }
    fn emitFn(&mut self, fun: Function) {
        self.block_map = self.build_block_map(&fun);
        self.emitted.push((
            Fn::new(fun.name, fun.args.iter().map(|x| *x.1).collect()),
            fun.args.iter().map(|x| x.0.clone()).collect(),
            fun.returned_bytes,
        ));
        self.current_fn = self.emitted.len() - 1;

        let mut blocks: Vec<Vec<Bytecode>> = vec![];
        let mut current_block: Vec<Bytecode> = vec![];

        for stmt in fun.body {
            match stmt.kind {
                StatementKind::Block(block) => {
                    if !current_block.is_empty() {
                        blocks.push(current_block);
                        current_block = vec![];
                    }
                    blocks.push(self.emitBlock(block));
                }
                _ => self.emitStmt(stmt, &mut current_block),
            }
        }
        if !current_block.is_empty() || blocks.is_empty() {
            blocks.push(current_block);
        }

        for (i, block) in blocks.into_iter().enumerate() {
            self.emitted[self.current_fn].0.add_block(block, i == 0);
        }
    }
    fn convertValueToBytecode(&mut self, val: Value) -> Option<Bytecode> {
        match val {
            Value::Int16(i) => Some(Bytecode::Int(i)),
            Value::Float(f) => Some(Bytecode::Float(f)),
            Value::Int32(i) => Some(Bytecode::Int32(i)),
            Value::Global(name) => match self.globals.get_index_of(&name) {
                Some(index) => Some(Bytecode::ConstantLoc(index)),
                None => {
                    self.emitError(&format!("Unknown global {:?}", name));
                    None
                }
            },
            Value::Symbol(name) => Some(Bytecode::Symbol(name, 0)),
            Value::Argument(name) => match self.emitted[self.current_fn]
                .1
                .iter()
                .position(|x| *x == name)
            {
                Some(idx) => Some(Bytecode::Argument(idx)),
                None => {
                    self.emitError(&format!("Unknown argument {:?}", name));
                    None
                }
            },
            Value::Register(r) => Some(Bytecode::Register(match r {
                RegisterType::R1 => CommandType::R1,
                RegisterType::R2 => CommandType::R2,
                RegisterType::R3 => CommandType::R3,
                RegisterType::R4 => CommandType::R4,
                RegisterType::R5 => CommandType::R5,
                RegisterType::F1 => CommandType::F1,
                RegisterType::F2 => CommandType::F2,
                RegisterType::EX1 => CommandType::EX1,
                RegisterType::EX2 => CommandType::EX2,
                RegisterType::ARP => CommandType::ARP,
                RegisterType::IP => CommandType::IP,
                RegisterType::SP => CommandType::SP,
                RegisterType::SRP => CommandType::SRP,
            })),
            Value::FnRef(name) => Some(Bytecode::FunctionRef(name)),
            Value::Block(name) => match self.block_map.get(&name) {
                Some(index) => Some(Bytecode::BlockLoc(*index as isize)),
                None => {
                    self.emitError(&format!("Unknown block {:?}", name));
                    None
                }
            },
            Value::Array(_) => {
                self.emitError("Array values are only supported in globals");
                None
            }
        }
    }
    fn build_block_map(&mut self, fun: &Function) -> IndexMap<String, usize> {
        let mut block_map = IndexMap::new();
        let mut blocks_len = 0usize;
        let mut current_block_has_stmts = false;
        for stmt in &fun.body {
            match &stmt.kind {
                StatementKind::Block(block) => {
                    if current_block_has_stmts {
                        blocks_len += 1;
                        current_block_has_stmts = false;
                    }
                    if block_map.contains_key(&block.name) {
                        self.emitError(&format!("Duplicate block {:?}", block.name));
                    } else {
                        block_map.insert(block.name.clone(), blocks_len);
                    }
                    blocks_len += 1;
                }
                _ => current_block_has_stmts = true,
            }
        }
        if current_block_has_stmts || blocks_len == 0 {
            blocks_len += 1;
        }
        block_map
    }
    fn emitStmts(&mut self, stmts: Vec<Statement>) -> Vec<Bytecode> {
        let mut block = vec![];
        for stmt in stmts {
            self.emitStmt(stmt, &mut block);
        }
        block
    }
    fn emitStmt(&mut self, stmt: Statement, block: &mut Vec<Bytecode>) {
        match stmt.kind {
            StatementKind::SymbolDef(sym) => {
                self.emitted[self.current_fn]
                    .0
                    .add_symbol(&sym.name, sym.size);
            }
            StatementKind::Command(cmd) => {
                let op = cmd.op;
                if op == CommandType::Return {
                    block.push(Bytecode::Command(op));
                    block.push(Bytecode::Int(self.emitted[self.current_fn].2 as i16));
                    block.push(Bytecode::SymbolSectionLen());
                    block.push(Bytecode::ArgCount());
                    return;
                };
                let mut values = vec![];
                for val in cmd.values {
                    if let Some(bytecode) = self.convertValueToBytecode(val) {
                        values.push(bytecode);
                    } else {
                        return;
                    }
                }
                block.push(Bytecode::Command(op));
                block.extend(values);
            }
            StatementKind::GlobalDef(_) => {}
            StatementKind::Block(block_def) => {
                self.emitError(&format!(
                    "Nested block {:?} is not supported",
                    block_def.name
                ));
            }
            _ => {}
        }
    }
    fn emitBlock(&mut self, block: Block) -> Vec<Bytecode> {
        self.emitStmts(block.stmts)
    }
    fn emitGlobal(&mut self, global: GlobalDef) {
        if self.globals.contains_key(&global.name) {
            self.emitError(&format!("Duplicate global {:?}", global.name));
            return;
        }
        if global.size.is_none() && global.val.is_none() {
            self.emitError(&format!(
                "Global {:?} requires a size or initializer",
                global.name
            ));
            return;
        }
        let mut v = match global.val {
            Some(val) => match self.genValue(val) {
                Some(value) => value,
                None => {
                    self.emitError(&format!("Invalid global initializer {:?}", global.name));
                    return;
                }
            },
            None => vec![],
        };
        let data_len = v.iter().map(|x| self.getDataLen(x)).sum::<usize>();
        let size = global.size.unwrap_or(data_len);
        if size < data_len {
            self.emitError(&format!(
                "Global {:?} initializer exceeds declared size",
                global.name
            ));
            return;
        }
        if size > data_len {
            v.extend(vec![Data::Int(0); size - data_len]);
        }
        self.globals.insert(global.name, v);
    }
    fn collectGlobals(&mut self, stmts: &[Statement]) {
        for stmt in stmts {
            match &stmt.kind {
                StatementKind::GlobalDef(global) => {
                    self.emitGlobal(global.clone());
                }
                StatementKind::Function(fun) => self.collectGlobals(&fun.body),
                StatementKind::Block(block) => self.collectGlobals(&block.stmts),
                _ => {}
            }
        }
    }
    fn getDataLen(&self, data: &Data) -> usize {
        match data {
            Data::Bytes(b) => b.len(),
            Data::Float(_f) => 2,
            Data::Int(_i) => 1,
            Data::Int32(_i) => 2,
            Data::ConstantLoc(_c) => 2,
        }
    }
    fn emitError(&mut self, message: &str) {
        println!("Error while parsing at {}:\n{}", self.file_name, message);
        self.error = true;
    }
}
