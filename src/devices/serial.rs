use crate::vm::{Machine, unpack_dt};
use std::io::{self, Write};

pub fn driver(machine: &mut Machine, command: i16, _device_id: usize) {
    match command {
        0 => {
            //write(ptr)
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let mut bytes = Vec::new();
            let mut i = 0usize;
            loop {
                let byte = machine.memory.read(ptr + i, machine);
                if byte == 0 {
                    break;
                }
                bytes.push(byte as u8);
                i += 1;
            }
            print!("{}", String::from_utf8_lossy(&bytes));
            io::stdout()
                .flush()
                .expect("Failed to flush serial console output");
            if machine.debug {
                println!("IO.serial.write %{}", ptr);
            }
        }
        1 => {
            //writeNum(i32)
            let value = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as i32;
            print!("{}\n", value);
            io::stdout()
                .flush()
                .expect("Failed to flush serial console output");
            if machine.debug {
                println!("IO.serial.writeNum {}", value);
            }
        }
        2 => {
            //writeFloat(f32)
            let dt = machine.core.stack.pop(&mut machine.core.srp);
            let value = unpack_dt(dt) as f32;
            print!("{}\n", value);
            io::stdout()
                .flush()
                .expect("Failed to flush serial console output");
            if machine.debug {
                println!("IO.serial.writeFloat {}", value);
            }
        }
        _ => {}
    }
}
