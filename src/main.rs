mod compiler;
mod devices;
mod executable;
mod genstdlibs;
mod test;
mod util;
mod vm;
use crate::devices::disk::*;
use compiler::compile_file;
use genstdlibs::gen_libs;
use std::{
    collections::HashMap,
    env::{self, consts::OS},
    ffi::CString,
    fs,
    path::{Path, PathBuf},
};
use test::run_cases;
use util::convert_i32_to_i16;
use vm::Machine;

fn help() {
    println!("Corallium CLI");
    println!("Usage:");
    println!(
        "  run --file <path.coral> [--debug] [--link <file_or_dir1> <file_or_dir2> ...] [--std <location to stdlib>]"
    );
    println!(
        "  compile --file <path.coral> [--debug] [--link <file_or_dir1> <file_or_dir2> ...] [--std <location to stdlib>]"
    );
    println!("  bytecode --file <path.cart> [--debug]");
    println!("  genstd");
    println!("  test");
    println!("  help");
}

fn collect_linked_files(path: &Path, linked_files: &mut Vec<String>) {
    if path.is_file() {
        linked_files.push(path.to_string_lossy().to_string());
        return;
    }
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .expect(&format!(
                "Failed to read linked directory: {}",
                path.display()
            ))
            .map(|entry| {
                entry
                    .expect(&format!(
                        "Failed to read linked directory entry in {}",
                        path.display()
                    ))
                    .path()
            })
            .collect::<Vec<PathBuf>>();
        entries.sort();
        for entry in entries {
            collect_linked_files(&entry, linked_files);
        }
        return;
    }
    panic!("Linked path does not exist: {}", path.display());
}

fn linked_files_from_args(args: &[String]) -> Vec<String> {
    match args.iter().position(|x| x == "--link") {
        Some(link_arg) => {
            let linked_paths = args
                .iter()
                .skip(link_arg + 1)
                .take_while(|arg| !arg.starts_with("--"))
                .cloned()
                .collect::<Vec<String>>();
            if linked_paths.is_empty() {
                panic!("Expected one or more files or directories after --link");
            }
            let mut linked_files = Vec::new();
            for linked_path in linked_paths {
                collect_linked_files(Path::new(&linked_path), &mut linked_files);
            }
            if linked_files.is_empty() {
                panic!("No files found to link after --link");
            }
            linked_files
        }
        None => Vec::new(),
    }
}

fn append_linked_files(args: &[String], disk: &mut Disk) {
    for linked_file in linked_files_from_args(args) {
        let file_data: Vec<i16> = fs::read(&linked_file)
            .expect(&format!("Failed to read linked file: {}", &linked_file))
            .chunks(2)
            .map(|chunk| {
                let lo = chunk[0];
                let hi = if chunk.len() == 2 { chunk[1] } else { 0 };
                i16::from_le_bytes([lo, hi])
            })
            .collect();
        let len = file_data.len();
        let len_i32 = i32::try_from(len).expect("Linked file too large");
        let name: Vec<i16> = linked_file
            .into_bytes()
            .into_iter()
            .map(|b| b as i16)
            .collect();
        let data = vec![
            vec![name.len() as i16],
            name,
            convert_i32_to_i16(len_i32).to_vec(),
            file_data,
        ]
        .concat();
        disk.push(DiskSection {
            section_type: DiskSectionType::Data,
            id: disk.len() as i16,
            data,
        });
    }
}

fn compile() {
    let args: Vec<String> = env::args().collect();
    let file = &args[args
        .iter()
        .position(|x| x == "--file")
        .expect("No file arg")
        + 1];
    let stdloc = match args.contains(&String::from("--std")) {
        true => args[args
            .iter()
            .position(|x| x == "--std")
            .expect("No std location provided")
            + 1]
        .clone(),
        false => match OS {
            "linux" => "/opt/Corallium/std".to_string(),
            "macos" => "/usr/local/opt/Corallium/std".to_string(),
            "windows" => "C:/Program Files/Corallium/std".to_string(),
            _ => panic!("Unsupported OS: {}", OS),
        },
    };
    let exe = compile_file(file, stdloc).expect("Compilation Failed");
    let debug = args.contains(&String::from("--debug"));

    let mut disk: Disk = vec![DiskSection {
        section_type: DiskSectionType::Entrypoint,
        id: 0,
        data: vec![],
    }] as Disk;
    exe.build(0, &mut disk, debug);
    append_linked_files(&args, &mut disk);
    let mut write_path = PathBuf::from(file);
    write_path.set_extension("cart");
    save_disk(&disk, write_path).expect("Failed to write disk image");

    //machine.dump_state();
}
fn compile_run() {
    let args: Vec<String> = env::args().collect();
    let file = &args[args
        .iter()
        .position(|x| x == "--file")
        .expect("No file arg")
        + 1];
    let stdloc = match args.contains(&String::from("--std")) {
        true => args[args
            .iter()
            .position(|x| x == "--std")
            .expect("No std location provided")
            + 1]
        .clone(),
        false => "/opt/Corallium/std".to_string(),
    };
    let exe = compile_file(file, stdloc).expect("Compilation Failed");
    let debug = args.contains(&String::from("--debug"));
    let mut disk: Disk = vec![DiskSection {
        section_type: DiskSectionType::Entrypoint,
        id: 0,
        data: vec![],
    }] as Disk;
    exe.build(0, &mut disk, debug);
    append_linked_files(&args, &mut disk);
    let mut machine = Machine::new(debug);
    machine.set_disk(disk);
    machine.run();
    if args.contains(&"--save-disk".to_string()) {
        //save_disk(machine.devic, path)
    }
}
fn run_bytecode() {
    let args: Vec<String> = env::args().collect();
    let file = &args[args
        .iter()
        .position(|x| x == "--file")
        .expect("No file arg")
        + 1];
    let debug = args.contains(&String::from("--debug"));
    let disk = load_disk(file).expect("Failed to read disk image");
    let mut machine = Machine::new(debug);
    machine.set_disk(disk);
    machine.run();
    if args.contains(&"--save-disk".to_string()) {
        //save_disk(machine.devic, path)
    }
}
fn main() {
    let map: HashMap<&'static str, fn()> = HashMap::from([
        ("test", run_cases as fn()),
        ("run", compile_run as fn()),
        ("compile", compile as fn()),
        ("genstd", gen_libs as fn()),
        ("bytecode", run_bytecode as fn()),
        ("help", help as fn()),
    ]);
    for (word, fun) in map {
        if env::args()
            .collect::<Vec<String>>()
            .contains(&format!("{}", word))
        {
            fun();
            return;
        }
    }
    help();
}
