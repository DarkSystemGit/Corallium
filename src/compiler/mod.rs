use crate::executable::Executable;
use backend::Backend;
pub mod backend;
pub mod ir;
pub mod lexer;
pub mod parser;
pub fn compile(name: &str, code: &str) -> (Executable, Vec<String>) {
    let mut back = Backend::new(code, name);
    back.select_instructions();
    (back.emit_bytecode(), back.logs)
}
