use crate::executable::{Bytecode, Fn, Library};
use crate::vm::CommandType;
use std::{fs, io, path::Path};

const GFX_DEVICE_ID: i16 = 3;
const AUDIO_DEVICE_ID: i16 = 1;
const CLOCK_DEVICE_ID: i16 = 2;
const SERIAL_DEVICE_ID: i16 = 4;
const DISK_DEVICE_ID: i16 = 0;
const EXEC_HEADER_BASE_ADDR: i16 = 512;
const EXEC_BASE_SECTOR_ADDR: i16 = EXEC_HEADER_BASE_ADDR + 1;
const EXEC_CODE_SECTOR_COUNT_ADDR: i16 = EXEC_HEADER_BASE_ADDR + 3;
const EXEC_DATA_SECTOR_COUNT_ADDR: i16 = EXEC_HEADER_BASE_ADDR + 5;
pub fn gen_libs() {
    gen_gfx().expect("Failed to generate gfx stdlib");
    gen_audio().expect("Failed to generate audio stdlib");
    gen_clock().expect("Failed to generate clock stdlib");
    gen_serial().expect("Failed to generate serial stdlib");
    gen_disk().expect("Failed to generate disk stdlib");
    gen_mem_core().expect("Failed to generate mem_core stdlib");
}
fn gen_gfx() -> io::Result<()> {
    let mut gfx = Library::new("gfx".to_string());

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "registerAtlas".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(0),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "registerSprite".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(2),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "removeSprite".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(7),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "registerLayer".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(1),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "removeLayer".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(8),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "render".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID), Bytecode::Int(3),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),             Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "deltaTime".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID), Bytecode::Int(11),
            Bytecode::Command(CommandType::Return), Bytecode::Int(1),              Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "pullControls".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(4),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "setPixel".to_string(),
        vec![1, 1, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(2),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(5),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "getPixel".to_string(),
        vec![1, 1],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(6),
            Bytecode::Command(CommandType::Return), Bytecode::Int(1),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "registerBitmap".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(9),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    gfx.add_fn(Fn::new_with_blocks(
        "removeBitmap".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID),         Bytecode::Int(10),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    let out = Path::new("src/std/gfx.bin");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    gfx.to_file(out)
}

fn gen_audio() -> io::Result<()> {
    let mut audio = Library::new("audio".to_string());

    #[rustfmt::skip]
    audio.add_fn(Fn::new_with_blocks(
        "pause".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::IO),     Bytecode::Int(AUDIO_DEVICE_ID), Bytecode::Int(0),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),               Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    audio.add_fn(Fn::new_with_blocks(
        "unpause".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::IO),     Bytecode::Int(AUDIO_DEVICE_ID), Bytecode::Int(1),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),               Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    audio.add_fn(Fn::new_with_blocks(
        "volume".to_string(),
        vec![1, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::Loadf),  Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::Pushf),  Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(AUDIO_DEVICE_ID),       Bytecode::Int(2),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    audio.add_fn(Fn::new_with_blocks(
        "pan".to_string(),
        vec![1, 2, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(2),
            Bytecode::Command(CommandType::Loadf),  Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::Pushf),  Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::Loadf),  Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::Pushf),  Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(AUDIO_DEVICE_ID),       Bytecode::Int(3),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    audio.add_fn(Fn::new_with_blocks(
        "frequency".to_string(),
        vec![1, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::Loadf),  Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::Pushf),  Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(AUDIO_DEVICE_ID),       Bytecode::Int(4),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    audio.add_fn(Fn::new_with_blocks(
        "masterVolume".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(AUDIO_DEVICE_ID),       Bytecode::Int(5),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    audio.add_fn(Fn::new_with_blocks(
        "loadSound".to_string(),
        vec![1, 2, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(2),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(AUDIO_DEVICE_ID),       Bytecode::Int(6),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    audio.add_fn(Fn::new_with_blocks(
        "setLoop".to_string(),
        vec![1, 1],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(AUDIO_DEVICE_ID),       Bytecode::Int(7),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    let out = Path::new("src/std/audio.bin");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    audio.to_file(out)
}

fn gen_clock() -> io::Result<()> {
    let mut clock = Library::new("clock".to_string());

    #[rustfmt::skip]
    clock.add_fn(Fn::new_with_blocks(
        "read".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::IO),     Bytecode::Int(CLOCK_DEVICE_ID), Bytecode::Int(0),
            Bytecode::Command(CommandType::Return), Bytecode::Int(1),               Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    let out = Path::new("src/std/clock.bin");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    clock.to_file(out)
}

fn gen_serial() -> io::Result<()> {
    let mut serial = Library::new("serial".to_string());

    #[rustfmt::skip]
    serial.add_fn(Fn::new_with_blocks(
        "write".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(SERIAL_DEVICE_ID),       Bytecode::Int(0),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                      Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    serial.add_fn(Fn::new_with_blocks(
        "writeNum".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(SERIAL_DEVICE_ID),       Bytecode::Int(1),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                      Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    serial.add_fn(Fn::new_with_blocks(
        "writeFloat".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Loadf), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::Pushf), Bytecode::Register(CommandType::F1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(SERIAL_DEVICE_ID),       Bytecode::Int(2),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                      Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    let out = Path::new("src/std/serial.bin");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    serial.to_file(out)
}

fn gen_disk() -> io::Result<()> {
    let mut disk = Library::new("disk".to_string());

    #[rustfmt::skip]
    disk.add_fn(Fn::new_with_blocks(
        "read".to_string(),
        vec![1, 2, 2, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(3),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(2),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(DISK_DEVICE_ID),        Bytecode::Int(0),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    disk.add_fn(Fn::new_with_blocks(
        "write".to_string(),
        vec![1, 2, 2, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(3),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(2),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(DISK_DEVICE_ID),        Bytecode::Int(1),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    disk.add_fn(Fn::new_with_blocks(
        "loadSectors".to_string(),
        vec![1, 1, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(2),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::IO),     Bytecode::Int(DISK_DEVICE_ID),        Bytecode::Int(2),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    disk.add_fn(Fn::new_with_blocks(
        "linkedFileStart".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::Load),   Bytecode::Int(EXEC_BASE_SECTOR_ADDR),       Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Load),   Bytecode::Int(EXEC_CODE_SECTOR_COUNT_ADDR), Bytecode::Register(CommandType::R2),
            Bytecode::Command(CommandType::Add),    Bytecode::Register(CommandType::R1),        Bytecode::Register(CommandType::R2),
            Bytecode::Command(CommandType::Load),   Bytecode::Int(EXEC_DATA_SECTOR_COUNT_ADDR), Bytecode::Register(CommandType::R2),
            Bytecode::Command(CommandType::Add),    Bytecode::Register(CommandType::R1),        Bytecode::Register(CommandType::R2),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Return), Bytecode::Int(1),                           Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));
    #[rustfmt::skip]
    disk.add_fn(Fn::new_with_blocks(
        "sectorCount".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::IO),Bytecode::Int(DISK_DEVICE_ID),Bytecode::Int(3),
            Bytecode::Command(CommandType::Return), Bytecode::Int(1),Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    let out = Path::new("src/std/disk.bin");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    disk.to_file(out)
}

fn gen_mem_core() -> io::Result<()> {
    let mut mem_core = Library::new("mem_core".to_string());

    #[rustfmt::skip]
    mem_core.add_fn(Fn::new_with_blocks(
        "read_mem".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Push),   Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Return), Bytecode::Int(1),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    mem_core.add_fn(Fn::new_with_blocks(
        "read_mem_ex".to_string(),
        vec![2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::PushEx), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::Return), Bytecode::Int(1),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    mem_core.add_fn(Fn::new_with_blocks(
        "write_mem".to_string(),
        vec![2, 1],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::Load),   Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::AddEx),  Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::Store),  Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::R1),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    mem_core.add_fn(Fn::new_with_blocks(
        "write_mem_ex".to_string(),
        vec![2, 2],
        vec![vec![
            Bytecode::Command(CommandType::AddEx),   Bytecode::Register(CommandType::ARP), Bytecode::Argument(1),
            Bytecode::Command(CommandType::LoadEx),  Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX2),
            Bytecode::Command(CommandType::AddEx),   Bytecode::Register(CommandType::ARP), Bytecode::Argument(0),
            Bytecode::Command(CommandType::LoadEx),  Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX1),
            Bytecode::Command(CommandType::StoreEx), Bytecode::Register(CommandType::EX1), Bytecode::Register(CommandType::EX2),
            Bytecode::Command(CommandType::Return),  Bytecode::Int(0),                     Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    #[rustfmt::skip]
    mem_core.add_fn(Fn::new_with_blocks(
        "heap_start".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::PushEx), Bytecode::HeapStart(),
            Bytecode::Command(CommandType::Return), Bytecode::Int(1), Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
        ]],
    ));

    let out = Path::new("src/std/mem_core.bin");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    mem_core.to_file(out)
}
