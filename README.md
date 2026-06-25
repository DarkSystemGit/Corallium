# Corallium

Corallium is a fantasy console, similar to a previous project of mine, Atto-24. It supports Coral, a C-like language with Rust-inspired syntax, plus built-in graphics, sound, disk, and clock devices.

This repository contains a Coral Compiler, an executable/linking layer, and the VM.

## Quick start

### Requirements

- Rust (Cargo)
- A desktop session (the VM opens a graphics window and initializes audio output)
- X11/XWayland support on Linux, due to the lack of window decorations on Wayland 

### Install
```bash
chmod +x ./install.sh && ./install.sh
```

This script should work for both linux and MacOS, though Mac is untested, along with Windows. Read over the script carefully before running it.
### Run a Coral program
Compile to a cartridge (`.cart`):

```bash
corallium compile --file test/importTest.coral
```

Link one or more extra files or directories into the cartridge:

```bash
corallium compile --file test/importTest.coral --link path/to/file1 path/to/dir
```

Run from a cartridge:

```bash
corallium run --file test/importTest.cart
```
Enable the runtime debugger:

```bash
corallium run --file test/importTest.cart --debug
```

Show CLI help:

```bash
corallium --help
```

### SDK file converters and binary formats

#### MIDI -> `.csfx`

```bash
corallium sdk convert_music path/to/song.mid
```

Optional preview WAV:

```bash
corallium sdk convert_music path/to/song.mid --wav-preview
```

`convert_music` writes `path/to/song.csfx` with this exact layout:

```
CSFX FILE
+------------------------------+
| Header (48 bytes total)      |
| 8 channel records, each:     |
|   pan        : f32 (4 bytes) |
|   event_count: i16 (2 bytes) |
+------------------------------+
| Event Body                   |
| Channel 0 events             |
| Channel 1 events             |
| ...                          |
| Channel 7 events             |
+------------------------------+
```

Each event entry is 10 bytes:

```
+----------------------------+
| timestamp_ms : i32 (4)     |
| frequency_hz : f32 (4)     |
| volume       : i16 (2)     |
+----------------------------+
```

All numeric fields are little-endian.

#### Image -> `.cbmp`

```bash
corallium sdk convert_image path/to/image.png
```

`convert_image` writes `path/to/image.cbmp` with this exact layout:

```
CBMP FILE
+------------------------------+
| width  : i16 (2 bytes)       |
| height : i16 (2 bytes)       |
+------------------------------+
| pixels : width*height*u32    |
|          row-major order     |
+------------------------------+
```

Each pixel is one `u32` value stored little-endian, where the packed color is:

```
0xRRGGBBAA
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

## Polyp assembly

Polyp is the assembly language for Corallium. See `docs/polyp.md` for the syntax, instruction reference, and examples.

## Specs

- Memory: 16 MiB base RAM, with stack-addressable memory above that range
- Display: 320x240 framebuffer (scaled in a window)
- Audio: 32 kHz output with built-in square/triangle/saw/sample channels
- ISA: integer, float, extended 32-bit, stack, control-flow, call/return, and device I/O ops

Device I/O is invoked as `IO(device_id, command_id)`, with command arguments passed on the VM stack.

## Built-in devices

| Device ID | Device | Command IDs |
| --- | --- | --- |
| `0` | Disk | `0=read`, `1=write`, `2=loadSectors` |
| `1` | Audio | `0=pause`, `1=unpause`, `2=volume`, `3=pan`, `4=frequency`, `5=masterVolume`, `6=loadSound`, `7=setLoop`, `8=schedule`, `9=masterClock` |
| `2` | Clock | `0=read` |
| `3` | Graphics | `0=registerAtlas`, `1=registerLayer`, `2=registerSprite`, `3=render`, `4=pullControls`, `5=setPixel`, `6=getPixel`, `7=removeSprite`, `8=removeLayer`, `9=registerBitmap`, `10=removeBitmap`, `11=deltaTime` |
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
- `src/assembler/` - Polyp frontend, AST, and Code Generation
- `src/executable.rs` - bytecode/function packing, constants, disk image build
- `src/vm.rs` - VM execution engine, stack/memory model, debug console
- `src/devices/` - disk, audio, clock, graphics, and serial drivers
- `test/` - small Coral examples & tests
## Demos

You can run a demo game made for corralium in `demos/strider/strider.cart`. You can run: 
```bash
corallium run --file demos/strider/strider.cart
```

## Credits
Credit to Subspace Audio for the start up sound (sourced from this pack: [512 8-bit sounds](https://opengameart.org/content/512-sound-effects-8-bit-style)).
