use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::compiler::ir::Header;
use crate::compiler::lexer::{
    KeywordKind, Lexer, OperatorKind, SourceLocation, Token, TokenKind, TypeKind,
};

use super::ir::{Definition, Symbol};
#[derive(Debug)]
pub struct Parser {
    input: Vec<Token>,
    pos: usize,
    src: String,
    file_name: String,
    type_table: TypeTable,
    quiet: bool,
}
#[derive(Debug, Clone)]
struct TypeTable {
    types: Vec<HashMap<String, UserType>>,
}
impl TypeTable {
    fn new() -> Self {
        TypeTable {
            types: vec![HashMap::new()],
        }
    }
    fn enter_scope(&mut self) {
        self.types.push(HashMap::new());
    }
    fn exit_scope(&mut self) {
        self.types.pop();
    }
    fn insert(&mut self, name: String, user_type: UserType) {
        self.types.last_mut().unwrap().insert(name, user_type);
    }
    fn lookup(&self, name: &str) -> Option<UserType> {
        for scope in self.types.iter().rev() {
            if let Some(r) = scope.get(name) {
                return Some((*r).clone());
            }
        }
        None
    }
    fn is_struct(&self, name: &str) -> bool {
        self.is_type(
            self.lookup(name).unwrap_or(UserType::None),
            UserType::Struct,
        )
    }
    fn is_union(&self, name: &str) -> bool {
        self.is_type(self.lookup(name).unwrap_or(UserType::None), UserType::Union)
    }
    fn is_enum(&self, name: &str) -> bool {
        self.is_type(self.lookup(name).unwrap_or(UserType::None), UserType::Enum)
    }
    fn is_alias(&self, name: &str) -> bool {
        self.lookup(name)
            .map_or(false, |t| matches!(t, UserType::Alias(_)))
    }
    fn is_type(&self, ty: UserType, match_type: UserType) -> bool {
        if match match_type {
            UserType::Struct => matches!(ty, UserType::Struct),
            UserType::Enum => matches!(ty, UserType::Enum),
            UserType::Union => matches!(ty, UserType::Union),
            _ => false,
        } {
            true
        } else if let UserType::Alias(kind) = ty {
            match kind {
                TypeKind::Enum(_) => matches!(match_type, UserType::Enum),
                TypeKind::Pointer(ptr) => match *ptr {
                    TypeKind::Struct(_) => matches!(match_type, UserType::Struct),
                    TypeKind::Union(_) => matches!(match_type, UserType::Union),
                    _ => false,
                },
                _ => false,
            }
        } else {
            false
        }
    }
}
#[derive(Debug, Clone)]
enum UserType {
    Struct,
    Enum,
    Union,
    Alias(TypeKind),
    None,
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
    Block(Vec<Statement>, Option<Expression>),
    If(IfStatement),
    While(WhileStatement),
    For(ForStatement),
    Function(FunctionDeclaration),
    Return(ReturnStatement),
    Struct(StructDeclaration),
    Enum(EnumDeclaration),
    Union(UnionDeclaration),
    Import(ImportDeclaration),
    ImplictRet(Expression),
    Defer(Box<Statement>),
    Break,
    Continue,
}
#[derive(Debug, Clone)]
pub struct MatchExpression {
    pub expr: Box<Expression>,
    pub cases: Vec<MatchCase>,
}
#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub body: Statement,
}
#[derive(Debug, Clone)]
pub enum Pattern {
    Identifier(String),
    Union(String, String, Box<Pattern>),
    Enum(String, String),
    Array(Vec<Pattern>),
    Struct(String, Vec<(String, Pattern)>),
    Wildcard,
    Literal(Literal),
    Some(Box<Pattern>),
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
    pub header: Header,
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
    AddressOf(Box<Expression>),
    Subscript(Box<Expression>, Box<Expression>),
    Match(MatchExpression),
    Sizeof(TypeKind),
    Try(Box<Expression>, Option<Box<Statement>>),
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
    None,
    Some(Box<Expression>),
}
#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Not,
    Neg,
    Deref,
}
#[derive(Debug, Clone, PartialEq)]
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
    Assign,
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
    pub init: Option<Box<Statement>>,
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
    pub params: BTreeMap<String, TypeKind>,
    pub return_ty: TypeKind,
    pub body: Box<Statement>,
}
#[derive(Debug, Clone)]
pub struct StructDeclaration {
    pub name: String,
    pub fields: BTreeMap<String, TypeKind>,
}
#[derive(Debug, Clone)]
pub struct EnumDeclaration {
    pub name: String,
    pub variants: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct UnionDeclaration {
    pub name: String,
    pub variants: BTreeMap<String, TypeKind>,
}
impl Parser {
    pub fn new(input: String, file_name: String, quiet: bool) -> Self {
        let mut lexer = Lexer::new(input.clone());
        let tokens = lexer.lex();
        Parser {
            input: tokens,
            pos: 0,
            src: input,
            file_name,
            type_table: TypeTable::new(),
            quiet,
        }
    }
    pub fn parse(&mut self) -> (Vec<Statement>, Header) {
        let mut statements: Vec<Option<Statement>> = Vec::new();
        while self.pos < self.input.len() {
            statements.push(self.parseStatement());
        }
        let stmts = statements
            .iter()
            .filter(|x| x.is_some())
            .map(|x| x.clone().unwrap())
            .collect();
        let header = self.gen_header(&stmts);
        (stmts, header)
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
            TokenKind::Keyword(KeywordKind::Type) => Some(self.parseTypeDecl()?),
            TokenKind::Keyword(KeywordKind::Defer) => Some(self.parseDefer()?),
            TokenKind::LeftBrace => Some(self.parseBlockStatement()?),
            _ => {
                let loc = self.peek().loc.get_src_loc(&self.src);
                let exp = self.parseExpression();
                if exp.is_none() {
                    self.emitError(&format!(
                        "Unexpected token, {:?}, expected statement, or expression",
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
                    self.parseExpressionStatement(exp.unwrap(), loc)
                }
            }
        }
    }
    fn parseExpressionStatement(
        &mut self,
        lhs: Expression,
        loc: SourceLocation,
    ) -> Option<Statement> {
        if self.peek().kind == TokenKind::Semicolon {
            self.next();
            Some(Statement {
                kind: StatementKind::Expression(lhs),
                loc,
            })
        } else {
            Some(Statement {
                kind: StatementKind::ImplictRet(lhs),
                loc,
            })
        }
    }
    fn parseDefer(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        let body = self.parseStatement()?;
        Some(Statement {
            kind: StatementKind::Defer(Box::new(body)),
            loc,
        })
    }
    fn parseMatch(&mut self) -> Option<Expression> {
        let expr = Box::new(self.parseExpression()?);
        let mut cases = vec![];
        self.matchToken(TokenKind::LeftBrace)?;
        while self.peek().kind != TokenKind::RightBrace {
            let pat = self.parsePattern()?;
            self.matchToken(TokenKind::Arrow)?;

            let stmt = self.parseStatement()?;

            cases.push(MatchCase {
                pattern: pat,
                body: stmt,
            });
            if self.peek().kind != TokenKind::RightBrace {
                self.matchToken(TokenKind::Comma);
            }
        }
        self.matchToken(TokenKind::RightBrace)?;
        Some(Expression::Match(MatchExpression { expr, cases }))
    }
    fn parsePattern(&mut self) -> Option<Pattern> {
        match self.peek().kind.clone() {
            TokenKind::Identifier(ident) => {
                self.next();
                match ident.as_str() {
                    "_" => Some(Pattern::Wildcard),
                    _ => {
                        if self.type_table.is_struct(&ident) {
                            let mut fields = Vec::new();
                            self.matchToken(TokenKind::LeftBrace)?;
                            while self.peek().kind != TokenKind::RightBrace {
                                let field = self.matchIdentifier()?;
                                self.matchToken(TokenKind::Colon)?;
                                let pat = self.parsePattern()?;
                                fields.push((field, pat));
                            }
                            self.matchToken(TokenKind::RightBrace)?;
                            Some(Pattern::Struct(ident, fields))
                        } else {
                            let mut name_unvariant_vec = ident.split("::").collect::<Vec<&str>>();
                            let variant = name_unvariant_vec.pop()?;
                            let name_unvariant = name_unvariant_vec.join("::");

                            if self.type_table.is_union(&name_unvariant) {
                                let actual_name =
                                    match self.type_table.lookup(&name_unvariant).unwrap() {
                                        UserType::Alias(ty) => match ty {
                                            TypeKind::Union(name) => Some(name),
                                            TypeKind::Enum(name) => Some(name),
                                            _ => None,
                                        },
                                        UserType::Union => Some(name_unvariant.clone()),
                                        UserType::Enum => Some(name_unvariant.clone()),
                                        _ => None,
                                    }
                                    .unwrap();
                                self.matchToken(TokenKind::LeftParen);

                                let pat = self.parsePattern()?;
                                self.matchToken(TokenKind::RightParen);
                                Some(Pattern::Union(
                                    actual_name,
                                    variant.to_string(),
                                    Box::new(pat),
                                ))
                            } else if self.type_table.is_enum(&name_unvariant) {
                                let actual_name =
                                    match self.type_table.lookup(&name_unvariant).unwrap() {
                                        UserType::Alias(ty) => match ty {
                                            TypeKind::Union(name) => Some(name),
                                            TypeKind::Enum(name) => Some(name),
                                            _ => None,
                                        },
                                        UserType::Union => Some(name_unvariant.clone()),
                                        UserType::Enum => Some(name_unvariant.clone()),
                                        _ => None,
                                    }
                                    .unwrap();
                                Some(Pattern::Enum(actual_name, variant.to_string()))
                            } else {
                                Some(Pattern::Identifier(ident))
                            }
                        }
                    }
                }
            }
            TokenKind::LeftBracket => {
                self.next();
                let mut elements = Vec::new();
                while self.peek().kind != TokenKind::RightBracket {
                    elements.push(self.parsePattern()?);
                    if self.peek().kind == TokenKind::RightBracket {
                        break;
                    }
                    self.matchToken(TokenKind::Comma)?;
                }
                self.matchToken(TokenKind::RightBracket)?;
                Some(Pattern::Array(elements))
            }
            TokenKind::Integer(i) => {
                self.next();
                Some(Pattern::Literal(Literal::Int(i)))
            }
            TokenKind::Float(f) => {
                self.next();
                Some(Pattern::Literal(Literal::Float(f)))
            }
            TokenKind::String(s) => {
                self.next();
                Some(Pattern::Literal(Literal::String(s)))
            }
            TokenKind::Bool(b) => {
                self.next();
                Some(Pattern::Literal(Literal::Bool(b)))
            }
            TokenKind::Keyword(KeywordKind::None) => {
                self.next();
                Some(Pattern::Literal(Literal::None))
            }
            TokenKind::Keyword(KeywordKind::Some) => {
                self.next();
                self.matchToken(TokenKind::LeftParen);
                let inner = self.parsePattern()?;
                self.matchToken(TokenKind::RightParen);
                Some(Pattern::Some(Box::new(inner)))
            }
            _ => None,
        }
    }
    fn parseTypeDecl(&mut self) -> Option<Statement> {
        self.next();
        let name = self.matchIdentifier()?;
        self.matchToken(TokenKind::Operator(OperatorKind::Assign));
        let ty = self.matchType()?;
        self.type_table.insert(name, UserType::Alias(ty));
        self.matchToken(TokenKind::Semicolon)?;
        None
    }
    fn parseBreakStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        if self.peek().kind == TokenKind::Semicolon {
            self.matchToken(TokenKind::Semicolon)?;
        }
        Some(Statement {
            kind: StatementKind::Break,
            loc,
        })
    }
    fn parseContinueStatement(&mut self) -> Option<Statement> {
        let src = &self.src.clone();
        let loc = self.next().loc.get_src_loc(src);
        if self.peek().kind == TokenKind::Semicolon {
            self.matchToken(TokenKind::Semicolon)?;
        }
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
        self.type_table.insert(name.clone(), UserType::Enum);
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
        self.type_table.insert(name.clone(), UserType::Struct);
        self.matchToken(TokenKind::LeftBrace)?;
        let mut fields = BTreeMap::new();
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
        self.type_table.insert(name.clone(), UserType::Union);
        self.matchToken(TokenKind::LeftBrace)?;
        let mut variants = BTreeMap::new();
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
        let mut params = BTreeMap::new();
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
        let file_name = self.file_name.clone();
        if let TokenKind::String(rpath) = &self.next().kind {
            let mut ancestors = Path::new(&file_name).ancestors();
            ancestors.next();
            let path = ancestors
                .next()?
                .join(&(rpath.clone()))
                .to_str()
                .unwrap()
                .to_string();
            self.matchToken(TokenKind::Semicolon)?;
            let file = std::fs::read_to_string(path.clone())
                .expect(&format!("Invalid import path: {}", &path));
            let import_name = Path::new(&path)
                .file_name()
                .expect(&format!("Invalid import path: {}", &path))
                .to_str()
                .unwrap()
                .to_string();
            let mut parser = Parser::new(file, import_name.clone(), true);
            let header = parser.parse().1;
            for i in header.symbols.iter() {
                match &i.body {
                    Definition::User(ty) => {
                        self.type_table.insert(
                            format!(
                                "{}::{}",
                                Path::new(&import_name).file_stem()?.display(),
                                i.name.clone()
                            ),
                            match ty {
                                super::ir::UserType::Enum(_) => UserType::Enum,
                                super::ir::UserType::Struct(_) => UserType::Struct,
                                super::ir::UserType::Union(_) => UserType::Union,
                            },
                        );
                    }
                    _ => {}
                }
            }
            Some(Statement {
                kind: StatementKind::Import(ImportDeclaration { path, header }),
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
        self.matchToken(TokenKind::Semicolon);
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
            _ => Some(Box::new(self.parseStatement()?)),
        };
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
        self.matchToken(TokenKind::Semicolon)?;
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
        self.matchToken(TokenKind::Semicolon)?;
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
        self.type_table.enter_scope();
        while self.peek().kind != TokenKind::RightBrace {
            statements.push(self.parseStatement()?);
        }
        let mut pop_stmt = false;
        let implicit_return = statements
            .last()
            .map(|stmt| match stmt.kind.clone() {
                StatementKind::ImplictRet(expr) => {
                    pop_stmt = true;
                    Some(expr)
                }
                _ => None,
            })
            .unwrap_or(None);
        if pop_stmt {
            statements.pop();
        }
        self.type_table.exit_scope();
        self.matchToken(TokenKind::RightBrace)?;
        Some(Statement {
            kind: StatementKind::Block(statements, implicit_return),
            loc,
        })
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
            TokenKind::Keyword(KeywordKind::None) => Expression::Literal(Literal::None),
            TokenKind::Keyword(KeywordKind::Some) => {
                self.matchToken(TokenKind::LeftParen)?;
                let expr = self.parse_expr_bp(0)?;
                self.matchToken(TokenKind::RightParen)?;
                Expression::Literal(Literal::Some(Box::new(expr)))
            }
            TokenKind::Identifier(name) => self.parseIdentifier(name)?,
            TokenKind::LeftBracket => self.parseArray()?,
            TokenKind::LeftParen => {
                let expr = self.parse_expr_bp(0)?;
                self.matchToken(TokenKind::RightParen)?;
                Expression::Grouped(Box::new(expr))
            }
            TokenKind::Keyword(KeywordKind::Match) => self.parseMatch()?,
            TokenKind::Keyword(KeywordKind::Try) => {
                let expr = self.parse_expr_bp(0)?;
                let catch_block = match self.peek().kind == TokenKind::Keyword(KeywordKind::Catch) {
                    true => {
                        self.next();
                        Some(Box::new(self.parseStatement()?))
                    }
                    _ => None,
                };
                Expression::Try(Box::new(expr), catch_block)
            }
            TokenKind::Operator(op) => {
                if op != OperatorKind::Sizeof {
                    let ((), r_bp) = self.prefix_binding_power(&op);
                    let rhs = self.parse_expr_bp(r_bp)?;
                    match op {
                        OperatorKind::Not => Expression::Unary(UnaryOperator::Not, Box::new(rhs)),
                        OperatorKind::Negate => {
                            Expression::Unary(UnaryOperator::Neg, Box::new(rhs))
                        }
                        OperatorKind::Ampersand => Expression::AddressOf(Box::new(rhs)),
                        OperatorKind::Asterisk => {
                            Expression::Unary(UnaryOperator::Deref, Box::new(rhs))
                        }
                        _ => {
                            self.emitError("Unexpected prefix operator");
                            return None;
                        }
                    }
                } else {
                    self.matchToken(TokenKind::LeftParen);
                    let r = self.matchType()?;
                    self.matchToken(TokenKind::RightParen);
                    Expression::Sizeof(r)
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
                        OperatorKind::Assign => BinaryOperator::Assign,
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
            | OperatorKind::Sizeof
            | OperatorKind::Negate
            | OperatorKind::Ampersand
            | OperatorKind::Asterisk => ((), 99),
            _ => ((), 0),
        }
    }

    fn infix_binding_power(&self, kind: &TokenKind) -> Option<(u8, u8)> {
        match kind {
            TokenKind::LeftParen => Some((100, 0)),
            TokenKind::LeftBracket => Some((100, 0)),
            TokenKind::Keyword(KeywordKind::As) => Some((90, 91)),
            TokenKind::Operator(op) => match op {
                OperatorKind::Asterisk | OperatorKind::Div | OperatorKind::Mod => Some((80, 81)),
                OperatorKind::Add | OperatorKind::Negate => Some((70, 71)),
                OperatorKind::Shl | OperatorKind::Shr => Some((60, 61)),
                OperatorKind::Ampersand => Some((50, 51)),
                OperatorKind::Xor => Some((40, 41)),
                OperatorKind::Or => Some((30, 31)),
                OperatorKind::Dot => Some((100, 0)),
                OperatorKind::Eq
                | OperatorKind::Neq
                | OperatorKind::Lt
                | OperatorKind::Gt
                | OperatorKind::Lte
                | OperatorKind::Gte => Some((20, 21)),
                OperatorKind::Assign => Some((10, 9)),
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
    fn parseIdentifier(&mut self, ident: String) -> Option<Expression> {
        if self.type_table.is_struct(&ident) {
            let lookup = self.type_table.lookup(&ident).unwrap();
            let name = match lookup {
                UserType::Alias(ty) => match ty {
                    TypeKind::Pointer(ptr) => {
                        if let TypeKind::Struct(name) = *ptr {
                            Some(name)
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                UserType::Struct => Some(ident.clone()),
                _ => None,
            }
            .unwrap();
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
            Some(Expression::Literal(Literal::Struct(name, fields)))
        } else {
            let mut name_unvariant_vec = ident.split("::").collect::<Vec<&str>>();
            let variant = name_unvariant_vec.pop();
            let name_unvariant = name_unvariant_vec.join("::");
            if self.type_table.is_union(&name_unvariant) {
                let actual_name = match self.type_table.lookup(&name_unvariant).unwrap() {
                    UserType::Alias(ty) => match ty {
                        TypeKind::Pointer(ptr) => {
                            if let TypeKind::Union(name) = *ptr {
                                Some(name)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    },
                    UserType::Union => Some(name_unvariant.clone()),
                    _ => None,
                }
                .unwrap();
                self.matchToken(TokenKind::LeftParen);
                let value = self.parseExpression()?;
                self.matchToken(TokenKind::RightParen);
                Some(Expression::Literal(Literal::Union(
                    actual_name,
                    variant?.to_string(),
                    Box::new(value),
                )))
            } else if self.type_table.is_enum(&name_unvariant) {
                let actual_name = match self.type_table.lookup(&name_unvariant).unwrap() {
                    UserType::Alias(ty) => match ty {
                        TypeKind::Union(name) => Some(name),
                        _ => None,
                    },
                    UserType::Enum => Some(name_unvariant.clone()),
                    _ => None,
                }
                .unwrap();
                Some(Expression::Literal(Literal::Enum(
                    actual_name,
                    variant?.to_string(),
                )))
            } else {
                Some(Expression::Identifier(ident))
            }
        }
    }
    fn matchType(&mut self) -> Option<TypeKind> {
        let next = self.peek().clone();
        match next.kind {
            TokenKind::Operator(OperatorKind::Ampersand) => {
                self.next();
                let inner_type = self.matchType()?;
                Some(TypeKind::Pointer(Box::new(inner_type)))
            }
            TokenKind::LeftBracket => {
                self.next();
                let content = self.matchType()?;
                self.matchToken(TokenKind::Semicolon);
                let count = self.matchInteger()? as usize;
                self.matchToken(TokenKind::RightBracket);
                Some(TypeKind::Pointer(Box::new(TypeKind::Array(
                    Box::new(content),
                    count,
                ))))
            }
            TokenKind::Type(t) => {
                self.next();
                Some(t)
            }
            TokenKind::Identifier(ident) => {
                self.next();
                let lookup = self.type_table.lookup(&ident);
                if let Some(lookup) = lookup {
                    match lookup {
                        UserType::Alias(a) => Some(a),
                        UserType::Struct => {
                            Some(TypeKind::Pointer(Box::new(TypeKind::Struct(ident))))
                        }
                        UserType::Union => {
                            Some(TypeKind::Pointer(Box::new(TypeKind::Union(ident))))
                        }
                        UserType::Enum => Some(TypeKind::Enum(ident)),
                        UserType::None => None,
                    }
                } else {
                    self.emitError(&format!("No such type {}", ident));
                    None
                }
            }
            _ => {
                self.emitError(&format!("Expected type, got {:?}", next));
                None
            }
        }
    }

    fn matchInteger(&mut self) -> Option<i32> {
        if let TokenKind::Integer(n) = self.peek().kind {
            self.next();
            Some(n)
        } else {
            self.emitError(&format!("Expected int, found {:?}", self.peek().kind));
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
        if !self.quiet {
            println!(
                "Error while parsing at {} {}:{}:\n{}",
                self.file_name, loc.line, loc.col, message
            );
        }
    }
    fn gen_header(&self, statements: &Vec<Statement>) -> Header {
        let mut symbols = vec![];
        let mut fn_id = 0;
        for statement in statements {
            match &statement.kind {
                StatementKind::Function(f) => {
                    symbols.push(Symbol {
                        name: f.name.clone(),
                        body: Definition::Function(
                            TypeKind::Function(
                                f.params.values().map(|x| x.clone()).collect(),
                                Box::new(f.return_ty.clone()),
                            ),
                            fn_id,
                        ),
                        id: 0,
                        size: None,
                    });
                    fn_id += 1;
                }
                StatementKind::Struct(s) => {
                    symbols.push(Symbol {
                        name: s.name.clone(),
                        body: Definition::User(super::ir::UserType::Struct(s.fields.clone())),
                        id: 0,
                        size: None,
                    });
                }
                StatementKind::Union(u) => {
                    symbols.push(Symbol {
                        name: u.name.clone(),
                        body: Definition::User(super::ir::UserType::Union(u.variants.clone())),
                        id: 0,
                        size: None,
                    });
                }
                StatementKind::Enum(e) => {
                    symbols.push(Symbol {
                        name: e.name.clone(),
                        body: Definition::User(super::ir::UserType::Enum(e.variants.clone())),
                        id: 0,
                        size: None,
                    });
                }
                _ => {}
            }
        }
        Header {
            module: Path::new(&self.file_name)
                .file_stem()
                .expect("Invalid File Path")
                .to_str()
                .expect("Invalid File Path")
                .to_string(),
            symbols,
        }
    }
}
