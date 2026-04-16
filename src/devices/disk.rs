use crate::devices::RawDevice;
use crate::util::pop_stack;
use crate::vm::Machine;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;
pub type Disk = Vec<DiskSection>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSection {
    pub section_type: DiskSectionType,
    pub data: Vec<i16>,
    pub id: i16,
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum DiskSectionType {
    Entrypoint,
    Libary,
    Code,
    Data,
}

pub fn save_disk<P: AsRef<Path>>(disk: &Disk, path: P) -> io::Result<()> {
    let encoded = bincode::serialize(disk)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    fs::write(path, encoded)
}

pub fn load_disk<P: AsRef<Path>>(path: P) -> io::Result<Disk> {
    let bytes = fs::read(path)?;
    bincode::deserialize(&bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}
pub fn driver(machine: &mut Machine, command: i16, device_id: usize) {
    if command == 1 {
        //write(section,addr,len,buf)
        let cargs = pop_stack(&mut machine.core, 4);
        let section = cargs[0] as usize;
        let addr = cargs[1] as i32;
        let len = cargs[2] as i32;
        let buf = cargs[3] as usize;
        let data = machine
            .memory
            .read_range(buf..(buf + len as usize), &machine);
        (if let RawDevice::Disk(disk) = &mut machine.devices[device_id].contents {
            Some(disk)
        } else {
            None
        })
        .expect("Could not get disk")[section]
            .data[addr as usize..(addr + len) as usize]
            .copy_from_slice(data.as_slice());
        if machine.debug {
            println!(
                "IO.disk.write %[{} {}] -> disk.%[{} {}]",
                cargs[2], cargs[3], cargs[0], cargs[1]
            );
        }
    }
    let disk = if let RawDevice::Disk(disk) = &mut machine.devices[device_id].contents {
        Some(disk)
    } else {
        None
    }
    .expect("Could not get disk");

    match command {
        0 => {
            //read(section,addr,len,dest)
            let cargs = pop_stack(&mut machine.core, 4);
            let section = cargs[0] as usize;
            let addr = cargs[1] as i32;
            let len = cargs[2] as i32;
            let dest = cargs[3] as usize;
            for i in addr as usize..(addr + len) as usize {
                machine.memory.write(
                    dest + (i - addr as usize),
                    disk[section].data[i],
                    &mut machine.core,
                );
            }
            if machine.debug {
                println!(
                    "IO.disk.read disk.%[{} {}] {} ->%{}",
                    cargs[0], cargs[1], cargs[2], cargs[3]
                );
            }
        }

        2 => {
            //loadSectors(start,count,dest)
            let cargs = pop_stack(&mut machine.core, 3)
                .iter()
                .map(|i| *i as usize)
                .collect::<Vec<usize>>();
            let mut next_mem = cargs[2];
            for i in cargs[0]..cargs[0] + cargs[1] {
                for (_j, byte) in disk[i].data.iter().enumerate() {
                    machine.memory.write(next_mem, *byte, &mut machine.core);
                    next_mem += 1;
                }
            }
            if machine.debug {
                println!(
                    "IO.disk.loadSectors disk.%[{}] {} ->%{}",
                    cargs[0], cargs[1], cargs[2]
                );
            }
        }
        3 => {
            //sectorCount()->i16
            let count = disk.len() as i16;
            machine
                .core
                .stack
                .push(crate::vm::DataType::Int(count), &mut machine.core.srp);
            if machine.debug {
                println!("IO.disk.sectorCount -> {}", count);
            }
        }
        _ => {}
    }
}
