
pub struct Lexer {
    input: InputStream,
    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(input: String) -> Self {
        Lexer {
            input: InputStream { input, position: 0 },
            tokens: Vec::new(),
        }
    }
    pub fn lex(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while self.input.peek().is_some() {
            let tk = self.read_token();
            if tk.kind == TokenKind::None {
                continue;
            }
            tokens.push(tk);
        }
        tokens
    }
    fn read_token(&mut self) -> Token {
        let start = self.input.position;
        let ch = self.input.next().unwrap();
        match ch {
            '(' => Token::new(TokenKind::LeftParen, start, start + 1),
            ')' => Token::new(TokenKind::RightParen, start, start + 1),
            '{' => Token::new(TokenKind::LeftBrace, start, start + 1),
            '}' => Token::new(TokenKind::RightBrace, start, start + 1),
            '[' => {
                let saved_pos = self.input.position;
                let mut candidate = String::from("[");
                let mut depth = 1;
                while let Some(c) = self.input.peek() {
                    let next_char = self.input.next().unwrap();
                    candidate.push(next_char);
                    match next_char {
                        '[' => depth += 1,
                        ']' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let loc = Location {
                    start,
                    end: start + candidate.len(),
                };
                if depth == 0 {
                    if let Some(ty_kind) = get_type(&candidate, &loc) {
                        return Token::new(TokenKind::Type(ty_kind), start, loc.end);
                    }
                }
                self.input.position = saved_pos;
                Token::new(TokenKind::LeftBracket, start, start + 1)
            }
            ']' => Token::new(TokenKind::RightBracket, start, start + 1),
            ':' => Token::new(TokenKind::Colon, start, start + 1),
            ';' => Token::new(TokenKind::Semicolon, start, start + 1),
            ',' => Token::new(TokenKind::Comma, start, start + 1),
            '=' => {
                if self.input.matchTk('=') {
                    Token::new(TokenKind::Operator(OperatorKind::Eq), start, start + 2)
                } else {
                    Token::new(TokenKind::Operator(OperatorKind::Assign), start, start + 1)
                }
            }
            '!' => {
                if self.input.matchTk('=') {
                    Token::new(TokenKind::Operator(OperatorKind::Neq), start, start + 2)
                } else {
                    Token::new(TokenKind::Operator(OperatorKind::Not), start, start + 1)
                }
            }

            '>' => {
                if self.input.matchTk('=') {
                    Token::new(TokenKind::Operator(OperatorKind::Gte), start, start + 2)
                } else if self.input.matchTk('>') {
                    Token::new(TokenKind::Operator(OperatorKind::Shr), start, start + 2)
                } else {
                    Token::new(TokenKind::Operator(OperatorKind::Gt), start, start + 1)
                }
            }
            '<' => {
                if self.input.matchTk('=') {
                    Token::new(TokenKind::Operator(OperatorKind::Lte), start, start + 2)
                } else if self.input.matchTk('<') {
                    Token::new(TokenKind::Operator(OperatorKind::Shl), start, start + 2)
                } else {
                    Token::new(TokenKind::Operator(OperatorKind::Lt), start, start + 1)
                }
            }
            '+' => Token::new(TokenKind::Operator(OperatorKind::Add), start, start + 1),
            '-' => {
                if self.input.matchTk('>') {
                    Token::new(TokenKind::Arrow, start, start + 2)
                } else {
                    Token::new(TokenKind::Operator(OperatorKind::Negate), start, start + 1)
                }
            }
            '*' => Token::new(
                TokenKind::Operator(OperatorKind::Asterisk),
                start,
                start + 1,
            ),
            '.' => Token::new(TokenKind::Operator(OperatorKind::Dot), start, start + 1),
            '/' => {
                if self.input.matchTk('/') {
                    while self.input.peek() != Some('\n') {
                        self.input.next();
                    }
                    Token::new(TokenKind::None, start, start)
                } else {
                    Token::new(TokenKind::Operator(OperatorKind::Div), start, start + 1)
                }
            }
            '%' => Token::new(TokenKind::Operator(OperatorKind::Mod), start, start + 1),
            '&' => {
                let saved_pos = self.input.position;
                let mut candidate = String::from("&");
                if self.input.peek() == Some('[') {
                    let mut depth = 0;
                    loop {
                        if let Some(c) = self.input.peek() {
                            self.input.next();
                            candidate.push(c);
                            if c == '[' {
                                depth += 1;
                            }
                            if c == ']' {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                } else {
                    while let Some(c) = self.input.peek() {
                        if is_alphanumeric(c) {
                            candidate.push(self.input.next().unwrap());
                        } else {
                            break;
                        }
                    }
                }
                let loc = Location {
                    start,
                    end: start + candidate.len(),
                };
                let looks_like_ptr = candidate.len() > 1 && !candidate.ends_with('&');
                if looks_like_ptr {
                    if let Some(ty_kind) = get_type(&candidate, &loc) {
                        return Token::new(TokenKind::Type(ty_kind), start, loc.end);
                    }
                }
                self.input.position = saved_pos;
                Token::new(
                    TokenKind::Operator(OperatorKind::Ampersand),
                    start,
                    start + 1,
                )
            }
            '|' => Token::new(TokenKind::Operator(OperatorKind::Or), start, start + 1),
            '^' => Token::new(TokenKind::Operator(OperatorKind::Xor), start, start + 1),
            '"' => {
                let mut string = String::new();
                while self.input.peek().is_some() && self.input.peek().unwrap() != '"' {
                    string.push(self.input.next().unwrap());
                }
                self.input.next();
                let end = start + string.len() + 2;
                Token::new(TokenKind::String(string), start, end)
            }
            c => {
                if is_whitespace(c) {
                    Token::new(TokenKind::None, start, start + 1)
                } else if is_digit(c) {
                    let mut num = String::from(c);
                    let mut float = false;
                    while self.input.peek().is_some()
                        && (is_digit(self.input.peek().unwrap())
                            || (self.input.peek().unwrap() == '.' && !float))
                    {
                        if self.input.peek().unwrap() == '.' {
                            float = true;
                        }
                        num.push(self.input.next().unwrap());
                    }
                    Token::new(
                        match float {
                            true => TokenKind::Float(num.parse().unwrap()),
                            false => TokenKind::Integer(num.parse().unwrap()),
                        },
                        start,
                        start + num.len(),
                    )
                } else if is_alpha(ch) {
                    let mut ident = String::from(c);
                    while self.input.peek().is_some()
                        && (is_alphanumeric(self.input.peek().unwrap())
                            || (self.input.peek().unwrap() == ':'
                                && self.input.peek_next(1).unwrap() == ':'))
                    {
                        if self.input.peek().unwrap() == ':'
                            && self.input.peek_next(1).unwrap() == ':'
                        {
                            ident.push_str("::");
                            self.input.next();
                            self.input.next();
                        } else {
                            ident.push(self.input.next().unwrap());
                        }
                    }
                    if get_keyword(&ident).is_some() {
                        Token::new(
                            TokenKind::Keyword(get_keyword(&ident).unwrap()),
                            start,
                            start + ident.len(),
                        )
                    } else if get_type(
                        &ident,
                        &Location {
                            start,
                            end: start + ident.len(),
                        },
                    )
                    .is_some()
                    {
                        Token::new(
                            TokenKind::Type(
                                get_type(
                                    &ident,
                                    &Location {
                                        start,
                                        end: start + ident.len(),
                                    },
                                )
                                .unwrap(),
                            ),
                            start,
                            start + ident.len(),
                        )
                    } else if ["true", "false"].contains(&ident.as_str()) {
                        Token::new(
                            TokenKind::Bool(match ident.as_str() {
                                "true" => true,
                                "false" => false,
                                _ => unreachable!(),
                            }),
                            start,
                            start + ident.len(),
                        )
                    } else if ["sizeof"].contains(&ident.as_str()) {
                        Token::new(
                            TokenKind::Operator(OperatorKind::Sizeof),
                            start,
                            start + ident.len(),
                        )
                    } else {
                        let len = ident.len();
                        Token::new(TokenKind::Identifier(ident), start, start + len)
                    }
                } else {
                    Token::new(TokenKind::None, start, start + 1)
                }
            }
        }
    }
}
fn is_digit(ch: char) -> bool {
    ch.is_digit(10)
}
fn is_whitespace(ch: char) -> bool {
    ch.is_whitespace()
}
fn is_alpha(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}
fn is_alphanumeric(ch: char) -> bool {
    is_alpha(ch) || is_digit(ch)
}
fn get_keyword(ident: &String) -> Option<KeywordKind> {
    match ident.as_str() {
        "let" => Some(KeywordKind::Let),
        "fn" => Some(KeywordKind::Fn),
        "if" => Some(KeywordKind::If),
        "else" => Some(KeywordKind::Else),
        "return" => Some(KeywordKind::Return),
        "as" => Some(KeywordKind::As),
        "match" => Some(KeywordKind::Match),
        "struct" => Some(KeywordKind::Struct),
        "union" => Some(KeywordKind::Union),
        "enum" => Some(KeywordKind::Enum),
        "while" => Some(KeywordKind::While),
        "for" => Some(KeywordKind::For),
        "break" => Some(KeywordKind::Break),
        "continue" => Some(KeywordKind::Continue),
        "import" => Some(KeywordKind::Import),
        "type" => Some(KeywordKind::Type),
        _ => None,
    }
}
fn get_type(ident: &String, loc: &Location) -> Option<TypeKind> {
    if ident.starts_with("&") {
        let inner_type = get_type(&(ident.strip_prefix("&").unwrap().to_string()), loc);
        if inner_type.is_none() {
            return None;
        }
        Some(TypeKind::Pointer(Box::new(inner_type.unwrap())))
    } else if ident.starts_with('[') && ident.ends_with(']') {
        let content = &ident[1..ident.len() - 1];
        if let Some((ty_str, len_str)) = content.split_once(';') {
            let inner_type_str = ty_str.trim().to_string();
            let inner_type = get_type(&inner_type_str, loc)?;
            let length = len_str.trim().parse::<usize>().ok()?;
            return Some(TypeKind::Pointer(Box::new(TypeKind::Array(
                Box::new(inner_type),
                length,
            ))));
        } else {
            None
        }
    } else {
        match ident.as_str() {
            "i16" => Some(TypeKind::Int16),
            "i32" => Some(TypeKind::Int32),
            "u16" => Some(TypeKind::Uint16),
            "u32" => Some(TypeKind::Uint32),
            "f32" => Some(TypeKind::Float32),
            "char" => Some(TypeKind::Char),
            "void" => Some(TypeKind::Void),
            "bool" => Some(TypeKind::Bool),
            _ => None,
        }
    }
}
struct InputStream {
    input: String,
    position: usize,
}
impl InputStream {
    fn new(input: String) -> Self {
        InputStream { input, position: 0 }
    }
    fn next(&mut self) -> Option<char> {
        if self.position < self.input.len() {
            let ch = self.input.chars().nth(self.position);
            self.position += 1;
            ch
        } else {
            None
        }
    }
    fn peek(&self) -> Option<char> {
        if self.position < self.input.len() {
            self.input.chars().nth(self.position)
        } else {
            None
        }
    }
    fn peek_next(&self, x: i32) -> Option<char> {
        if (self.position + x as usize) < self.input.len() {
            self.input.chars().nth(self.position + x as usize)
        } else {
            None
        }
    }
    fn matchTk(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.next();
            true
        } else {
            false
        }
    }
}
#[derive(Debug, Clone)]
pub struct Location {
    start: usize,
    end: usize,
}
#[derive(Debug, Clone, Copy)]
pub struct SourceLocation {
    pub line: usize,
    pub col: usize,
}
impl Location {
    pub fn get_src_loc(&self, src: &str) -> SourceLocation {
        let mut line = 0;
        let mut col = 1;
        for i in 0..self.start {
            if src.chars().nth(i) == Some('\n') {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        SourceLocation { line, col }
    }
}
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub loc: Location,
}
impl Token {
    fn new(kind: TokenKind, start: usize, end: usize) -> Self {
        Token {
            kind,
            loc: Location { start, end },
        }
    }
    pub fn display(&self, src: &str) -> String {
        let loc = self.loc.get_src_loc(src);
        format!("Line {}, Column {}: {:?}", loc.line, loc.col, self.kind)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Float(f32),
    Integer(i32),
    String(String),
    Bool(bool),
    Identifier(String),
    Keyword(KeywordKind),
    Operator(OperatorKind),
    Type(TypeKind),
    Semicolon,
    Colon,
    Comma,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Arrow,
    None,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeywordKind {
    If,
    Else,
    While,
    For,
    Let,
    Return,
    Fn,
    Struct,
    Union,
    Enum,
    As,
    Match,
    Break,
    Continue,
    Import,
    Type,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperatorKind {
    Add,
    Negate,
    Asterisk,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    Ampersand,
    Or,
    Not,
    Xor,
    Shl,
    Shr,
    Assign,
    Dot,
    Sizeof,
}
#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Int16,
    Int32,
    Float32,
    Char,
    Uint16,
    Uint32,
    Void,
    Bool,
    Pointer(Box<TypeKind>),
    Array(Box<TypeKind>, usize),
    Struct(String),
    Union(String),
    Enum(String),
    Function(Vec<TypeKind>, Box<TypeKind>),
}
