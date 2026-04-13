# Coral language guide

Coral is a C-like language with Rust-inspired syntax for the Corallium VM.

## Quick start

Run a Coral file:

```bash
cargo run -- --run --file test/importTest.coral
```

Compile a Coral file to `.cart`:

```bash
cargo run -- --compile --file test/importTest.coral
```

Run a `.cart`:

```bash
cargo run -- --bytecode --file test/importTest.cart
```

## Basic syntax

### Program layout

```rust
import "../src/std/serial.h";

fn main() -> void {
    serial::write("Hello, Coral!");
    return;
}
```

- Entry point is `fn main() -> void`.
- Imports are relative to the importing file.
- Namespacing is `module::symbol`.

### Variables, functions, and arrays

```rust
let a: i16 = 10;
let b: i32 = 1000;
let text: [char] = "hello";
let arr: [i16; 4] = [1, 2, 3, 4];
let zeros: [i16; 8] = [0; 8];

fn add(x: i32, y: i32) -> i32 {
    return x + y;
}
```

- Declaration form: `let name: Type = expression;`
- Array literals support `[a, b, c]` and `[value; count]`

### Control flow

```rust
let a: i16= 0;
if (a > 0) {
    a = a - 1;
} else {
    a = a + 1;
};

while (a < 10) {
    a = a + 1;
};

for (let i: i16 = 0; i < 4; i = i + 1) {
    a = a + (i as i16);
};
```

**Semicolons after block statements are required** (outside of function declarations).

### User-defined types

```rust
struct Vec2 {
    x: i16,
    y: i16,
}

enum Color {
    Red,
    Green,
    Blue,
}

union Value {
    I: i32,
    F: f32,
}
```

```rust
let p: Vec2 = Vec2{ x: 10, y: 20 };
let c: Color = Color::Green;
let v: Value = Value::I(42 as i32);
```

### Optionals and `match`

```rust
type MaybeInt = i32?;

fn unwrap_or_zero(v: MaybeInt) -> i32 {
    return match v {
        Some(x) -> x,
        None -> 0 as i32
    };
}
```

## Type behavior (important)

Coral syntax is straightforward, but aggregate types are pointer-backed in compiler lowering.

| Type syntax | Runtime/IR behavior |
| --- | --- |
| `&T` | Pointer |
| `[T]` | Pointer-backed array view |
| `[T; N]` | Pointer-backed fixed-size array view |
| `struct Name` | Pointer-backed aggregate |
| `union Name` | Pointer-backed aggregate |
| `enum Name` | Value-like enum |

### Returning pointer-backed values

Do not return nested structs or other pointer-backed aggregate values that only live in the current stack frame.

If the value must outlive the function return, allocate backing storage (for example with `mem::alloc`) and return that pointer-backed value.

## Imports and modules

- `import "module.coral";` compiles and links Coral source.
- `import "module.h";` imports declarations and links a matching precompiled `.bin`.
- Stdlib modules are in `src/std/`.

## Full stdlib reference

### `serial` (`src/std/serial.h`)

| Function | Description |
| --- | --- |
| `fn write(message: [char]) -> void` | Write a null-terminated string to serial output. |
| `fn writeNum(value: i32) -> void` | Write an integer value to serial output. |

### `clock` (`src/std/clock.h`)

| Function | Description |
| --- | --- |
| `fn read() -> i32` | Read the current clock tick/time value from the clock device. |

### `audio` (`src/std/audio.h`)

| Function | Description |
| --- | --- |
| `fn pause() -> void` | Pause audio playback. |
| `fn unpause() -> void` | Resume audio playback. |
| `fn volume(channel: i16, newVolume: f32) -> void` | Set volume for a channel. |
| `fn pan(channel: i16, left: f32, right: f32) -> void` | Set left/right panning for a channel. |
| `fn frequency(channel: i16, newFrequency: f32) -> void` | Set playback frequency for a channel. |
| `fn masterVolume(newVolume: i32) -> void` | Set master output volume. |
| `fn loadSound(channel: i16, sample: [f32], len: i32) -> void` | Load sample data into a channel. |

### `disk` (`src/std/disk.h`)

| Function | Description |
| --- | --- |
| `fn read(section: i16, addr: i32, len: i16, dest: [i16]) -> void` | Read bytes/words from a disk section into destination memory. |
| `fn write(section: i16, addr: i32, byte: i16) -> void` | Write one value at an address in a disk section. |
| `fn loadSectors(start: i16, count: i16, dest: i32) -> void` | Bulk-load sectors into memory. |
| `fn linkedFileStart() -> i16` | Get the first linked data section after executable data. |

### `gfx` (`src/std/gfx.h`)

#### Types

- `enum Transform { Flat, SingleMatrixAffine, MultiMatrixAffine }`
- `struct Sprite`
  - `id: i16`
  - `x: i16`
  - `y: i16`
  - `priority: i16`
  - `tilemap_height: i16`
  - `tilemap_width: i16`
  - `tilemap: [i16]`
- `struct Layer`
  - `id: i16`
  - `x: i16`
  - `y: i16`
  - `tilemap_height: i16`
  - `tilemap_width: i16`
  - `tilemap: [i16]`
  - `transform: Transform`
  - `transformInfo: [f32]`

#### Functions

| Function | Description |
| --- | --- |
| `fn registerAtlas(atlas: [i16]) -> void` | Register a tile atlas. |
| `fn registerSprite(sprite: Sprite) -> void` | Register/update a sprite. |
| `fn registerLayer(layer: Layer) -> void` | Register/update a layer. |
| `fn render() -> void` | Render the current frame. |
| `fn pullControls(writeLoc: [bool; 11]) -> void` | Read controller state into a bool array. |
| `fn setPixel(x: i16, y: i16, color: i32) -> void` | Set a pixel color. |
| `fn getPixel(x: i16, y: i16) -> i32` | Read a pixel color. |

### `mem_core` (`src/std/mem_core.h`)

| Function | Description |
| --- | --- |
| `fn read_mem(addr: i32) -> i16` | Read a 16-bit value from memory. |
| `fn read_mem_ex(addr: i32) -> i32` | Read a 32-bit value from memory. |
| `fn write_mem(addr: i32, val: i16) -> void` | Write a 16-bit value to memory. |
| `fn write_mem_ex(addr: i32, val: i32) -> void` | Write a 32-bit value to memory. |
| `fn heap_start() -> i32` | Get the heap base address used by `mem`. |

### `mem` (`src/std/mem.coral`)

| Function | Description |
| --- | --- |
| `fn create_entry(ptr: i32, size: i32, free: bool, prev: i32, next: bool) -> void` | Internal allocator helper for creating heap metadata entries. |
| `fn init() -> void` | Initialize allocator state. Call once before allocation. |
| `fn alloc(size: i32) -> &void` | Allocate heap memory and return a pointer. |
| `fn free(ptr: &void) -> void` | Free a previously allocated pointer. |
| `fn copy(buf1: &void, buf2: &void, len: i32, offset: i32) -> void` | Copy `len` words from `buf1` to `buf2` at `offset`. |
| `fn compare(buf1: &void, buf2: &void, len: i32) -> bool` | Compare two buffers over `len` words. |

### `string` (`src/std/string.coral`)

| Function | Description |
| --- | --- |
| `fn len(str: [char]) -> i32` | Return string length (up to null terminator). |
| `fn append(str1: [char], str2: [char]) -> [char]` | Allocate and return concatenated string. |

## Minimal heap example

```rust
import "../src/std/mem.coral";
import "../src/std/serial.h";

fn main() -> void {
    mem::init();
    let buf: [i16; 16] = mem::alloc(16 as i32) as [i16; 16];
    buf[0] = 123;
    serial::writeNum(buf[0] as i32);
    mem::free(buf as &void);
    return;
}
```
