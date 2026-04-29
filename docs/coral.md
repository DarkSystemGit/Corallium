# Coral language guide

Coral is a C-like language with Rust-inspired syntax for the Corallium VM.

## Installation and setup

### Requirements

- **Rust** (with Cargo) - The Coral compiler and VM are written in Rust. Install from [rustup.rs](https://rustup.rs/)
- **Desktop environment** - The VM opens a graphics window and initializes audio output, so a display server is required
- **X11 or XWayland** - On Linux, X11 or XWayland support is needed; native Wayland is not yet supported due to lack of window decorations

### Build and install

Clone the repository:

```bash
git clone https://github.com/DarkSystemGit/micro-16.git
cd micro-16
```

Run the appropriate install script for your OS:

**Linux and macOS:**
```bash
./install.sh
```

This script:
- Builds the project in release mode
- Detects your OS (Linux or macOS)
- Installs the `corallium` binary and standard library to the appropriate location:
  - **Linux**: Binary to `/bin/corallium`, stdlib to `/opt/Corallium/src/std`
  - **macOS**: Binary to `/usr/local/bin/corallium`, stdlib to `/usr/local/opt/Corallium/src/std`

**Windows:**
```cmd
install.bat
```

This script:
- Builds the project in release mode
- Installs the `corallium` binary and standard library to `C:\Program Files\Corallium`
- Displays instructions for adding the directory to your PATH

After installation, you can run Coral programs from anywhere:

```bash
corallium run --file myprogram.coral
```

### Manual build without installation

If you prefer not to use the install script, you can build and run directly:

```bash
cargo build --release
./target/release/Corallium run --file test/importTest.coral
```

Or via Cargo (slower, but no build step needed):

```bash
cargo run --release -- run --file test/importTest.coral
```

### Custom stdlib location

If the stdlib is not in the default location, use the `--std` flag to specify its path:

```bash
corallium run --file myprogram.coral --std /path/to/stdlib
```

## Quick start

Run a Coral file directly:

```bash
cargo run -- run --file test/importTest.coral
```

Compile a Coral file to a `.cart` cartridge:

```bash
cargo run -- compile --file test/importTest.coral
```

Run a compiled cartridge:

```bash
cargo run -- bytecode --file test/importTest.cart
```

Link extra data files or directories into a cartridge:

```bash
cargo run -- compile --file test/importTest.coral --link path/to/file path/to/dir
```

Enable debug output during execution:

```bash
cargo run -- run --file test/importTest.coral --debug
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

Use `and` / `or` for short-circuit boolean logic. `&` / `|` are non-short-circuit bitwise/logical ops.

### Comments

```rust
// Single-line comment
let x: i16 = 5;  // inline comment

/* Multi-line comment
   can span multiple
   lines */
```

### Type casting

Type casts use the `as` keyword to convert between types:

```rust
let a: i16 = 10;
let b: i32 = a as i32;  // Convert i16 to i32
let c: i16 = b as i16;  // Convert i32 back to i16
let f: f32 = 3.14;
let i: i16 = f as i16;  // Convert float to int (truncates)
```

Casts are explicit and required; implicit conversions are not permitted.

### Pointers

Pointers are created with the `&` operator and dereference through function calls or field access:

```rust
let value: i32 = 42;
let ptr: &i32 = &value;  // Address of value

// Pointers are commonly used for:
mem::free(ptr);           // Freeing allocated memory
disk::read(section, addr, len, ptr as &void);  // Passing to device functions
```

Pointer-backed types (arrays, structs, unions) are managed automatically during pointer operations.

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

## Type system

### Data types

Coral supports the following primitive types:

| Type | Range | Notes |
| --- | --- | --- |
| `i16` | -32,768 to 32,767 | 16-bit signed integer |
| `i32` | -2,147,483,648 to 2,147,483,647 | 32-bit signed integer |
| `f32` | IEEE 754 single-precision | 32-bit floating-point |
| `bool` | `true` or `false` | Boolean value |
| `char` | UTF-8 code point | Single character |

Aggregate types and arrays are discussed below.

### Type behavior

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
| `fn setLoop(channel: i16, enabled: bool) -> void` | Enable or disable looping for a sample channel. |

#### How audio works

The audio system plays sounds through **channels**, each independently controlled with their own playback parameters. Audio is produced at a 32 kHz sample rate, output as stereo (left and right speakers).

**Channel types:**

The system provides 10 channels with different capabilities:
- 4 **square wave** channels: Generate procedural square wave tones (useful for synthesized bleeps/bloops).
- 2 **triangle wave** channels: Generate procedural triangle wave tones (smoother than square).
- 2 **sawtooth wave** channels: Generate procedural sawtooth wave tones (bright, harsh tone).
- 2 **sample playback** channels: Play arbitrary sound sample data (for music, voice, sound effects).

**Sound data and samples:**

- Sound is represented as a stream of floating-point numbers (`[f32]`), each between `-1.0` and `1.0`.
- Each number is one **sample**—a snapshot of the audio waveform at one point in time.
- At 32 kHz, 32,000 samples play back in one second.
- Mono audio uses a single sample stream; stereo audio mixes left and right channels together.

**Playing synthesized waves:**

Procedural channels (square, triangle, sawtooth) generate tones based on frequency:
- `frequency(channel, freq)` sets the tone pitch.
- The waveform shape is determined by the channel type (square produces a harsh tone, triangle is smoother, sawtooth is bright).
- The wave continuously loops; use `pause()` and `unpause()` to stop/resume all channels.

**Playing sample data:**

Sample channels play loaded sound data:
- `loadSound(channel, sample, len)` loads a `[f32]` array into a channel and immediately begins playback.
- Playback starts from index 0 and advances one sample per output sample.
- `setLoop(channel, enabled)` controls whether the sample loops back to index 0 after reaching the end.
- `frequency(channel, speed)` adjusts playback speed (e.g., `2.0` plays at 2× speed, `0.5` at half speed).

**Volume and mixing:**

- Each channel has independent volume control.
- `volume(channel, level)` sets a channel's amplitude (range typically `0.0` to `1.0`).
- `pan(channel, left, right)` routes the channel to speakers: `pan(0, 1.0, 0.0)` sends fully left, `pan(0, 0.5, 0.5)` sends to both equally.
- `masterVolume(level)` scales the entire output (e.g., `100` is full volume).
- All 10 channels mix together for final output, clamped to prevent clipping.

Example: Play a looping sound sample with manual controls

```rust
import <audio>;

fn main() -> void {
    let my_sound: [f32] = [/* your sample data here */];
    
    // Load sound into sample channel 0
    audio::loadSound(0, my_sound, 512);
    
    // Set volume and stereo pan
    audio::volume(0, 0.8);
    audio::pan(0, 0.5, 0.5);       // center the sound
    
    // Enable looping and play at normal speed
    audio::setLoop(0, true);
    audio::frequency(0, 1.0);
    
    return;
}
```

### `disk` (`src/std/disk.h`)

| Function | Description |
| --- | --- |
| `fn read(section: i16, addr: i32, len: i32, dest: &void) -> void` | Read values from a disk section into destination memory. |
| `fn write(section: i16, addr: i32, len: i32, buffer: &void) -> void` | Write `len` values from `buffer` into a disk section starting at `addr`. |
| `fn loadSectors(start: i16, count: i16, dest: i32) -> void` | Bulk-load sectors into memory. |
| `fn linkedFileStart() -> i16` | Get the first linked data section after executable data. |
| `fn sectorCount() -> i16` | Get the total number of sectors on disk. |

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
  - `scale_x: f32`
  - `scale_y: f32`
- `struct Layer`
  - `id: i16`
  - `x: i16`
  - `y: i16`
  - `tilemap_height: i16`
  - `tilemap_width: i16`
  - `tilemap: [i16]`
  - `transform: Transform`
  - `transformInfo: [f32]?`
  - `loc: [i32;2]?` (camera center for affine transforms)
- `struct Bitmap`
  - `length: i16`
  - `width: i16`
  - `data: [i32]`

#### Functions

| Function | Description |
| --- | --- |
| `fn registerAtlas(atlas: [i16]) -> void` | Register a tile atlas. |
| `fn registerSprite(sprite: Sprite) -> void` | Register/update a sprite. |
| `fn removeSprite(sprite: Sprite) -> void` | Unregister a sprite using the same sprite pointer used for registration. |
| `fn registerLayer(layer: Layer) -> void` | Register/update a layer. |
| `fn removeLayer(layer: Layer) -> void` | Unregister a layer using the same layer pointer used for registration. |
| `fn render() -> void` | Render the current frame. |
| `fn deltaTime() -> i32` | Milliseconds elapsed since the previous `render()` call. |
| `fn pullControls(writeLoc: [bool; 11]) -> void` | Read controller state into a bool array, order [A,B,X,Y,Left,Right,Up,Down,Start,LTrigger,RTrigger] |
| `fn setPixel(x: i16, y: i16, color: i32) -> void` | Set a pixel color. |
| `fn getPixel(x: i16, y: i16) -> i32` | Read a pixel color. |
| `fn registerBitmap(bitmap: Bitmap) -> void` | Register a bitmap pointer. Registered bitmaps are reloaded and drawn every `render()` (in registration order) with alpha transparency from the low byte (`0xRRGGBBAA`). |
| `fn removeBitmap(bitmap: Bitmap) -> void` | Unregister a previously registered bitmap pointer. |

`pullControls` keybinds:

| Control | Keyboard key |
| --- | --- |
| `A` | `A` |
| `B` | `S` |
| `X` | `D` |
| `Y` | `F` |
| `Left` | `Left Arrow` |
| `Right` | `Right Arrow` |
| `Up` | `Up Arrow` |
| `Down` | `Down Arrow` |
| `Start` | `Space` |
| `LTrigger` | `Q` |
| `RTrigger` | `E` |

#### How gfx works

Render flow:

1. `registerAtlas` uploads tile data.
2. `registerLayer` and `registerSprite` register/update objects by `id`; `removeLayer` and `removeSprite` unregister by pointer.
3. `registerBitmap` registers bitmap pointers for per-frame drawing, and `removeBitmap` unregisters them.
4. `setPixel` queues manual pixel writes.
5. `render()` draws layers, then sprites, then registered bitmaps, then queued pixel writes.

Layer transform modes:

- `Flat`: direct tilemap draw at `(x, y)`.
- `SingleMatrixAffine`: one 2x2 matrix for the whole frame.
- `MultiMatrixAffine`: one 2x2 matrix per screen row (scanline).

Sprite scaling:

- `scale_x` and `scale_y` are nearest-neighbor scale factors for sprite rendering.
- Use `1.0` for normal size, values above `1.0` to enlarge, and values between `0.0` and `1.0` to shrink.

Affine inputs:

- `transformInfo: [f32]?`
  - Single: `Some([m00, m01, m10, m11])`
  - Multi: `Some([...])` with 4 floats per scanline, in row order
- `loc: [i32;2]?`
  - `Some([cx, cy])` camera/sample center

Math used by affine modes:

For each destination pixel `(x_d, y_d)`, the renderer samples from source `(x_s, y_s)` using the inverse of the selected matrix:

GitHub uses a specific flavor of Markdown that supports LaTeX through the use of `$` for inline math and `$$` for block math. Here is the content ready to be pasted directly into your `README.md` or GitHub issue.

---

### Math used by Affine Modes

For each destination pixel $(x_d, y_d)$, the renderer samples from source $(x_s, y_s)$ using the inverse of the selected matrix:

#### Matrix Definition and Inversion

$$M = \begin{bmatrix} a & b \\ c & d \end{bmatrix}$$

$$\det(M) = ad - bc$$

$$M^{-1} = \frac{1}{\det(M)} \begin{bmatrix} d & -b \\ -c & a \end{bmatrix}$$

#### Coordinate Transformation

To calculate the source coordinates, we first find the relative distance from the screen center:
```math
r_x = x_d - \text{screen\_center}_x
r_y = y_d - \text{screen\_center}_y
```
Then, we apply the inverse matrix elements ($im_{nn}$) to map back to the source:


```math
\begin{bmatrix} x_s \\ y_s \end{bmatrix} = \begin{bmatrix} im_{00} & im_{01} \\ im_{10} & im_{11} \end{bmatrix} \begin{bmatrix} r_x \\ r_y \end{bmatrix} + \begin{bmatrix} \text{screen\_center}_x \\ \text{screen\_center}_y \end{bmatrix}
```
---

- `SingleMatrixAffine` uses one `M` for every scanline.
- `MultiMatrixAffine` picks `M[y_d]`, so each scanline can warp differently.

Notes:

- Affine modes require `transformInfo` and `loc` to be `Some(...)`.
- Because inverse mapping is used, scale feels inverted: `0.5` zooms in, `2.0` zooms out.

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

### `fs` (`src/std/fs.coral`)

High-level file system abstraction built on `disk`. Provides file I/O through `FileBuf` handles.

#### Types

- `struct FileBuf`
  - `id: i16` - Internal sector ID
  - `offset: i32` - Current file offset
  - `len: i32` - Total file size in bytes
  - `buf_offset: i32` - Current buffer memory offset
  - `buf_len: i32` - Size of loaded buffer
  - `buf: &void` - Pointer to loaded buffer data

- `struct FileList`
  - `buf: [[char]]` - Array of filenames
  - `len: i16` - Number of files

#### Functions

| Function | Description |
| --- | --- |
| `fn openFile(name: [char]) -> FileBuf` | Open a file by name and return a file handle. |
| `fn readFile(buf: FileBuf, offset: i32, len: i32?) -> void` | Read from a file at offset (optional len, defaults to full file). |
| `fn writeFile(buf: FileBuf) -> void` | Write the buffer contents back to the file. |
| `fn closeFile(buf: FileBuf) -> void` | Close a file handle and free its buffer. |
| `fn list_files() -> FileList` | List all files on disk. |
| `fn free_file_list(list: FileList) -> void` | Free a file list. |

### `conv` (`src/std/conv.coral`)

Low-level bit and byte manipulation utilities for converting between multi-word representations.

| Function | Description |
| --- | --- |
| `fn to_u16(word: i16) -> i32` | Convert a signed i16 to an unsigned 32-bit representation. |
| `fn i32_from_i16_parts(low: i16, high: i16) -> i32` | Combine two i16s (low and high) into one i32. |
| `fn i16_part_from_i32(value: i32, part: i16) -> i16` | Extract low (part=0) or high (part=1) i16 from an i32. |
| `fn set_i16_part_in_i32(value: i32, part: i16, part_value: i16) -> i32` | Replace low or high i16 in an i32. |
| `fn get_i8_from_i16(word: i16, byte_index: i32) -> i16` | Extract a byte (byte_index 0 or 1) from an i16. |
| `fn set_i8_in_i16(word: i16, byte_index: i32, value: i16) -> i16` | Set a byte in an i16. |

## Execution modes

### `run` - Direct execution

Compiles and runs a Coral file immediately:

```bash
cargo run -- run --file myprogram.coral
```

Useful for development and testing. Output appears in the console and graphics/audio play in real-time.

### `compile` - Build to cartridge

Compiles to a `.cart` (cartridge) file, which is a portable binary containing compiled bytecode and linked data:

```bash
cargo run -- compile --file myprogram.coral
```

The `.cart` format is optimized for distribution and re-execution without recompilation.

### `bytecode` - Execute cartridge

Runs a pre-compiled `.cart` file:

```bash
cargo run -- bytecode --file myprogram.cart
```

Cartridges execute faster than direct `run` since compilation is skipped.

### Linking data

Use `--link` to bundle extra files into a cartridge:

```bash
cargo run -- compile --file myprogram.coral --link assets/sprites.bin data/
```

Linked files are accessible through the `disk` module and file system APIs. Use `disk::linkedFileStart()` to find where linked data begins.

### Debug mode

Enable debug output to trace execution:

```bash
cargo run -- run --file myprogram.coral --debug
```

Debug output includes:
- Device I/O operations (serial writes, audio updates, disk reads)
- Memory operations (allocations, frees)
- Function calls and returns
- Bytecode instructions

Useful for diagnosing crashes or understanding program flow.

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
