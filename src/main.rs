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
use std::{collections::HashMap, env, fs, path::PathBuf};
use test::run_cases;
use vm::Machine;

fn help() {
    println!("Corallium CLI");
    println!("Usage:");
    println!("  --run --file <path.coral> [--debug] [--link <file1> <file2> ...]");
    println!("  --compile --file <path.coral> [--debug] [--link <file1> <file2> ...]");
    println!("  --bytecode --file <path.cart> [--debug]");
    println!("  --genstd");
    println!("  --test");
    println!("  --help");
}

fn linked_files_from_args(args: &[String]) -> Vec<String> {
    match args.iter().position(|x| x == "--link") {
        Some(link_arg) => {
            let linked_files = args
                .iter()
                .skip(link_arg + 1)
                .take_while(|arg| !arg.starts_with("--"))
                .cloned()
                .collect::<Vec<String>>();
            if linked_files.is_empty() {
                panic!("Expected one or more files after --link");
            }
            linked_files
        }
        None => Vec::new(),
    }
}

fn append_linked_files(args: &[String], disk: &mut Disk) {
    for linked_file in linked_files_from_args(args) {
        let data = fs::read(&linked_file)
            .expect(&format!("Failed to read linked file: {}", &linked_file));
        disk.push(DiskSection {
            section_type: DiskSectionType::Data,
            id: disk.len() as i16,
            data: data.into_iter().map(|b| b as i16).collect(),
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
    let exe = compile_file(file).expect("Compilation Failed");
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
    let exe = compile_file(file).expect("Compilation Failed");
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
            .contains(&format!("--{}", word))
        {
            fun();
            return;
        }
    }
    help();
}
