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
use std::{collections::HashMap, env};
use test::run_cases;
use vm::Machine;
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
    let mut machine = Machine::new(debug);
    machine.set_disk(disk);
    machine.run();
    //machine.dump_state();
}
fn main() {
    let map: HashMap<&'static str, fn()> = HashMap::from([
        ("test", run_cases as fn()),
        ("compile", compile as fn()),
        ("genstd", gen_libs as fn()),
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
}
