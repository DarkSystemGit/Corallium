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
    let disk = if let RawDevice::Disk(disk) = &mut machine.devices[device_id].contents {
        Some(disk)
    } else {
        None
    }
    .expect("Could not get disk");
    match command as i16 {
        0 => {
            //read(section,addr,len,dest)
            let cargs = pop_stack(&mut machine.core, 4);
            for i in (cargs[1] as usize)..(cargs[1] + cargs[2]) as usize {
                machine.memory.write(
                    cargs[3] as usize + (i - cargs[1] as usize),
                    disk[cargs[0] as usize].data[i],
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
        1 => {
            //write(section,addr,byte)
            let cargs = pop_stack(&mut machine.core, 3);
            disk[cargs[0] as usize].data[cargs[1] as usize] = cargs[2] as i16;
            if machine.debug {
                println!(
                    "IO.disk.write {} -> disk.%[{} {}]",
                    cargs[2], cargs[0], cargs[1]
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
        _ => {}
    }
}
