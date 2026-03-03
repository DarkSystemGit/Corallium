use crate::devices::gfx::{Matrix, Point};
use crate::vm::{CommandType, Core, unpack_dt};
use byteorder::{ByteOrder, LittleEndian};

pub fn resize_vec<T>(len: usize, vec: &mut Vec<T>, fill: T)
where
    T: Clone,
{
    if vec.len() <= len {
        vec.resize(len, fill);
    }
}

pub fn flatten_vec<T>(i: Vec<Vec<T>>) -> Vec<T> {
    i.into_iter().flat_map(|row| row).collect()
}
pub fn pack_float(f: f32) -> Vec<i16> {
    let mut rvec = vec![i16::MIN, 0];
    rvec.extend(convert_float(f));
    rvec
}
pub fn unpack_float(bytes: &[i16]) -> Option<f32> {
    if bytes.len() != 2 {
        return None;
    }
    let i16_1 = bytes[0];
    let i16_2 = bytes[1];
    let mut rbytes = [0u8; 4];
    LittleEndian::write_i16(&mut rbytes[0..2], i16_1);
    LittleEndian::write_i16(&mut rbytes[2..4], i16_2);
    Some(f32::from_le_bytes(rbytes))
}
pub fn gen_rotation_matrix(degrees: f32) -> Matrix {
    let radians = degrees.to_radians();
    [
        [radians.cos(), -(radians.sin())],
        [radians.sin(), radians.cos()],
    ]
}
pub fn gen_3d_matrix(
    cam_x: f32,
    cam_y: f32,
    cam_z: f32,
    horizon: f32,
    rotation: f32,
    fov: f32,
    scanlines: usize,
) -> (Vec<Matrix>, Point) {
    let mut matrices = Vec::new();
    let cos_r = rotation.to_radians().cos();
    let sin_r = rotation.to_radians().sin();
    for i in 0..scanlines {
        let screen_y_diff = i as f32 - horizon;
        if screen_y_diff <= 0.0 {
            matrices.push([[0.0, 0.0], [0.0, 0.0]]);
            continue;
        }
        // Perspective distance
        let z = (cam_z * fov) / screen_y_diff;
        let s = 1.0 / z;
        let m00 = s * cos_r;
        let m01 = s * -sin_r;
        let m10 = s * sin_r;
        let m11 = s * cos_r;

        matrices.push([[m00, m01], [m10, m11]]);
    }
    (matrices, [cam_x as i32, cam_y as i32])
}
pub fn convert_u32_to_i16(val: u32) -> Vec<i16> {
    let native = val.to_le_bytes();
    vec![
        LittleEndian::read_i16(&native[0..2]),
        LittleEndian::read_i16(&native[2..4]),
    ]
}
pub fn convert_i16_to_u32(bytes: &[i16]) -> Option<u32> {
    if bytes.len() != 2 {
        return None;
    }
    let i16_1 = bytes[0];
    let i16_2 = bytes[1];
    let mut rbytes = [0u8; 4];
    LittleEndian::write_i16(&mut rbytes[0..2], i16_1);
    LittleEndian::write_i16(&mut rbytes[2..4], i16_2);
    Some(u32::from_le_bytes(rbytes))
}
pub fn convert_float(f: f32) -> Vec<i16> {
    let native = f.to_le_bytes();
    vec![
        LittleEndian::read_i16(&native[0..2]),
        LittleEndian::read_i16(&native[2..4]),
    ]
}
pub fn pop_stack(machine: &mut Core, bytes: i32) -> Vec<f64> {
    let mut ret = Vec::new();
    for _i in 0..bytes {
        ret.push(unpack_dt(machine.stack.pop(&mut machine.srp)));
    }
    ret
}

pub fn convert_int_to_command(i: i16) -> CommandType {
    match i {
        32 => CommandType::Add,
        1 => CommandType::Sub,
        2 => CommandType::Mul,
        3 => CommandType::Div,
        4 => CommandType::Mod,
        5 => CommandType::Addf,
        6 => CommandType::Subf,
        7 => CommandType::Mulf,
        8 => CommandType::Divf,
        9 => CommandType::And,
        10 => CommandType::Not,
        11 => CommandType::Or,
        12 => CommandType::Xor,
        13 => CommandType::Push,
        14 => CommandType::Pop,
        15 => CommandType::Load,
        16 => CommandType::Store,
        17 => CommandType::Mov,
        19 => CommandType::Jump,
        20 => CommandType::JumpNotZero,
        21 => CommandType::Greater,
        22 => CommandType::LessThan,
        23 => CommandType::Exit,
        30 => CommandType::IP,
        31 => CommandType::SP,
        0 => CommandType::NOP,
        33 => CommandType::IO,
        34 => CommandType::Call,
        35 => CommandType::Return,
        36 => CommandType::JumpZero,
        37 => CommandType::Loadf,
        38 => CommandType::LoadEx,
        39 => CommandType::AddEx,
        40 => CommandType::SubEx,
        41 => CommandType::MulEx,
        42 => CommandType::DivEx,
        43 => CommandType::Pushf,
        44 => CommandType::PushEx,
        45 => CommandType::StoreEx,
        46 => CommandType::Storef,
        47 => CommandType::Equals,
        48 => CommandType::Shl,
        49 => CommandType::Shr,
        50 => CommandType::ShlEx,
        51 => CommandType::ShrEx,
        52 => CommandType::AddU,
        53 => CommandType::SubU,
        54 => CommandType::MulU,
        55 => CommandType::DivU,
        56 => CommandType::AddExU,
        57 => CommandType::SubExU,
        58 => CommandType::MulExU,
        59 => CommandType::DivExU,
        60 => CommandType::AndEx,
        61 => CommandType::OrEx,
        62 => CommandType::NotEx,
        63 => CommandType::XorEx,
        _ => CommandType::NOP,
    }
}
pub fn pack_command(c: CommandType) -> i16 {
    match c {
        CommandType::Add => 32,
        CommandType::Sub => 1,
        CommandType::Mul => 2,
        CommandType::Div => 3,
        CommandType::Mod => 4,
        CommandType::Addf => 5,
        CommandType::Subf => 6,
        CommandType::Mulf => 7,
        CommandType::Divf => 8,
        CommandType::And => 9,
        CommandType::Not => 10,
        CommandType::Or => 11,
        CommandType::Xor => 12,
        CommandType::Push => 13,
        CommandType::Pop => 14,
        CommandType::Load => 15,
        CommandType::Store => 16,
        CommandType::Mov => 17,
        CommandType::Jump => 19,
        CommandType::JumpNotZero => 20,
        CommandType::Greater => 21,
        CommandType::LessThan => 22,
        CommandType::Exit => 23,
        CommandType::NOP => 0,
        CommandType::IO => 33,
        CommandType::Call => 34,
        CommandType::Return => 35,
        CommandType::JumpZero => 36,
        CommandType::Loadf => 37,
        CommandType::LoadEx => 38,
        CommandType::AddEx => 39,
        CommandType::SubEx => 40,
        CommandType::MulEx => 41,
        CommandType::DivEx => 42,
        CommandType::Pushf => 43,
        CommandType::PushEx => 44,
        CommandType::StoreEx => 45,
        CommandType::Storef => 46,
        CommandType::Equals => 47,
        CommandType::Shl => 48,
        CommandType::Shr => 49,
        CommandType::ShlEx => 50,
        CommandType::ShrEx => 51,
        CommandType::AddU => 52,
        CommandType::SubU => 53,
        CommandType::MulU => 54,
        CommandType::DivU => 55,
        CommandType::AddExU => 56,
        CommandType::SubExU => 57,
        CommandType::MulExU => 58,
        CommandType::DivExU => 59,
        CommandType::AndEx => 60,
        CommandType::OrEx => 61,
        CommandType::NotEx => 62,
        CommandType::XorEx => 63,
        _ => 0,
    }
}
pub fn pack_i32(i: i32) -> Vec<i16> {
    let mut base = vec![i16::MIN, 2];
    base.extend_from_slice(&convert_i32_to_i16(i));
    base
}
pub fn pack_register(r: CommandType) -> Vec<i16> {
    vec![
        i16::MIN,
        1,
        match r {
            CommandType::R1 => 1,
            CommandType::R2 => 2,
            CommandType::R3 => 3,
            CommandType::R4 => 4,
            CommandType::F1 => 5,
            CommandType::F2 => 6,
            CommandType::IP => 7,
            CommandType::SP => 8,
            CommandType::SRP => 9,
            CommandType::EX1 => 10,
            CommandType::EX2 => 11,
            CommandType::ARP => 12,
            CommandType::R5 => 13,
            _ => 0,
        },
    ]
}
pub fn convert_i16_to_i32(bytes: &[i16]) -> i32 {
    LittleEndian::read_i32(
        &(bytes
            .iter()
            .flat_map(|x| x.to_le_bytes())
            .collect::<Vec<u8>>()),
    )
}
pub fn convert_i32_to_i16(bytes: i32) -> [i16; 2] {
    let native = bytes.to_le_bytes();
    [
        LittleEndian::read_i16(&native[0..2]),
        LittleEndian::read_i16(&native[2..4]),
    ]
}
pub fn get_reg(reg: i16, machine: &Core) -> f64 {
    match reg {
        1 => machine.r1 as f64,
        2 => machine.r2 as f64,
        3 => machine.r3 as f64,
        4 => machine.r4 as f64,
        5 => machine.f1 as f64,
        6 => machine.f2 as f64,
        7 => machine.ip as f64,
        8 => machine.stack.len() as f64,
        9 => machine.srp as f64,
        10 => convert_i16_to_i32(&[machine.r2, machine.r3]) as f64,
        11 => convert_i16_to_i32(&[machine.r4, machine.r5]) as f64,
        12 => machine.arp as f64,
        13 => machine.r5 as f64,
        _ => panic!("Invalid register"),
    }
}
pub fn set_reg(reg: i16, machine: &mut Core, value: f64) {
    match reg {
        1 => machine.r1 = value as i16,
        2 => machine.r2 = value as i16,
        3 => machine.r3 = value as i16,
        4 => machine.r4 = value as i16,
        5 => machine.f1 = value as f32,
        6 => machine.f2 = value as f32,
        7 => machine.ip = value as usize,
        8 => machine.stack.resize(value as usize, &mut machine.srp),
        9 => machine.srp = value as usize,
        10 => {
            let bytes = (value as i32).to_le_bytes();
            machine.r2 = LittleEndian::read_i16(&bytes[0..2]);
            machine.r3 = LittleEndian::read_i16(&bytes[2..4]);
        }
        11 => {
            let bytes = (value as i32).to_le_bytes();
            machine.r4 = LittleEndian::read_i16(&bytes[0..2]);
            machine.r5 = LittleEndian::read_i16(&bytes[2..4]);
        }
        12 => machine.arp = value as usize,
        13 => machine.r5 = value as i16,
        _ => (),
    }
}
