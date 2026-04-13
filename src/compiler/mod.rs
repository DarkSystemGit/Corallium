use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    path::PathBuf,
};

use crate::executable::{Executable, Library};
use backend::Backend;
pub mod backend;
pub mod ir;
pub mod lexer;
pub mod parser;

fn normalize_path(path: &Path) -> String {
    fs::canonicalize(path)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn collect_import_libs(
    imports: &[String],
    libs: &mut Vec<Library>,
    active_sources: &mut HashSet<String>,
    source_cache: &mut HashMap<String, Library>,
    header_cache: &mut HashMap<String, Library>,
) -> Option<()> {
    for import in imports {
        match Path::new(import).extension()?.to_str()? == "h" {
            true => {
                let mut bin_path = PathBuf::from(import);
                bin_path.set_extension("bin");
                let normalized_bin = normalize_path(&bin_path);
                if let Some(lib) = header_cache.get(&normalized_bin) {
                    libs.push(lib.clone());
                } else {
                    let lib = Library::from_file(bin_path.clone())
                        .expect(&format!("Failed to read path {}", bin_path.display()));
                    header_cache.insert(normalized_bin, lib.clone());
                    libs.push(lib);
                }
            }
            false => {
                let normalized_source = normalize_path(Path::new(import));
                if let Some(lib) = source_cache.get(&normalized_source) {
                    libs.push(lib.clone());
                    continue;
                }
                if !active_sources.insert(normalized_source.clone()) {
                    continue;
                }
                let file =
                    fs::read_to_string(import).expect(&format!("Failed to read path {}", import));
                let mut back = Backend::new(&file, import);
                back.select_instructions();
                let mut nested_libs = vec![];
                collect_import_libs(
                    &back.input.imports.clone(),
                    &mut nested_libs,
                    active_sources,
                    source_cache,
                    header_cache,
                )?;

                let mut lib = Library::new(Path::new(import).file_stem()?.to_str()?.to_string());
                for nested in nested_libs {
                    nested.link_lib(&mut lib);
                }
                back.emit_bytecode().into_iter().for_each(|x| {
                    lib.add_fn(x);
                });
                active_sources.remove(&normalized_source);
                source_cache.insert(normalized_source, lib.clone());
                libs.push(lib);
            }
        }
    }
    Some(())
}

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
    let mut active_sources = HashSet::new();
    let mut source_cache = HashMap::new();
    let mut header_cache = HashMap::new();
    active_sources.insert(normalize_path(Path::new(path)));
    collect_import_libs(
        &back.input.imports.clone(),
        &mut imports,
        &mut active_sources,
        &mut source_cache,
        &mut header_cache,
    )?;
    for i in imports {
        i.link(&mut exe);
    }
    Some(exe)
}
