#![allow(warnings)]
mod assembler;
mod compiler;
mod devices;
mod executable;
mod genstdlibs;
mod sdk;
mod test;
mod util;
mod vm;
use crate::devices::RawDevice;
use crate::devices::disk::*;
use assembler::assemble;
use assembler::codegen::Object;
use compiler::{collect_import_libs, compile_file, normalize_path};

use genstdlibs::gen_libs;
use std::{
    collections::{HashMap, HashSet},
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
        "  compile --file <path.coral> [--debug] [--link <file_or_dir1> <file_or_dir2> ...] [--std <location to stdlib>]"
    );
    println!(
        "  assemble --file <path.polyp> [--debug] [--lib] [--link <file_or_dir1> <file_or_dir2> ...]"
    );
    println!("  run --file <path.cart> [--debug]");
    println!("  sdk");
    println!("      convert_music <input_midi_file> [--wav-preview]");
    println!("      convert_image <input_image_file>");
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

fn linked_file_record(name: &str, file_data: Vec<i16>, id: i16) -> DiskSection {
    let len_i32 = i32::try_from(file_data.len()).expect("Linked file too large");
    let name = name
        .as_bytes()
        .iter()
        .map(|b| *b as i16)
        .collect::<Vec<i16>>();
    let data = vec![
        vec![name.len() as i16],
        name,
        convert_i32_to_i16(len_i32).to_vec(),
        file_data,
    ]
    .concat();

    DiskSection {
        section_type: DiskSectionType::Data,
        id,
        data,
    }
}

fn read_linked_file_words(path: &str) -> Vec<i16> {
    fs::read(path)
        .expect(&format!("Failed to read linked file: {}", path))
        .chunks(2)
        .map(|chunk| {
            let lo = chunk[0];
            let hi = if chunk.len() == 2 { chunk[1] } else { 0 };
            i16::from_le_bytes([lo, hi])
        })
        .collect()
}

fn child_linked_file_start(child_disk: &Disk) -> usize {
    let Some(entrypoint) = child_disk.first() else {
        return 0;
    };

    let header_word =
        |addr: usize| -> usize { entrypoint.data.get(addr).copied().unwrap_or(0).max(0) as usize };

    (header_word(513) + header_word(515) + header_word(517)).max(1)
}

fn append_linked_cart(path: &str, disk: &mut Disk) {
    let mut child_disk = load_disk(path).expect(&format!("Failed to read linked cart: {}", path));
    if child_disk.is_empty() {
        panic!("Linked cart has no disk sections: {}", path);
    }

    let first_physical_sector = disk.len();

    // Update the child's LSO header field (index 518) to the physical sector offset
    if child_disk[0].data.len() > 518 {
        child_disk[0].data[518] = first_physical_sector as i16;
        let fcso = 3 + path.len();
        //dbg!(fcso);
        child_disk[0].data[519] = convert_i32_to_i16(fcso as i32)[0];
        child_disk[0].data[520] = convert_i32_to_i16(fcso as i32)[1];
    }

    disk.push(linked_file_record(
        path,
        child_disk[0].data.clone(),
        first_physical_sector as i16,
    ));

    let linked_file_start = child_linked_file_start(&child_disk);
    for (logical_sector, section) in child_disk.into_iter().enumerate().skip(1) {
        let mut section = section;
        section.id = disk.len() as i16;
        if logical_sector < linked_file_start {
            section.section_type = DiskSectionType::Code;
        }
        disk.push(DiskSection {
            section_type: section.section_type,
            id: section.id,
            data: section.data,
        });
    }
}

fn append_linked_data_file(path: &str, disk: &mut Disk) {
    let id = disk.len() as i16;
    disk.push(linked_file_record(path, read_linked_file_words(path), id));
}

fn append_linked_files(args: &[String], disk: &mut Disk) {
    for linked_file in linked_files_from_args(args) {
        let path = Path::new(&linked_file);
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("cart"))
        {
            append_linked_cart(&linked_file, disk);
        } else {
            append_linked_data_file(&linked_file, disk);
        }
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
    /*if args.contains(&"--save-disk".to_string()) {
        save_disk(
            (if let RawDevice::Disk(disk) = &machine.devices[0].contents {
                Some(disk)
            } else {
                None
            })
            .unwrap(),
            path,
        );
    }*/
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
        save_disk(
            (if let RawDevice::Disk(disk) = &machine.devices[0].contents {
                Some(disk)
            } else {
                None
            })
            .unwrap(),
            file,
        );
    }
}
fn assembler() {
    let args: Vec<String> = env::args().collect();
    let path = &args[args
        .iter()
        .position(|x| x == "--file")
        .expect("No file arg")
        + 1];
    let file = fs::read_to_string(path).expect(&format!("Failed to read path {}", path));
    let (obj, imports) = assembler::assemble(path, &file, args.contains(&("--lib".to_string())));
    let obj = obj.expect("Assembly failed");
    let debug = args.contains(&String::from("--debug"));

    let mut disk: Disk = vec![DiskSection {
        section_type: DiskSectionType::Entrypoint,
        id: 0,
        data: vec![],
    }] as Disk;
    let mut import_libs = vec![];
    let mut active_sources = HashSet::new();
    let mut source_cache = HashMap::new();
    let mut header_cache = HashMap::new();
    active_sources.insert(normalize_path(Path::new(path)));
    collect_import_libs(
        &imports,
        &mut import_libs,
        &mut active_sources,
        &mut source_cache,
        &mut header_cache,
        String::new(),
    )
    .expect("Failed to resolve imports");
    match obj {
        Object::Exe(mut exe) => {
            for lib in import_libs {
                lib.link(&mut exe);
            }
            exe.build(0, &mut disk, debug);
            append_linked_files(&args, &mut disk);
            let mut write_path = PathBuf::from(path);
            write_path.set_extension("cart");
            save_disk(&disk, write_path).expect("Failed to write disk image");
        }
        Object::Lib(mut lib) => {
            for ilib in import_libs {
                ilib.link_lib(&mut lib);
            }
            let mut write_path = PathBuf::from(path);
            write_path.set_extension("bin");
            lib.to_file(write_path).expect("Couldn't write file");
        }
    }
}
fn sdk() {
    let args: Vec<String> = env::args().collect();
    match args.get(2).map(|s| s.as_str()) {
        Some("convert_music") => {
            let input = args.get(3).expect("No input file provided");
            let wav_preview = args.get(4).map(|s| s.as_str()) == Some("--wav-preview");
            sdk::music_converter::convert_music(input, wav_preview)
                .expect("Music conversion failed");
        }
        Some("convert_image") => {
            let input = args.get(3).expect("No input image file provided");
            sdk::image_converter::convert_image(input).expect("Image conversion failed");
        }
        _ => {
            println!("SDK Commands:");
            println!("  convert_music <input_midi_file> [--wav-preview]");
            println!("  convert_image <input_image_file>");
        }
    }
}
fn main() {
    let map: HashMap<&'static str, fn()> = HashMap::from([
        ("test", run_cases as fn()),
        ("compile", compile as fn()),
        ("assemble", assembler as fn()),
        ("genstd", gen_libs as fn()),
        ("run", run_bytecode as fn()),
        ("help", help as fn()),
        ("sdk", sdk as fn()),
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
