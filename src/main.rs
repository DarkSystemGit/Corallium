mod compiler;
mod devices;
mod executable;
mod test;
mod util;
mod vm;
use crate::devices::disk::*;
use compiler::compile_file;
use std::env;
use test::run_cases;
use vm::Machine;
fn compile() {
    let args: Vec<String> = env::args().collect();
    let exe = compile_file(&args[1]).expect("Compilation Failed");
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
}
fn main() {
    //run_cases();
    compile();
}
