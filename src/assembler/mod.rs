pub mod codegen;
pub mod lexer;
pub mod parser;
use codegen::Object;
use lexer::Lexer;
use parser::Parser;
pub fn assemble(name: &str, code: &str, lib: bool) -> (Option<Object>, Vec<String>) {
    let mut lex = Lexer::new(code.to_string());
    let tokens = lex.lex();
    let mut parser = Parser::new(tokens, name.to_string());
    let stmts = parser.parse();
    let mut codegen = codegen::CodeGen::new(name.to_string());
    (codegen.genBytecode(stmts, lib), codegen.imports)
}
