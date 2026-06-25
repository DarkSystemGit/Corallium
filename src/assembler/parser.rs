use indexmap::IndexMap;
use std::collections::HashSet;

use super::lexer::{Location, RegisterType, SourceLocation, Token, TokenKind};
use crate::vm::CommandType;
pub struct Parser {
    input: Vec<Token>,
    pos: usize,
    file_name: String,
    error: bool,
    symbols: SymbolTable,
    block_labels: Vec<HashSet<String>>,
}
#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub loc: SourceLocation,
}
#[derive(Debug, Clone)]
pub enum StatementKind {
    Command(Command),
    Function(Function),
    Block(Block),
    SymbolDef(SymbolDef),
    GlobalDef(GlobalDef),
    Import(String),
}
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub args: IndexMap<String, usize>,
    pub body: Vec<Statement>,
    pub blocks: Vec<String>,
    pub returned_bytes: usize,
}
#[derive(Debug, Clone)]
pub struct Block {
    pub name: String,
    pub stmts: Vec<Statement>,
}
#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub name: String,
    pub size: usize,
}
#[derive(Debug, Clone)]
pub struct GlobalDef {
    pub name: String,
    pub size: Option<usize>,
    pub val: Option<Value>,
}
#[derive(Debug, Clone)]
pub enum Value {
    Int16(i16),
    Int32(i32),
    Float(f32),
    Register(RegisterType),
    Symbol(String),
    Argument(String),
    FnRef(String),
    Block(String),
    Array(Vec<Value>),
    Global(String),
}
#[derive(Debug, Clone)]
pub struct Command {
    pub op: CommandType,
    pub values: Vec<Value>,
}
#[derive(Debug, Clone)]
enum SymbolTableEntry {
    Symbol(usize),
    Global,
    Argument(usize),
    Fn,
}
#[derive(Debug, Clone)]
struct SymbolTable {
    symbols: Vec<IndexMap<String, SymbolTableEntry>>,
}
impl SymbolTable {
    fn new() -> Self {
        SymbolTable {
            symbols: vec![IndexMap::new()],
        }
    }
    fn enter_scope(&mut self) {
        self.symbols.push(IndexMap::new());
    }
    fn exit_scope(&mut self) {
        self.symbols.pop();
    }
    fn insert(&mut self, name: String, entry: SymbolTableEntry) {
        self.symbols.last_mut().unwrap().insert(name, entry);
    }
    fn lookup(&self, name: &str) -> Option<SymbolTableEntry> {
        for scope in self.symbols.iter().rev() {
            if let Some(r) = scope.get(name) {
                return Some((*r).clone());
            }
        }
        None
    }
}
impl Parser {
    pub fn new(input: Vec<Token>, file_name: String) -> Parser {
        Parser {
            input,
            pos: 0,
            file_name,
            error: false,
            symbols: SymbolTable { symbols: vec![] },
            block_labels: vec![],
        }
    }
    pub fn parse(&mut self) -> Vec<Statement> {
        let mut statements = vec![];
        while self.pos < self.input.len() {
            statements.push(self.parseStatement());
        }
        return statements
            .iter()
            .filter(|x| x.is_some())
            .map(|x| x.clone().unwrap())
            .collect();
    }
    fn parseStatement(&mut self) -> Option<Statement> {
        let tk = self.next().clone();
        match tk.kind {
            TokenKind::Command(op) => self.parseCommand(op, tk.srcLoc),
            TokenKind::Fn => self.parseFunction(tk.srcLoc),
            TokenKind::Symbol => self.parseSymbolDef(tk.srcLoc),
            TokenKind::Global => self.parseGlobalDef(tk.srcLoc),
            TokenKind::Identifier(label) => self.parseBlock(label, tk.srcLoc),
            TokenKind::Import => {
                if let TokenKind::String(str) = self.peek().kind.clone() {
                    self.next();
                    self.matchToken(TokenKind::Semicolon);
                    Some(Statement {
                        kind: StatementKind::Import(str),
                        loc: tk.srcLoc,
                    })
                } else {
                    self.emitError(&format!("Expected string, found {:?}", self.peek().kind));
                    None
                }
            }
            _ => None,
        }
    }
    fn parseGlobalDef(&mut self, loc: SourceLocation) -> Option<Statement> {
        let name = self.matchIdentifier()?;
        let mut size: Option<usize> = None;
        let mut val: Option<Value> = None;
        match self.peek().kind {
            TokenKind::Colon => {
                self.next();
                size = Some(self.matchInteger()? as usize);
                if self.peek().kind == TokenKind::Equals {
                    self.next();
                    let valtk = self.next().clone();
                    val = Some(self.parse_value(valtk)?);
                }
            }
            TokenKind::Equals => {
                self.next();
                let valtk = self.next().clone();
                val = Some(self.parse_value(valtk)?);
            }
            _ => {
                self.emitError("Expected ':' or '=' after global name");
                return None;
            }
        }
        self.matchToken(TokenKind::Semicolon);
        if size.is_none() && val.is_none() {
            self.emitError("Global definition requires a size or initializer");
            return None;
        }
        self.symbols
            .insert(name.clone(), SymbolTableEntry::Global);
        Some(Statement {
            kind: StatementKind::GlobalDef(GlobalDef { name, size, val }),
            loc,
        })
    }
    fn parseSymbolDef(&mut self, loc: SourceLocation) -> Option<Statement> {
        let name = self.matchIdentifier()?;
        self.matchToken(TokenKind::Colon);
        let size = self.matchInteger()? as usize;
        self.symbols
            .insert(name.clone(), SymbolTableEntry::Symbol(size));
        self.matchToken(TokenKind::Semicolon);
        Some(Statement {
            kind: StatementKind::SymbolDef(SymbolDef { name, size }),
            loc,
        })
    }
    fn parseBlock(&mut self, name: String, loc: SourceLocation) -> Option<Statement> {
        self.matchToken(TokenKind::Colon);
        self.matchToken(TokenKind::LBrace);
        let mut stmts = vec![];
        while self.peek().kind != TokenKind::RBrace {
            stmts.push(self.parseStatement()?);
        }
        self.matchToken(TokenKind::RBrace);
        return Some(Statement {
            kind: StatementKind::Block(Block { name, stmts }),
            loc,
        });
    }
    fn parseFunction(&mut self, loc: SourceLocation) -> Option<Statement> {
        let name = self.matchIdentifier()?;
        let mut args: IndexMap<String, usize> = IndexMap::new();
        self.matchToken(TokenKind::LParen);
        self.symbols.enter_scope();
        while self.peek().kind != TokenKind::RParen {
            let name = self.matchIdentifier()?;
            self.matchToken(TokenKind::Colon);
            let size = self.matchInteger()?;
            args.insert(name.clone(), size as usize);
            self.symbols
                .insert(name, SymbolTableEntry::Argument(size as usize));
            if self.peek().kind != TokenKind::RParen {
                self.matchToken(TokenKind::Comma);
            }
        }
        self.next();
        self.matchToken(TokenKind::Arrow);
        let returned_bytes = self.matchInteger()? as usize;
        self.matchToken(TokenKind::LBrace);
        self.symbols.insert(name.clone(), SymbolTableEntry::Fn);
        self.block_labels
            .push(self.collect_block_labels(self.pos));
        let mut stmts = vec![];
        while self.peek().kind != TokenKind::RBrace {
            match self.parseStatement() {
                Some(stmt) => stmts.push(stmt),
                None => {
                    self.block_labels.pop();
                    self.symbols.exit_scope();
                    return None;
                }
            }
        }
        self.block_labels.pop();
        self.symbols.exit_scope();
        self.matchToken(TokenKind::RBrace);

        let mut blocks = vec![];
        for stmt in &stmts {
            if let StatementKind::Block(block) = &stmt.kind {
                blocks.push(block.name.clone());
            }
        }

        Some(Statement {
            kind: StatementKind::Function(Function {
                name,
                args,
                body: stmts,
                returned_bytes,
                blocks,
            }),
            loc,
        })
    }
    fn parseCommand(&mut self, op: CommandType, loc: SourceLocation) -> Option<Statement> {
        let mut values = vec![];
        while self.peek().kind != TokenKind::Semicolon {
            let tk = self.next().clone();
            values.push(self.parse_value(tk)?);
            if self.peek().kind != TokenKind::Semicolon {
                self.matchToken(TokenKind::Comma);
            }
        }
        let expected = Parser::command_arg_count(op);
        if values.len() != expected {
            self.emitError(&format!(
                "Expected {} argument(s) for {:?}, found {}",
                expected,
                op,
                values.len()
            ));
            self.matchToken(TokenKind::Semicolon);
            return None;
        }
        self.matchToken(TokenKind::Semicolon);
        Some(Statement {
            kind: StatementKind::Command(Command { op, values }),
            loc,
        })
    }
    fn parse_value(&mut self, tk: Token) -> Option<Value> {
        let r = match tk.kind {
            TokenKind::Int(i) => Some(Value::Int16(i as i16)),
            TokenKind::Int32(i) => Some(Value::Int32(i)),
            TokenKind::Float(f) => Some(Value::Float(f)),
            TokenKind::Register(r) => Some(Value::Register(r)),
            TokenKind::String(val) => Some(Value::Array(
                [val.into_bytes(), vec![0 as u8]]
                    .concat()
                    .iter()
                    .map(|x| Value::Int16(*x as i16))
                    .collect(),
            )),
            TokenKind::LBrace => {
                let mut vals = vec![];
                while self.peek().kind != TokenKind::RBrace {
                    let tk = self.next().clone();
                    vals.push(self.parse_value(tk)?);
                    if self.peek().kind != TokenKind::RBrace {
                        self.matchToken(TokenKind::Comma);
                    }
                }
                self.next();
                Some(Value::Array(vals))
            }
            TokenKind::Identifier(ident) => {
                let sym = self.symbols.lookup(ident.as_str());
                if let Some(symbol) = sym {
                    Some(match symbol {
                        SymbolTableEntry::Symbol(_) => Value::Symbol(ident),
                        SymbolTableEntry::Global => Value::Global(ident),
                        SymbolTableEntry::Argument(_) => Value::Argument(ident),
                        SymbolTableEntry::Fn => Value::FnRef(ident),
                    })
                } else if self.is_block_label(&ident) {
                    Some(Value::Block(ident))
                } else {
                    self.emitError(&format!("No such identifier, {:?}", ident));
                    None
                }
            }
            _ => {
                self.emitError(&format!("Expected value, not {:?}", tk));
                None
            }
        };
        r
    }
    fn peek(&self) -> &Token {
        if self.pos >= self.input.len() {
            &self.input[self.input.len() - 1]
        } else {
            &self.input[self.pos]
        }
    }
    fn next(&mut self) -> &Token {
        self.pos += 1;
        &self.input[self.pos - 1]
    }
    fn matchToken(&mut self, kind: TokenKind) -> Option<&Token> {
        if self.peek().kind == kind {
            Some(self.next())
        } else {
            self.emitError(&format!(
                "Expected {:?}, found {:?}",
                kind,
                self.peek().kind
            ));
            None
        }
    }
    fn matchIdentifier(&mut self) -> Option<String> {
        if let TokenKind::Identifier(x) = &(self.peek().kind) {
            let name = x.clone();
            self.next();
            Some(name)
        } else {
            self.emitError(&format!(
                "Expected identifier, found {:?}",
                self.peek().kind
            ));
            None
        }
    }
    fn matchInteger(&mut self) -> Option<i32> {
        if let TokenKind::Int(n) = self.peek().kind {
            self.next();
            Some(n)
        } else {
            self.emitError(&format!("Expected int, found {:?}", self.peek().kind));
            None
        }
    }
    fn collect_block_labels(&self, start_pos: usize) -> HashSet<String> {
        let mut labels = HashSet::new();
        let mut depth = 0usize;
        let mut pos = start_pos;
        while pos < self.input.len() {
            match &self.input[pos].kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                TokenKind::Identifier(name) => {
                    if matches!(
                        self.input.get(pos + 1).map(|t| &t.kind),
                        Some(TokenKind::Colon)
                    ) && matches!(
                        self.input.get(pos + 2).map(|t| &t.kind),
                        Some(TokenKind::LBrace)
                    ) {
                        labels.insert(name.clone());
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        labels
    }
    fn is_block_label(&self, name: &str) -> bool {
        self.block_labels
            .last()
            .map(|labels| labels.contains(name))
            .unwrap_or(false)
    }
    fn emitError(&mut self, message: &str) {
        let loc = self.input[self.pos].srcLoc;
        println!(
            "Error while parsing at {} {}:{}:\n{}",
            self.file_name, loc.line, loc.col, message
        );
        self.error = true;
    }
    fn command_arg_count(op: CommandType) -> usize {
        match op {
            CommandType::Exit | CommandType::NOP => 0,
            CommandType::Not
            | CommandType::NotEx
            | CommandType::Push
            | CommandType::Pushf
            | CommandType::PushEx
            | CommandType::Pop
            | CommandType::Jump
            | CommandType::Call => 1,
            CommandType::Return => 3,
            CommandType::Add
            | CommandType::Sub
            | CommandType::Mul
            | CommandType::Div
            | CommandType::Mod
            | CommandType::Addf
            | CommandType::Subf
            | CommandType::Mulf
            | CommandType::Divf
            | CommandType::AddEx
            | CommandType::SubEx
            | CommandType::MulEx
            | CommandType::DivEx
            | CommandType::AddU
            | CommandType::SubU
            | CommandType::MulU
            | CommandType::DivU
            | CommandType::AddExU
            | CommandType::SubExU
            | CommandType::MulExU
            | CommandType::DivExU
            | CommandType::And
            | CommandType::Or
            | CommandType::Xor
            | CommandType::AndEx
            | CommandType::OrEx
            | CommandType::XorEx
            | CommandType::Greater
            | CommandType::LessThan
            | CommandType::Equals
            | CommandType::Shl
            | CommandType::Shr
            | CommandType::ShlEx
            | CommandType::ShrEx
            | CommandType::Store
            | CommandType::StoreEx
            | CommandType::Storef
            | CommandType::Mov
            | CommandType::Load
            | CommandType::LoadEx
            | CommandType::Loadf
            | CommandType::JumpNotZero
            | CommandType::JumpZero
            | CommandType::IO => 2,
            CommandType::R1
            | CommandType::R2
            | CommandType::R3
            | CommandType::R4
            | CommandType::R5
            | CommandType::F1
            | CommandType::F2
            | CommandType::IP
            | CommandType::SP
            | CommandType::SRP
            | CommandType::ARP
            | CommandType::EX1
            | CommandType::EX2 => 0,
        }
    }
}
