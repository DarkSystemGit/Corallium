use super::parser::{BinaryOperator, UnaryOperator};
use crate::compiler::lexer::{
    KeywordKind, Lexer, OperatorKind, SourceLocation, Token, TokenKind, TypeKind,
};
use crate::compiler::parser::{Declaration, Expression, Literal, Parser, Statement, StatementKind};
use rand::distr::Alphanumeric;
use rand::{Rng, thread_rng};
use std::collections::HashMap;
#[derive(Clone, Debug)]
struct Register {
    id: u16,
    ty: TypeKind,
}
#[derive(Clone, Debug)]
struct Immediate {
    value: f64,
    ty: TypeKind,
}
#[derive(Clone, Debug)]
enum Value {
    Register(Register),
    Immediate(Immediate),
    Location(Location),
}
type Output = Register;
#[derive(Clone, Debug)]
enum Location {
    Block(usize),
    Symbol(usize, usize),
    Function(String),
}
#[derive(Clone, Debug)]
pub enum Command {
    Add(Value, Value, Output),
    Sub(Value, Value, Output),
    Mul(Value, Value, Output),
    Div(Value, Value, Output),
    Mod(Value, Value, Output),
    And(Value, Value, Output),
    Or(Value, Value, Output),
    Not(Value, Output),
    Xor(Value, Value, Output),
    Shl(Value, Value, Output),
    Shr(Value, Value, Output),
    Gt(Value, Value, Output),
    Lt(Value, Value, Output),
    Eq(Value, Value, Output),
    Jump(Location),
    JumpTrue(Location, Value),
    JumpFalse(Location, Value),
    Call(Value, u8),
    Ret(Value),
    Load(Value, Output),
    Store(Value, Value),
    Push(Value),
    Pop(Output),
    Move(Value, Output),
}
#[derive(Clone, Debug)]
struct RegisterManager {
    registers: Vec<Register>,
}
impl RegisterManager {
    fn new() -> Self {
        RegisterManager {
            registers: Vec::new(),
        }
    }
    fn allocate_register(&mut self, ty: TypeKind) -> u16 {
        let id = self.registers.len() as u16;
        let register = Register { id, ty };
        self.registers.push(register);
        id
    }
    fn get_register(&self, id: u16) -> Option<Register> {
        Some((self.registers.get(id as usize)?).clone())
    }
    fn deallocate_register(&mut self, id: u16) {
        self.registers.remove(id as usize);
    }
}
struct Symbol {
    name: String,
    body: Definition,
    id: usize,
}
enum Definition {
    User(UserType),
    Var(TypeKind),
}
enum UserType {
    Struct(HashMap<String, TypeKind>),
    Enum(Vec<String>),
    Union(HashMap<String, TypeKind>),
}
struct SymbolTable {
    scopes: Vec<Vec<Symbol>>,
    next_id: usize,
}
impl SymbolTable {
    fn new() -> Self {
        SymbolTable {
            scopes: vec![Vec::new()],
            next_id: 0,
        }
    }
    fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    fn define_var(&mut self, name: String, ty: TypeKind) {
        let symbol = Symbol {
            name,
            body: Definition::Var(ty),
            id: self.next_id,
        };
        self.next_id += 1;
        self.scopes.last_mut().unwrap().push(symbol);
    }
    fn define_user_type(&mut self, name: String, ty: UserType) {
        let symbol = Symbol {
            name,
            body: Definition::User(ty),
            id: self.next_id,
        };
        self.next_id += 1;
        self.scopes.last_mut().unwrap().push(symbol);
    }
    fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.iter().find(|s| s.name == name) {
                return Some(symbol);
            }
        }
        None
    }
    fn get_symbol(&self, id: usize) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.iter().find(|s| s.id == id) {
                return Some(symbol);
            }
        }
        None
    }
}
struct Function {
    name: String,
    parameters: Vec<String>,
    body: Vec<Command>,
}
struct Compiler {
    symbol_table: SymbolTable,
    register_manager: RegisterManager,
    input: Vec<Statement>,
    functions: Vec<Function>,
    imports: Vec<String>,
    file_path: String,
    current_fn: usize,
    emit: bool,
}
impl Compiler {
    fn new(filename: String, input: String) -> Self {
        let mut parser = Parser::new(input, filename.clone());
        Compiler {
            symbol_table: SymbolTable::new(),
            register_manager: RegisterManager::new(),
            input: parser.parse(),
            functions: vec![Function {
                name: "_start".to_string(),
                parameters: vec![],
                body: vec![],
            }],
            imports: Vec::new(),
            file_path: filename,
            current_fn: 0,
            emit: true,
        }
    }
    fn compile_block(&mut self, statements: Vec<Statement>) {
        self.symbol_table.push_scope();
        for statement in statements.iter() {
            match statement.kind {
                StatementKind::Expression(expr) => {
                    self.compile_expression(expr, statement.loc);
                }
                StatementKind::Declaration(declaration) => {
                    self.compile_declaration(declaration, statement.loc);
                }
                StatementKind::Block(stmts) => {
                    self.compile_block(stmts);
                }
                StatementKind::Return(stmt) => {
                    self.compile_return(stmt, statement.loc);
                }
                StatementKind::Break => {
                    self.compile_break(statement.loc);
                }
                StatementKind::Continue => {
                    self.compile_continue(statement.loc);
                }
                StatementKind::If(stmt) => {
                    self.compile_if(stmt, statement.loc);
                }
                StatementKind::While(stmt) => {
                    self.compile_while(stmt, statement.loc);
                }
                StatementKind::For(stmt) => {
                    self.compile_for(stmt, statement.loc);
                }
                StatementKind::Enum(decl) => {
                    self.compile_enum(decl, statement.loc);
                }
                StatementKind::Function(fun) => {
                    self.compile_function(fun, statement.loc);
                }
                StatementKind::Import(import) => {
                    self.imports.push(import.path);
                }
                StatementKind::Struct(decl) => {
                    self.compile_struct(decl, statement.loc);
                }
                StatementKind::Union(decl) => {
                    self.compile_union(decl, statement.loc);
                }
            }
        }
        self.symbol_table.pop_scope();
    }
    fn compile_expression(&mut self, expr: Expression, loc: SourceLocation) -> Option<Output> {
        match expr {
            Expression::Binary(left, op, right) => {
                if let BinaryOperator::PropertyAccess = op {
                    let obj = self.compile_expression(*left, loc)?;
                    if let TypeKind::Pointer(ptr) = obj.ty {
                        if let TypeKind::Struct(strct) = *ptr {
                            let ty = self.symbol_table.lookup_symbol(&strct)?;
                            if let Definition::User(UserType::Struct(utype)) = ty.body {
                                if let Expression::Identifier(property) = *right {
                                    let fieldTy = utype.get(&property);
                                    if let Some(fieldTy) = fieldTy {
                                        let offset = utype.keys().fold(0, |acc, k| {
                                            acc + self.size_of(utype[k], loc).unwrap_or(0)
                                        });
                                        let temp = self.register_manager.get_register(
                                            self.register_manager
                                                .allocate_register(TypeKind::Int32),
                                        )?;
                                        let reg = self.register_manager.get_register(
                                            self.register_manager.allocate_register(*fieldTy),
                                        )?;
                                        self.emit_instruction(Command::Add(
                                            Value::Immediate(Immediate {
                                                value: offset as f64,
                                                ty: TypeKind::Int32,
                                            }),
                                            Value::Register(obj),
                                            temp,
                                        ));
                                        self.emit_instruction(Command::Load(
                                            Value::Register(temp),
                                            reg,
                                        ));
                                        self.register_manager.deallocate_register(temp.id);
                                        return Some(reg);
                                    } else {
                                        self.emitError(
                                            loc,
                                            &format!(
                                                "No such property on struct {}, {}",
                                                strct, property
                                            ),
                                        );
                                    }
                                } else {
                                    self.emitError(loc, "Invalid property access");
                                }
                            } else {
                                self.emitError(
                                    loc,
                                    &format!(
                                        "Cannot access property of non-defined struct, {}",
                                        strct
                                    ),
                                );
                            }
                        } else {
                            self.emitError(
                                loc,
                                &format!("Cannot access property of non-struct type, {:?} is not a struct", *ptr),
                            );
                        }
                    } else {
                        self.emitError(loc, "Cannot access property of non-struct type");
                    }
                    None
                } else {
                    let outL = self.compile_expression(*left, loc)?;
                    let outR = self.compile_expression(*right, loc)?;
                    if outL.ty != outR.ty {
                        self.emitError(
                            loc,
                            &format!(
                                "Type mismatch, type {:?} and {:?} aren't equivalent",
                                outL.ty, outR.ty
                            ),
                        );
                        return None;
                    }
                    let left = self.convert_output_to_value(outL);
                    let right = self.convert_output_to_value(outR);
                    let out = self.register_manager.allocate_register(outL.ty);
                    match op {
                        BinaryOperator::Add => self.emit_instruction(Command::Add(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Sub => self.emit_instruction(Command::Sub(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Mul => self.emit_instruction(Command::Mul(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Div => self.emit_instruction(Command::Div(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Mod => self.emit_instruction(Command::Mod(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::And => self.emit_instruction(Command::And(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Or => self.emit_instruction(Command::Or(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Xor => self.emit_instruction(Command::Xor(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Shl => self.emit_instruction(Command::Shl(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Shr => self.emit_instruction(Command::Shr(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Eq => self.emit_instruction(Command::Eq(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Ne => {
                            let temp = self.register_manager.allocate_register(outL.ty);
                            self.emit_instruction(Command::Eq(
                                left,
                                right,
                                self.register_manager.get_register(temp)?,
                            ));
                            self.emit_instruction(Command::Not(
                                self.convert_output_to_value(
                                    self.register_manager.get_register(temp)?,
                                ),
                                self.register_manager.get_register(out)?,
                            ));
                            self.register_manager.deallocate_register(temp);
                        }
                        BinaryOperator::Lt => self.emit_instruction(Command::Lt(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Gt => self.emit_instruction(Command::Gt(
                            left,
                            right,
                            self.register_manager.get_register(out)?,
                        )),
                        BinaryOperator::Le => {
                            let tempA = self.register_manager.allocate_register(outL.ty);
                            let tempB = self.register_manager.allocate_register(outL.ty);
                            self.emit_instruction(Command::Lt(
                                left,
                                right,
                                self.register_manager.get_register(tempA)?,
                            ));
                            self.emit_instruction(Command::Eq(
                                left,
                                right,
                                self.register_manager.get_register(tempB)?,
                            ));
                            self.emit_instruction(Command::Or(
                                self.convert_output_to_value(
                                    self.register_manager.get_register(tempA)?,
                                ),
                                self.convert_output_to_value(
                                    self.register_manager.get_register(tempB)?,
                                ),
                                self.register_manager.get_register(out)?,
                            ));
                            self.register_manager.deallocate_register(tempA);
                            self.register_manager.deallocate_register(tempB);
                        }

                        BinaryOperator::Ge => {
                            let tempA = self.register_manager.allocate_register(outL.ty);
                            let tempB = self.register_manager.allocate_register(outL.ty);
                            self.emit_instruction(Command::Gt(
                                left,
                                right,
                                self.register_manager.get_register(tempA)?,
                            ));
                            self.emit_instruction(Command::Eq(
                                left,
                                right,
                                self.register_manager.get_register(tempB)?,
                            ));
                            self.emit_instruction(Command::Or(
                                self.convert_output_to_value(
                                    self.register_manager.get_register(tempA)?,
                                ),
                                self.convert_output_to_value(
                                    self.register_manager.get_register(tempB)?,
                                ),
                                self.register_manager.get_register(out)?,
                            ));
                            self.register_manager.deallocate_register(tempA);
                            self.register_manager.deallocate_register(tempB);
                        }
                        _ => {}
                    }
                    self.register_manager.get_register(out)
                }
            }
            Expression::Unary(op, expr) => {
                let left = self.compile_expression(*expr, loc)?;
                let mut out = self
                    .register_manager
                    .get_register(self.register_manager.allocate_register(left.ty))?;
                let value_l = self.convert_output_to_value(left);
                match op {
                    UnaryOperator::Neg => {
                        self.emit_instruction(Command::Mod(
                            value_l,
                            Value::Immediate(Immediate {
                                value: -1.0,
                                ty: TypeKind::Int16,
                            }),
                            out,
                        ));
                        self.register_manager.deallocate_register(left.id);
                    }
                    UnaryOperator::Not => {
                        self.emit_instruction(Command::Not(value_l, out));
                        self.register_manager.deallocate_register(left.id);
                    }
                    UnaryOperator::Deref => {
                        if let TypeKind::Pointer(x) = left.ty {
                            self.register_manager.deallocate_register(out.id);
                            out = self
                                .register_manager
                                .get_register(self.register_manager.allocate_register(*x))?;
                            self.emit_instruction(Command::Load(value_l, out));
                            self.register_manager.deallocate_register(left.id);
                        } else {
                            return None;
                        }
                    }
                }
                Some(out)
            }
            Expression::FunctionCall(func, args) => {
                let argc = args.len() as u8;
                let args = args
                    .into_iter()
                    .rev()
                    .map(|arg| {
                        let reg = self.compile_expression(arg, loc).unwrap_or(Register {
                            id: 0,
                            ty: TypeKind::Void,
                        });
                        self.emit_instruction(Command::Push(Value::Register(reg)));
                        self.register_manager.deallocate_register(reg.id);
                        reg.ty
                    })
                    .collect::<Vec<TypeKind>>();
                let func = self.compile_expression(*func, loc)?;
                let (params, ret) = self.unwrap_fn_type(func.ty, loc)?;
                if params.len() as u8 != argc {
                    self.emitError(loc, "function call argument count mismatch");
                }
                params.iter().zip(args).for_each(|(param, arg)| {
                    if *param != arg {
                        self.emitError(
                            loc,
                            &format!(
                                "function call argument type mismatch: expected {:?}, got {:?}",
                                param, arg
                            ),
                        );
                    }
                });
                self.emit_instruction(Command::Call(self.convert_output_to_value(func), argc));
                self.register_manager.deallocate_register(func.id);
                self.emit_instruction(Command::Pop(
                    self.register_manager
                        .get_register(self.register_manager.allocate_register(ret))?,
                ));
                None
            }
            Expression::Grouped(expr) => self.compile_expression(*expr, loc),
            Expression::Literal(lit) => match lit {
                Literal::Int(value) => {
                    let ty = match value > (i16::MAX as i32) {
                        true => TypeKind::Int32,
                        false => TypeKind::Int16,
                    };
                    let out = self
                        .register_manager
                        .get_register(self.register_manager.allocate_register(ty))?;
                    self.emit_instruction(Command::Move(
                        Value::Immediate(Immediate {
                            value: value as f64,
                            ty,
                        }),
                        out,
                    ));
                    Some(out)
                }
                Literal::Bool(val) => {
                    let out = self
                        .register_manager
                        .get_register(self.register_manager.allocate_register(TypeKind::Int16))?;
                    self.emit_instruction(Command::Move(
                        Value::Immediate(Immediate {
                            value: match val {
                                true => 1.0,
                                false => 0.0,
                            },
                            ty: TypeKind::Int16,
                        }),
                        out,
                    ));
                    Some(out)
                }
                Literal::Float(f) => {
                    let out = self
                        .register_manager
                        .get_register(self.register_manager.allocate_register(TypeKind::Float32))?;
                    self.emit_instruction(Command::Move(
                        Value::Immediate(Immediate {
                            value: f as f64,
                            ty: TypeKind::Float32,
                        }),
                        out,
                    ));
                    Some(out)
                }
                Literal::Array(arr) => {
                    let name = format!(
                        "__internal_array_{}",
                        rand::thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(32)
                            .map(|c| c as char)
                            .collect::<String>()
                    );
                    self.emit = false;
                    let ty = self.compile_expression(arr[0], loc)?.ty;
                    let size = self.size_of(ty.clone(), loc.clone())?;
                    self.symbol_table
                        .define_var(name, TypeKind::Array(Box::new(ty), arr.len()));
                    let symbol = self.symbol_table.lookup_symbol(&name)?;
                    self.emit = true;
                    for (i, expr) in arr.iter().enumerate() {
                        let reg = self.compile_expression(*expr, loc)?;
                        self.emit_instruction(Command::Store(
                            self.convert_output_to_value(reg),
                            Value::Location(Location::Symbol(symbol.id, i * size)),
                        ));
                        self.register_manager.deallocate_register(reg.id);
                    }
                    let out = self.register_manager.get_register(
                        self.register_manager
                            .allocate_register(TypeKind::Pointer(Box::new(TypeKind::Array(
                                Box::new(ty),
                                arr.len(),
                            )))),
                    )?;
                    self.emit_instruction(Command::Move(
                        Value::Location(Location::Symbol(symbol.id, 0)),
                        out,
                    ));
                    Some(out)
                }
                Literal::String(str) => {
                    let name = format!(
                        "__internal_str_{}",
                        rand::thread_rng()
                            .sample_iter(&Alphanumeric)
                            .take(32)
                            .map(|c| c as char)
                            .collect::<String>()
                    );
                    self.symbol_table.define_var(
                        name,
                        TypeKind::Array(Box::new(TypeKind::Char), str.len() + 1),
                    );
                    let symbol = self.symbol_table.lookup_symbol(&name)?;
                    for (i, c) in str.chars().enumerate() {
                        self.emit_instruction(Command::Store(
                            Value::Immediate(Immediate {
                                value: c as u8 as f64,
                                ty: TypeKind::Char,
                            }),
                            Value::Location(Location::Symbol(symbol.id, i)),
                        ));
                    }
                    let out = self.register_manager.get_register(
                        self.register_manager
                            .allocate_register(TypeKind::Pointer(Box::new(TypeKind::Array(
                                Box::new(TypeKind::Char),
                                str.len() + 1,
                            )))),
                    )?;
                    self.emit_instruction(Command::Move(
                        Value::Location(Location::Symbol(symbol.id, 0)),
                        out,
                    ));
                    Some(out)
                }
                Literal::Struct(name, fields) => {
                    let ty = self.symbol_table.lookup_symbol(&name)?;
                    if let Definition::User(UserType::Struct(def_fields)) = ty.body {
                        let sname = format!(
                            "__internal_struct_{}",
                            rand::thread_rng()
                                .sample_iter(&Alphanumeric)
                                .take(32)
                                .map(|c| c as char)
                                .collect::<String>()
                        );
                        self.symbol_table.define_var(sname, TypeKind::Struct(name));
                        let symbol = self.symbol_table.lookup_symbol(&sname)?;
                        let mut offset = 0;
                        for (field, value) in fields {
                            let expr = self.compile_expression(value, loc)?;
                            if expr.ty == def_fields[&field] {
                                self.emit_instruction(Command::Store(
                                    self.convert_output_to_value(expr),
                                    Value::Location(Location::Symbol(symbol.id, offset)),
                                ));
                                self.register_manager.deallocate_register(expr.id);
                                offset += self.size_of(expr.ty, loc)?;
                            } else {
                                self.emitError(loc, &format!("Type mismatch for field {}", field));
                                return None;
                            }
                        }
                        let out = self.register_manager.get_register(
                            self.register_manager
                                .allocate_register(TypeKind::Pointer(Box::new(TypeKind::Struct(
                                    name,
                                )))),
                        )?;
                        self.emit_instruction(Command::Move(
                            Value::Location(Location::Symbol(symbol.id, 0)),
                            out,
                        ));
                        return Some(out);
                    } else {
                        self.emitError(loc, &format!("No such struct {}", name));
                    }
                    None
                }
                Literal::Union(name, variant, expr) => {
                    let type_def = self.symbol_table.lookup_symbol(&name)?;
                    if let Definition::User(UserType::Union(def)) = type_def.body {
                        let expr = self.compile_expression(*expr, loc)?;
                        let variant_type = def.get(&variant);
                        if let Some(variant_type) = variant_type {
                            let sname = format!(
                                "__internal_union_{}",
                                rand::thread_rng()
                                    .sample_iter(&Alphanumeric)
                                    .take(32)
                                    .map(|c| c as char)
                                    .collect::<String>()
                            );
                            self.symbol_table.define_var(
                                sname,
                                TypeKind::TaggedUnion(
                                    def.keys().position(|x| *x == variant)?,
                                    def.values().map(|x| *x).collect(),
                                ),
                            );
                            let symbol = self.symbol_table.lookup_symbol(&sname).unwrap();
                            self.emit_instruction(Command::Store(
                                Value::Immediate(Immediate {
                                    value: def.keys().position(|x| *x == variant)? as f64,
                                    ty: TypeKind::Int16,
                                }),
                                Value::Location(Location::Symbol(symbol.id, 0)),
                            ));
                            self.emit_instruction(Command::Store(
                                Value::Register(expr),
                                Value::Location(Location::Symbol(symbol.id, 1)),
                            ));
                            self.register_manager.deallocate_register(expr.id);
                        } else {
                            self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                        }
                    } else {
                        self.emitError(loc, &format!("No such union {}", name));
                    }

                    None
                }
                Literal::Enum(name, variant) => {
                    let def = self.symbol_table.lookup_symbol(&name);
                    if let Some(def) = def {
                        if let Definition::User(UserType::Enum(variants)) = def.body {
                            let variant_def = variants.iter().position(|v| v.name == variant);
                            if variant_def.is_none() {
                                self.emitError(
                                    loc,
                                    &format!("No such variant {}::{}", name, variant),
                                );
                                return None;
                            }
                            let out = self.register_manager.get_register(
                                self.register_manager.allocate_register(TypeKind::Int16),
                            )?;
                            self.emit_instruction(Command::Move(
                                Value::Immediate(Immediate {
                                    value: variant_def.unwrap() as f64,
                                    ty: TypeKind::Int16,
                                }),
                                out,
                            ));
                            return Some(out);
                        } else {
                            self.emitError(loc, &format!("No such enum {}", name));
                        }
                    } else {
                        self.emitError(loc, &format!("No such enum {}", name));
                    }

                    None
                }
            },
            Expression::Identifier(ident) => {
                let symbol = self.symbol_table.lookup_symbol(&ident)?;
                if let Definition::Var(ty) = symbol.body {
                    let loc = Value::Location(Location::Symbol(symbol.id, 0));
                    let reg = self
                        .register_manager
                        .get_register(self.register_manager.allocate_register(ty))?;
                    self.emit_instruction(Command::Load(loc, reg));
                    Some(reg)
                } else {
                    None
                }
            }
            Expression::Cast(ty, expr) => {
                let out = self
                    .register_manager
                    .get_register(self.register_manager.allocate_register(ty))?;
                let prev = self.compile_expression(*expr, loc)?;
                self.emit_instruction(Command::Move(self.convert_output_to_value(prev), out));
                self.register_manager.deallocate_register(prev.id);
                Some(out)
            }
            Expression::AddressOf(ident) => {
                let symbol = self.symbol_table.lookup_symbol(&ident)?;
                if let Definition::Var(ty) = symbol.body {
                    let loc = Value::Location(Location::Symbol(symbol.id, 0));
                    let reg = self.register_manager.get_register(
                        self.register_manager
                            .allocate_register(TypeKind::Pointer(Box::new(ty))),
                    )?;
                    self.emit_instruction(Command::Move(loc, reg));
                    Some(reg)
                } else {
                    None
                }
            }
            Expression::Subscript(array, index) => {
                let arrayptr = self.compile_expression(*array, loc)?;
                if let TypeKind::Pointer(array) = arrayptr.ty {
                    if let TypeKind::Array(ty, count) = *array {
                        let size = Value::Immediate(Immediate {
                            value: self.size_of(*ty, loc)? as f64,
                            ty: TypeKind::Int32,
                        });
                        let indexR = self.compile_expression(*index, loc)?;
                        let index = self.convert_output_to_value(indexR);
                        let offset = self.register_manager.get_register(
                            self.register_manager.allocate_register(TypeKind::Int32),
                        )?;
                        self.emit_instruction(Command::Mul(index, size, offset));
                        self.register_manager.deallocate_register(indexR.id);
                        let addr = self.register_manager.get_register(
                            self.register_manager
                                .allocate_register(TypeKind::Pointer(Box::new(*ty))),
                        )?;
                        self.emit_instruction(Command::Add(
                            Value::Register(arrayptr),
                            Value::Register(offset),
                            addr,
                        ));
                        self.register_manager.deallocate_register(offset.id);
                        self.register_manager.deallocate_register(arrayptr.id);
                        let value = self
                            .register_manager
                            .get_register(self.register_manager.allocate_register(*ty))?;
                        self.emit_instruction(Command::Load(Value::Register(addr), value));
                        self.register_manager.deallocate_register(addr.id);
                        Some(value)
                    } else {
                        self.emitError(loc, "Type mismatch, expected array");
                        None
                    }
                } else {
                    self.emitError(loc, "Type mismatch, expected array");
                    None
                }
            }
        }
    }
    fn emit_instruction(&mut self, instruction: Command) {
        if self.emit {
            self.functions[self.current_fn].body.push(instruction);
        }
    }
    fn convert_output_to_value(&mut self, output: Output) -> Value {
        Value::Register(output)
    }
    fn emitError(&self, loc: SourceLocation, message: &str) {
        if self.emit {
            println!(
                "Error while compiling at {} {}:{}:\n{}",
                self.file_path, loc.line, loc.col, message
            );
        }
    }
    fn unwrap_fn_type(
        &self,
        ty: TypeKind,
        loc: SourceLocation,
    ) -> Option<(Vec<TypeKind>, TypeKind)> {
        match ty {
            TypeKind::Pointer(ptr) => match *ptr {
                TypeKind::Function(params, ret) => return Some((params, *ret)),
                _ => self.emitError(loc, &format!("Expected function type, got {:?}", ptr)),
            },
            _ => self.emitError(loc, &format!("Expected function type, got {:?}", ty)),
        }
        None
    }
    fn size_of(&self, ty: TypeKind, loc: SourceLocation) -> Option<usize> {
        match ty {
            TypeKind::Int16 => Some(1),
            TypeKind::Uint16 => Some(1),
            TypeKind::Int32 => Some(2),
            TypeKind::Uint32 => Some(2),
            TypeKind::Char => Some(1),
            TypeKind::Float32 => Some(2),
            TypeKind::Pointer(_) => Some(2),
            TypeKind::Array(aty, size) => Some(size * self.size_of(*aty, loc)?),
            TypeKind::Struct(name) => {
                let struct_def = self.symbol_table.lookup_symbol(&name)?;
                if let Definition::User(UserType::Struct(fields)) = &struct_def.body {
                    fields
                        .iter()
                        .map(|(_, ty)| Some(self.size_of(ty.clone(), loc.clone())?))
                        .sum::<Option<usize>>()
                } else {
                    self.emitError(loc, &format!("Expected struct type"));
                    None
                }
            }
            TypeKind::Function(_, _) => None,
            TypeKind::Void => Some(0),
            TypeKind::Enum(_) => Some(1),
            TypeKind::Union(name) => {
                let union_def = self.symbol_table.lookup_symbol(&name)?;
                if let Definition::User(UserType::Union(fields)) = &union_def.body {
                    fields
                        .iter()
                        .map(|(_, ty)| Some(self.size_of(ty.clone(), loc.clone())?))
                        .max()?
                } else {
                    self.emitError(loc, &format!("Expected union type"));
                    None
                }
            }
            TypeKind::TaggedUnion(_id, fields) => fields
                .iter()
                .map(|ty| Some(self.size_of(ty.clone(), loc.clone())?))
                .max()?,
        }
    }
}
