use crate::executable::{Bytecode, Fn, Library};
use crate::vm::CommandType;
use std::{fs, io, path::Path};

const GFX_DEVICE_ID: i16 = 3;
pub fn gen_libs() {
    gen_gfx();
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
        "render".to_string(),
        vec![],
        vec![vec![
            Bytecode::Command(CommandType::IO),     Bytecode::Int(GFX_DEVICE_ID), Bytecode::Int(3),
            Bytecode::Command(CommandType::Return), Bytecode::Int(0),             Bytecode::SymbolSectionLen(), Bytecode::ArgCount(),
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

    let out = Path::new("src/std/gfx.bin");
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    gfx.to_file(out)
}
