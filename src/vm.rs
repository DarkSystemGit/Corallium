use crate::devices;
use crate::devices::disk::Disk;
use crate::devices::{Device, RawDevice};
use crate::util::*;
use prompted::input;
use std::ops::Range;
use std::panic;
use std::time::Instant;
fn exec_bytecode(machine: &mut Machine) {
    let byte = convert_int_to_command(take_bytes(machine, 1)[0] as i16);
    if machine.debug {
        print!("%{:07}: ", machine.core.ip - 1);
    }
    match byte {
        CommandType::Add => {
            //add(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] + args[1]) as i16;
            if machine.debug {
                println!("Add {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Sub => {
            //sub(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] - args[1]) as i16;
            if machine.debug {
                println!("Sub {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Mul => {
            //mul(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] * args[1]) as i16;
            if machine.debug {
                println!("Mul {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Div => {
            //div(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] / args[1]) as i16;
            if machine.debug {
                println!("Div {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::AddU => {
            //addu(u16,u16)->r1
            // TODO: add map of all regs with unsigned currently within them to vm
            let args: Vec<u16> = take_bytes(machine, 2)
                .iter()
                .map(|x| (*x as i16) as u16)
                .collect();
            machine.core.r1 = ((args[0] + args[1]) as u16) as i16;
            if machine.debug {
                println!("AddUnsigned {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::SubU => {
            //subu(u16,u16)->r1
            let args: Vec<u16> = take_bytes(machine, 2)
                .iter()
                .map(|x| (*x as i16) as u16)
                .collect();
            machine.core.r1 = ((args[0] - args[1]) as u16) as i16;
            if machine.debug {
                println!("SubUnsigned {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::MulU => {
            //mulu(u16,u16)->r1
            let args: Vec<u16> = take_bytes(machine, 2)
                .iter()
                .map(|x| (*x as i16) as u16)
                .collect();
            machine.core.r1 = ((args[0] * args[1]) as u16) as i16;
            if machine.debug {
                println!("MulUnsigned {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::DivU => {
            //divu(u16,u16)->r1
            let args: Vec<u16> = take_bytes(machine, 2)
                .iter()
                .map(|x| (*x as i16) as u16)
                .collect();
            machine.core.r1 = ((args[0] / args[1]) as u16) as i16;
            if machine.debug {
                println!("DivUnsigned {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::AddExU => {
            //addExu(u32,u32) -> ex1
            let args: Vec<u32> = take_bytes(machine, 2)
                .iter()
                .map(|x| (*x as i32) as u32)
                .collect();
            set_reg(10, &mut machine.core, ((args[0] + args[1]) as u32) as f64);
            if machine.debug {
                println!(
                    "AddExUnsigned {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::SubExU => {
            //subExu(u32,u32) -> ex1
            let args: Vec<u32> = take_bytes(machine, 2)
                .iter()
                .map(|x| (*x as i32) as u32)
                .collect();
            set_reg(10, &mut machine.core, ((args[0] - args[1]) as u32) as f64);
            if machine.debug {
                println!(
                    "SubExUnsigned {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::MulExU => {
            //mulExu(u32,u32) -> ex1
            let args: Vec<u32> = take_bytes(machine, 2)
                .iter()
                .map(|x| (*x as i32) as u32)
                .collect();
            set_reg(10, &mut machine.core, ((args[0] * args[1]) as u32) as f64);
            if machine.debug {
                println!(
                    "MulExUnsigned {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::DivExU => {
            //divExu(u32,u32) -> ex1
            let args: Vec<u32> = take_bytes(machine, 2)
                .iter()
                .map(|x| (*x as i32) as u32)
                .collect();
            set_reg(10, &mut machine.core, ((args[0] / args[1]) as u32) as f64);
            if machine.debug {
                println!(
                    "DivExUnsigned {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::AddEx => {
            //addEx(i32,i32) -> ex1
            let args = take_bytes(machine, 2);
            set_reg(10, &mut machine.core, (args[0] + args[1]) as f64);
            if machine.debug {
                println!(
                    "AddEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::SubEx => {
            //subEx(i32,i32) -> ex1
            let args = take_bytes(machine, 2);
            set_reg(10, &mut machine.core, (args[0] - args[1]) as f64);
            if machine.debug {
                println!(
                    "SubEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::MulEx => {
            //mulEx(i32,i32) -> ex1
            let args = take_bytes(machine, 2);
            set_reg(10, &mut machine.core, (args[0] * args[1]) as f64);
            if machine.debug {
                println!(
                    "MulEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::DivEx => {
            //divEx(i32,i32) -> ex1
            let args = take_bytes(machine, 2);
            set_reg(10, &mut machine.core, (args[0] / args[1]) as f64);
            if machine.debug {
                println!(
                    "DivEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }

        CommandType::Greater => {
            //greater(f64,f64) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] > args[1]) as i16;
            if machine.debug {
                println!("GreaterThan {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Addf => {
            //addf(f32,f32) -> f1
            let args = take_bytes(machine, 2);
            machine.core.f1 = (args[0] + args[1]) as f32;
            if machine.debug {
                println!("Addf {} {} -> {}", args[0], args[1], machine.core.f1);
            }
        }
        CommandType::Subf => {
            //subf(f32,f32) -> f1
            let args = take_bytes(machine, 2);
            machine.core.f1 = (args[0] - args[1]) as f32;
            if machine.debug {
                println!("Subf {} {} -> {}", args[0], args[1], machine.core.f1);
            }
        }
        CommandType::Mulf => {
            //mulf(f32,f32) -> f1
            let args = take_bytes(machine, 2);
            machine.core.f1 = (args[0] * args[1]) as f32;
            if machine.debug {
                println!("Mulf {} {} -> {}", args[0], args[1], machine.core.f1);
            }
        }
        CommandType::Divf => {
            //divf(f32,f32) -> f1
            let args = take_bytes(machine, 2);
            machine.core.f1 = (args[0] / args[1]) as f32;
            if machine.debug {
                println!("Divf {} {} -> {}", args[0], args[1], machine.core.f1);
            }
        }
        CommandType::Mod => {
            //mod(f64,f64) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] % args[1]) as i16;
            if machine.debug {
                println!("Modulo {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Pop => {
            //pop() -> Register
            let val = unpack_dt(machine.core.stack.pop(&mut machine.core.srp));
            let reg = take_registers(machine, 1)[0];
            set_reg(reg, &mut machine.core, val);
            if machine.debug {
                println!("Pop {} -> R{}", val, reg);
            }
        }
        CommandType::LessThan => {
            //less_than(f64,f64) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] < args[1]) as i16;
            if machine.debug {
                println!("LessThan {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Jump => {
            //jump(address)
            let addr = take_bytes(machine, 1)[0];
            machine.core.ip = addr as usize;
            if machine.debug {
                println!("Jump {}", addr);
            }
        }
        CommandType::And => {
            //and(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = args[0] as i16 & args[1] as i16;
            if machine.debug {
                println!("And {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Or => {
            //or(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = args[0] as i16 | args[1] as i16;
            if machine.debug {
                println!("Or {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Not => {
            //not(i16) -> r1
            let args = take_bytes(machine, 1);
            machine.core.r1 = !(args[0] as i16);
            if machine.debug {
                println!("Not {} -> {}", args[0], machine.core.r1);
            }
        }
        CommandType::Xor => {
            //xor(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = args[0] as i16 ^ (args[1] as i16);
            if machine.debug {
                println!("Xor {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::AndEx => {
            //andEx(i32,i32)->ex1
            let args = take_bytes(machine, 2);
            set_reg(
                10,
                &mut machine.core,
                (args[0] as i32 & (args[1] as i32)) as f64,
            );
            if machine.debug {
                println!(
                    "AndEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::OrEx => {
            //andEx(i32,i32)->ex1
            let args = take_bytes(machine, 2);
            set_reg(
                10,
                &mut machine.core,
                (args[0] as i32 | (args[1] as i32)) as f64,
            );
            if machine.debug {
                println!(
                    "OrEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::XorEx => {
            //andEx(i32,i32)->ex1
            let args = take_bytes(machine, 2);
            set_reg(
                10,
                &mut machine.core,
                (args[0] as i32 ^ (args[1] as i32)) as f64,
            );
            if machine.debug {
                println!(
                    "XorEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::NotEx => {
            //notEx(i32) -> ex1
            let args = take_bytes(machine, 1);
            set_reg(10, &mut machine.core, (!(args[0] as i32) as i32) as f64);
            if machine.debug {
                println!("NotEx {} -> {}", args[0], get_reg(10, &machine.core));
            }
        }
        CommandType::Shl => {
            //shl(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] as i16) << (args[1] as i16);
            if machine.debug {
                println!("Shl {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Shr => {
            //shr(i16,i16) -> r1
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] as i16) >> (args[1] as i16);
            if machine.debug {
                println!("Shr {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::ShlEx => {
            //shlex(i16,i16) -> ex1
            let args = take_bytes(machine, 2);
            set_reg(
                10,
                &mut machine.core,
                ((args[0] as i32) << (args[1] as i32)) as f64,
            );
            if machine.debug {
                println!(
                    "ShlEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }
        CommandType::ShrEx => {
            //shrex(i16,i16) -> ex1
            let args = take_bytes(machine, 2);
            set_reg(
                10,
                &mut machine.core,
                ((args[0] as i32) >> (args[1] as i32)) as f64,
            );
            if machine.debug {
                println!(
                    "ShrEx {} {} -> {}",
                    args[0],
                    args[1],
                    get_reg(10, &machine.core)
                );
            }
        }

        CommandType::Push => {
            //push(i16)
            let args = take_bytes(machine, 1);
            machine
                .core
                .stack
                .push(DataType::Int(args[0] as i16), &mut machine.core.srp);
            if machine.debug {
                println!("Push {}", args[0]);
            }
        }
        CommandType::PushEx => {
            //pushEx(i32)
            let args = take_bytes(machine, 1);
            machine
                .core
                .stack
                .push(DataType::Int32(args[0] as i32), &mut machine.core.srp);
            if machine.debug {
                println!("PushEx {}", args[0]);
            }
        }
        CommandType::Pushf => {
            //pushf(f32)
            let args = take_bytes(machine, 1);
            machine
                .core
                .stack
                .push(DataType::Float(args[0] as f32), &mut machine.core.srp);
            if machine.debug {
                println!("Pushf {}", args[0]);
            }
        }
        CommandType::Mov => {
            //mov(f64) -> Register
            let args = take_bytes(machine, 1);
            let reg = take_registers(machine, 1)[0];
            set_reg(reg, &mut machine.core, args[0]);
            if machine.debug {
                println!("Mov {} -> R{}", args[0], reg);
            }
        }
        CommandType::JumpNotZero => {
            //jnz(address,f64)
            let args = take_bytes(machine, 2);
            if args[1] != 0.0 {
                machine.core.ip = args[0] as usize;
            }
            if machine.debug {
                println!("JumpNotZero {} {}", args[0], args[1]);
            }
        }
        CommandType::JumpZero => {
            //jz(address,f64)
            let args = take_bytes(machine, 2);
            if args[1] == 0.0 {
                machine.core.ip = args[0] as usize;
            }
            if machine.debug {
                println!("JumpZero {} {}", args[0], args[1]);
            }
        }
        CommandType::Load => {
            //load(address) -> Register
            let args = take_bytes(machine, 1);
            let val = machine.memory.read(args[0] as usize, machine) as f64;
            let reg = take_registers(machine, 1)[0];
            set_reg(reg, &mut machine.core, val);
            if machine.debug {
                println!("Load %{} -> R{}", args[0], reg);
            }
        }
        CommandType::LoadEx => {
            //load(address) -> Register
            let args = take_bytes(machine, 1);
            let val = machine
                .memory
                .read_range(args[0] as usize..args[0] as usize + 2usize, machine);
            let reg = take_registers(machine, 1)[0];
            set_reg(
                reg,
                &mut machine.core,
                convert_i16_to_i32(val.as_slice()) as f64,
            );
            if machine.debug {
                println!("LoadEx %{} -> R{}", args[0], reg);
            }
        }
        CommandType::Store => {
            //store(address,i16)
            let args = take_bytes(machine, 2);
            machine
                .memory
                .write(args[0] as usize, args[1] as i16, &mut machine.core);
            if machine.debug {
                println!("Store {} -> %{}", args[1], args[0]);
            }
        }
        CommandType::StoreEx => {
            //storeEx(address,i32)
            let args = take_bytes(machine, 2);
            machine.memory.write_range(
                args[0] as usize..(args[0] + 2.0) as usize,
                convert_i32_to_i16(args[1] as i32).to_vec(),
                &mut machine.core,
            );
            if machine.debug {
                println!("StoreEx {} -> %{}", args[1], args[0]);
            }
        }
        CommandType::Storef => {
            //storef(address,f32)
            let args = take_bytes(machine, 2);
            let f = convert_float(args[1] as f32);
            machine.memory.write_range(
                args[0] as usize..args[0] as usize + f.len(),
                f,
                &mut machine.core,
            );
            if machine.debug {
                println!("Storef {} -> %{}", args[1], args[0]);
            }
        }
        CommandType::Equals => {
            //Eq(a,b)
            let args = take_bytes(machine, 2);
            machine.core.r1 = (args[0] == args[1]) as i16;
            if machine.debug {
                println!("Equals {} {} -> {}", args[0], args[1], machine.core.r1);
            }
        }
        CommandType::Exit => {
            //exit()
            machine.on = false;
            if machine.debug {
                println!("Exit");
            }
        }
        CommandType::Loadf => {
            //loadf(address) -> Register
            let args = take_bytes(machine, 1);
            let val_bytes = &machine
                .memory
                .read_range(args[0] as usize..args[0] as usize + 2usize, machine);
            let val = unpack_float(val_bytes)
                .expect(&format!("Couldn't get float at memory address {}", args[0]));
            let reg = take_registers(machine, 1)[0];
            set_reg(reg, &mut machine.core, val as f64);
            if machine.debug {
                println!("Loadf %{} -> R{}", args[0], reg);
            }
        }
        CommandType::IO => {
            //io(device,command), driverags are on stack
            let args = take_bytes(machine, 2);
            if machine.debug {
                println!("IO {} {}", args[0], args[1]);
            }
            (machine.devices[args[0] as usize].driver.clone())(
                machine,
                args[1] as i16,
                args[0] as usize,
            );
        }
        //[callStack]
        //...args
        //prev arp
        //returnAddr
        // R1
        // R2
        // R3
        // R4
        // R5
        // F1
        // F2
        //...vars
        // returnedBytes
        CommandType::Call => {
            //call(fnptr)
            let func = take_bytes(machine, 1)[0];
            let arp = machine.core.stack.byte_len() + 16 * 1024 * 1024;
            machine.core.stack.push(
                DataType::Int32(machine.core.arp as i32),
                &mut machine.core.srp,
            );
            machine.core.arp = arp;
            machine.core.stack.push(
                DataType::Int32(machine.core.ip as i32),
                &mut machine.core.srp,
            );
            machine
                .core
                .stack
                .push(DataType::Int(machine.core.r1), &mut machine.core.srp);
            machine
                .core
                .stack
                .push(DataType::Int(machine.core.r2), &mut machine.core.srp);
            machine
                .core
                .stack
                .push(DataType::Int(machine.core.r3), &mut machine.core.srp);
            machine
                .core
                .stack
                .push(DataType::Int(machine.core.r4), &mut machine.core.srp);
            machine
                .core
                .stack
                .push(DataType::Int(machine.core.r5), &mut machine.core.srp);
            machine
                .core
                .stack
                .push(DataType::Float(machine.core.f1), &mut machine.core.srp);
            machine
                .core
                .stack
                .push(DataType::Float(machine.core.f2), &mut machine.core.srp);
            machine.core.ip = func as usize;
            if machine.debug {
                println!("Call %{}", func);
            }
        }
        CommandType::Return => {
            //return(returned_byte_count,fn_symbol_len,args)
            let args = take_bytes(machine, 3);
            machine.core.stack.pop_range(
                machine.core.srp - args[0] as usize - args[1] as usize
                    ..machine.core.srp - args[0] as usize,
                &mut machine.core.srp,
            );
            for x in 0..7 {
                let val = unpack_dt(machine.core.stack.remove(
                    machine.core.srp - (args[0] as usize + 1),
                    &mut machine.core.srp,
                ));
                match x {
                    0 => machine.core.f2 = val as f32,
                    1 => machine.core.f1 = val as f32,
                    2 => machine.core.r5 = val as i16,
                    3 => machine.core.r4 = val as i16,
                    4 => machine.core.r3 = val as i16,
                    5 => machine.core.r2 = val as i16,
                    6 => machine.core.r1 = val as i16,
                    _ => {}
                }
            }
            machine.core.ip = unpack_dt(machine.core.stack.remove(
                machine.core.srp - (args[0] as usize + 1),
                &mut machine.core.srp,
            )) as usize;
            machine.core.arp = unpack_dt(machine.core.stack.remove(
                machine.core.srp - (args[0] as usize + 1),
                &mut machine.core.srp,
            )) as usize;
            machine.core.stack.pop_range(
                machine.core.srp - 1 - args[2] as usize..machine.core.srp - 1,
                &mut machine.core.srp,
            );
            if machine.debug {
                println!("Return {} {} {}", args[0], args[1], args[2]);
            }
        }
        CommandType::NOP => {
            //nop()
            if machine.debug {
                println!("NOP");
            }
        }
        _ => {}
    }
}

fn take_bytes(machine: &mut Machine, bytecount: i16) -> Vec<f64> {
    let mut offset = machine.core.ip;
    let mut real_byte_count = 0;
    let mut bytes: Vec<f64> = Vec::new();
    for i in 0..bytecount {
        let byte = machine.memory.read(offset + i as usize, machine);
        if byte == i16::MIN {
            match machine.memory.read(offset + i as usize + 1, machine) {
                0 => {
                    bytes.push(
                        unpack_float(&[
                            machine.memory.read(offset + 2 + i as usize, machine),
                            machine.memory.read(offset + 3 + i as usize, machine),
                        ])
                        .expect("Couldn't convert bytes from i16 to float")
                            as f64,
                    );
                    offset += 3;
                    real_byte_count += 4;
                }
                1 => {
                    bytes.push(get_reg(
                        machine.memory.read(offset + 2 + i as usize, machine),
                        &machine.core,
                    ));
                    offset += 2;
                    real_byte_count += 3;
                }
                2 => {
                    bytes.push(convert_i16_to_i32(&[
                        machine.memory.read(offset + 2 + i as usize, machine),
                        machine.memory.read(offset + 3 + i as usize, machine),
                    ]) as f64);
                    offset += 3;
                    real_byte_count += 4;
                }
                _ => {}
            }
        } else {
            bytes.push(byte as f64);
            real_byte_count += 1;
        }
    }
    machine.core.ip += real_byte_count;
    bytes
}
fn take_registers(machine: &mut Machine, count: i16) -> Vec<i16> {
    let mut bytes: Vec<i16> = Vec::new();
    for i in 0..count {
        let byte = machine
            .memory
            .read(machine.core.ip + 2 + (i * 3) as usize, machine);
        bytes.push(byte);
    }
    machine.core.ip += (count * 3) as usize;
    bytes
}
pub struct Machine {
    pub devices: Vec<Device>,
    pub core: Core,
    pub debug: bool,
    pub memory: Memory,
    pub on: bool,
    pub freq: (u64, Instant),
}
impl Machine {
    pub fn new(debug: bool) -> Machine {
        let m = Machine {
            devices: devices::get_device_list(),
            core: Core::new(),
            debug,
            on: true,
            memory: Memory::new(16 * 1024 * 1024), //16MB max
            freq: (0, Instant::now()),
        };
        m
    }
    fn panic(&self, addr: usize) {
        println!("PANIC at %{}", addr);
        println!("__________________________________________");
        println!("State:");
        self.dump_state();
        //println!("Memory");
        //println!("{:?}", self.memory.data);
    }
    pub fn dump_state(&self) {
        println!("Core:");
        println!("IP: {}", self.core.ip);
        println!(
            "Frequency: {:.6}Mhz",
            self.freq.0 / (self.freq.1.elapsed().as_secs() + 1)
        );
        println!("Registers:");
        println!("R1: {}", self.core.r1);
        println!("R2: {}", self.core.r2);
        println!("R3: {}", self.core.r3);
        println!("R4: {}", self.core.r4);
        println!("R5: {}", self.core.r5);
        println!("F1: {}", self.core.f1);
        println!("F2: {}", self.core.f2);
        println!("EX1: {}", get_reg(10, &self.core));
        println!("EX2: {}", get_reg(11, &self.core));
        println!("ARP: {}", self.core.arp);
        println!("Stack:");
        println!("SRP: {}", self.core.srp);
        println!("Stack Pointer: {}", self.core.stack.len());
        println!("Stack Contents:");
        self.core.stack.dump();
    }
    pub fn run(&mut self) {
        if let RawDevice::Disk(disk) = &mut self.devices[0].contents {
            if disk[0].data.len() >= 256 {
                self.memory
                    .write_range(0..256, disk[0].data[0..256].to_vec(), &mut self.core);
            } else {
                self.memory.write_range(
                    0..disk[0].data.len(),
                    disk[0].data.clone(),
                    &mut self.core,
                );
            }
        } else {
            println!("No Disk Plugged In");
        }
        let mut debug_console = true;
        let mut breakpoints = Vec::new();
        while self.on {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                if self.debug && debug_console || (breakpoints.contains(&self.core.ip)) {
                    if breakpoints.contains(&self.core.ip) {
                        debug_console = true;
                    }
                    let input = input!("%{}>", self.core.ip);
                    let command = input.split_whitespace().collect::<Vec<&str>>();
                    if command.len() == 0 {
                        self.freq.0 += 1;
                        exec_bytecode(self)
                    } else {
                        match command[0] {
                            "help" => {
                                println!("Available commands:");
                                println!("  help - Display this help message");
                                println!("  step - Execute the next instruction");
                                println!("  dumpMem - Dump memory contents");
                                println!("  debugOff - Exit Debugger");
                                println!("  goto - Jump to an address");
                                println!("  stack - Display the stack");
                                println!("  exitConsole - Exit debug console");
                                println!("  breakpoint - Set a breakpoint");
                                println!("  device - Dump a device");
                                println!("  registers - Dump registers");
                                println!("  stop - Stops execution");
                                println!(
                                    "  nextCommand - Reads the byte at IP and displays it as a command"
                                );
                                println!(
                                    "  readMem - Reads x bytes from an address and displays it"
                                );
                            }
                            "step" => {
                                self.freq.0 += 1;
                                exec_bytecode(self)
                            }
                            "dumpMem" => {
                                let loc = 0;
                                let mut len = self.memory.len() + 1;
                                if len + loc >= self.memory.len() {
                                    len = self.memory.len() - loc;
                                }
                                let data = &self.memory.read_range(loc..len + loc, self);
                                let mut printed_data = "".to_string();
                                for i in 0..(len as f32 / 50.0).ceil() as usize {
                                    if i * 50 < data.len() {
                                        let end = if data.len() > (i + 1) * 50 {
                                            (i + 1) * 50
                                        } else {
                                            data.len()
                                        };
                                        printed_data.extend(
                                            format!(
                                                "%{:07}:{}\n",
                                                loc + i * 50,
                                                (&data[i * 50..end])
                                                    .iter()
                                                    .map(|x| format!(" {}", x))
                                                    .collect::<String>()
                                            )
                                            .chars(),
                                        )
                                    }
                                }
                                println!("{}", printed_data)
                            }
                            "debugOff" => {
                                self.debug = false;
                                println!("Debug Off");
                                self.freq.0 += 1;
                                exec_bytecode(self)
                            }
                            "goto" => {
                                let loc = command[1]
                                    .parse::<usize>()
                                    .expect("Could not parse goto loc");
                                self.core.ip = loc;
                            }
                            "stack" => {
                                println!("{:?}", self.core.stack.data)
                            }
                            "exitConsole" => {
                                debug_console = false;
                                self.freq.0 += 1;
                                exec_bytecode(self);
                            }
                            "breakpoint" => {
                                breakpoints.push(
                                    command[1]
                                        .parse::<usize>()
                                        .expect("Could not parse breakpoint loc"),
                                );
                            }
                            "device" => {
                                let device =
                                    command[1].parse::<usize>().expect("Invalid device ID");
                                println!("{:?}", self.devices[device].contents)
                            }
                            "registers" => {
                                println!(
                                    "R1: {}, R2: {}, R3: {}, R4: {}, R5:{}, EX1: {}, EX2: {}, F1: {}, F2: {}, SP: {}, SRP: {}, IP: {}, ARP: {}",
                                    self.core.r1,
                                    self.core.r2,
                                    self.core.r3,
                                    self.core.r4,
                                    self.core.r5,
                                    get_reg(10, &(self.core)),
                                    get_reg(11, &(self.core)),
                                    self.core.f1,
                                    self.core.f2,
                                    self.core.stack.len(),
                                    self.core.srp,
                                    self.core.ip,
                                    self.core.arp
                                )
                            }
                            "stop" => {
                                self.on = false;
                                return;
                            }
                            "nextCommand" => {
                                println!(
                                    "Command: {:?}",
                                    convert_int_to_command(
                                        self.memory.read(self.core.ip, &self) as i16
                                    )
                                );
                            }
                            "readMem" => {
                                let loc = command[1].parse::<usize>().expect("Invalid mem loc");
                                let mut len =
                                    command[2].parse::<usize>().expect("Invalid mem len") + 1;
                                if len + loc >= self.memory.len() {
                                    len = self.memory.len() - loc;
                                }
                                let data = &self.memory.read_range(loc..len + loc, self);
                                let mut printed_data = "".to_string();
                                for i in 0..(len as f32 / 50.0).ceil() as usize {
                                    if i * 50 < data.len() {
                                        let end = if data.len() > (i + 1) * 50 {
                                            (i + 1) * 50
                                        } else {
                                            data.len()
                                        };
                                        printed_data.extend(
                                            format!(
                                                "%{:07}:{}\n",
                                                loc + i * 50,
                                                (&data[i * 50..end])
                                                    .iter()
                                                    .map(|x| format!(" {}", x))
                                                    .collect::<String>()
                                            )
                                            .chars(),
                                        )
                                    }
                                }
                                println!("{}", printed_data)
                            }
                            _ => {}
                        }
                    }
                } else {
                    self.freq.0 += 1;
                    exec_bytecode(self)
                }
            }));
            if result.is_err() {
                self.panic(self.core.ip);
                return;
            }
        }
    }
    pub fn set_disk(&mut self, disk: Disk) {
        self.devices[0].contents = RawDevice::Disk(disk);
    }
}

#[derive(Debug)]
pub struct Core {
    pub ip: usize,
    pub stack: Stack,
    pub r1: i16,
    pub r2: i16,
    pub r3: i16,
    pub r4: i16,
    pub r5: i16,
    pub f1: f32,
    pub f2: f32,
    pub srp: usize,
    pub arp: usize,
}
impl Core {
    fn new() -> Core {
        Core {
            ip: 0,
            stack: Stack::new(),
            r1: 0,
            r2: 0,
            r3: 0,
            r4: 0,
            r5: 0,
            f1: 0.0,
            f2: 0.0,
            srp: 0,
            arp: 16 * 1024 * 1024,
        }
    }
}
type CoreState = (i16, i16, i16, i16, i16, f32, f32, usize);
#[derive(Debug)]
pub struct Memory {
    data: Vec<i16>,
    max_size: usize,
}
impl Memory {
    fn new(max_size: usize) -> Memory {
        Memory {
            data: vec![0; max_size],
            max_size,
        }
    }
    pub fn read(&self, index: usize, machine: &Machine) -> i16 {
        if index >= self.max_size {
            //gotta allow multiple bytes
            machine.core.stack.read_bytes(index - self.max_size, 1)[0]
        } else {
            self.data.get(index).copied().unwrap_or(0)
        }
    }
    pub fn read_range(&self, range: Range<usize>, machine: &Machine) -> Vec<i16> {
        let mut result = Vec::new();
        for i in range {
            result.push(self.read(i, machine));
        }
        result
    }
    pub fn write(&mut self, index: usize, value: i16, core: &mut Core) {
        if index >= self.max_size {
            core.stack
                .write_bytes(index - self.max_size, vec![value], &mut core.srp)
        } else {
            if index < self.data.len() {
                self.data[index] = value;
            } else {
                self.data.resize(index + 1, 0);
                self.data[index] = value;
            }
        }
    }
    pub fn write_range(&mut self, range: Range<usize>, value: Vec<i16>, core: &mut Core) {
        for (i, v) in range.zip(value) {
            self.write(i, v, core);
        }
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
}
#[derive(Debug, Clone, Copy)]
pub enum DataType {
    Float(f32),
    Int(i16),
    Int32(i32),
    None,
}
#[derive(Debug)]
pub struct Stack {
    data: Vec<DataType>,
}

pub fn unpack_dt(i: DataType) -> f64 {
    match i {
        DataType::Float(f) => f as f64,
        DataType::Int(i) => i as f64,
        DataType::Int32(e) => e as f64,
        _ => 0.0,
    }
}
fn unpack_dt_to_bytes(i: DataType) -> Vec<i16> {
    match i {
        DataType::Float(f) => convert_float(f),
        DataType::Int(i) => vec![i],
        DataType::Int32(e) => convert_i32_to_i16(e).to_vec(),
        _ => vec![0],
    }
}
fn dt_size(i: DataType) -> usize {
    match i {
        DataType::Float(_f) => 2,
        DataType::Int32(_i) => 2,
        DataType::Int(_i) => 1,
        DataType::None => 1,
    }
}
//srp is kinda broken
impl Stack {
    fn new() -> Stack {
        Stack { data: Vec::new() }
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn byte_len(&self) -> usize {
        self.data.iter().map(|x| dt_size(*x)).sum()
    }
    pub fn dump(&self) {
        self.data
            .chunks(16)
            .enumerate()
            .for_each(|(i, x)| println!("%{}: {:?}", i, x));
    }
    pub fn push(&mut self, x: DataType, srp: &mut usize) {
        if *srp >= self.data.len() {
            self.data.resize(*srp, DataType::None);
        }
        self.data.insert(*srp, x);
        *srp += 1;
    }
    fn convert_byte_index_to_stack(&self, byte_index: usize) -> Option<(usize, usize)> {
        let mut byte_count = 0;
        for (i, dt) in self.data.iter().enumerate() {
            let size = dt_size(*dt);
            if byte_count + size > byte_index {
                return Some((i, byte_index - byte_count));
            }
            byte_count += size;
        }
        None
    }
    pub fn read_bytes(&self, byte_index: usize, len: usize) -> Vec<i16> {
        let mut bytes = Vec::new();
        for i in byte_index..byte_index + len {
            let (index, offset) = self
                .convert_byte_index_to_stack(i)
                .unwrap_or_else(|| panic!("Address out of bounds: {}", byte_index));
            bytes.push(
                unpack_dt_to_bytes(
                    *self
                        .data
                        .get(index)
                        .expect(format!("Failed to get index {} of stack", index).as_str()),
                )[offset],
            );
        }
        bytes
    }
    pub fn write_bytes(&mut self, byte_index: usize, bytes: Vec<i16>, srp: &mut usize) {
        for i in byte_index..byte_index + bytes.len() {
            let (index, offset) = self.convert_byte_index_to_stack(i).unwrap_or_else(|| {
                self.resize(i + 1, srp);
                self.convert_byte_index_to_stack(i).unwrap()
            });
            *self
                .data
                .get_mut(index)
                .expect(format!("Failed to get index {} of stack", index).as_str()) =
                match *self
                    .data
                    .get(index)
                    .expect(format!("Failed to get index {} of stack", index).as_str())
                {
                    DataType::Int(_i) => DataType::Int(bytes[i - byte_index]),
                    DataType::Float(_f) => {
                        let mut unpacked =
                            unpack_dt_to_bytes(*self.data.get(index).expect(
                                format!("Failed to get index {} of stack", index).as_str(),
                            ));
                        unpacked[offset] = bytes[i - byte_index];
                        DataType::Float(
                            unpack_float(unpacked.as_slice()).expect("Couldn't write to stack"),
                        )
                    }
                    DataType::Int32(_e) => {
                        let mut unpacked =
                            unpack_dt_to_bytes(*self.data.get(index).expect(
                                format!("Failed to get index {} of stack", index).as_str(),
                            ));
                        unpacked[offset] = bytes[i - byte_index];
                        DataType::Int32(convert_i16_to_i32(unpacked.as_slice()))
                    }
                    _ => DataType::Int(bytes[i - byte_index]),
                };
        }
    }
    pub fn pop(&mut self, srp: &mut usize) -> DataType {
        self.remove(*srp - 1, srp)
    }
    pub fn pop_range(&mut self, range: std::ops::Range<usize>, srp: &mut usize) {
        let rlen = range.len();
        self.data.drain(range);
        *srp -= rlen;
    }
    pub fn remove(&mut self, index: usize, srp: &mut usize) -> DataType {
        *srp -= 1;
        self.data.remove(index)
    }
    pub fn resize(&mut self, size: usize, srp: &mut usize) {
        if size <= self.data.len() {
            *srp = size;
        }
        self.data.resize(size, DataType::None);
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
pub enum CommandType {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Addf,
    Subf,
    Mulf,
    Divf,
    AddEx,
    SubEx,
    MulEx,
    DivEx,
    AddU,
    SubU,
    MulU,
    DivU,
    AddExU,
    SubExU,
    MulExU,
    DivExU,
    And,
    Not,
    Or,
    Xor,
    AndEx,
    OrEx,
    NotEx,
    XorEx,
    Push,
    Pushf,
    PushEx,
    Pop,
    Load,
    LoadEx,
    Store,
    StoreEx,
    Storef,
    Mov,
    Jump,
    JumpNotZero,
    JumpZero,
    Greater,
    LessThan,
    Exit,
    R1,
    R2,
    R3,
    R4,
    F1,
    F2,
    IP,
    SP,
    SRP,
    ARP,
    R5,
    EX1,
    EX2,
    NOP,
    IO,
    Loadf,
    Call,
    Return,
    Equals,
    Shl,
    Shr,
    ShlEx,
    ShrEx,
}
