use crate::vm::CommandType;
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
    fn read_escape(&mut self) -> Option<char> {
        match self.input.next() {
            Some('n') => Some('\n'),
            Some('r') => Some('\r'),
            Some('t') => Some('\t'),
            Some('0') => Some('\0'),
            Some('\\') => Some('\\'),
            Some('"') => Some('"'),
            Some('\'') => Some('\''),
            Some('x') => {
                let hi = self.input.peek();
                let lo = self.input.peek_next(1);
                if hi.map_or(false, is_hex_digit) && lo.map_or(false, is_hex_digit) {
                    let hi = self.input.next().unwrap();
                    let lo = self.input.next().unwrap();
                    let hex = format!("{}{}", hi, lo);
                    u8::from_str_radix(&hex, 16).ok().map(|v| v as char)
                } else {
                    Some('x')
                }
            }
            Some(c) => Some(c),
            None => None,
        }
    }
    fn read_number(&mut self, start: usize, first: Option<char>, negative: bool) -> Token {
        let is_hex = match first {
            Some('0') => self.input.peek() == Some('x'),
            None => self.input.peek() == Some('0') && self.input.peek_next(1) == Some('x'),
            _ => false,
        };
        if is_hex {
            if first.is_none() {
                self.input.next();
            }
            self.input.next();
            let mut str = vec![];
            while self.input.peek().is_some() && is_hex_digit(self.input.peek().unwrap()) {
                str.push(self.input.next().unwrap());
            }

            let mut force_i32 = false;
            if self.input.peek() == Some('_') {
                let saved_pos = self.input.position;
                self.input.next();
                if self.input.peek() == Some('i') {
                    self.input.next();
                    if self.input.peek() == Some('3') {
                        self.input.next();
                        if self.input.peek() == Some('2') {
                            self.input.next();
                            force_i32 = true;
                        }
                    }
                }
                if !force_i32 {
                    self.input.position = saved_pos;
                }
            }

            let mut num = i32::from_str_radix(&str.iter().collect::<String>(), 16).unwrap();
            if negative {
                num = -num;
            }
            return Token::new(
                if force_i32 || num >= (i16::MAX as i32) || num <= (i16::MIN as i32) {
                    TokenKind::Int32(num)
                } else {
                    TokenKind::Int(num)
                },
                start,
                self.input.position,
                &self.input.input,
            );
        }

        let mut is_float = false;
        let mut str = vec![];
        if negative {
            str.push('-');
        }
        if let Some(first) = first {
            str.push(first);
        }
        while self.input.peek().is_some()
            && (is_digit(self.input.peek().unwrap())
                || (self.input.peek() == Some('.') && !is_float))
        {
            let next = self.input.next().unwrap();
            if next == '.' {
                is_float = true;
            }
            str.push(next);
        }

        let mut force_i32 = false;
        if !is_float && self.input.peek() == Some('_') {
            let saved_pos = self.input.position;
            self.input.next();
            if self.input.peek() == Some('i') {
                self.input.next();
                if self.input.peek() == Some('3') {
                    self.input.next();
                    if self.input.peek() == Some('2') {
                        self.input.next();
                        force_i32 = true;
                    }
                }
            }
            if !force_i32 {
                self.input.position = saved_pos;
            }
        }

        Token::new(
            if is_float {
                TokenKind::Float(str.iter().collect::<String>().parse::<f32>().unwrap())
            } else {
                let num = str.iter().collect::<String>().parse::<i32>().unwrap();
                if force_i32 || num >= (i16::MAX as i32) || num <= (i16::MIN as i32) {
                    TokenKind::Int32(num)
                } else {
                    TokenKind::Int(num)
                }
            },
            start,
            self.input.position,
            &self.input.input,
        )
    }
    fn read_token(&mut self) -> Token {
        let start = self.input.position;
        let ch = self.input.next().unwrap();
        match ch {
            ':' => Token::new(TokenKind::Colon, start, start + 1, &self.input.input),
            ';' => Token::new(TokenKind::Semicolon, start, start + 1, &self.input.input),
            '(' => Token::new(TokenKind::LParen, start, start + 1, &self.input.input),
            ')' => Token::new(TokenKind::RParen, start, start + 1, &self.input.input),
            '{' => Token::new(TokenKind::LBrace, start, start + 1, &self.input.input),
            '}' => Token::new(TokenKind::RBrace, start, start + 1, &self.input.input),
            '[' => Token::new(TokenKind::LBracket, start, start + 1, &self.input.input),
            ']' => Token::new(TokenKind::RBracket, start, start + 1, &self.input.input),
            ',' => Token::new(TokenKind::Comma, start, start + 1, &self.input.input),
            '=' => Token::new(TokenKind::Equals, start, start + 1, &self.input.input),
            '-' => {
                if self.input.peek() == Some('>') {
                    self.input.next();
                    Token::new(TokenKind::Arrow, start, start + 2, &self.input.input)
                } else if self.input.peek().map_or(false, is_digit) {
                    self.read_number(start, None, true)
                } else {
                    Token::new(TokenKind::None, start, start + 1, &self.input.input)
                }
            }
            '\'' => {
                let value = if self.input.peek() == Some('\\') {
                    self.input.next();
                    self.read_escape().unwrap_or('\0')
                } else {
                    self.input.next().unwrap_or('\0')
                };
                if self.input.peek() == Some('\'') {
                    self.input.next();
                }
                return Token::new(
                    TokenKind::Char(value),
                    start,
                    self.input.position,
                    &self.input.input,
                );
            }
            '"' => {
                let mut str = String::new();
                while self.input.peek().is_some() && self.input.peek() != Some('"') {
                    if self.input.peek() == Some('\\') {
                        self.input.next();
                        if let Some(escaped) = self.read_escape() {
                            str.push(escaped);
                        } else {
                            break;
                        }
                    } else {
                        str.push(self.input.next().unwrap());
                    }
                }
                if self.input.peek() == Some('"') {
                    self.input.next();
                }
                Token::new(
                    TokenKind::String(str),
                    start,
                    self.input.position,
                    &self.input.input,
                )
            }
            _ => {
                if is_digit(ch) {
                    return self.read_number(start, Some(ch), false);
                }
                if is_alpha(ch) {
                    let mut str = vec![ch];
                    while self.input.peek().is_some() && is_alphanumeric(self.input.peek().unwrap())
                    {
                        str.push(self.input.next().unwrap());
                    }
                    if getKeyword(&str.iter().collect()) != TokenKind::None {
                        return Token::new(
                            getKeyword(&str.iter().collect()),
                            start,
                            start + str.len() - 1,
                            &self.input.input,
                        );
                    }
                    return Token::new(
                        TokenKind::Identifier(str.iter().collect()),
                        start,
                        start + str.len() - 1,
                        &self.input.input,
                    );
                }
                if ch == '/' && self.input.peek() == Some('/') {
                    while self.input.peek().is_some() && self.input.peek() != Some('\n') {
                        self.input.next();
                    }
                    if self.input.peek() == Some('\n') {
                        self.input.next();
                    }
                }
                Token {
                    kind: TokenKind::None,
                    loc: Location { start: 0, end: 0 },
                    srcLoc: SourceLocation { line: 0, col: 0 },
                }
            }
        }
    }
}
fn is_digit(ch: char) -> bool {
    ch.is_digit(10)
}
fn is_hex_digit(ch: char) -> bool {
    ch.is_digit(16)
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
fn getKeyword(str: &String) -> TokenKind {
    match str.as_str() {
        //keywords
        "fn" => TokenKind::Fn,
        "symbol" => TokenKind::Symbol,
        "global" => TokenKind::Global,
        "import" => TokenKind::Import,
        //Registers
        "r1" => TokenKind::Register(RegisterType::R1),
        "r2" => TokenKind::Register(RegisterType::R2),
        "r3" => TokenKind::Register(RegisterType::R3),
        "r4" => TokenKind::Register(RegisterType::R4),
        "r5" => TokenKind::Register(RegisterType::R5),
        "f1" => TokenKind::Register(RegisterType::F1),
        "f2" => TokenKind::Register(RegisterType::F2),
        "ex1" => TokenKind::Register(RegisterType::EX1),
        "ex2" => TokenKind::Register(RegisterType::EX2),
        "arp" => TokenKind::Register(RegisterType::ARP),
        "srp" => TokenKind::Register(RegisterType::SRP),
        "sp" => TokenKind::Register(RegisterType::SP),
        "ip" => TokenKind::Register(RegisterType::IP),
        // Arithmetic Commands
        "add" => TokenKind::Command(CommandType::Add),
        "sub" => TokenKind::Command(CommandType::Sub),
        "mul" => TokenKind::Command(CommandType::Mul),
        "div" => TokenKind::Command(CommandType::Div),
        "mod" => TokenKind::Command(CommandType::Mod),
        // Float Commands
        "addf" => TokenKind::Command(CommandType::Addf),
        "subf" => TokenKind::Command(CommandType::Subf),
        "mulf" => TokenKind::Command(CommandType::Mulf),
        "divf" => TokenKind::Command(CommandType::Divf),
        // Extended Commands
        "addEx" => TokenKind::Command(CommandType::AddEx),
        "subEx" => TokenKind::Command(CommandType::SubEx),
        "mulEx" => TokenKind::Command(CommandType::MulEx),
        "divEx" => TokenKind::Command(CommandType::DivEx),
        // Unsigned Commands
        "addU" => TokenKind::Command(CommandType::AddU),
        "subU" => TokenKind::Command(CommandType::SubU),
        "mulU" => TokenKind::Command(CommandType::MulU),
        "divU" => TokenKind::Command(CommandType::DivU),
        // Extended Unsigned
        "addExU" => TokenKind::Command(CommandType::AddExU),
        "subExU" => TokenKind::Command(CommandType::SubExU),
        "mulExU" => TokenKind::Command(CommandType::MulExU),
        "divExU" => TokenKind::Command(CommandType::DivExU),
        // Logic
        "and" => TokenKind::Command(CommandType::And),
        "not" => TokenKind::Command(CommandType::Not),
        "or" => TokenKind::Command(CommandType::Or),
        "xor" => TokenKind::Command(CommandType::Xor),
        "andEx" => TokenKind::Command(CommandType::AndEx),
        "notEx" => TokenKind::Command(CommandType::NotEx),
        "orEx" => TokenKind::Command(CommandType::OrEx),
        "xorEx" => TokenKind::Command(CommandType::XorEx),
        // Memory / Stack
        "push" => TokenKind::Command(CommandType::Push),
        "pushf" => TokenKind::Command(CommandType::Pushf),
        "pushEx" => TokenKind::Command(CommandType::PushEx),
        "pop" => TokenKind::Command(CommandType::Pop),
        "load" => TokenKind::Command(CommandType::Load),
        "loadEx" => TokenKind::Command(CommandType::LoadEx),
        "loadf" => TokenKind::Command(CommandType::Loadf),
        "store" => TokenKind::Command(CommandType::Store),
        "storeEx" => TokenKind::Command(CommandType::StoreEx),
        "storef" => TokenKind::Command(CommandType::Storef),
        "mov" => TokenKind::Command(CommandType::Mov),
        // Control Flow
        "jmp" => TokenKind::Command(CommandType::Jump),
        "jnz" => TokenKind::Command(CommandType::JumpNotZero),
        "jz" => TokenKind::Command(CommandType::JumpZero),
        "call" => TokenKind::Command(CommandType::Call),
        "ret" => TokenKind::Command(CommandType::Return),
        "exit" => TokenKind::Command(CommandType::Exit),
        "nop" => TokenKind::Command(CommandType::NOP),
        // Comparison
        "gt" => TokenKind::Command(CommandType::Greater),
        "lt" => TokenKind::Command(CommandType::LessThan),
        "eq" => TokenKind::Command(CommandType::Equals),
        // Bitwise
        "shl" => TokenKind::Command(CommandType::Shl),
        "shr" => TokenKind::Command(CommandType::Shr),
        "shlEx" => TokenKind::Command(CommandType::ShlEx),
        "shrEx" => TokenKind::Command(CommandType::ShrEx),
        // IO
        "io" => TokenKind::Command(CommandType::IO),
        //breakpoint
        "breakpoint" => TokenKind::Command(CommandType::Breakpoint),
        "_HEAP_START_" => TokenKind::HeapStart,
        _ => TokenKind::None,
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
    pub srcLoc: SourceLocation,
}
impl Token {
    fn new(kind: TokenKind, start: usize, end: usize, src: &str) -> Self {
        Token {
            kind,
            loc: Location { start, end },
            srcLoc: Location { start, end }.get_src_loc(src),
        }
    }
    pub fn display(&self, src: &str) -> String {
        let loc = self.loc.get_src_loc(src);
        format!("Line {}, Column {}: {:?}", loc.line, loc.col, self.kind)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Command(CommandType),
    Register(RegisterType),
    HeapStart,
    Import,
    Symbol,
    Fn,
    Global,
    Identifier(String),
    Int(i32),
    Int32(i32),
    Float(f32),
    Colon,
    Semicolon,
    String(String),
    Char(char),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Arrow,
    Equals,
    None,
}
#[derive(Debug, Clone, PartialEq)]
pub enum RegisterType {
    R1,
    R2,
    R3,
    R4,
    R5,
    F1,
    F2,
    EX1,
    EX2,
    ARP,
    SRP,
    SP,
    IP,
}
