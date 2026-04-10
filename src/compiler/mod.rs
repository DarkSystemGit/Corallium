use std::{fs, path::Path, path::PathBuf};

use crate::executable::{Executable, Library};
use backend::Backend;
pub mod backend;
pub mod ir;
pub mod lexer;
pub mod parser;
pub fn compile(name: &str, code: &str) -> (Executable, Vec<String>) {
    let mut back = Backend::new(code, name);
    back.select_instructions();
    let mut exe = Executable::new();
    back.emit_bytecode().into_iter().for_each(|x| {
        exe.add_fn(x);
    });
    (exe, back.logs)
}
pub fn compile_file(path: &str) -> Option<Executable> {
    let file = fs::read_to_string(path).expect(&format!("Failed to read path {}", path));
    let mut back = Backend::new(&file, path);
    back.select_instructions();
    let mut exe = Executable::new();
    back.emit_bytecode().into_iter().for_each(|x| {
        exe.add_fn(x);
    });
    let mut imports = vec![];
    for import in back.input.imports.clone() {
        match Path::new(&import).extension()?.to_str()? == "h" {
            true => {
                let mut asm_path = PathBuf::from(&import);
                asm_path.set_extension("bin");
                let lib = Library::from_file(asm_path.clone())
                    .expect(&format!("Failed to read path {}", asm_path.display()));
                imports.push(lib);
            }
            false => {
                let file = fs::read_to_string(import.clone())
                    .expect(&format!("Failed to read path {}", import));
                let mut back = Backend::new(&file, &import);
                back.select_instructions();
                let mut lib = Library::new(Path::new(&import).file_stem()?.to_str()?.to_string());
                back.emit_bytecode().into_iter().for_each(|x| {
                    lib.add_fn(x);
                });
                imports.push(lib);
            }
        }
    }
    for i in imports {
        i.link(&mut exe);
    }
    Some(exe)
}
