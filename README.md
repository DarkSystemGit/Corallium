# Corallium

Corallium is a fantasy console, similar to a previous project of mine, Atto-24. It supports Coral, a C-like language with Rust-inspired syntax, plus built-in graphics, sound, disk, and clock devices.

This repository contains a Coral Compiler, an executable/linking layer, and the VM.

## Quick start

### Requirements

- Rust (Cargo)
- A desktop session (the VM opens a graphics window and initializes audio output)
- X11/XWayland support on Linux, due to the lack of window decorations on Wayland 

### Run a Coral program

```bash
cargo run -- --run --file test/importTest.coral
```

Enable the runtime debugger:

```bash
cargo run -- --run --file test/importTest.coral --debug
```

Compile to a serialized disk image (`.cart`):

```bash
cargo run -- --compile --file test/importTest.coral
```

Link one or more extra files or directories into the disk image:

```bash
cargo run -- --compile --file test/importTest.coral --link path/to/file1 path/to/dir
```

Run from a serialized disk image:

```bash
cargo run -- --bytecode --file test/importTest.cart
```

Show CLI help:

```bash
cargo run -- --help
```

## Coral language

Coral currently includes:

- `let` bindings, arithmetic, bitwise ops, and `as` casts
- `if`/`else`, `while`, `for`, `break`, and `continue`
- `match` expressions with literal, wildcard, enum, union, struct, and optional patterns
- `struct`, `union`, `enum`, and `type` declarations
- optionals (`T?`) with `Some`, `None`, and `try ... catch`
- `defer`, function calls, pointers, arrays, and `sizeof(...)`

Example:

```rust
fn main() -> void {
  let x: i16=1+2+3;
  let y: i16=4*5*6;
  let z: i32=(y as i32)/(x as i32);
  return;
}
```

## Specs

- Memory: 16 MiB base RAM, with stack-addressable memory above that range
- Display: 320x240 framebuffer (scaled in a window)
- Audio: 32 kHz output with built-in square/triangle/saw/sample channels
- VM ISA: integer, float, extended 32-bit, stack, control-flow, call/return, and device I/O ops

Device I/O is invoked as `IO(device_id, command_id)`, with command arguments passed on the VM stack.

## Built-in devices

| Device ID | Device | Command IDs |
| --- | --- | --- |
| `0` | Disk | `0=read`, `1=write`, `2=loadSectors` |
| `1` | Audio | `0=pause`, `1=unpause`, `2=volume`, `3=pan`, `4=frequency`, `5=masterVolume`, `6=loadSound` |
| `2` | Clock | `0=read` |
| `3` | Graphics | `0=registerAtlas`, `1=registerLayer`, `2=registerSprite`, `3=render`, `4=pullControls`, `5=setPixel`, `6=getPixel`, `7=removeSprite`, `8=removeLayer` |
| `4` | Serial | `0=write (null-terminated string ptr)`, `1=writeNum (i32)` |

Graphics control mapping:

- `A/S/D/F` -> `A/B/X/Y`
- Arrow keys -> D-pad
- `Space` -> Start
- `Q/E` -> Left/Right trigger

## Imports and linking

- `import "module.coral";` loads Coral source modules relative to the importing file
- Importing a `.h` path loads symbols from the header and links a matching precompiled `.bin` library
- `disk::linkedFileStart()` returns the first sector after executable code/data, useful for reading files linked with `--link`

## Project layout

- `src/compiler/` - Coral frontend, AST, IR generation, and backend lowering
- `src/std/` - Coral standard libary
- `src/executable.rs` - bytecode/function packing, constants, disk image build
- `src/vm.rs` - VM execution engine, stack/memory model, debug console
- `src/devices/` - disk, audio, clock, graphics, and serial drivers
- `test/` - small Coral examples & tests
