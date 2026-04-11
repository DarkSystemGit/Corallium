use super::lexer::OperatorKind;
use super::parser::{
    BinaryOperator, EnumDeclaration, ForStatement, IfStatement, Pattern, ReturnStatement,
    StructDeclaration, UnaryOperator, UnionDeclaration, WhileStatement,
};
use crate::compiler::lexer::{SourceLocation, TypeKind};
use crate::compiler::parser::{
    Declaration, Expression, FunctionDeclaration, Literal, Parser, Statement, StatementKind,
};
use indexmap::IndexMap;
use rand::Rng;
use rand::distr::Alphanumeric;
use std::collections::HashMap;
use std::fs::{self, read_to_string};
use std::path::Path;
#[derive(Clone, Debug)]
pub struct Header {
    pub symbols: Vec<Symbol>,
    pub module: String,
}
#[derive(Clone, Debug)]
pub struct Register {
    pub id: u16,
    pub ty: TypeKind,
    pub start: [usize; 3],
    pub end: Option<[usize; 3]>,
}
#[derive(Clone, Debug)]
pub struct Immediate {
    pub value: f64,
    pub ty: TypeKind,
}
#[derive(Clone, Debug)]
pub enum Value {
    Register(Register),
    Immediate(Immediate),
    Location(Location),
    ARP,
}
pub type Output = Register;
#[derive(Clone, Debug)]
pub enum Location {
    Block(usize),
    Symbol(usize, usize),
    Function(usize),
    Argument(String),
    None,
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
    Ret(Option<Value>),
    Load(Value, Output),
    Store(Value, Value),
    Push(Value),
    Pop(Output),
    Move(Value, Output),
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: String,
    pub body: Definition,
    pub id: usize,
    pub size: Option<usize>,
}
#[derive(Clone, Debug)]
pub enum Definition {
    User(UserType),
    Var(TypeKind),
    Function(TypeKind, usize),
    Parameter(TypeKind),
}
#[derive(Clone, Debug)]
pub enum UserType {
    Struct(IndexMap<String, TypeKind>),
    Enum(Vec<String>),
    Union(IndexMap<String, TypeKind>),
}
#[derive(Debug, Clone, PartialEq)]
pub enum ImplicitParamType {
    ReturnPassthorugh,
}
#[derive(Debug, Clone)]
pub struct ImplicitParam {
    pub name: Option<String>,
    pub ty: TypeKind,
    pub param_ty: ImplicitParamType,
}
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub body: Vec<Vec<Command>>,
    jump_stack: Vec<[usize; 2]>,
    current_block: usize,
    loop_stack: Vec<LoopStackEntry>,
    loop_patches: HashMap<usize, Vec<usize>>,
    next_id: usize,
    next_param_id: usize,
    pub symbols: Vec<Symbol>,
    pub return_ty: TypeKind,
    pub implict_params: Vec<ImplicitParam>,
    pub compile: bool,
}
#[derive(Debug)]
pub struct IrGen {
    input: Vec<Statement>,
    header: Header,
    pub functions: Vec<Function>,
    pub imports: Vec<String>,
    pub imported_symbols: Vec<Symbol>,
    file_path: String,
    current_fn: usize,
    emit: bool,
    scopes: Vec<Vec<Symbol>>,
    next_fn_id: usize,
    pub registers: Vec<Register>,
    next_reg: u16,
    defer_stack: Vec<Vec<Statement>>,
}
impl IrGen {
    pub fn new(filename: &str, input: String) -> Self {
        let mut parser = Parser::new(input, filename.to_string(), false);
        let parse = parser.parse();
        IrGen {
            input: parse.0,
            header: parse.1,
            functions: vec![Function {
                name: "_start".to_string(),
                body: vec![vec![]],
                jump_stack: Vec::new(),
                current_block: 0,
                loop_stack: Vec::new(),
                loop_patches: HashMap::new(),
                symbols: Vec::new(),
                next_id: 0,
                next_param_id: 0,
                return_ty: TypeKind::Void,
                implict_params: Vec::new(),
                compile: false,
            }],
            defer_stack: Vec::new(),
            imports: Vec::new(),
            file_path: filename.to_string(),
            current_fn: 0,
            emit: true,
            scopes: vec![Vec::new()],
            next_fn_id: 1,
            registers: Vec::new(),
            next_reg: 0,
            imported_symbols: Vec::new(),
        }
    }
    fn incorperate_header(&mut self, header: Header, loc: SourceLocation) {
        let fn_base = self.next_fn_id;
        for sym in header.symbols {
            match sym.body {
                Definition::Function(ty, id) => {
                    self.define_function(
                        format!("{}::{}", header.module, sym.name),
                        ty.clone(),
                        id + fn_base,
                    );
                    let mut implict_params = Vec::new();
                    let rty = match ty {
                        TypeKind::Function(_, r) => *r,
                        _ => unreachable!(),
                    };
                    if self.is_internal_ptr(rty.clone()) {
                        implict_params.push(ImplicitParam {
                            name: Some(format!(
                                ".sret_{}",
                                rand::rng()
                                    .sample_iter(&Alphanumeric)
                                    .take(32)
                                    .map(|c| c as char)
                                    .collect::<String>()
                            )),
                            ty: self.unwrap_ptr_ty(rty.clone()).unwrap(),
                            param_ty: ImplicitParamType::ReturnPassthorugh,
                        });
                    }
                    self.functions.push(Function {
                        name: format!("{}::{}", header.module, sym.name),
                        body: vec![],
                        jump_stack: vec![],
                        current_block: 0,
                        loop_stack: vec![],
                        loop_patches: HashMap::new(),
                        next_id: 0,
                        next_param_id: 0,
                        symbols: vec![],
                        return_ty: rty,
                        implict_params,
                        compile: false,
                    });
                    self.next_fn_id += 1;
                }
                Definition::User(userty) => {
                    self.define_user_type(format!("{}::{}", header.module, sym.name), userty);
                }
                _ => {
                    self.emitError(loc, "Global variables are not supported");
                }
            }
        }
    }
    fn get_next_symbol_id(&mut self) -> usize {
        let r = self.functions[self.current_fn].next_id.clone();
        self.functions[self.current_fn].next_id += 1;
        r
    }
    fn get_next_param_symbol_id(&mut self) -> usize {
        let r = self.functions[self.current_fn].next_param_id.clone();
        self.functions[self.current_fn].next_param_id += 1;
        r
    }
    fn get_loc(&self) -> [usize; 3] {
        [
            self.current_fn,
            self.functions[self.current_fn].current_block,
            match self.functions[self.current_fn].body
                [self.functions[self.current_fn].current_block]
                .len()
            {
                0 => 0,
                n => n - 1,
            },
        ]
    }
    fn allocate_register(&mut self, ty: TypeKind) -> u16 {
        let id = self.next_reg;
        let register = Register {
            id,
            ty,
            start: self.get_loc(),
            end: None,
        };
        self.registers.push(register);
        self.next_reg += 1;
        id
    }
    fn get_register(&self, id: u16) -> Option<Register> {
        Some((self.registers.iter().find(|x| x.id == id)?).clone())
    }
    fn deallocate_register(&mut self, id: u16) {
        (*self.registers.iter_mut().find(|x| x.id == id).unwrap()).end = Some(self.get_loc());
    }
    fn alloc_get(&mut self, ty: TypeKind) -> Option<Register> {
        let x = self.allocate_register(ty);
        self.get_register(x)
    }
    fn push_scope(&mut self) {
        self.scopes.push(Vec::new());
        self.defer_stack.push(Vec::new());
    }
    fn pop_scope(&mut self) {
        let defers = self.defer_stack.pop();
        if defers.is_some() {
            defers
                .unwrap()
                .into_iter()
                .for_each(|x| self.compile_statement(x));
        }
        self.scopes.pop();
    }
    fn define_var(&mut self, name: String, ty: TypeKind) {
        let symbol = Symbol {
            name,
            body: Definition::Var(ty.clone()),
            id: self.get_next_symbol_id(),
            size: self.size_of(ty, SourceLocation { line: 0, col: 0 }),
        };
        self.functions[self.current_fn].symbols.push(symbol.clone());
        self.scopes.last_mut().unwrap().push(symbol);
    }
    fn define_user_type(&mut self, name: String, ty: UserType) {
        let symbol = Symbol {
            name,
            body: Definition::User(ty),
            id: 0,
            size: None,
        };
        self.scopes.last_mut().unwrap().push(symbol);
    }
    fn define_function(&mut self, name: String, functy: TypeKind, id: usize) {
        let symbol = Symbol {
            name,
            body: Definition::Function(functy, id),
            id: 0,
            size: None,
        };
        self.scopes.last_mut().unwrap().push(symbol);
    }
    fn define_parameter(&mut self, name: String, ty: TypeKind) {
        let symbol = Symbol {
            name,
            body: Definition::Parameter(ty.clone()),
            id: self.get_next_param_symbol_id(),
            size: self.size_of(ty, SourceLocation { line: 0, col: 0 }),
        };
        self.functions[self.current_fn].symbols.push(symbol.clone());
        self.scopes.last_mut().unwrap().push(symbol);
    }
    fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.iter().find(|s| s.name == name) {
                return Some(symbol);
            }
        }
        if let Some(symbol) = self.imported_symbols.iter().find(|s| s.name == name) {
            return Some(symbol);
        }
        None
    }
    pub fn compile(&mut self) {
        for stmt in self.input.clone().iter() {
            self.compile_statement(stmt.clone());
        }
    }
    fn compile_block(&mut self, statements: Vec<Statement>) -> usize {
        let id = self.new_block();
        self.push_scope();
        for statement in statements.iter() {
            self.compile_statement(statement.clone());
        }
        self.pop_scope();
        self.new_block();
        id
    }

    fn compile_statement(&mut self, statement: Statement) {
        match statement.kind {
            StatementKind::Expression(expr) => {
                self.compile_expression(expr, statement.loc);
            }
            StatementKind::Declaration(declaration) => {
                self.compile_declaration(declaration, statement.loc);
            }
            StatementKind::ImplictRet(e) => {
                self.emitError(
                    statement.loc,
                    "Expected semicolon after expression, got none",
                );
            }
            StatementKind::Block(stmts, implict_ret) => {
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
                self.compile_while(stmt, statement.loc, None);
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
                self.incorperate_header(import.header, statement.loc);
                self.imports.push(import.path);
            }
            StatementKind::Struct(decl) => {
                self.compile_struct(decl, statement.loc);
            }
            StatementKind::Union(decl) => {
                self.compile_union(decl, statement.loc);
            }
            StatementKind::Defer(stmt) => {
                self.compile_defer(*stmt);
            }
        }
    }
    fn compile_struct(&mut self, structDef: StructDeclaration, loc: SourceLocation) {
        self.define_user_type(structDef.name, UserType::Struct(structDef.fields));
    }
    fn compile_union(&mut self, unionDef: UnionDeclaration, loc: SourceLocation) {
        self.define_user_type(unionDef.name, UserType::Union(unionDef.variants));
    }
    fn compile_enum(&mut self, enumDef: EnumDeclaration, loc: SourceLocation) {
        self.define_user_type(enumDef.name, UserType::Enum(enumDef.variants));
    }
    fn compile_function(&mut self, func: FunctionDeclaration, loc: SourceLocation) {
        self.define_function(
            func.name.clone(),
            TypeKind::Function(
                func.params.values().map(|x| x.clone()).collect(),
                Box::new(func.return_ty.clone()),
            ),
            self.next_fn_id,
        );
        let curr_fn = self.current_fn;
        self.next_fn_id += 1;
        self.current_fn = self.next_fn_id - 1;
        self.push_scope();
        let mut implict_params = Vec::new();
        if self.is_internal_ptr(func.return_ty.clone()) {
            implict_params.push(ImplicitParam {
                name: Some(format!(
                    ".sret_{}",
                    rand::rng()
                        .sample_iter(&Alphanumeric)
                        .take(32)
                        .map(|c| c as char)
                        .collect::<String>()
                )),
                ty: self.unwrap_ptr_ty(func.return_ty.clone()).unwrap(),
                param_ty: ImplicitParamType::ReturnPassthorugh,
            });
        }
        self.functions.push(Function {
            name: func.name.clone(),
            body: vec![vec![]],
            jump_stack: vec![],
            current_block: 0,
            loop_stack: vec![],
            loop_patches: HashMap::new(),
            symbols: Vec::new(),
            next_id: 0,
            next_param_id: 0,
            return_ty: func.return_ty,
            implict_params,
            compile: true,
        });
        for (name, ty) in func.params.iter() {
            self.define_parameter(name.clone(), ty.clone());
        }
        self.compile_statement(*func.body);
        self.current_fn = curr_fn;
        self.pop_scope();
    }
    fn compile_expression(&mut self, expr: Expression, loc: SourceLocation) -> Option<Output> {
        match expr {
            Expression::Try(expr, catch) => {
                let expr_reg = self.compile_expression(*expr, loc)?;
                match catch.is_some() {
                    true => {
                        self.emit_instruction(Command::JumpTrue(
                            Location::None,
                            Value::Register(expr_reg.clone()),
                        ));
                        let jump = self.get_last_jump_id();
                        let ret = self.alloc_get(self.unwrap_ptr_ty(expr_reg.ty.clone())?)?;
                        match catch.unwrap().kind {
                            StatementKind::ImplictRet(return_expr) => {
                                let val = self.compile_expression(return_expr, loc)?;
                                self.emit_instruction(Command::Move(
                                    Value::Register(val),
                                    ret.clone(),
                                ));
                            }
                            StatementKind::Block(stmts, return_expr) => {
                                self.compile_block(stmts);
                                if let Some(return_expr) = return_expr {
                                    let val = self.compile_expression(return_expr, loc)?;
                                    if val.ty != ret.clone().ty {
                                        self.emitError(
                                    loc,
                                    &format!(
                                        "Type mismatch, expected block to return {}, got {}",
                                        ret.clone().ty,
                                        val.ty
                                    ),
                                );
                                    }
                                    self.emit_instruction(Command::Move(
                                        Value::Register(val),
                                        ret.clone(),
                                    ));
                                }
                            }
                            _ => {
                                self.emitError(loc, "Catch expresssion must produce a value");
                                return None;
                            }
                        }
                        self.emit_instruction(Command::Jump(Location::None));
                        let catch_jump = self.get_last_jump_id();
                        self.new_block();
                        let mut currb = self.current_block();
                        self.update_jump(
                            jump,
                            Command::JumpTrue(
                                Location::Block(currb),
                                Value::Register(expr_reg.clone()),
                            ),
                        );
                        self.emit_instruction(Command::Load(
                            Value::Register(expr_reg),
                            ret.clone(),
                        ));
                        self.new_block();
                        currb = self.current_block();
                        self.update_jump(catch_jump, Command::Jump(Location::Block(currb)));
                        return Some(ret);
                    }
                    false => {
                        self.emit_instruction(Command::JumpFalse(
                            Location::None,
                            Value::Register(expr_reg.clone()),
                        ));
                        let catch_jump = self.get_last_jump_id();
                        let ret = self.alloc_get(self.unwrap_ptr_ty(expr_reg.ty.clone())?)?;
                        self.emit_instruction(Command::Load(
                            Value::Register(expr_reg.clone()),
                            ret.clone(),
                        ));
                        self.emit_instruction(Command::Jump(Location::None));
                        let load_jump = self.get_last_jump_id();
                        let mut currb = self.new_block();
                        self.update_jump(
                            catch_jump,
                            Command::JumpFalse(
                                Location::Block(currb),
                                Value::Register(expr_reg.clone()),
                            ),
                        );
                        self.emit_instruction(Command::Ret(Some(Value::Immediate(Immediate {
                            value: 0.0,
                            ty: TypeKind::Optional(None),
                        }))));
                        currb = self.new_block();
                        self.update_jump(load_jump, Command::Jump(Location::Block(currb)));
                        return Some(ret);
                    }
                }
            }
            Expression::Sizeof(ty) => {
                let size = self.size_of(ty, loc)?;
                let out = self.alloc_get(TypeKind::Uint32)?;
                self.emit_instruction(Command::Move(
                    Value::Immediate(Immediate {
                        value: size as f64,
                        ty: TypeKind::Uint32,
                    }),
                    out.clone(),
                ));
                Some(out)
            }
            Expression::Binary(left, op, right) => {
                if BinaryOperator::PropertyAccess == op {
                    let obj = self.compile_expression(*left, loc.clone())?;
                    if let TypeKind::Pointer(ptr) = obj.ty.clone() {
                        if let TypeKind::Struct(strct) = *ptr {
                            let ty = self.lookup_symbol(&strct)?.clone();
                            if let Definition::User(UserType::Struct(utype)) = &ty.body {
                                if let Expression::Identifier(property) = *right {
                                    let fieldTy = utype.get(&property);
                                    if let Some(fieldTy) = fieldTy {
                                        // FIX: only sum fields that come before `property`,
                                        // not all fields (which would give total struct size).
                                        let offset = utype
                                            .keys()
                                            .take_while(|k| *k != &property)
                                            .fold(0, |acc, k| {
                                                acc + self.size_of(utype[k].clone(), loc).expect(
                                                    "INTERNAL ERROR: Failed to calculate size of type",
                                                )
                                            });
                                        let temp = self.alloc_get(TypeKind::Int32)?;
                                        let reg = self.alloc_get(fieldTy.clone())?;
                                        self.emit_instruction(Command::Add(
                                            Value::Immediate(Immediate {
                                                value: offset as f64,
                                                ty: TypeKind::Int32,
                                            }),
                                            Value::Register(obj.clone()),
                                            temp.clone(),
                                        ));
                                        self.emit_instruction(Command::Load(
                                            Value::Register(temp.clone()),
                                            reg.clone(),
                                        ));
                                        self.deallocate_register(temp.id);
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
                                    //we got a problemo, we cant assign

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
                                &format!(
                                    "Cannot access property of non-struct type, {} is not a struct",
                                    *ptr
                                ),
                            );
                        }
                    } else {
                        self.emitError(loc, "Cannot access property of non-struct type");
                    }
                    None
                } else if BinaryOperator::Assign == op {
                    self.compile_assignment(*left, *right, loc)
                } else {
                    let outL = self.compile_expression(*left.clone(), loc)?;
                    let outR = self.compile_expression(*right.clone(), loc)?;
                    if outL.ty != outR.ty {
                        self.emitError(
                            loc,
                            &format!(
                                "Type mismatch, type {} and {} aren't equivalent",
                                outL.ty, outR.ty
                            ),
                        );
                        return None;
                    }
                    let left = self.convert_output_to_value(outL.clone());
                    let right = self.convert_output_to_value(outR.clone());
                    let mut out = if matches!(
                        op,
                        BinaryOperator::Eq
                            | BinaryOperator::Ne
                            | BinaryOperator::Lt
                            | BinaryOperator::Gt
                            | BinaryOperator::Le
                            | BinaryOperator::Ge
                    ) {
                        self.allocate_register(TypeKind::Bool)
                    } else {
                        self.allocate_register(outL.ty.clone())
                    };
                    match op {
                        BinaryOperator::Add => self.emit_instruction(Command::Add(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::Sub => self.emit_instruction(Command::Sub(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::Mul => self.emit_instruction(Command::Mul(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::Div => self.emit_instruction(Command::Div(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::Mod => self.emit_instruction(Command::Mod(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::And => self.emit_instruction(Command::And(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::Or => {
                            self.emit_instruction(Command::Or(left, right, self.get_register(out)?))
                        }
                        BinaryOperator::Xor => self.emit_instruction(Command::Xor(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::Shl => self.emit_instruction(Command::Shl(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::Shr => self.emit_instruction(Command::Shr(
                            left,
                            right,
                            self.get_register(out)?,
                        )),
                        BinaryOperator::Eq => {
                            self.emit_instruction(Command::Eq(left, right, self.get_register(out)?))
                        }
                        BinaryOperator::Ne => {
                            self.deallocate_register(out);
                            out = self.allocate_register(TypeKind::Bool);
                            let temp = self.alloc_get(TypeKind::Bool)?;
                            self.emit_instruction(Command::Eq(left, right, temp.clone()));
                            let temp_val = self.convert_output_to_value(temp.clone());
                            self.emit_instruction(Command::Not(temp_val, self.get_register(out)?));
                            self.deallocate_register(temp.id);
                        }
                        BinaryOperator::Lt => {
                            self.emit_instruction(Command::Lt(left, right, self.get_register(out)?))
                        }
                        BinaryOperator::Gt => {
                            self.emit_instruction(Command::Gt(left, right, self.get_register(out)?))
                        }
                        BinaryOperator::Le => {
                            let tempA = self.alloc_get(outL.ty.clone())?;
                            let tempB = self.alloc_get(outL.ty.clone())?;
                            self.emit_instruction(Command::Lt(
                                left.clone(),
                                right.clone(),
                                tempA.clone(),
                            ));
                            self.emit_instruction(Command::Eq(
                                left.clone(),
                                right.clone(),
                                tempB.clone(),
                            ));

                            self.emit_instruction(Command::Or(
                                Value::Register(tempA.clone()),
                                Value::Register(tempB.clone()),
                                self.get_register(out)?,
                            ));
                            self.deallocate_register(tempA.id);
                            self.deallocate_register(tempB.id);
                        }

                        BinaryOperator::Ge => {
                            let tempA = self.alloc_get(outL.ty.clone())?;
                            let tempB = self.alloc_get(outL.ty.clone())?;
                            self.emit_instruction(Command::Gt(
                                left.clone(),
                                right.clone(),
                                tempA.clone(),
                            ));
                            self.emit_instruction(Command::Eq(left, right, tempB.clone()));
                            self.emit_instruction(Command::Or(
                                Value::Register(tempA.clone()),
                                Value::Register(tempB.clone()),
                                self.get_register(out)?,
                            ));
                            self.deallocate_register(tempA.id);
                            self.deallocate_register(tempB.id);
                        }
                        _ => {}
                    }
                    self.get_register(out)
                }
            }
            Expression::Unary(op, expr) => {
                let left = self.compile_expression(*expr, loc)?;
                let mut out = self.alloc_get(left.ty.clone())?;
                let value_l = self.convert_output_to_value(left.clone());
                match op {
                    UnaryOperator::Neg => {
                        self.emit_instruction(Command::Mul(
                            value_l,
                            Value::Immediate(Immediate {
                                value: -1.0,
                                ty: TypeKind::Int16,
                            }),
                            out.clone(),
                        ));
                        self.deallocate_register(left.id);
                    }
                    UnaryOperator::Not => {
                        self.emit_instruction(Command::Not(value_l, out.clone()));
                        self.deallocate_register(left.id);
                    }
                    UnaryOperator::Deref => {
                        if let TypeKind::Pointer(x) = left.ty {
                            self.deallocate_register(out.id);
                            out = self.alloc_get(*x)?;
                            self.emit_instruction(Command::Load(value_l, out.clone()));
                            self.deallocate_register(left.id);
                        } else {
                            self.emitError(loc, "Cannot dereference non-pointer type");
                            return None;
                        }
                    }
                }
                Some(out)
            }
            Expression::FunctionCall(func, args) => {
                let func = self.compile_expression(*func, loc);
                if let Some(func) = func {
                    let (params, ret_ty) = self.unwrap_fn_type(func.ty.clone(), loc)?;
                    let mut sret_name = None;
                    if self.is_internal_ptr(ret_ty.clone()) {
                        sret_name = Some(format!(
                            "__internal_ptr_{}",
                            rand::rng()
                                .sample_iter(&Alphanumeric)
                                .take(32)
                                .map(|c| c as char)
                                .collect::<String>()
                        ));
                        self.define_var(
                            sret_name.clone().unwrap(),
                            self.unwrap_ptr_ty(ret_ty.clone())?,
                        );
                        let id = self.lookup_symbol((&sret_name.clone().unwrap()))?.id;
                        let temp = self.alloc_get(ret_ty.clone())?;
                        self.emit_instruction(Command::Add(
                            Value::Location(Location::Symbol(id, 0)),
                            Value::ARP,
                            temp.clone(),
                        ));
                        self.emit_instruction(Command::Push(Value::Register(temp)));
                    }
                    let argc = args.len() as u8;
                    let args = args
                        .into_iter()
                        .map(|arg| {
                            let reg = self.compile_expression(arg, loc).unwrap_or(Register {
                                id: 0,
                                ty: TypeKind::Void,
                                start: [0, 0, 0],
                                end: None,
                            });
                            self.emit_instruction(Command::Push(Value::Register(reg.clone())));
                            self.deallocate_register(reg.id);
                            reg.ty
                        })
                        .collect::<Vec<TypeKind>>();
                    if params.len() as u8 != argc {
                        self.emitError(loc, "function call argument count mismatch");
                    }
                    params.iter().zip(args).for_each(|(param, arg)| {
                        if *param != arg {
                            self.emitError(
                                loc,
                                &format!(
                                    "function call argument type mismatch: expected {}, got {}",
                                    param, arg
                                ),
                            );
                        }
                    });
                    self.emit_instruction(Command::Call(
                        Value::Register(func.clone()),
                        argc + match self.is_internal_ptr(ret_ty.clone()) {
                            true => 1,
                            false => 0,
                        },
                    ));
                    self.deallocate_register(func.id);
                    if ret_ty != TypeKind::Void {
                        match sret_name {
                            Some(name) => match ret_ty {
                                TypeKind::Optional(_) => {
                                    let ret_reg = self.alloc_get(ret_ty.clone())?;
                                    self.emit_instruction(Command::Pop(ret_reg.clone()));
                                    return Some(ret_reg);
                                }
                                _ => {
                                    let ret_reg = self.alloc_get(ret_ty.clone())?;
                                    let id = self.lookup_symbol(&name)?.id;
                                    self.emit_instruction(Command::Add(
                                        Value::Location(Location::Symbol(id, 0)),
                                        Value::ARP,
                                        ret_reg.clone(),
                                    ));
                                    return Some(ret_reg);
                                }
                            },
                            None => {
                                let ret_reg = self.alloc_get(ret_ty.clone())?;
                                self.emit_instruction(Command::Pop(ret_reg.clone()));
                                return Some(ret_reg);
                            }
                        }
                    }
                    None
                } else {
                    self.emitError(loc, "Invalid expression provided for function call");
                    None
                }
            }
            Expression::Grouped(expr) => self.compile_expression(*expr, loc),
            Expression::Literal(lit) => self.compile_literal(lit, loc),
            Expression::Identifier(ident) => {
                let place = self.compile_place_expr(Expression::Identifier(ident), loc)?;
                if let TypeKind::Function(_, _) = self.unwrap_ptr_ty(place.ty.clone())? {
                    return Some(place);
                }
                let reg = self.alloc_get(self.unwrap_ptr_ty(place.ty.clone())?)?;
                self.emit_instruction(Command::Load(Value::Register(place), reg.clone()));
                Some(reg)
            }
            Expression::Cast(ty, expr) => {
                let out = self.alloc_get(ty)?;
                let prev = self.compile_expression(*expr, loc)?;
                self.emit_instruction(Command::Move(Value::Register(prev.clone()), out.clone()));
                self.deallocate_register(prev.id);
                Some(out)
            }
            Expression::AddressOf(expr) => {
                let addr = self.compile_place_expr(*expr, loc);
                addr
            }
            Expression::Subscript(array, index) => {
                let place = self.compile_place_expr(Expression::Subscript(array, index), loc)?;
                let out = self.alloc_get(self.unwrap_ptr_ty(place.ty.clone())?)?;
                self.emit_instruction(Command::Load(Value::Register(place), out.clone()));
                Some(out)
            }
            Expression::Match(match_expr) => {
                let val = self.compile_expression(*match_expr.expr, loc)?;
                let mut end_id = Vec::new();
                let mut ret: Option<Register> = None;
                for case in match_expr.cases {
                    self.push_scope();
                    let pat = Value::Register(self.compile_pattern(
                        case.pattern,
                        val.clone(),
                        loc,
                        None,
                    )?);
                    self.emit_instruction(Command::JumpFalse(Location::None, pat.clone()));
                    let jump = self.get_last_jump_id();
                    match case.body.kind {
                        StatementKind::ImplictRet(return_expr) => {
                            let val = self.compile_expression(return_expr, loc)?;
                            if ret.is_none() {
                                ret = self.alloc_get(val.ty.clone());
                            }
                            if val.ty != ret.clone().unwrap().ty {
                                self.emitError(
                                    loc,
                                    &format!(
                                        "Invalid return type for match case, expected {}, got {}",
                                        ret.clone().unwrap().ty,
                                        val.ty
                                    ),
                                );
                            }
                            self.emit_instruction(Command::Move(
                                Value::Register(val),
                                ret.clone().unwrap(),
                            ));
                        }
                        StatementKind::Block(stmts, return_expr) => {
                            self.compile_block(stmts);
                            if let Some(return_expr) = return_expr {
                                let val = self.compile_expression(return_expr, loc)?;
                                if ret.is_none() {
                                    ret = self.alloc_get(val.ty.clone());
                                }
                                if val.ty != ret.clone().unwrap().ty {
                                    self.emitError(
                                        loc,
                                        &format!(
                                            "Type mismatch, expected block to return {}, got {}",
                                            ret.clone().unwrap().ty,
                                            val.ty
                                        ),
                                    );
                                }
                                self.emit_instruction(Command::Move(
                                    Value::Register(val),
                                    ret.clone().unwrap(),
                                ));
                            }
                        }
                        _ => self.compile_statement(case.body),
                    }
                    let nb = self.current_block() + 1;
                    self.pop_scope();
                    self.emit_instruction(Command::Jump(Location::None));
                    end_id.push(self.get_last_jump_id());
                    self.update_jump(jump, Command::JumpFalse(Location::Block(nb), pat));
                    self.new_block();
                }
                self.new_block();
                let current_loc = self.current_block();
                for j in end_id {
                    self.update_jump(j, Command::Jump(Location::Block(current_loc)));
                }
                ret
            }
        }
    }
    fn compile_literal(&mut self, lit: Literal, loc: SourceLocation) -> Option<Output> {
        match lit {
            Literal::None => {
                let out = self.alloc_get(TypeKind::Optional(None))?;
                self.emit_instruction(Command::Move(
                    Value::Immediate(Immediate {
                        value: 0.0,
                        ty: TypeKind::Int32,
                    }),
                    out.clone(),
                ));
                Some(out)
            }
            Literal::Some(opt) => {
                let opt = self.compile_expression(*opt, loc)?;
                let name = format!(
                    "__internal_option_{}",
                    rand::rng()
                        .sample_iter(&Alphanumeric)
                        .take(32)
                        .map(|c| c as char)
                        .collect::<String>()
                );
                self.define_var(name.clone(), opt.ty.clone());
                let sym = self.lookup_symbol(&name).unwrap().id;
                let loc = self.alloc_get(TypeKind::Optional(Some(Box::new(opt.ty.clone()))))?;
                self.emit_instruction(Command::Add(
                    Value::ARP,
                    Value::Location(Location::Symbol(sym, 0)),
                    loc.clone(),
                ));
                self.emit_instruction(Command::Store(
                    Value::Register(opt.clone()),
                    Value::Register(loc.clone()),
                ));
                self.deallocate_register(opt.id);
                Some(loc)
            }
            Literal::Int(value) => {
                let ty = match (value > (i16::MAX as i32)) || (value < (i16::MIN as i32)) {
                    true => TypeKind::Int32,
                    false => TypeKind::Int16,
                };
                let out = self.alloc_get(ty.clone())?;
                self.emit_instruction(Command::Move(
                    Value::Immediate(Immediate {
                        value: value as f64,
                        ty,
                    }),
                    out.clone(),
                ));
                Some(out)
            }
            Literal::Bool(val) => {
                let out = self.alloc_get(TypeKind::Bool)?;
                self.emit_instruction(Command::Move(
                    Value::Immediate(Immediate {
                        value: match val {
                            true => 1.0,
                            false => 0.0,
                        },
                        ty: TypeKind::Bool,
                    }),
                    out.clone(),
                ));
                Some(out)
            }
            Literal::Float(f) => {
                let out = self.alloc_get(TypeKind::Float32)?;
                self.emit_instruction(Command::Move(
                    Value::Immediate(Immediate {
                        value: f as f64,
                        ty: TypeKind::Float32,
                    }),
                    out.clone(),
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
                let ty = match arr.get(0) {
                    Some(elm) => self.compile_expression(elm.clone(), loc)?.ty,
                    None => TypeKind::Void,
                };
                self.emit = true;
                let size = self.size_of(ty.clone(), loc.clone())?;
                self.define_var(
                    name.clone(),
                    TypeKind::Array(Box::new(ty.clone()), arr.len()),
                );
                let symbol = (*self.lookup_symbol(&name)?).clone();
                for (i, expr) in arr.iter().enumerate() {
                    let reg = self.compile_expression(expr.clone(), loc)?;
                    if reg.ty == ty {
                        let temp = self.alloc_get(TypeKind::Pointer(Box::new(ty.clone())))?;
                        self.emit_instruction(Command::Add(
                            Value::Location(Location::Symbol(symbol.id, i * size)),
                            Value::ARP,
                            temp.clone(),
                        ));
                        // FIX: Store(val, ptr) — value first, pointer second.
                        self.emit_instruction(Command::Store(
                            Value::Register(reg.clone()),
                            Value::Register(temp.clone()),
                        ));
                        self.deallocate_register(reg.id);
                        self.deallocate_register(temp.id);
                    } else {
                        self.emitError(
                            loc,
                            &format!("Invalid type, expected type: {}, got type: {}", ty, reg.ty),
                        );
                    }
                }
                let out = self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Array(
                    Box::new(ty),
                    arr.len(),
                ))))?;
                self.emit_instruction(Command::Add(
                    Value::Location(Location::Symbol(symbol.id, 0)),
                    Value::ARP,
                    out.clone(),
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
                self.define_var(
                    name.clone(),
                    TypeKind::Array(Box::new(TypeKind::Char), str.len() + 1),
                );
                let symbol = (*self.lookup_symbol(&name)?).clone();
                for (i, c) in str.chars().enumerate() {
                    let temp = self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Char)))?;
                    self.emit_instruction(Command::Add(
                        Value::ARP,
                        Value::Location(Location::Symbol(symbol.id, i)),
                        temp.clone(),
                    ));
                    // FIX: Store(val, ptr) — immediate value first, pointer second.
                    self.emit_instruction(Command::Store(
                        Value::Immediate(Immediate {
                            value: c as u8 as f64,
                            ty: TypeKind::Char,
                        }),
                        Value::Register(temp.clone()),
                    ));
                    self.deallocate_register(temp.id);
                }
                let temp = self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Char)))?;
                self.emit_instruction(Command::Add(
                    Value::ARP,
                    Value::Location(Location::Symbol(symbol.id, str.len())),
                    temp.clone(),
                ));
                // FIX: Store(val, ptr) — null terminator first, pointer second.
                self.emit_instruction(Command::Store(
                    Value::Immediate(Immediate {
                        value: 0.0,
                        ty: TypeKind::Char,
                    }),
                    Value::Register(temp.clone()),
                ));
                self.deallocate_register(temp.id);
                let out = self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Array(
                    Box::new(TypeKind::Char),
                    str.len() + 1,
                ))))?;
                self.emit_instruction(Command::Add(
                    Value::Location(Location::Symbol(symbol.id, 0)),
                    Value::ARP,
                    out.clone(),
                ));
                Some(out)
            }
            Literal::Struct(name, fields) => {
                let ty = self.lookup_symbol(&name);
                if let Some(ty) = ty {
                    if let Definition::User(UserType::Struct(def_fields)) = ty.body.clone() {
                        let sname = format!(
                            "__internal_struct_{}",
                            rand::thread_rng()
                                .sample_iter(&Alphanumeric)
                                .take(32)
                                .map(|c| c as char)
                                .collect::<String>()
                        );
                        self.define_var(sname.clone(), TypeKind::Struct(name.clone()));
                        let symbol = (*self.lookup_symbol(&sname)?).clone();
                        let mut offset = 0;
                        for field in def_fields.keys() {
                            if let Some(value) = fields.get(field) {
                                let expr = self.compile_expression(value.clone(), loc)?;
                                if expr.ty == def_fields[field] {
                                    let temp = self
                                        .alloc_get(TypeKind::Pointer(Box::new(expr.ty.clone())))?;
                                    self.emit_instruction(Command::Add(
                                        Value::Location(Location::Symbol(symbol.id, offset)),
                                        Value::ARP,
                                        temp.clone(),
                                    ));
                                    // FIX: Store(val, ptr) — field value first, pointer second.
                                    self.emit_instruction(Command::Store(
                                        Value::Register(expr.clone()),
                                        Value::Register(temp.clone()),
                                    ));
                                    self.deallocate_register(expr.id);
                                    self.deallocate_register(temp.id);
                                    offset += self.size_of(expr.ty, loc)?;
                                } else {
                                    self.emitError(
                                        loc,
                                        &format!(
                                            "Type mismatch for field {}, expected {}, got {}",
                                            field, def_fields[field], expr.ty
                                        ),
                                    );
                                    return None;
                                }
                            } else {
                                self.emitError(loc, &format!("Expected field {}", field));
                            }
                        }
                        let out =
                            self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Struct(name))))?;
                        self.emit_instruction(Command::Add(
                            Value::Location(Location::Symbol(symbol.id, 0)),
                            Value::ARP,
                            out.clone(),
                        ));
                        return Some(out);
                    } else {
                        self.emitError(loc, &format!("No such struct {}", name));
                    }
                } else {
                    self.emitError(loc, &format!("No such struct {}", name));
                }

                None
            }
            Literal::Union(name, variant, expr) => {
                let type_def = self.lookup_symbol(&name);
                if let Some(type_def) = type_def {
                    if let Definition::User(UserType::Union(def)) = type_def.body.clone() {
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
                            self.define_var(sname.clone(), TypeKind::Union(name.clone()));
                            let symbol = (*self.lookup_symbol(&sname).unwrap()).clone();
                            let temp =
                                self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Uint16)))?;
                            self.emit_instruction(Command::Add(
                                Value::Location(Location::Symbol(symbol.id, 0)),
                                Value::ARP,
                                temp.clone(),
                            ));
                            // FIX: Store(val, ptr) — tag value first, pointer second.
                            self.emit_instruction(Command::Store(
                                Value::Immediate(Immediate {
                                    value: def.keys().position(|x| *x == variant)? as f64,
                                    ty: TypeKind::Uint16,
                                }),
                                Value::Register(temp.clone()),
                            ));
                            self.emit_instruction(Command::Add(
                                Value::Location(Location::Symbol(symbol.id, 1)),
                                Value::ARP,
                                temp.clone(),
                            ));
                            // FIX: Store(val, ptr) — union payload first, pointer second.
                            self.emit_instruction(Command::Store(
                                Value::Register(expr.clone()),
                                Value::Register(temp.clone()),
                            ));
                            self.deallocate_register(expr.id);
                            self.deallocate_register(temp.id);
                            let out = self.alloc_get(TypeKind::Pointer(Box::new(
                                TypeKind::Union(name.clone()),
                            )))?;
                            self.emit_instruction(Command::Add(
                                Value::Location(Location::Symbol(symbol.id, 0)),
                                Value::ARP,
                                out.clone(),
                            ));
                            return Some(out);
                        } else {
                            self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                        }
                    } else {
                        self.emitError(loc, &format!("No such union {}", name));
                    }
                } else {
                    self.emitError(loc, &format!("No such union {}", name));
                }

                None
            }
            Literal::Enum(name, variant) => {
                let def = self.lookup_symbol(&name);
                if let Some(def) = def {
                    if let Definition::User(UserType::Enum(variants)) = def.body.clone() {
                        let variant_def = variants.iter().position(|v| *v == variant);
                        if variant_def.is_none() {
                            self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                            return None;
                        }
                        let out = self.alloc_get(TypeKind::Enum(name.clone()))?;
                        self.emit_instruction(Command::Move(
                            Value::Immediate(Immediate {
                                value: variant_def.unwrap() as f64,
                                ty: TypeKind::Enum(name),
                            }),
                            out.clone(),
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
        }
    }
    fn is_internal_ptr(&self, ty: TypeKind) -> bool {
        if let TypeKind::Optional(_) = ty {
            return true;
        }
        match self.unwrap_ptr_ty(ty) {
            Some(TypeKind::Array(_, _) | TypeKind::Struct(_) | TypeKind::Union(_)) => true,
            _ => false,
        }
    }
    fn compile_pattern(
        &mut self,
        pat: Pattern,
        val: Register,
        loc: SourceLocation,
        decl_ty: Option<TypeKind>,
    ) -> Option<Output> {
        let reg = self.alloc_get(TypeKind::Bool)?;
        self.emit_instruction(Command::Move(
            Value::Immediate(Immediate {
                value: 0.0,
                ty: TypeKind::Bool,
            }),
            reg.clone(),
        ));
        let true_move = Command::Move(
            Value::Immediate(Immediate {
                value: 1.0,
                ty: TypeKind::Bool,
            }),
            reg.clone(),
        );
        match pat {
            Pattern::Some(some) => {
                self.emit_instruction(Command::JumpTrue(
                    Location::None,
                    Value::Register(val.clone()),
                ));
                let start_jump = self.get_last_jump_id();
                let r = self.alloc_get(self.unwrap_ptr_ty(val.ty.clone())?)?;
                self.emit_instruction(Command::Load(Value::Register(val.clone()), r.clone()));
                let inner = self.compile_pattern(*some, r, loc, None)?;
                self.emit_instruction(Command::Move(Value::Register(inner), reg.clone()));
                self.emit_instruction(Command::Jump(Location::None));
                let ld_jump = self.get_last_jump_id();
                self.new_block();
                let mut currb = self.current_block();
                self.update_jump(
                    start_jump,
                    Command::JumpTrue(Location::Block(currb), Value::Register(val.clone())),
                );
                self.emit_instruction(Command::Move(
                    Value::Immediate(Immediate {
                        value: 0.0,
                        ty: TypeKind::Bool,
                    }),
                    reg.clone(),
                ));
                self.new_block();
                currb = self.current_block();
                self.update_jump(ld_jump, Command::Jump(Location::Block(currb)));
            }
            Pattern::Wildcard => {
                self.emit_instruction(true_move);
                self.deallocate_register(val.id);
            }
            Pattern::Literal(lit) => {
                let lit_value = self.compile_literal(lit, loc)?;
                let lit_value_r = Value::Register(lit_value.clone());
                self.emit_instruction(Command::Eq(
                    lit_value_r,
                    Value::Register(val.clone()),
                    reg.clone(),
                ));
                self.deallocate_register(lit_value.id);
                self.deallocate_register(val.id);
            }
            Pattern::Array(array) => {
                if let TypeKind::Pointer(ptr) = val.ty.clone() {
                    if let TypeKind::Array(ty, count) = *ptr {
                        for (i, elem) in array.iter().enumerate() {
                            let val_elm = self.alloc_get(*ty.clone())?;
                            let elm_ptr = self.alloc_get(TypeKind::Pointer(ty.clone()))?;
                            self.emit_instruction(Command::Add(
                                Value::Register(val.clone()),
                                Value::Immediate(Immediate {
                                    value: (i * self.size_of(*ty.clone(), loc)?) as f64,
                                    ty: TypeKind::Int32,
                                }),
                                elm_ptr.clone(),
                            ));
                            self.emit_instruction(Command::Load(
                                Value::Register(elm_ptr.clone()),
                                val_elm.clone(),
                            ));
                            self.deallocate_register(elm_ptr.id);
                            let pat_val = self.compile_pattern(
                                elem.clone(),
                                val_elm.clone(),
                                loc,
                                Some(*ty.clone()),
                            )?;
                            self.deallocate_register(val_elm.id);
                            if i != 0 {
                                self.emit_instruction(Command::And(
                                    Value::Register(reg.clone()),
                                    Value::Register(pat_val.clone()),
                                    reg.clone(),
                                ));
                            } else {
                                self.emit_instruction(Command::Move(
                                    Value::Register(pat_val.clone()),
                                    reg.clone(),
                                ));
                            }
                            self.deallocate_register(pat_val.id);
                            self.deallocate_register(val.id);
                        }
                    } else {
                        self.emitError(
                            loc,
                            &format!("Expected match value to be an array, got {}", val.ty),
                        );
                    }
                } else {
                    self.emitError(
                        loc,
                        &format!("Expected match value to be an array, got {}", val.ty),
                    );
                }
            }
            Pattern::Identifier(ident) => {
                self.define_var(ident.clone(), decl_ty.unwrap_or(val.ty.clone()));
                let temp = self.alloc_get(TypeKind::Pointer(Box::new(val.ty.clone())))?;
                let symbol = self.lookup_symbol(&ident)?.clone();
                self.emit_instruction(Command::Add(
                    Value::Location(Location::Symbol(symbol.id, 0)),
                    Value::ARP,
                    temp.clone(),
                ));
                // FIX: Store(val, ptr) — matched value first, pointer second.
                self.emit_instruction(Command::Store(
                    Value::Register(val.clone()),
                    Value::Register(temp.clone()),
                ));
                self.deallocate_register(temp.id);
                self.emit_instruction(true_move);
                self.deallocate_register(val.id);
            }
            Pattern::Enum(name, variant) => {
                let ty = self.lookup_symbol(&name)?;
                if let Definition::User(UserType::Enum(variants)) = ty.body.clone() {
                    let index = variants.iter().position(|v| *v == variant);
                    if let Some(index) = index {
                        self.emit_instruction(Command::Eq(
                            Value::Register(val.clone()),
                            Value::Immediate(Immediate {
                                value: index as f64,
                                ty: TypeKind::Enum(name),
                            }),
                            reg.clone(),
                        ));
                        self.deallocate_register(val.id);
                    } else {
                        self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                    }
                } else {
                    self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                }
            }
            Pattern::Union(name, variant, child_pattern) => {
                let tdef = self.lookup_symbol(&name)?.body.clone();
                let index = match tdef.clone() {
                    Definition::User(UserType::Union(variants)) => variants
                        .iter()
                        .position(|v| *v.0 == variant)
                        .unwrap_or_else(|| {
                            self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                            0
                        }),
                    _ => {
                        self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                        0
                    }
                };

                let variant_ty = match tdef.clone() {
                    Definition::User(UserType::Union(variants)) => variants
                        .iter()
                        .find(|v| *v.0 == variant)
                        .map(|v| v.1.clone())
                        .unwrap_or_else(|| {
                            self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                            TypeKind::Void
                        }),
                    _ => {
                        self.emitError(loc, &format!("No such variant {}::{}", name, variant));
                        TypeKind::Void
                    }
                };
                if let TypeKind::Pointer(ptr) = val.ty.clone() {
                    if let TypeKind::Union(_) = *ptr {
                        let tag = self.alloc_get(TypeKind::Uint16)?;
                        self.emit_instruction(Command::Load(
                            Value::Register(val.clone()),
                            tag.clone(),
                        ));
                        self.emit_instruction(Command::Eq(
                            Value::Register(tag.clone()),
                            Value::Immediate(Immediate {
                                value: index as f64,
                                ty: TypeKind::Uint16,
                            }),
                            reg.clone(),
                        ));
                        self.deallocate_register(tag.id);
                        self.emit_instruction(Command::JumpFalse(
                            Location::None,
                            Value::Register(reg.clone()),
                        ));
                        let jid = self.get_last_jump_id();
                        self.new_block();
                        let union_data = self.alloc_get(variant_ty.clone())?;
                        let temp = self.alloc_get(TypeKind::Int32)?;
                        self.emit_instruction(Command::Add(
                            Value::Register(val),
                            Value::Immediate(Immediate {
                                value: 1.0,
                                ty: TypeKind::Int32,
                            }),
                            temp.clone(),
                        ));
                        self.emit_instruction(Command::Load(
                            Value::Register(temp.clone()),
                            union_data.clone(),
                        ));
                        self.deallocate_register(temp.id);
                        let result = self.compile_pattern(
                            *child_pattern,
                            union_data.clone(),
                            loc,
                            Some(variant_ty),
                        )?;
                        self.emit_instruction(Command::Move(
                            Value::Register(result.clone()),
                            reg.clone(),
                        ));
                        self.deallocate_register(result.id);
                        self.deallocate_register(union_data.id);
                        let nb = self.new_block();
                        self.update_jump(
                            jid,
                            Command::JumpFalse(Location::Block(nb), Value::Register(reg.clone())),
                        );
                    } else {
                        self.emitError(loc, &format!("Expected union, got {}", val.ty));
                    }
                } else {
                    self.emitError(loc, &format!("Expected union, got {}", val.ty));
                }
            }
            Pattern::Struct(name, fields) => {
                let dsym = Symbol {
                    name: "".to_string(),
                    body: Definition::Var(TypeKind::Void),
                    id: 0,
                    size: None,
                };
                let type_def = if let Definition::User(UserType::Struct(map)) = self
                    .lookup_symbol(&name)
                    .unwrap_or_else(|| {
                        self.emitError(loc, &format!("Undefined struct type '{}'", name));
                        &dsym
                    })
                    .body
                    .clone()
                {
                    Some(map)
                } else {
                    self.emitError(loc, &format!("Undefined struct type '{}'", name));
                    None
                }?;
                for (count, (field, pat)) in fields.iter().enumerate() {
                    if !type_def.contains_key(field) {
                        self.emitError(
                            loc,
                            &format!("Undefined field '{}' in struct '{}'", field, name),
                        );
                        return None;
                    }
                    let field_id = type_def.keys().position(|x| *x == *field)?;
                    let offset = type_def.iter().enumerate().fold(0, |acc, (i, (key, ty))| {
                        if i < field_id {
                            acc + self.size_of(ty.clone(), loc).unwrap()
                        } else {
                            acc
                        }
                    });
                    let field_loc =
                        self.alloc_get(TypeKind::Pointer(Box::new(type_def[field].clone())))?;
                    self.emit_instruction(Command::Add(
                        Value::Register(val.clone()),
                        Value::Immediate(Immediate {
                            value: offset as f64,
                            ty: TypeKind::Uint16,
                        }),
                        field_loc.clone(),
                    ));
                    let field_reg = self.alloc_get(type_def[field].clone())?;
                    self.emit_instruction(Command::Load(
                        Value::Register(field_loc.clone()),
                        field_reg.clone(),
                    ));
                    self.deallocate_register(field_loc.id);
                    let pattern_matches = self.compile_pattern(
                        pat.clone(),
                        field_reg,
                        loc,
                        Some(type_def[field].clone()),
                    )?;
                    if count != 0 {
                        self.emit_instruction(Command::And(
                            Value::Register(reg.clone()),
                            Value::Register(pattern_matches.clone()),
                            reg.clone(),
                        ));
                    } else {
                        self.emit_instruction(Command::Move(
                            Value::Register(pattern_matches),
                            reg.clone(),
                        ));
                    }
                }
            }
        };
        return Some(reg);
    }
    fn compile_assignment(
        &mut self,
        var: Expression,
        assign: Expression,
        loc: SourceLocation,
    ) -> Option<Output> {
        let val = self.compile_expression(assign, loc)?;
        let place = Value::Register(self.compile_place_expr(var, loc)?);
        self.emit_instruction(Command::Store(Value::Register(val.clone()), place));
        Some(val)
    }
    fn unwrap_ptr_ty(&self, wrapped: TypeKind) -> Option<TypeKind> {
        if let TypeKind::Pointer(ptr) = wrapped {
            return Some(*ptr);
        }
        if let TypeKind::Optional(Some(inner)) = wrapped {
            return Some(*inner);
        }
        None
    }

    fn compile_place_expr(&mut self, expr: Expression, loc: SourceLocation) -> Option<Output> {
        let r = match expr {
            Expression::Subscript(expr, offset_expr) => {
                let mut arrptr = self.compile_place_expr(*expr, loc)?;
                if let Some(TypeKind::Pointer(inner)) = self.unwrap_ptr_ty(arrptr.ty.clone()) {
                    if let TypeKind::Array(_, _) = *inner.clone() {
                        // `x` where `x: [T]` is an lvalue slot of type `**[T]`.
                        // Load once so subscript math uses the actual `*[T]` base pointer.
                        let loaded = self.alloc_get(TypeKind::Pointer(inner.clone()))?;
                        self.emit_instruction(Command::Load(
                            Value::Register(arrptr.clone()),
                            loaded.clone(),
                        ));
                        self.deallocate_register(arrptr.id);
                        arrptr = loaded;
                    }
                }
                if let Some(TypeKind::Array(arr_elm_ty, _)) = self.unwrap_ptr_ty(arrptr.ty.clone())
                {
                    let sizeof = self.size_of(*arr_elm_ty.clone(), loc)?;
                    let offset = self.compile_expression(*offset_expr, loc)?;
                    let reg = self.alloc_get(TypeKind::Pointer(Box::new(*arr_elm_ty.clone())))?;
                    self.emit_instruction(Command::Mul(
                        Value::Register(offset),
                        Value::Immediate(Immediate {
                            value: sizeof as f64,
                            ty: TypeKind::Uint32,
                        }),
                        reg.clone(),
                    ));
                    self.emit_instruction(Command::Add(
                        Value::Register(arrptr),
                        Value::Register(reg.clone()),
                        reg.clone(),
                    ));
                    return Some(reg);
                } else {
                    self.emitError(loc, "Expression isn't an array");
                }
                None
            }
            Expression::Binary(lhs, op, rhs) => match op {
                BinaryOperator::PropertyAccess => {
                    let base = self.compile_place_expr(*lhs, loc)?;
                    let (addr, name) = match self.unwrap_ptr_ty(base.ty.clone()) {
                        Some(TypeKind::Struct(name)) => (base, name),
                        Some(TypeKind::Pointer(inner)) => {
                            if let TypeKind::Struct(name) = *inner {
                                // `base` points to a variable slot that stores `*Struct`.
                                // Load once to get the actual struct base pointer.
                                let loaded = self.alloc_get(TypeKind::Pointer(Box::new(
                                    TypeKind::Struct(name.clone()),
                                )))?;
                                self.emit_instruction(Command::Load(
                                    Value::Register(base),
                                    loaded.clone(),
                                ));
                                (loaded, name)
                            } else {
                                self.emitError(
                                    loc,
                                    "Cannot access property of non-struct pointer type",
                                );
                                return None;
                            }
                        }
                        _ => {
                            self.emitError(loc, "Cannot access property of non-struct type");
                            return None;
                        }
                    };
                    let sym = self.lookup_symbol(&name)?.clone();
                    if let Definition::User(UserType::Struct(fields)) = sym.body {
                        if let Expression::Identifier(prop) = *rhs {
                            let fieldTy = fields.get(&prop).clone();
                            if let Some(fieldTy) = fieldTy {
                                // FIX: only sum fields that come before `prop`,
                                // not all fields (which would give total struct size).
                                let offset = fields.keys().take_while(|k| *k != &prop).fold(
                                    0,
                                    |acc, k| {
                                        acc + self.size_of(fields[k].clone(), loc).expect(
                                            "INTERNAL ERROR: Failed to calculate size of type",
                                        )
                                    },
                                );
                                let reg =
                                    self.alloc_get(TypeKind::Pointer(Box::new(fieldTy.clone())))?;
                                self.emit_instruction(Command::Add(
                                    Value::Immediate(Immediate {
                                        value: offset as f64,
                                        ty: TypeKind::Int32,
                                    }),
                                    Value::Register(addr),
                                    reg.clone(),
                                ));
                                return Some(reg);
                            } else {
                                self.emitError(
                                    loc,
                                    &format!("No such property {} on struct {}", prop, name),
                                );
                            }
                        } else {
                            self.emitError(loc, "Invalid property expression");
                        }
                    } else {
                        self.emitError(loc, &format!("No such struct {}", name));
                    }
                    None
                }
                _ => None,
            },
            Expression::Identifier(ident) => {
                let symbol = self.lookup_symbol(ident.as_str())?.clone();
                match symbol.body.clone() {
                    Definition::Function(ty, id) => {
                        let loc = Value::Location(Location::Function(id));
                        let reg = self.alloc_get(TypeKind::Pointer(Box::new(ty)))?;
                        self.emit_instruction(Command::Move(loc, reg.clone()));
                        Some(reg)
                    }
                    Definition::Var(ty) | Definition::Parameter(ty) => {
                        let loc = Value::Location(match symbol.body {
                            Definition::Var(_) => Location::Symbol(symbol.id, 0),
                            Definition::Parameter(_) => Location::Argument(symbol.name),
                            _ => unreachable!(),
                        });
                        let reg = self.alloc_get(TypeKind::Pointer(Box::new(ty.clone())))?;
                        self.emit_instruction(Command::Add(loc, Value::ARP, reg.clone()));
                        Some(reg)
                    }
                    _ => None,
                }
            }
            Expression::Unary(op, rhs) => match op {
                UnaryOperator::Deref => {
                    let container = self.compile_expression(*rhs, loc)?;
                    Some(container)
                }
                _ => None,
            },
            Expression::Grouped(expr) => self.compile_place_expr(*expr, loc),
            _ => None,
        };
        return match r.is_some() {
            true => r,
            false => {
                self.emitError(loc, "Invalid place expression");
                None
            }
        };
    }
    fn compile_defer(&mut self, stmt: Statement) {
        self.defer_stack.last_mut().unwrap().push(stmt);
    }
    fn compile_declaration(&mut self, decl: Declaration, loc: SourceLocation) -> Option<Output> {
        self.define_var(decl.name.clone(), decl.ty);
        if decl.value.is_some() {
            let value = self.compile_expression(decl.value.unwrap(), loc)?;
            let symbol = self.lookup_symbol(&decl.name)?.id;
            let temp = self.alloc_get(TypeKind::Pointer(Box::new(value.ty.clone())))?;
            self.emit_instruction(Command::Add(
                Value::Location(Location::Symbol(symbol, 0)),
                Value::ARP,
                temp.clone(),
            ));
            self.emit_instruction(Command::Store(
                Value::Register(value.clone()),
                Value::Register(temp.clone()),
            ));
            self.deallocate_register(value.id);
            self.deallocate_register(temp.id);
        }
        None
    }
    fn compile_return(&mut self, stmt: ReturnStatement, loc: SourceLocation) -> Option<Output> {
        for dstmt in self.defer_stack.pop().unwrap_or_default() {
            self.compile_statement(dstmt);
        }
        if let Some(expr) = stmt.value {
            let output = self.compile_expression(expr, loc)?;
            let value = self.convert_output_to_value(output.clone());
            if output.ty.clone() != self.functions[self.current_fn].return_ty {
                self.emitError(
                    loc,
                    &format!(
                        "Return type mismatch, expected {}, got {}",
                        self.functions[self.current_fn].return_ty,
                        output.ty.clone()
                    ),
                );
                return None;
            }
            let return_ty = self.functions[self.current_fn].return_ty.clone();
            if let TypeKind::Optional(op) = return_ty.clone() {
                //if output is 0, just return 0, if not, fill implict param and return ptr to it
                let copy = self.alloc_get(TypeKind::Bool)?;
                self.emit_instruction(Command::Eq(
                    value,
                    Value::Immediate(Immediate {
                        value: 0.0,
                        ty: TypeKind::Optional(None),
                    }),
                    copy.clone(),
                ));
                self.emit_instruction(Command::JumpFalse(
                    Location::None,
                    Value::Register(copy.clone()),
                ));
                let copy_jump = self.get_last_jump_id();
                self.emit_instruction(Command::Ret(Some(Value::Immediate(Immediate {
                    value: 0.0,
                    ty: TypeKind::Optional(None),
                }))));
                let nb = self.new_block();
                self.update_jump(
                    copy_jump,
                    Command::JumpFalse(Location::Block(nb), Value::Register(copy.clone())),
                );
                let sizeof = self.size_of(self.unwrap_ptr_ty(return_ty.clone())?, loc)?;
                let arg = self.alloc_get(TypeKind::Pointer(Box::new(return_ty.clone())))?;
                let param_name = self.functions[self.current_fn]
                    .implict_params
                    .iter()
                    .filter(|x| x.param_ty == ImplicitParamType::ReturnPassthorugh)
                    .collect::<Vec<&ImplicitParam>>()[0]
                    .name
                    .clone()
                    .unwrap();
                self.emit_instruction(Command::Add(
                    Value::ARP,
                    Value::Location(Location::Argument(param_name)),
                    arg.clone(),
                ));
                let wloc_base = self.alloc_get(return_ty.clone())?;
                self.emit_instruction(Command::Load(
                    Value::Register(arg.clone()),
                    wloc_base.clone(),
                ));
                for i in 0..sizeof {
                    let offset = Value::Immediate(Immediate {
                        value: i as f64,
                        ty: TypeKind::Int16,
                    });
                    let rloc = self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Int16)))?;
                    self.emit_instruction(Command::Add(
                        offset.clone(),
                        Value::Register(output.clone()),
                        rloc.clone(),
                    ));
                    let byte = self.alloc_get(TypeKind::Int16)?;
                    self.emit_instruction(Command::Load(Value::Register(rloc), byte.clone()));
                    let wloc = self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Int16)))?;
                    self.emit_instruction(Command::Add(
                        offset,
                        Value::Register(wloc_base.clone()),
                        wloc.clone(),
                    ));
                    self.emit_instruction(Command::Store(
                        Value::Register(byte),
                        Value::Register(wloc),
                    ));
                }
                self.emit_instruction(Command::Ret(Some(Value::Register(wloc_base))));
                return None;
            }
            match self.is_internal_ptr(return_ty.clone()) {
                true => {
                    let size_of = self.size_of(self.unwrap_ptr_ty(return_ty.clone())?, loc)?;
                    let arg = self.alloc_get(TypeKind::Pointer(Box::new(return_ty.clone())))?;
                    let param_name = self.functions[self.current_fn]
                        .implict_params
                        .iter()
                        .filter(|x| x.param_ty == ImplicitParamType::ReturnPassthorugh)
                        .collect::<Vec<&ImplicitParam>>()[0]
                        .name
                        .clone()
                        .unwrap();
                    self.emit_instruction(Command::Add(
                        Value::ARP,
                        Value::Location(Location::Argument(param_name)),
                        arg.clone(),
                    ));
                    let sret = self.alloc_get(return_ty.clone())?;
                    self.emit_instruction(Command::Load(
                        Value::Register(arg.clone()),
                        sret.clone(),
                    ));
                    for i in 0..size_of {
                        let rloc = self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Int16)))?;
                        self.emit_instruction(Command::Add(
                            Value::Register(output.clone()),
                            Value::Immediate(Immediate {
                                value: i as f64,
                                ty: TypeKind::Int16,
                            }),
                            rloc.clone(),
                        ));
                        let temp = self.alloc_get(TypeKind::Int16)?;
                        self.emit_instruction(Command::Load(Value::Register(rloc), temp.clone()));
                        let wloc = self.alloc_get(TypeKind::Pointer(Box::new(TypeKind::Int16)))?;
                        self.emit_instruction(Command::Add(
                            Value::Register(sret.clone()),
                            Value::Immediate(Immediate {
                                value: i as f64,
                                ty: TypeKind::Int16,
                            }),
                            wloc.clone(),
                        ));
                        self.emit_instruction(Command::Store(
                            Value::Register(temp),
                            Value::Register(wloc),
                        ));
                    }
                    self.emit_instruction(Command::Ret(None))
                }
                false => {
                    self.emit_instruction(Command::Ret(Some(value)));
                    self.deallocate_register(output.id);
                }
            }
        } else {
            self.emit_instruction(Command::Ret(None));
        }
        None
    }
    fn compile_if(&mut self, stmt: IfStatement, loc: SourceLocation) -> Option<Output> {
        let condition = self.compile_expression(stmt.condition, loc.clone())?;
        let o_val = self.convert_output_to_value(condition.clone());
        self.emit_instruction(Command::JumpFalse(Location::None, o_val.clone()));
        self.deallocate_register(condition.id);
        let jump_id = self.get_last_jump_id();
        self.compile_statement(*stmt.then_block);
        let current_block = self.current_block();
        self.emit_instruction(Command::Jump(Location::Block(current_block + 1)));
        let end_jump = self.get_last_jump_id();
        if let Some(block) = stmt.else_block {
            let block = self.convert_stmt_to_block(*block, loc)?;
            let id = self.compile_block(block);
            self.update_jump(jump_id, Command::JumpFalse(Location::Block(id), o_val));
            let current_block = self.current_block();
            self.update_jump(end_jump, Command::Jump(Location::Block(current_block + 1)));
        } else {
            self.update_jump(
                jump_id,
                Command::JumpFalse(Location::Block(current_block + 1), o_val),
            );
        }
        self.new_block();
        None
    }
    fn compile_for(&mut self, stmt: ForStatement, loc: SourceLocation) {
        if let Some(init) = stmt.init {
            self.compile_statement(*init);
        }
        let body = stmt.body;
        let wstmt = WhileStatement {
            condition: match stmt.condition {
                Some(condition) => condition,
                None => Expression::Literal(Literal::Bool(true)),
            },
            body,
        };
        self.compile_while(
            wstmt,
            loc,
            match stmt.increment.is_some() {
                true => Some(Statement {
                    kind: StatementKind::Expression(stmt.increment.unwrap()),
                    loc: loc.clone(),
                }),
                _ => None,
            },
        );
    }
    fn compile_while(
        &mut self,
        stmt: WhileStatement,
        loc: SourceLocation,
        for_inc: Option<Statement>,
    ) -> Option<Output> {
        let condition_block = self.new_block();
        let condition = self.compile_expression(stmt.condition, loc)?;
        self.emit_instruction(Command::JumpFalse(
            Location::None,
            Value::Register(condition.clone()),
        ));
        self.deallocate_register(condition.id);
        let body_jump = self.get_last_jump_id();

        let body_block = self.new_block();
        self.add_loop(condition_block, body_block, for_inc.clone());
        self.compile_statement(*stmt.body);
        if for_inc.is_some() {
            self.compile_statement(for_inc?);
        }
        self.emit_instruction(Command::Jump(Location::Block(condition_block)));
        let next_block = self.new_block();
        self.update_jump(
            body_jump,
            Command::JumpFalse(
                Location::Block(next_block),
                Value::Register(condition.clone()),
            ),
        );
        let patches = self.functions[self.current_fn].loop_patches
            [&(self.functions[self.current_fn].loop_stack.len() - 1)]
            .clone();
        for patch in patches {
            self.update_jump(patch, Command::Jump(Location::Block(next_block)));
        }
        self.end_loop();
        None
    }
    fn compile_continue(&mut self, loc: SourceLocation) {
        if self.functions[self.current_fn].loop_stack.is_empty() {
            self.emitError(loc, "Continue statement not within a loop");
            return;
        }
        let currLoop = self.get_last_loop();
        if currLoop.increment.is_some() {
            self.compile_statement(currLoop.increment.unwrap());
        }
        self.emit_instruction(Command::Jump(Location::Block(currLoop.condition)));
    }
    fn compile_break(&mut self, loc: SourceLocation) {
        if self.functions[self.current_fn].loop_stack.is_empty() {
            self.emitError(loc, "Break statement not within a loop");
            return;
        }
        self.emit_instruction(Command::Jump(Location::None));
        let jump = self.get_last_jump_id();
        let last_loop = self.functions[self.current_fn].loop_stack.len() - 1;
        let patches = self.functions[self.current_fn]
            .loop_patches
            .get_mut(&last_loop);
        if let Some(patches) = patches {
            patches.push(jump);
        } else {
            self.emitError(loc, "Not in loop");
        }
    }
    fn get_last_jump_id(&self) -> usize {
        self.functions[self.current_fn].jump_stack.len() - 1
    }
    fn update_jump(&mut self, id: usize, replacement: Command) {
        let last_jump = self.functions[self.current_fn].jump_stack[id];
        let block = last_jump[0];
        let pos = last_jump[1];
        self.functions[self.current_fn].body[block][pos] = replacement;
    }
    fn current_block(&mut self) -> usize {
        self.functions[self.current_fn].current_block
    }
    fn convert_stmt_to_block(
        &mut self,
        stmt: Statement,
        loc: SourceLocation,
    ) -> Option<Vec<Statement>> {
        if let StatementKind::Block(block, _) = stmt.kind {
            Some(block)
        } else {
            self.emitError(loc, "Expected block statement");
            None
        }
    }
    fn add_loop(&mut self, conditon_block: usize, jump_block: usize, increment: Option<Statement>) {
        self.functions[self.current_fn]
            .loop_stack
            .push(LoopStackEntry {
                condition: conditon_block,
                jump: jump_block,
                increment,
            });
        let i = self.functions[self.current_fn].loop_stack.len() - 1;
        self.functions[self.current_fn]
            .loop_patches
            .insert(i, vec![]);
    }
    fn get_last_loop(&mut self) -> LoopStackEntry {
        self.functions[self.current_fn].loop_stack
            [self.functions[self.current_fn].loop_stack.len() - 1]
            .clone()
    }
    fn end_loop(&mut self) {
        self.functions[self.current_fn].loop_stack.pop();
        let i = self.functions[self.current_fn].loop_stack.len();
        self.functions[self.current_fn].loop_patches.remove(&i);
    }
    fn new_block(&mut self) -> usize {
        let id = self.functions[self.current_fn].body.len();
        self.functions[self.current_fn].current_block += 1;
        self.functions[self.current_fn].body.push(vec![]);
        id
    }
    fn get_current_block(&mut self) -> &mut Vec<Command> {
        let current_block = self.functions[self.current_fn].current_block;
        &mut self.functions[self.current_fn].body[current_block]
    }
    fn emit_instruction(&mut self, instruction: Command) {
        if self.emit {
            match &instruction {
                Command::JumpTrue(_j, _v) => {
                    let len = self.get_current_block().len();
                    let curr_block = self.functions[self.current_fn].current_block;
                    self.functions[self.current_fn]
                        .jump_stack
                        .push([curr_block, len]);
                    self.functions[self.current_fn].body[curr_block].push(instruction)
                }
                Command::JumpFalse(_j, _v) => {
                    let len = self.get_current_block().len();
                    let curr_block = self.functions[self.current_fn].current_block;
                    self.functions[self.current_fn]
                        .jump_stack
                        .push([curr_block, len]);
                    self.functions[self.current_fn].body[curr_block].push(instruction)
                }
                Command::Jump(_j) => {
                    let len = self.get_current_block().len();
                    let curr_block = self.functions[self.current_fn].current_block;
                    self.functions[self.current_fn]
                        .jump_stack
                        .push([curr_block, len]);
                    self.functions[self.current_fn].body[curr_block].push(instruction)
                }
                _ => {
                    let curr_block = self.functions[self.current_fn].current_block;
                    self.functions[self.current_fn].body[curr_block].push(instruction);
                }
            }
        }
    }
    fn convert_output_to_value(&mut self, output: Output) -> Value {
        Value::Register(output)
    }
    fn unwrap_reg_value(&self, val: Value) -> Option<Register> {
        match val {
            Value::Register(reg) => Some(reg),
            _ => None,
        }
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
        &mut self,
        ty: TypeKind,
        loc: SourceLocation,
    ) -> Option<(Vec<TypeKind>, TypeKind)> {
        match ty {
            TypeKind::Pointer(ptr) => match *ptr {
                TypeKind::Function(params, ret) => return Some((params, *ret)),
                _ => self.emitError(loc, &format!("Expected function type, got {}", ptr)),
            },
            _ => self.emitError(loc, &format!("Expected function type, got {}", ty)),
        }
        None
    }
    pub fn size_of(&self, ty: TypeKind, loc: SourceLocation) -> Option<usize> {
        let size = match ty {
            TypeKind::Int16 => Some(1),
            TypeKind::Uint16 => Some(1),
            TypeKind::Bool => Some(1),
            TypeKind::Int32 => Some(2),
            TypeKind::Uint32 => Some(2),
            TypeKind::Char => Some(1),
            TypeKind::Float32 => Some(2),
            TypeKind::Pointer(_) => Some(2),
            TypeKind::Array(aty, size) => {
                if size == usize::MAX {
                    self.emitError(loc, "Cannot take size of unsized array");
                    None
                } else {
                    Some(size * self.size_of(*aty, loc)?)
                }
            }
            TypeKind::Struct(name) => {
                let struct_def = self.lookup_symbol(&name)?;
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
            TypeKind::Union(ref name) => {
                let union_def = self.lookup_symbol(&name)?;
                if let Definition::User(UserType::Union(fields)) = &union_def.body {
                    Some(
                        (fields
                            .iter()
                            .map(|(_, ty)| Some(self.size_of(ty.clone(), loc.clone())?))
                            .max()??)
                            + 1,
                    )
                } else {
                    self.emitError(loc, &format!("Expected union type"));
                    None
                }
            }
            TypeKind::Optional(_) => Some(2),
        };
        size
    }
}
#[derive(Debug, Clone)]
struct LoopStackEntry {
    condition: usize,
    jump: usize,
    increment: Option<Statement>,
}
