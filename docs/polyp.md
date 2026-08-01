# Polyp assembly language guide

Polyp is the assembly language used by the Corallium assembler (`corallium assemble`). It targets the Corallium ISA directly and is useful for low-level code.

## Files and build

- File extension: `.polyp`
- Assemble to a cartridge:

```bash
corallium assemble --file path/to/program.polyp
```

- Run the resulting cartridge:

```bash
corallium bytecode --file path/to/program.cart
```

- Build a library instead of a cartridge:

```bash
corallium assemble --file path/to/program.polyp --lib
```

## Lexical rules

- Whitespace is ignored.
- Line comments use `//`.
- Statements end with `;` except for function and block bodies.

## Program structure

Polyp files are made of imports, global data, and function definitions:

```polyp
import "path/to/module.h";

global message = "Hello!\n";

fn main()->0 {
    // ...
}
```

### Imports

`import "path";` links the referenced module, the same as Coral:

- `*.h` loads a precompiled `.bin` library
- `*.coral` compiles a Coral module and links it
- `*.polyp` does the same for other Polyp files

### Functions

```polyp
fn name(arg1: 2, arg2: 1) -> 0 {
    // statements
}
```

- Argument sizes are in bytes.
- The return value is the number of bytes returned on the stack, eg pushing an i16 is 1 byte, i32 is 2 byets, so on and so forth.

### Blocks (labels)

Blocks create jump targets. They are defined as `label: { ... }` inside a function.

```polyp
loop: {
    // ...
    jmp loop;
}
```

Block labels can be referenced before they are defined inside the same function.

### Symbols (stack locals)

```
symbol counter: 1;
```

`symbol` reserves stack space in the current function. The symbol name evaluates to an offset (in bytes) from `arp` (the active frame pointer). To access the actual memory location, add the symbol offset to `arp` and then `load`/`store` through that address.

### Globals (constant data)

```
global msg = "Hi!\n";
global table: 4 = { 1, 2, 3, 4 };
global reserved: 8;
```

- Globals are stored in the executable data section.
- String literals are encoded as a null-terminated byte array.
- Array literals (`{ ... }`) are only valid in globals.

## Values

Operands are untyped in source, but the instruction dictates how they are interpreted.

Supported value forms:

- Decimal numbers (`123`)
- Hex numbers (`0x1f`)
- 32-bit integers via `_i32` suffix (`70000_i32`, `0x1ffff_i32`)
- Registers (`r1`, `ex1`, `f1`, `arp`, etc.)
- Identifiers (symbols, globals, arguments, block labels, functions)

Negative literals are supported with a leading `-` (for example `-1`, `-0x20`, `-30000_i32`).

## Registers

| Register | Notes |
| --- | --- |
| `r1`..`r5` | 16-bit integer registers |
| `f1`, `f2` | 32-bit float registers |
| `ex1`, `ex2` | 32-bit integer registers |
| `ip` | instruction pointer |
| `sp` | stack pointer (raw) |
| `srp` | stack read pointer |
| `arp` | active frame pointer |

## Instruction reference

Operands below use:

- `value` = immediate or register value (i16)
- `valueEx` = immediate or register value (i32)
- `addr` = memory address (i16) or register holding an address
- `addrEx` = memory address (i32) or register holding an address

### Arithmetic and logic (16-bit, result in `r1`)

| Instruction | Args | Effect |
| --- | --- | --- |
| `add` | `value, value` | `r1 = a + b` |
| `sub` | `value, value` | `r1 = a - b` |
| `mul` | `value, value` | `r1 = a * b` |
| `div` | `value, value` | `r1 = a / b` |
| `mod` | `value, value` | `r1 = a % b` |
| `and` | `value, value` | `r1 = a & b` |
| `or` | `value, value` | `r1 = a \| b` |
| `xor` | `value, value` | `r1 = a ^ b` |
| `not` | `value` | `r1 = ~a` |
| `shl` | `value, value` | `r1 = a << b` |
| `shr` | `value, value` | `r1 = a >> b` |

### Arithmetic and logic (32-bit, result in `ex1`)

| Instruction | Args | Effect |
| --- | --- | --- |
| `addEx` | `valueEx, valueEx` | `ex1 = a + b` |
| `subEx` | `valueEx, valueEx` | `ex1 = a - b` |
| `mulEx` | `valueEx, valueEx` | `ex1 = a * b` |
| `divEx` | `valueEx, valueEx` | `ex1 = a / b` |
| `andEx` | `valueEx, valueEx` | `ex1 = a & b` |
| `orEx` | `valueEx, valueEx` | `ex1 = a \| b` |
| `xorEx` | `valueEx, valueEx` | `ex1 = a ^ b` |
| `notEx` | `valueEx` | `ex1 = ~a` |
| `shlEx` | `valueEx, valueEx` | `ex1 = a << b` |
| `shrEx` | `valueEx, valueEx` | `ex1 = a >> b` |

Unsigned variants write to `r1` or `ex1`:

`addU`, `subU`, `mulU`, `divU`, `addExU`, `subExU`, `mulExU`, `divExU`

### Float math (result in `f1`)

| Instruction | Args | Effect |
| --- | --- | --- |
| `addf` | `value, value` | `f1 = a + b` |
| `subf` | `value, value` | `f1 = a - b` |
| `mulf` | `value, value` | `f1 = a * b` |
| `divf` | `value, value` | `f1 = a / b` |

### Comparisons (result in `r1`, 0/1)

| Instruction | Args | Effect |
| --- | --- | --- |
| `gt` | `value, value` | `r1 = a > b` |
| `lt` | `value, value` | `r1 = a < b` |
| `eq` | `value, value` | `r1 = a == b` |

### Stack and register ops

| Instruction | Args | Effect |
| --- | --- | --- |
| `push` | `value` | push 16-bit value |
| `pushEx` | `valueEx` | push 32-bit value |
| `pushf` | `value` | push 32-bit float |
| `pop` | `register` | pop into register |
| `mov` | `value, register` | set register |

### Memory ops

| Instruction | Args | Effect |
| --- | --- | --- |
| `load` | `addr, register` | read i16 into register |
| `loadEx` | `addrEx, register` | read i32 into register |
| `loadf` | `addrEx, register` | read f32 into register |
| `store` | `addr, value` | write i16 to memory |
| `storeEx` | `addrEx, valueEx` | write i32 to memory |
| `storef` | `addrEx, value` | write f32 to memory |

### Control flow and calls

| Instruction | Args | Effect |
| --- | --- | --- |
| `jmp` | `label` or `addr` | jump unconditionally |
| `jnz` | `label` or `addr`, `value` | jump if value != 0 |
| `jz` | `label` or `addr`, `value` | jump if value == 0 |
| `call` | `function` | call function by name |
| `ret` | `returnedBytes, symbolBytes, argBytes` | return to caller |
| `exit` | (none) | stop the VM |
| `nop` | (none) | no-op |

`ret` expects:

- `returnedBytes`: number of bytes left on the stack as return value
- `symbolBytes`: total size of local symbols in this frame
- `argBytes`: total size of arguments pushed for this call

### I/O

| Instruction | Args | Effect |
| --- | --- | --- |
| `io` | `deviceId, commandId` | call device driver (args on stack) |

`io` invokes a VM device driver. Drivers pop arguments from the stack, so the **first argument is the last value you pushed**. Push args right-to-left (last parameter first). Return values, when present, are pushed onto the stack.

#### Device 0: Disk

| Command | Signature | Stack push order | Returns | Notes |
| --- | --- | --- | --- | --- |
| `0` | `read(section: i16, addr: i32, len: i32, dest: i32)` | `dest, len, addr, section` | — | Copies `len` words from disk section `section` at `addr` into memory at `dest`. |
| `1` | `write(section: i16, addr: i32, len: i32, buf: i32)` | `buf, len, addr, section` | — | Writes `len` words from memory at `buf` into disk section `section` at `addr`. |
| `2` | `loadSectors(start: i16, count: i16, dest: i32)` | `dest, count, start` | — | Loads raw disk sectors `[start, start+count)` into memory at `dest`. |
| `3` | `sectorCount() -> i16` | — | pushes `i16` | Total number of disk sections. |

#### Device 1: Audio

| Command | Signature | Stack push order | Returns | Notes |
| --- | --- | --- | --- | --- |
| `0` | `pause()` | — | — | Mutes all channels. |
| `1` | `unpause()` | — | — | Restores master volume. |
| `2` | `volume(channel: i16, newVolume: f32)` | `newVolume, channel` | — | Per-channel volume. |
| `3` | `pan(channel: i16, left: f32, right: f32)` | `right, left, channel` | — | Stereo panning. |
| `4` | `frequency(channel: i16, newFrequency: f32)` | `newFrequency, channel` | — | Playback frequency. |
| `5` | `masterVolume(newVolume: i32)` | `newVolume` | — | Master volume (0-100). |
| `6` | `loadSound(channel: i16, samplePtr: i32, len: i32)` | `len, samplePtr, channel` | — | `len` is **count of i16 words** (2 per f32 sample). |
| `7` | `setLoop(channel: i16, enabled: bool)` | `enabled, channel` | — | `enabled` is non-zero/zero. |

#### Device 2: Clock

| Command | Signature | Stack push order | Returns | Notes |
| --- | --- | --- | --- | --- |
| `0` | `read() -> i32` | — | pushes `i32` | Seconds since VM start. |

#### Device 3: Graphics

| Command | Signature | Stack push order | Returns | Notes |
| --- | --- | --- | --- | --- |
| `0` | `registerAtlas(atlasPtr: i32)` | `atlasPtr` | — | Sets active atlas pointer. |
| `1` | `registerLayer(layerPtr: i32)` | `layerPtr` | — | Adds a background layer. |
| `2` | `registerSprite(spritePtr: i32)` | `spritePtr` | — | Adds a sprite. |
| `3` | `render()` | — | — | Renders layers/sprites/bitmaps. |
| `4` | `pullControls(writePtr: i32)` | `writePtr` | — | Writes 11 booleans to memory (see layout below). |
| `5` | `setPixel(x: i16, y: i16, color: i32)` | `color, y, x` | — | Sets framebuffer pixel. |
| `6` | `getPixel(x: i16, y: i16) -> i32` | `y, x` | pushes `i32` | Reads framebuffer pixel. |
| `7` | `removeSprite(spritePtr: i32)` | `spritePtr` | — | Stops rendering a sprite pointer. |
| `8` | `removeLayer(layerPtr: i32)` | `layerPtr` | — | Stops rendering a layer pointer. |
| `9` | `registerBitmap(bitmapPtr: i32)` | `bitmapPtr` | — | Adds a bitmap pointer. Negative bitmap priority draws below sprites; non-negative draws above. |
| `10` | `removeBitmap(bitmapPtr: i32)` | `bitmapPtr` | — | Stops rendering a bitmap pointer. |
| `11` | `deltaTime() -> i32` | — | pushes `i32` | Milliseconds since last frame. |

**Graphics data layouts (memory):**

```
Atlas:
  i16 len
  [u32; len * 64] tiles   // each tile is 8x8 pixels

Sprite:
  i16 id
  i16 x
  i16 y
  i16 priority
  i16 tilemap_height
  i16 tilemap_width
  i32 tilemap_ptr         // pointer to i16 tile IDs
  f32 scale_x
  f32 scale_y

Layer:
  i16 id
  i16 xOffset
  i16 yOffset
  i16 tilemap_height
  i16 tilemap_width
  i32 tilemap_ptr         // pointer to i16 tile IDs
  i16 transform_type      // 0=Regular, 1=SingleMatrixAffine, 2=MultiMatrixAffine
  i32 transform_opt_ptr   // pointer to option payload
  i32 loc_opt_ptr         // pointer to option payload

Bitmap:
  i16 x
  i16 y
  i16 priority
  i16 length
  i16 width
  i32 data_ptr            // pointer to [i32] pixels (RGBA)

Controls (pullControls writes 11 bytes):
  [A, B, X, Y, Left, Right, Up, Down, Start, LTrigger, RTrigger]
```

For transform types 1 and 2, `transform_opt_ptr` and `loc_opt_ptr` should point to an option wrapper that contains a pointer to the actual data.

#### Device 4: Serial

| Command | Signature | Stack push order | Returns | Notes |
| --- | --- | --- | --- | --- |
| `0` | `write(ptr: i32)` | `ptr` | — | `ptr` points to a null-terminated byte string. |
| `1` | `writeNum(value: i32)` | `value` | — | Prints integer with newline. |
| `2` | `writeFloat(value: f32)` | `value` | — | Prints float with newline. |

## Example

```polyp
global msg = "Hello!\n";

fn main()->0 {
    symbol i: 1;

    // i = 0
    addEx i, arp;
    store ex1, 0;

    loop: {
        pushEx msg;
        io 4, 0;      // serial::write

        addEx i, arp;
        load ex1, r1;
        add r1, 1;
        store ex1, r1;

        jmp loop;
    }
}
```
