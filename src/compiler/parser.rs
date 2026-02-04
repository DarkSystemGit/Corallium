use std::collections::HashMap;

use crate::compiler::lexer::{
    KeywordKind, Lexer, OperatorKind, SourceLocation, Token, TokenKind, TypeKind,
};
#[derive(Debug)]
pub struct Parser {
    input: Vec<Token>,
    pos: usize,
    src: String,
    file_name: String,
    struct_names: Vec<String>,
    union_names: Vec<String>,
    enum_names: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    pub loc: SourceLocation,
}
#[derive(Debug, Clone)]
pub enum StatementKind {
    Expression(Expression),
    Declaration(Declaration),
    Block(Vec<Statement>),
    If(IfStatement),
    While(WhileStatement),
    For(ForStatement),
    Function(FunctionDeclaration),
    Return(ReturnStatement),
    Struct(StructDeclaration),
    Enum(EnumDeclaration),
    Union(UnionDeclaration),
    Import(ImportDeclaration),
    Break,
    Continue,
}
#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub ty: TypeKind,
    pub value: Option<Expression>,
}
#[derive(Debug, Clone)]
pub struct ImportDeclaration {
    pub path: String,
}
#[derive(Debug, Clone)]
pub enum Expression {
    Unary(UnaryOperator, Box<Expression>),
    Binary(Box<Expression>, BinaryOperator, Box<Expression>),
    Grouped(Box<Expression>),
    Literal(Literal),
    Identifier(String),
    FunctionCall(Box<Expression>, Vec<Expression>),
    Cast(TypeKind, Box<Expression>),
    AddressOf(String),
    Subscript(Box<Expression>, Box<Expression>),
}
#[derive(Debug, Clone)]
pub enum Literal {
    Int(i32),
    Float(f32),
    String(String),
    Bool(bool),
    Array(Vec<Expression>),
    Struct(String, HashMap<String, Expression>),
    Enum(String, String),
    Union(String, String, Box<Expression>),
}
#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Not,
    Neg,
    Deref,
}
#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    PropertyAccess,
}
#[derive(Debug, Clone)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_block: Box<Statement>,
    pub else_block: Option<Box<Statement>>,
}
#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub condition: Expression,
    pub body: Box<Statement>,
}
#[derive(Debug, Clone)]
pub struct ForStatement {
    pub init: Option<Expression>,
    pub condition: Option<Expression>,
    pub increment: Option<Expression>,
    pub body: Box<Statement>,
}
#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
}
#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub params: HashMap<String, TypeKind>,
    pub return_ty: TypeKind,
    pub body: Box<Statement>,
}
#[derive(Debug, Clone)]
pub struct StructDeclaration {
    pub name: String,
    pub fields: HashMap<String, TypeKind>,
}
#[derive(Debug, Clone)]
pub struct EnumDeclaration {
    pub name: String,
    pub variants: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct UnionDeclaration {
    pub name: String,
    pub variants: HashMap<String, TypeKind>,
}
impl Parser {
    pub fn new(input: String, file_name: String) -> Self {
        let mut lexer = Lexer::new(input.clone());
        let tokens = lexer.lex();
        Parser {
            input: tokens,
            pos: 0,
            src: input,
            file_name,
            union_names: Vec::new(),
            struct_names: Vec::new(),
            enum_names: Vec::new(),
        }
    }
    pub fn parse(&mut self) -> Vec<Statement> {
        let mut statements: Vec<Option<Statement>> = Vec::new();
        while self.pos < self.input.len() {
            statements.push(self.parseStatement());
        }
        statements
            .iter()
            .filter(|x| x.is_some())
            .map(|x| x.clone().unwrap())
            .collect()
    }
    fn parseStatement(&mut self) -> Option<Statement> {
        match self.peek().kind {
            TokenKind::Keyword(KeywordKind::Let) => Some(self.parseLetStatement()?),
            TokenKind::Keyword(KeywordKind::If) => Some(self.parseIfStatement()?),
            TokenKind::Keyword(KeywordKind::While) => Some(self.parseWhileStatement()?),
            TokenKind::Keyword(KeywordKind::For) => Some(self.parseForStatement()?),
            TokenKind::Keyword(KeywordKind::Return) => Some(self.parseReturnStatement()?),
            TokenKind::Keyword(KeywordKind::Fn) => Some(self.parseFunctionDeclaration()?),
            TokenKind::Keyword(KeywordKind::Struct) => Some(self.parseStructDeclaration()?),
            TokenKind::Keyword(KeywordKind::Enum) => Some(self.parseEnumDeclaration()?),
            TokenKind::Keyword(KeywordKind::Union) => Some(self.parseUnionDeclaration()?),
            TokenKind::Keyword(KeywordKind::Import) => Some(self.parseImportDeclaration()?),
            TokenKind::Keyword(KeywordKind::Break) => Some(self.parseBreakStatement()?),
            TokenKind::Keyword(KeywordKind::Continue) => Some(self.parseContinueStatement()?),
            _ => {
                let loc = self.peek().loc.get_src_loc(&self.src);
                let exp = self.parseExpression();
                if exp.is_none() {
                    self.emitError(&format!(
                    "Unexpected token, {:?}, expected one of: let, if, while, for, return, fn, struct, enum, union, import, or expression",
                    self.input[self.pos].kind
                ));
                    while self.pos < self.input.len() && self.peek().kind != TokenKind::Semicolon {
                        self.next();
                    }
                    if self.pos < self.input.len() {
                        self.next();
                    }
                    None
                } else {
                    self.matchToken(TokenKind::Semicolon)?;
                    Some(Statement {
                        kind: StatementKind::Expression(exp.unwrap()),
                        loc,
                    })
                }
            }
        }
    }
    fn parseBreakStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        self.matchToken(TokenKind::Semicolon)?;
        Some(Statement {
            kind: StatementKind::Break,
            loc,
        })
    }
    fn parseContinueStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        self.matchToken(TokenKind::Semicolon)?;
        Some(Statement {
            kind: StatementKind::Continue,
            loc,
        })
    }
    fn parseEnumDeclaration(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        let name = self.matchIdentifier()?;
        self.matchToken(TokenKind::LeftBrace)?;
        let mut variants = Vec::new();
        while self.peek().kind != TokenKind::RightBrace {
            let variant_name = self.matchIdentifier()?;
            if self.peek().kind != TokenKind::RightBrace {
                self.matchToken(TokenKind::Comma)?;
            }
            /*self.enum_names
            .push(format!("{}::{}", name, variant_name.clone()));*/
            variants.push(variant_name);
        }
        self.enum_names.push(name.clone());
        self.matchToken(TokenKind::RightBrace)?;
        Some(Statement {
            kind: StatementKind::Enum(EnumDeclaration { name, variants }),
            loc,
        })
    }
    fn parseStructDeclaration(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        let name = self.matchIdentifier()?;
        self.struct_names.push(name.clone());
        self.matchToken(TokenKind::LeftBrace)?;
        let mut fields = HashMap::new();
        while self.peek().kind != TokenKind::RightBrace {
            let field_name = self.matchIdentifier()?;
            self.matchToken(TokenKind::Colon)?;
            let field_type = self.matchType()?;
            fields.insert(field_name, field_type);
            if self.peek().kind != TokenKind::RightBrace {
                self.matchToken(TokenKind::Comma)?;
            }
        }
        self.matchToken(TokenKind::RightBrace)?;
        Some(Statement {
            kind: StatementKind::Struct(StructDeclaration { name, fields }),
            loc,
        })
    }
    fn parseUnionDeclaration(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        let name = self.matchIdentifier()?;
        self.union_names.push(name.clone());
        self.matchToken(TokenKind::LeftBrace)?;
        let mut variants = HashMap::new();
        while self.peek().kind != TokenKind::RightBrace {
            let variant_name = self.matchIdentifier()?;
            self.matchToken(TokenKind::Colon)?;
            let variant_type = self.matchType()?;
            variants.insert(variant_name, variant_type);
            if self.peek().kind != TokenKind::RightBrace {
                self.matchToken(TokenKind::Comma)?;
            }
        }
        self.matchToken(TokenKind::RightBrace)?;
        Some(Statement {
            kind: StatementKind::Union(UnionDeclaration { name, variants }),
            loc,
        })
    }
    fn parseFunctionDeclaration(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        let name = self.matchIdentifier()?;
        self.matchToken(TokenKind::LeftParen)?;
        let mut params = HashMap::new();
        while self.peek().kind != TokenKind::RightParen {
            let param_name = self.matchIdentifier()?;
            self.matchToken(TokenKind::Colon)?;
            let param_type = self.matchType()?;
            if self.peek().kind != TokenKind::RightParen {
                self.matchToken(TokenKind::Comma)?;
            }
            params.insert(param_name, param_type);
        }
        self.matchToken(TokenKind::RightParen)?;
        self.matchToken(TokenKind::Arrow)?;
        let return_ty = self.matchType()?;
        let body = Box::new(self.parseBlockStatement()?);
        Some(Statement {
            kind: StatementKind::Function(FunctionDeclaration {
                name,
                body,
                return_ty,
                params,
            }),
            loc,
        })
    }
    fn parseImportDeclaration(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        if let TokenKind::String(rpath) = &self.next().kind {
            let path = rpath.clone();
            self.matchToken(TokenKind::Semicolon)?;
            Some(Statement {
                kind: StatementKind::Import(ImportDeclaration { path }),
                loc,
            })
        } else {
            self.emitError(&format!("Expected string literal for import path"));
            return None;
        }
    }
    fn parseWhileStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        self.matchToken(TokenKind::LeftParen)?;
        let condition = self.parseExpression()?;
        self.matchToken(TokenKind::RightParen)?;
        let body = Box::new(self.parseBlockStatement()?);
        Some(Statement {
            kind: StatementKind::While(WhileStatement { condition, body }),
            loc,
        })
    }
    fn parseForStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        self.matchToken(TokenKind::LeftParen)?;
        let init = match self.peek().kind {
            TokenKind::Semicolon => None,
            _ => Some(self.parseExpression()?),
        };
        self.matchToken(TokenKind::Semicolon)?;
        let condition = match self.peek().kind {
            TokenKind::Semicolon => None,
            _ => Some(self.parseExpression()?),
        };
        self.matchToken(TokenKind::Semicolon)?;
        let increment = match self.peek().kind {
            TokenKind::RightParen => None,
            _ => Some(self.parseExpression()?),
        };
        self.matchToken(TokenKind::RightParen)?;
        let body = Box::new(self.parseBlockStatement()?);
        Some(Statement {
            kind: StatementKind::For(ForStatement {
                init,
                condition,
                increment,
                body,
            }),
            loc,
        })
    }
    fn parseReturnStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        let expr = match self.peek().kind {
            TokenKind::Semicolon => None,
            _ => Some(self.parseExpression()?),
        };
        self.matchToken(TokenKind::Semicolon)?;
        Some(Statement {
            kind: StatementKind::Return(ReturnStatement { value: expr }),
            loc,
        })
    }
    fn parseIfStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        self.matchToken(TokenKind::LeftParen)?;
        let condition = self.parseExpression()?;
        self.matchToken(TokenKind::RightParen)?;
        let body = self.parseBlockStatement()?;
        let else_block = match self.peek().kind {
            TokenKind::Keyword(KeywordKind::Else) => {
                self.next();
                Some(Box::new(self.parseBlockStatement()?))
            }
            _ => None,
        };
        Some(Statement {
            kind: StatementKind::If(IfStatement {
                condition,
                then_block: Box::new(body),
                else_block,
            }),
            loc,
        })
    }
    fn parseBlockStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.matchToken(TokenKind::LeftBrace)?.loc.get_src_loc(src);
        let mut statements = Vec::new();
        while self.peek().kind != TokenKind::RightBrace {
            statements.push(self.parseStatement()?);
        }
        self.matchToken(TokenKind::RightBrace)?;
        Some(Statement {
            kind: StatementKind::Block(statements),
            loc,
        })
    }

    fn parseIdentifier(&mut self, ident: String) -> Option<Expression> {
        if self.struct_names.contains(&ident) {
            let mut fields = HashMap::new();
            self.matchToken(TokenKind::LeftBrace);
            while self.peek().kind != TokenKind::RightBrace {
                let field_name = self.matchIdentifier()?;
                self.matchToken(TokenKind::Colon);
                let field_value = self.parseExpression()?;
                fields.insert(field_name, field_value);
                if self.peek().kind != TokenKind::RightBrace {
                    self.matchToken(TokenKind::Comma)?;
                }
            }
            self.matchToken(TokenKind::RightBrace);
            Some(Expression::Literal(Literal::Struct(ident, fields)))
        } else {
            let mut name_unvariant_vec = ident.split("::").collect::<Vec<&str>>();
            let variant = name_unvariant_vec.pop();
            let name_unvariant = name_unvariant_vec.join("::");
            if self.union_names.contains(&name_unvariant) {
                self.matchToken(TokenKind::LeftParen);
                let value = self.parseExpression()?;
                self.matchToken(TokenKind::RightParen);
                Some(Expression::Literal(Literal::Union(
                    name_unvariant,
                    variant?.to_string(),
                    Box::new(value),
                )))
            } else if self.enum_names.contains(&name_unvariant) {
                Some(Expression::Literal(Literal::Enum(
                    name_unvariant,
                    variant?.to_string(),
                )))
            } else {
                Some(Expression::Identifier(ident))
            }
        }
    }
    fn parseArray(&mut self) -> Option<Expression> {
        let mut elements = Vec::new();
        while !(self.peek().kind == TokenKind::RightBracket) {
            elements.push(self.parseExpression()?);
            if self.peek().kind == TokenKind::Comma {
                self.next();
            } else if self.peek().kind == TokenKind::RightBracket {
                break;
            } else {
                self.emitError("Expected ',' or ']' after array element");
            }
        }
        self.matchToken(TokenKind::RightBracket)?;
        Some(Expression::Literal(Literal::Array(elements)))
    }
    fn parseExpression(&mut self) -> Option<Expression> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Option<Expression> {
        if self.pos >= self.input.len() {
            return None;
        }

        let token = self.next().clone();

        let mut lhs = match token.kind {
            TokenKind::Integer(i) => Expression::Literal(Literal::Int(i)),
            TokenKind::Float(f) => Expression::Literal(Literal::Float(f)),
            TokenKind::String(s) => Expression::Literal(Literal::String(s)),
            TokenKind::Bool(b) => Expression::Literal(Literal::Bool(b)),
            TokenKind::Identifier(name) => self.parseIdentifier(name)?,
            TokenKind::LeftBracket => self.parseArray()?,
            TokenKind::LeftParen => {
                let expr = self.parse_expr_bp(0)?;
                self.matchToken(TokenKind::RightParen)?;
                Expression::Grouped(Box::new(expr))
            }
            TokenKind::Operator(op) => {
                let ((), r_bp) = self.prefix_binding_power(&op);
                let rhs = self.parse_expr_bp(r_bp)?;
                match op {
                    OperatorKind::Not => Expression::Unary(UnaryOperator::Not, Box::new(rhs)),
                    OperatorKind::Negate => Expression::Unary(UnaryOperator::Neg, Box::new(rhs)),
                    OperatorKind::Ampersand => {
                        if let Expression::Identifier(ident) = rhs {
                            Expression::AddressOf(ident)
                        } else {
                            self.emitError("Address of operator can only be used with identifiers");
                            return None;
                        }
                    }
                    OperatorKind::Asterisk => {
                        Expression::Unary(UnaryOperator::Deref, Box::new(rhs))
                    }
                    _ => {
                        self.emitError("Unexpected prefix operator");
                        return None;
                    }
                }
            }
            _ => {
                self.pos -= 1;
                return None;
            }
        };

        while let Some((l_bp, r_bp)) = self.infix_binding_power(&self.peek().kind) {
            if l_bp < min_bp {
                break;
            }

            if self.peek().kind == TokenKind::LeftParen {
                lhs = self.parseFnCall(lhs)?;
                continue;
            }

            let op_token = self.next().clone();

            lhs = match op_token.kind {
                TokenKind::Operator(op) => {
                    let rhs = self.parse_expr_bp(r_bp)?;
                    let bin_op = match op {
                        OperatorKind::Add => BinaryOperator::Add,
                        OperatorKind::Negate => BinaryOperator::Sub,
                        OperatorKind::Asterisk => BinaryOperator::Mul,
                        OperatorKind::Div => BinaryOperator::Div,
                        OperatorKind::Mod => BinaryOperator::Mod,
                        OperatorKind::Eq => BinaryOperator::Eq,
                        OperatorKind::Neq => BinaryOperator::Ne,
                        OperatorKind::Lt => BinaryOperator::Lt,
                        OperatorKind::Lte => BinaryOperator::Le,
                        OperatorKind::Gt => BinaryOperator::Gt,
                        OperatorKind::Gte => BinaryOperator::Ge,
                        OperatorKind::Ampersand => BinaryOperator::And,
                        OperatorKind::Or => BinaryOperator::Or,
                        OperatorKind::Xor => BinaryOperator::Xor,
                        OperatorKind::Shl => BinaryOperator::Shl,
                        OperatorKind::Shr => BinaryOperator::Shr,
                        OperatorKind::Dot => BinaryOperator::PropertyAccess,
                        _ => return None,
                    };
                    Expression::Binary(Box::new(lhs), bin_op, Box::new(rhs))
                }
                TokenKind::Keyword(KeywordKind::As) => {
                    let ty = self.matchType()?;
                    Expression::Cast(ty, Box::new(lhs))
                }
                TokenKind::LeftBracket => {
                    let rhs = self.parseExpression()?;
                    self.matchToken(TokenKind::RightBracket)?;
                    Expression::Subscript(Box::new(lhs), Box::new(rhs))
                }
                _ => break,
            };
        }

        Some(lhs)
    }

    fn prefix_binding_power(&self, op: &OperatorKind) -> ((), u8) {
        match op {
            OperatorKind::Not
            | OperatorKind::Negate
            | OperatorKind::Ampersand
            | OperatorKind::Asterisk => ((), 99),
            _ => ((), 0),
        }
    }

    fn infix_binding_power(&self, kind: &TokenKind) -> Option<(u8, u8)> {
        match kind {
            TokenKind::LeftParen => Some((100, 0)),
            TokenKind::Keyword(KeywordKind::As) => Some((90, 91)),
            TokenKind::Operator(op) => match op {
                OperatorKind::Asterisk | OperatorKind::Div | OperatorKind::Mod => Some((80, 81)),
                OperatorKind::Add | OperatorKind::Negate => Some((70, 71)),
                OperatorKind::Shl | OperatorKind::Shr => Some((60, 61)),
                OperatorKind::Ampersand => Some((50, 51)),
                OperatorKind::Xor => Some((40, 41)),
                OperatorKind::Or => Some((30, 31)),
                OperatorKind::Eq
                | OperatorKind::Neq
                | OperatorKind::Lt
                | OperatorKind::Gt
                | OperatorKind::Lte
                | OperatorKind::Gte => Some((20, 21)),
                _ => None,
            },
            _ => None,
        }
    }

    fn parseFnCall(&mut self, left: Expression) -> Option<Expression> {
        self.matchToken(TokenKind::LeftParen)?;
        let mut args = Vec::new();
        while self.peek().kind != TokenKind::RightParen {
            args.push(self.parseExpression()?);
            if self.peek().kind == TokenKind::Comma {
                self.next();
            }
        }
        self.matchToken(TokenKind::RightParen)?;
        Some(Expression::FunctionCall(Box::new(left), args))
    }
    fn parseLetStatement(&mut self) -> Option<Statement> {
        let src = self.src.clone();
        let loc = self.next().loc.get_src_loc(src.as_str());
        let name = self.matchIdentifier()?;
        self.matchToken(TokenKind::Colon);
        let decl_type = self.matchType()?;
        self.matchToken(TokenKind::Operator(OperatorKind::Assign));
        let expr = self.parseExpression();
        self.matchToken(TokenKind::Semicolon)?;
        Some(Statement {
            kind: StatementKind::Declaration(Declaration {
                name,
                ty: decl_type,
                value: expr,
            }),
            loc,
        })
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
    fn matchType(&mut self) -> Option<TypeKind> {
        if let TokenKind::Type(t) = &(self.peek().kind) {
            let tk_type = t.clone();
            self.next();
            Some(tk_type)
        } else if let TokenKind::Identifier(ident) = &(self.peek().kind) {
            if self.struct_names.contains(ident) {
                let name = ident.clone();
                self.next();
                Some(TypeKind::Struct(name))
            } else if self.union_names.contains(ident) {
                let name = ident.clone();
                self.next();
                Some(TypeKind::Union(name))
            } else if self.enum_names.contains(ident) {
                let name = ident.clone();
                self.next();
                Some(TypeKind::Enum(name))
            } else {
                self.emitError(&format!("Expected type, found {:?}", self.peek().kind));
                None
            }
        } else if let TokenKind::Operator(OperatorKind::Ampersand) = &(self.peek().kind) {
            self.next();
            Some(TypeKind::Pointer(Box::new(self.matchType()?)))
        } else {
            self.emitError(&format!("Expected type, found {:?}", self.peek().kind));
            None
        }
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
    fn emitError(&self, message: &str) {
        let loc = self.input[self.pos].loc.get_src_loc(&self.src);
        println!(
            "Error while parsing at {} {}:{}:\n{}",
            self.file_name, loc.line, loc.col, message
        );
    }
}
