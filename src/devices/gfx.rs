use crate::util::{convert_i16_to_i32, convert_i16_to_u32};
use crate::vm::{DataType, Machine, unpack_dt};
use crate::{devices::RawDevice, util::unpack_float};
use gamepads::Gamepads;
use pixelscreen::{Key, PixelFormat, Scale, Window};
use std::{cell::RefCell, process, rc::Rc, thread, time::Duration, time::Instant, vec};

pub fn driver(machine: &mut Machine, command: i16, device_id: usize) {
    let on_console = false;
    let menu_binary = "/home/main/corallium_config/target/release/corallium_config";
    //Types
    //struct Atlas{
    //  i16 len
    //  [u32*64; len] tiles
    //}
    //struct Tilemap{
    //  i16 tilemap_height
    //  i16 tilemap_width
    //  &[i16] tilemap
    //}
    //struct Sprite{
    //  i16 id
    //  i16 x
    //  i16 y
    //  u8 priority
    //  Tilemap tilemap
    //}
    //enum LayerTransform{
    //  Regular=>0,
    //  SingleMatrixAffine=>1,
    //  MultiMatrixAffine=>2
    //}
    //type Matrix:([f32;4],Point);
    //type Point:[i16;2]
    //struct Layer{
    //  i16 id
    //  i16 xOffset
    //  i16 yOffset
    //  Tilemap tilemap
    //  LayerTransform transform
    //  enum(&Matrix,&[Matrix],NULL) transformData
    //}
    //struct Bitmap{
    //  i16 x
    //  i16 y
    //  i16 priority
    //  i16 length
    //  i16 width
    //  *[i32] data
    //}
    //type NULL:u32=&0
    //type Controls: [bool]=[A,B,X,Y,Left,Right,Up,Down,Start,LTrigger,RTrigger]
    match command {
        0 => {
            //registerAtlas(&Atlas)
            // Sets the ptr to the atlas of the graphics system
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let gs: &mut GraphicsSystem =
                (if let RawDevice::Graphics(gs) = &mut machine.devices[device_id].contents {
                    Some(gs)
                } else {
                    None
                })
                .expect("Couldn't get graphics system");
            gs.ptrs.atlas = ptr;
            if machine.debug {
                println!("IO.gfx.registerAtlas %{}", ptr);
            }
        }
        1 => {
            //registerLayerPtr(&Layer)
            //Sets the ptr to a layer
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let gs: &mut GraphicsSystem =
                (if let RawDevice::Graphics(gs) = &mut machine.devices[device_id].contents {
                    Some(gs)
                } else {
                    None
                })
                .expect("Couldn't get graphics system");
            if !gs.ptrs.layers.contains(&ptr) {
                gs.ptrs.layers.push(ptr);
            }
            if machine.debug {
                println!("IO.gfx.registerLayer %{}", ptr);
            }
        }
        2 => {
            //registerSprite(&Sprite)
            //Adds a sprite to be rendered
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let gs: &mut GraphicsSystem =
                (if let RawDevice::Graphics(gs) = &mut machine.devices[device_id].contents {
                    Some(gs)
                } else {
                    None
                })
                .expect("Couldn't get graphics system");
            if !gs.ptrs.sprites.contains(&ptr) {
                gs.ptrs.sprites.push(ptr);
            }
            if machine.debug {
                println!("IO.gfx.registerSprite %{}", ptr);
            }
        }
        9 => {
            //registerBitmap(&Bitmap)
            //Sets the ptr to a bitmap
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let gs: &mut GraphicsSystem =
                (if let RawDevice::Graphics(gs) = &mut machine.devices[device_id].contents {
                    Some(gs)
                } else {
                    None
                })
                .expect("Couldn't get graphics system");
            if !gs.ptrs.bitmaps.contains(&ptr) {
                gs.ptrs.bitmaps.push(ptr);
            }
            if machine.debug {
                println!("IO.gfx.registerBitmap %{}", ptr);
            }
        }
        10 => {
            //removeBitmap(&Bitmap)
            //Stops rendering a bitmap pointer
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let gs = get_gs(machine, device_id);
            gs.ptrs.bitmaps.retain(|x| *x != ptr);
            if machine.debug {
                println!("IO.gfx.removeBitmap %{}", ptr);
            }
        }
        7 => {
            //removeSprite(&Sprite)
            //Stops rendering a sprite pointer and clears cached state for that id
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let id = usize::try_from(machine.memory.read(ptr, machine))
                .expect("Sprite id must be non-negative");
            let gs = get_gs(machine, device_id);
            gs.ptrs.sprites.retain(|x| *x != ptr);
            if let Some(sprite) = gs.sprites.1.get_mut(id) {
                *sprite = None;
            }
            if machine.debug {
                println!("IO.gfx.removeSprite %{}", ptr);
            }
        }
        8 => {
            //removeLayer(&Layer)
            //Stops rendering a layer pointer and clears cached state for that id
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let id = usize::try_from(machine.memory.read(ptr, machine))
                .expect("Layer id must be non-negative");
            let gs = get_gs(machine, device_id);
            gs.ptrs.layers.retain(|x| *x != ptr);
            if let Some(layer) = gs.background_layers.get_mut(id) {
                layer.clear();
                layer.offset = [0, 0];
                layer.render_type = RenderType::Regular;
            }
            if machine.debug {
                println!("IO.gfx.removeLayer %{}", ptr);
            }
        }
        3 => {
            //render()
            //render layers, sprites, and bitmaps
            let (atlas_ptr, sprite_ptrs, layer_ptrs, bitmap_ptrs, scanlines) = {
                let gs = get_gs(machine, device_id);
                (
                    gs.ptrs.atlas,
                    gs.ptrs.sprites.clone(),
                    gs.ptrs.layers.clone(),
                    gs.ptrs.bitmaps.clone(),
                    gs.display.height,
                )
            };
            get_gs(machine, device_id).clear_bitmaps();
            load_atlas(atlas_ptr, machine, device_id);
            for sp in sprite_ptrs {
                load_sprite(sp, machine, device_id);
            }
            for lp in layer_ptrs {
                load_layer(lp, machine, device_id, scanlines);
            }
            for bp in bitmap_ptrs {
                load_bitmap(bp, machine, device_id);
            }
            get_gs(machine, device_id).render();
            if !get_gs(machine, device_id).display.is_open() {
                machine.on = false;
            }
            if machine.debug {
                println!("IO.gfx.render");
            }
        }
        4 => {
            //pullControls(writeLoc)->Controls
            //writes the currently pressed controls to ptr, in (A,B,X,Y,Left,Right,Up,Down,Start,LTrigger,RTrigger) order.
            let ptr = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let gs: &mut GraphicsSystem =
                (if let RawDevice::Graphics(gs) = &mut machine.devices[device_id].contents {
                    Some(gs)
                } else {
                    None
                })
                .expect("Couldn't get graphics system");
            let mut rkeys = gs
                .display
                .pull_keys()
                .iter()
                .map(|x| map_key_to_control(*x))
                .flatten()
                .collect::<Vec<Controls>>();
            gs.gamepads.poll();
            for gamepad in gs.gamepads.all() {
                for button in gamepad.all_currently_pressed() {
                    match button {
                        gamepads::Button::ActionUp => rkeys.push(Controls::Y),
                        gamepads::Button::ActionDown => rkeys.push(Controls::A),
                        gamepads::Button::ActionLeft => rkeys.push(Controls::X),
                        gamepads::Button::ActionRight => rkeys.push(Controls::B),
                        gamepads::Button::FrontLeftLower => rkeys.push(Controls::LeftTrigger),
                        gamepads::Button::FrontRightLower => rkeys.push(Controls::RightTrigger),
                        gamepads::Button::RightCenterCluster => rkeys.push(Controls::Start),
                        gamepads::Button::DPadUp => rkeys.push(Controls::Up),
                        gamepads::Button::DPadDown => rkeys.push(Controls::Down),
                        gamepads::Button::DPadLeft => rkeys.push(Controls::Left),
                        gamepads::Button::DPadRight => rkeys.push(Controls::Right),
                        gamepads::Button::Mode => rkeys.push(Controls::Home),
                        _ => {}
                    }
                }
                let axis_threshold = 0.5;
                if (gamepad.left_stick_x() > axis_threshold) {
                    rkeys.push(Controls::Right);
                } else if (gamepad.left_stick_x() < -axis_threshold) {
                    rkeys.push(Controls::Left);
                }
                if (gamepad.left_stick_y() > axis_threshold) {
                    rkeys.push(Controls::Up);
                } else if (gamepad.left_stick_y() < -axis_threshold) {
                    rkeys.push(Controls::Down);
                }
                if (gamepad.right_stick_x() > axis_threshold) {
                    rkeys.push(Controls::Right);
                } else if (gamepad.right_stick_x() < -axis_threshold) {
                    rkeys.push(Controls::Left);
                }
                if (gamepad.right_stick_y() > axis_threshold) {
                    rkeys.push(Controls::Up);
                } else if (gamepad.right_stick_y() < -axis_threshold) {
                    rkeys.push(Controls::Down);
                }
            }
            let mut key_b = vec![0; 11];
            for i in rkeys {
                match i {
                    Controls::A => {
                        key_b[0] = 1;
                    }
                    Controls::B => {
                        key_b[1] = 1;
                    }
                    Controls::X => {
                        key_b[2] = 1;
                    }
                    Controls::Y => {
                        key_b[3] = 1;
                    }
                    Controls::Left => {
                        key_b[4] = 1;
                    }
                    Controls::Right => {
                        key_b[5] = 1;
                    }
                    Controls::Up => {
                        key_b[6] = 1;
                    }
                    Controls::Down => {
                        key_b[7] = 1;
                    }
                    Controls::Start => {
                        key_b[8] = 1;
                    }
                    Controls::LeftTrigger => {
                        key_b[9] = 1;
                    }
                    Controls::RightTrigger => {
                        key_b[10] = 1;
                    }
                    Controls::Home => {
                        if on_console {
                            let mut child = process::Command::new(menu_binary)
                                .spawn()
                                .expect("Failed to spawn menu process");
                            let status = child.wait().expect("Failed to wait on menu");
                            if let Some(4) = status.code() {
                                machine.reset = true;
                            }
                        }
                    }
                }
            }
            machine
                .memory
                .write_range(ptr..ptr + 11, key_b, &mut machine.core);
        }
        5 => {
            //setPixel(x,y,color)
            let x = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let y = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let color = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as u32;
            let gs = get_gs(machine, device_id);
            gs.set_pixel(x, y, color);
        }
        6 => {
            //getPixel(x,y)
            let x = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let y = unpack_dt(machine.core.stack.pop(&mut machine.core.srp)) as usize;
            let gs = get_gs(machine, device_id);
            let color = gs.get_pixel(x, y);
            machine
                .core
                .stack
                .push(DataType::Int32(color as i32), &mut machine.core.srp);
        }
        11 => {
            //deltaTime() -> i32
            let delta = get_gs(machine, device_id).delta_time_ms();
            machine
                .core
                .stack
                .push(DataType::Int32(delta), &mut machine.core.srp);
            if machine.debug {
                println!("IO.gfx.deltaTime -> {}", delta);
            }
        }
        _ => {}
    }
}
fn get_gs(machine: &mut Machine, device_id: usize) -> &mut GraphicsSystem {
    (if let RawDevice::Graphics(gs) = &mut machine.devices[device_id].contents {
        Some(gs)
    } else {
        None
    })
    .expect("Couldn't get graphics system")
}
fn load_atlas(ptr: usize, machine: &mut Machine, device_id: usize) {
    //[atlas]
    // i16 len
    // [u32*64; len] tiles
    let len = machine.memory.read(ptr, machine) as usize;
    let tiles = machine
        .memory
        .read_range(ptr + 1..(ptr + 1 + (2 * 64 * len)), machine)
        .chunks(2)
        .map(|c| convert_i16_to_u32(c).expect("Couldn't convert i16 to color"))
        .collect::<Vec<u32>>()
        .chunks(64)
        .map(|x| x.try_into().unwrap())
        .collect::<Vec<[u32; 64]>>();
    get_gs(machine, device_id).atlas.borrow_mut().tiles = tiles;
}
fn load_sprite(ptr: usize, machine: &mut Machine, device_id: usize) {
    //[sprite layout]
    // i16 id
    // i16 x
    // i16 y
    // u8 priority
    // i16 tilemap_height
    // i16 tilemap_width
    // *[i16] tilemap
    // f32 scale_x
    // f32 scale_y
    let rsprite = machine.memory.read_range(ptr..ptr + 12, machine);
    let sprite_id = usize::try_from(rsprite[0]).expect("Sprite id must be non-negative");
    let tilemapptr =
        convert_i16_to_u32(&[rsprite[6], rsprite[7]]).expect("Couldn't get tilemap ptr") as usize;
    let scale_x = unpack_float(&rsprite[8..10])
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 16.0)
        .unwrap_or(1.0);
    let scale_y = unpack_float(&rsprite[10..12])
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 16.0)
        .unwrap_or(1.0);
    let tiles = machine
        .memory
        .read_range(
            tilemapptr..(tilemapptr + (rsprite[4] * rsprite[5]) as usize),
            machine,
        )
        .iter()
        .map(|x| *x as usize)
        .collect();
    let gs: &mut GraphicsSystem = get_gs(machine, device_id);
    match gs.sprite_exists(sprite_id) {
        true => {
            let sprite = gs.get_sprite(sprite_id);
            sprite.loc = [rsprite[1] as i32, rsprite[2] as i32];
            sprite.priority = rsprite[3] as u8;
            sprite.tilemap.height = rsprite[4] as usize;
            sprite.tilemap.width = rsprite[5] as usize;
            sprite.tilemap.tiles = tiles;
            sprite.scale = [scale_x, scale_y];
        }
        false => {
            let mut tilemap = gs.get_tilemap(rsprite[5] as usize, rsprite[4] as usize);
            tilemap.tiles = tiles;
            let mut sprite = Sprite::new(
                tilemap,
                [rsprite[1] as i32, rsprite[2] as i32],
                rsprite[3] as u8,
                [scale_x, scale_y],
            );
            sprite.id = sprite_id;
            gs.sprites.1.resize(sprite_id + 1, None);
            gs.sprites.1[sprite_id] = Some(sprite);
        }
    }
}
fn load_layer(ptr: usize, machine: &mut Machine, device_id: usize, scanlines: usize) {
    //[BGLayer layout]
    // i16 id
    // i16 xOffset
    // i16 yOffset
    // i16 tilemap_height
    // i16 tilemap_width
    // *[i16] tilemap
    // u8 enum(0: Regular,1: SingleMatrixAffine,2: MultiMatrixAffine) transform
    // *[f32] transformData
    // *[i32;2] loc
    let rdata = machine.memory.read_range(ptr..ptr + 12, machine);
    let (
        id,
        off_x,
        off_y,
        tilemap_height,
        tilemap_width,
        tilemap_ptr,
        transform_type,
        transform_opt_ptr,
        loc_opt_ptr,
    ) = (
        rdata[0],
        rdata[1],
        rdata[2],
        rdata[3],
        rdata[4],
        convert_i16_to_u32(&[rdata[5], rdata[6]]).expect("Couldn't get tilemap") as usize,
        rdata[7],
        convert_i16_to_u32(&[rdata[8], rdata[9]]).expect("Couldn't get transform data") as usize,
        convert_i16_to_u32(&[rdata[10], rdata[11]]).expect("Couldn't get loc data") as usize,
    );
    let offset = [off_x as i32, off_y as i32];
    let render_type = match transform_type {
        0 => Some(RenderType::Regular),
        1 => {
            let transform_ptr = convert_i16_to_u32(
                &machine
                    .memory
                    .read_range(transform_opt_ptr..transform_opt_ptr + 2, machine),
            )
            .expect("Couldn't dereference transform data option")
                as usize;
            let loc_ptr = convert_i16_to_u32(
                &machine
                    .memory
                    .read_range(loc_opt_ptr..loc_opt_ptr + 2, machine),
            )
            .expect("Couldn't dereference loc option") as usize;
            let rmatrix = machine.memory.read_range(
                transform_ptr as usize..transform_ptr as usize + (2 * 4),
                machine,
            ); //4 f32s
            let matrix = rmatrix[0..(4 * 2)]
                .chunks(2)
                .map(|x| unpack_float(x).expect("Couldn't parse floats"))
                .collect::<Vec<f32>>();
            let loc = machine.memory.read_range(loc_ptr..loc_ptr + 4, machine);

            Some(RenderType::Matrix((
                [[matrix[0], matrix[1]], [matrix[2], matrix[3]]],
                [
                    convert_i16_to_i32(&loc[0..2]),
                    convert_i16_to_i32(&loc[2..4]),
                ],
            )))
        }
        2 => {
            let transform_ptr = convert_i16_to_u32(
                &machine
                    .memory
                    .read_range(transform_opt_ptr..transform_opt_ptr + 2, machine),
            )
            .expect("Couldn't dereference transform data option")
                as usize;
            let loc_ptr = convert_i16_to_u32(
                &machine
                    .memory
                    .read_range(loc_opt_ptr..loc_opt_ptr + 2, machine),
            )
            .expect("Couldn't dereference loc option") as usize;
            //matracies: [matrix; scanlines]; loc: [i32,i32]
            let rmatrix = machine.memory.read_range(
                transform_ptr as usize..transform_ptr as usize + (4 * 2) * scanlines,
                machine,
            );
            let matricies = rmatrix[0..(4 * 2) * scanlines]
                .chunks(2)
                .map(|x| unpack_float(x).expect("Couldn't parse floats"))
                .collect::<Vec<f32>>()
                .chunks(4)
                .map(|x| [[x[0] as f32, x[1] as f32], [x[2] as f32, x[3] as f32]])
                .collect::<Vec<Matrix>>();
            let loc = machine.memory.read_range(loc_ptr..loc_ptr + 4, machine);
            Some(RenderType::MultiMatrix((
                matricies,
                [
                    convert_i16_to_i32(&loc[0..2]),
                    convert_i16_to_i32(&loc[2..4]),
                ],
            )))
        }
        _ => None,
    }
    .expect("Couldn't determine rendertype");
    let tiles = machine
        .memory
        .read_range(
            tilemap_ptr..(tilemap_ptr + (tilemap_height * tilemap_width) as usize),
            machine,
        )
        .iter()
        .map(|x| *x as usize)
        .collect();
    let gs: &mut GraphicsSystem = get_gs(machine, device_id);
    let layer = &mut gs.background_layers[id as usize];
    layer.tilemap.height = tilemap_height as usize;
    layer.tilemap.width = tilemap_width as usize;
    layer.tilemap.tiles = tiles;
    layer.render_type = render_type;
    layer.offset = offset;
}
fn load_bitmap(ptr: usize, machine: &mut Machine, device_id: usize) {
    //[Bitmap layout]
    // i16 x
    // i16 y
    // i16 priority
    // i16 length
    // i16 width
    // *[i32] data
    let bitmap = machine.memory.read_range(ptr..ptr + 7, machine);
    let x = bitmap[0] as i32;
    let y = bitmap[1] as i32;
    let priority = bitmap[2];
    let length = usize::try_from(bitmap[3]).expect("Bitmap length must be non-negative");
    let width = usize::try_from(bitmap[4]).expect("Bitmap width must be non-negative");
    let data_ptr = convert_i16_to_u32(&bitmap[5..7]).expect("Couldn't get bitmap data") as usize;
    let pixel_count = width
        .checked_mul(length)
        .expect("Bitmap dimensions overflow");
    let data_words = pixel_count
        .checked_mul(2)
        .expect("Bitmap data words overflow");
    let data = machine
        .memory
        .read_range(data_ptr..data_ptr + data_words, machine)
        .chunks(2)
        .map(|chunk| convert_i16_to_u32(chunk).expect("Couldn't convert i16 to color"))
        .collect::<Vec<u32>>();
    get_gs(machine, device_id).add_bitmap(x, y, priority, length, width, data);
}
#[derive(Debug)]
struct BGLayer {
    tilemap: TileMap,
    offset: [i32; 2],
    render_type: RenderType,
}

impl BGLayer {
    fn new(tilemap: TileMap) -> BGLayer {
        BGLayer {
            tilemap,
            offset: [0, 0],
            render_type: RenderType::Regular,
        }
    }
    fn set_tile(&mut self, tileId: usize, loc: Point) {
        self.tilemap.set_tile(loc, tileId);
    }
    fn clear(&mut self) {
        self.tilemap.tiles.fill(0);
    }
    fn set_render_type(&mut self, render_type: RenderType) {
        self.render_type = render_type;
    }
    fn render(&mut self, buf: &mut Vec<u32>, buf_width: u32) {
        match &self.render_type {
            RenderType::Regular => {
                self.tilemap.render(self.offset, buf, buf_width);
            }
            RenderType::Matrix((matrix, cam)) => {
                let scanlines = buf.len() / buf_width as usize;
                self.tilemap.transform_render(
                    self.offset,
                    buf,
                    buf_width,
                    &vec![*matrix; scanlines],
                    *cam,
                );
            }
            RenderType::MultiMatrix((matrix, cam)) => {
                self.tilemap
                    .transform_render(self.offset, buf, buf_width, matrix, *cam);
            }
        }
    }
}
#[derive(Debug)]
enum RenderType {
    MultiMatrix((Vec<Matrix>, Point)),
    Matrix((Matrix, Point)),
    Regular,
}
pub type Matrix = [[f32; 2]; 2];
#[derive(Debug)]
pub struct GraphicsSystem {
    background_layers: Vec<BGLayer>,
    sprites: (Point, Vec<Option<Sprite>>),
    bitmaps: Vec<RegisteredBitmap>,
    atlas: Rc<RefCell<TileAtlas>>,
    pub display: Display,
    controls: Vec<Controls>,
    ptrs: GraphicsPtrs,
    queuedPixels: Vec<(usize, usize, u32)>,
    last_render_at: Option<Instant>,
    delta_time_ms: i32,
    min_frame_ms: i32,
    gamepads: debug_ignore::DebugIgnore<Gamepads>,
}
#[derive(Debug, Clone)]
struct RegisteredBitmap {
    x: i32,
    y: i32,
    priority: i16,
    length: usize,
    width: usize,
    data: Vec<u32>,
}
#[derive(Debug, Clone)]
struct GraphicsPtrs {
    sprites: Vec<usize>,
    layers: Vec<usize>,
    bitmaps: Vec<usize>,
    atlas: usize,
}
#[derive(Debug, PartialEq)]
enum Controls {
    A,
    B,
    X,
    Y,
    Left,
    Right,
    Up,
    Down,
    Start,
    LeftTrigger,
    RightTrigger,
    Home,
}

fn map_key_to_control(key: Key) -> Option<Controls> {
    match key {
        Key::KeyA => Some(Controls::A),
        Key::KeyS => Some(Controls::B),
        Key::KeyD => Some(Controls::X),
        Key::KeyF => Some(Controls::Y),
        Key::ArrowLeft => Some(Controls::Left),
        Key::ArrowRight => Some(Controls::Right),
        Key::ArrowUp => Some(Controls::Up),
        Key::ArrowDown => Some(Controls::Down),
        Key::Space => Some(Controls::Start),
        Key::KeyQ => Some(Controls::LeftTrigger),
        Key::KeyE => Some(Controls::RightTrigger),
        Key::Escape => Some(Controls::Home),
        _ => None,
    }
}
impl GraphicsSystem {
    /// Target frame rate: gates how often `render()` will actually pace
    /// out a call (see `render()`), and is also what used to be passed to
    /// minifb's `set_target_fps` (now unused by `Display` itself, but kept
    /// as the one source of truth for both).
    const TARGET_FPS: usize = 65;
    pub(crate) fn new_with_display(display: Display, resolution: [u32; 2]) -> GraphicsSystem {
        let mut gs = GraphicsSystem {
            background_layers: vec![],
            sprites: ([0, 0], Vec::new()),
            bitmaps: vec![],
            atlas: Rc::new(RefCell::new(TileAtlas::new())),
            gamepads: Gamepads::new().into(),
            display,
            controls: Vec::new(),
            ptrs: GraphicsPtrs {
                sprites: vec![],
                layers: vec![],
                bitmaps: vec![],
                atlas: 0,
            },
            queuedPixels: vec![],
            last_render_at: None,
            delta_time_ms: 0,
            min_frame_ms: (1000 / Self::TARGET_FPS.max(1)) as i32,
        };
        gs.background_layers.extend([
            BGLayer::new(TileMap::new(
                gs.atlas.clone(),
                (resolution[0] / 8) as usize,
                (resolution[1] / 8) as usize,
            )),
            BGLayer::new(TileMap::new(
                gs.atlas.clone(),
                (resolution[0] / 8) as usize,
                (resolution[1] / 8) as usize,
            )),
            BGLayer::new(TileMap::new(
                gs.atlas.clone(),
                (resolution[0] / 8) as usize,
                (resolution[1] / 8) as usize,
            )),
        ]);
        gs
    }
    pub fn reset_with_same_display(&mut self, resolution: [u32; 2]) -> GraphicsSystem {
        let old_display = std::mem::replace(
            &mut self.display,
            Display::new(
                resolution[0] as usize,
                resolution[1] as usize,
                "Corallium",
                Scale::FitScreen,
            ),
        );
        GraphicsSystem::new_with_display(old_display, resolution)
    }
    pub fn new(resolution: [u32; 2]) -> GraphicsSystem {
        let mut gs = GraphicsSystem {
            background_layers: vec![],
            sprites: ([0, 0], Vec::new()),
            bitmaps: vec![],
            atlas: Rc::new(RefCell::new(TileAtlas::new())),
            gamepads: Gamepads::new().into(),
            display: Display::new(
                resolution[0] as usize,
                resolution[1] as usize,
                "Corallium",
                Scale::FitScreen,
            ),
            controls: Vec::new(),
            ptrs: GraphicsPtrs {
                sprites: vec![],
                layers: vec![],
                bitmaps: vec![],
                atlas: 0,
            },
            queuedPixels: vec![],
            last_render_at: None,
            delta_time_ms: 0,
            min_frame_ms: (1000 / Self::TARGET_FPS.max(1)) as i32,
        };
        gs.background_layers.extend([
            BGLayer::new(TileMap::new(
                gs.atlas.clone(),
                (resolution[0] / 8) as usize,
                (resolution[1] / 8) as usize,
            )),
            BGLayer::new(TileMap::new(
                gs.atlas.clone(),
                (resolution[0] / 8) as usize,
                (resolution[1] / 8) as usize,
            )),
            BGLayer::new(TileMap::new(
                gs.atlas.clone(),
                (resolution[0] / 8) as usize,
                (resolution[1] / 8) as usize,
            )),
        ]);
        gs
    }
    pub fn get_tilemap(&mut self, width: usize, height: usize) -> TileMap {
        TileMap::new(self.atlas.clone(), width, height)
    }
    pub fn add_tile(&mut self, tile: Tile) {
        self.atlas.borrow_mut().add_tile(tile);
    }
    pub fn add_tile_with_id(&mut self, id: u8, tile: Tile) {
        if self.atlas.borrow_mut().tiles.len() <= id as usize {
            self.atlas
                .borrow_mut()
                .tiles
                .resize(id as usize + 1, [0; 64]);
        }
        self.atlas.borrow_mut().tiles[id as usize] = tile;
    }
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        self.queuedPixels.push((x, y, color));
    }
    pub fn clear_bitmaps(&mut self) {
        self.bitmaps.clear();
    }
    pub fn add_bitmap(
        &mut self,
        x: i32,
        y: i32,
        priority: i16,
        length: usize,
        width: usize,
        data: Vec<u32>,
    ) {
        self.bitmaps.push(RegisteredBitmap {
            x,
            y,
            priority,
            length,
            width,
            data,
        });
    }
    pub fn get_pixel(&mut self, x: usize, y: usize) -> u32 {
        self.display.buffer[y * self.display.width + x]
    }
    pub fn add_sprite(&mut self, mut sprite: Sprite) -> usize {
        sprite.id = self.sprites.1.len();
        self.sprites.1.push(Some(sprite));
        self.sprites.1.len() - 1
    }
    pub fn get_sprite(&mut self, id: usize) -> &mut Sprite {
        self.sprites.1[id].as_mut().expect("nonexistent sprite")
    }
    pub fn sprite_exists(&self, id: usize) -> bool {
        matches!(self.sprites.1.get(id), Some(Some(_)))
    }
    pub fn set_tile(&mut self, loc: Point, layer: u8, tile_id: usize) {
        self.background_layers[layer as usize]
            .tilemap
            .set_tile(loc, tile_id);
    }
    pub fn get_tile(&mut self, loc: Point, layer: u8) {
        self.background_layers[layer as usize].tilemap.get_tile(loc);
    }
    pub fn render(&mut self) {
        if let Some(last) = self.last_render_at {
            let elapsed_ms = last.elapsed().as_millis().min(i32::MAX as u128) as i32;
            if elapsed_ms < self.min_frame_ms {
                thread::sleep(Duration::from_millis(
                    (self.min_frame_ms - elapsed_ms) as u64,
                ));
            }
        }

        let now = Instant::now();
        self.delta_time_ms = self
            .last_render_at
            .map(|last| {
                let elapsed_ms = now.duration_since(last).as_millis();
                elapsed_ms.min(i32::MAX as u128) as i32
            })
            .unwrap_or(0);
        self.last_render_at = Some(now);

        self.display.clear();

        for layer in &mut self.background_layers {
            layer.render(&mut self.display.buffer, self.display.width as u32);
        }
        let mut bitmaps = self.bitmaps.clone();
        bitmaps.sort_by_key(|bitmap| bitmap.priority);
        for bitmap in bitmaps.iter().filter(|bitmap| bitmap.priority < 0) {
            render_bitmap(
                bitmap,
                &mut self.display.buffer,
                self.display.width,
                self.display.height,
            );
        }
        let mut sprites = self.sprites.1.clone();

        sprites.sort_by_key(|sprite| match sprite.is_some() {
            true => sprite.as_ref().unwrap().priority,
            false => 0,
        });
        for sprite in sprites {
            if let Some(sprite) = sprite {
                sprite.render(
                    self.sprites.0,
                    &mut self.display.buffer,
                    self.display.width as u32,
                );
            }
        }

        for bitmap in bitmaps.iter().filter(|bitmap| bitmap.priority >= 0) {
            render_bitmap(
                bitmap,
                &mut self.display.buffer,
                self.display.width,
                self.display.height,
            );
        }
        for pix in self.queuedPixels.drain(..) {
            self.display.buffer[pix.1 * self.display.width + pix.0] = pix.2;
        }

        // Pacing above already guarantees at least MIN_FRAME_MS has passed,
        // so every call now blits -- no more skip-vs-draw branch needed.
        self.display.render();

        self.controls.clear();
        for k in self.display.pull_keys() {
            if let Some(control) = map_key_to_control(k) {
                if !self.controls.contains(&control) {
                    self.controls.push(control);
                }
            }
        }
    }
    pub fn delta_time_ms(&self) -> i32 {
        self.delta_time_ms
    }
}

fn blend_over(base: u32, top: u32, top_alpha: u32) -> u32 {
    (0..4).fold(0, |acc, i| {
        let shift = i * 8;
        let base_ch = (base >> shift) & 0xff;
        let top_ch = (top >> shift) & 0xff;
        let out_ch = (base_ch * (255 - top_alpha) + top_ch * top_alpha) / 255;
        acc | (out_ch << shift)
    })
}

fn render_bitmap(
    bitmap: &RegisteredBitmap,
    buffer: &mut [u32],
    display_width: usize,
    display_height: usize,
) {
    for src_y in 0..bitmap.length {
        let dst_y = bitmap.y + src_y as i32;
        if dst_y < 0 || dst_y >= display_height as i32 {
            continue;
        }
        let src_row_start = src_y * bitmap.width;
        let dst_row_start = dst_y as usize * display_width;
        for src_x in 0..bitmap.width {
            let dst_x = bitmap.x + src_x as i32;
            if dst_x < 0 || dst_x >= display_width as i32 {
                continue;
            }
            let bitmap_pixel = bitmap.data[src_row_start + src_x];
            let alpha = bitmap_pixel & 0xff;
            if alpha == 0 {
                continue;
            }
            let dst_idx = dst_row_start + dst_x as usize;
            if alpha == 255 {
                buffer[dst_idx] = bitmap_pixel;
                continue;
            }
            let base_pixel = buffer[dst_idx];
            buffer[dst_idx] = blend_over(base_pixel, bitmap_pixel, alpha as u32);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub tilemap: TileMap,
    pub loc: Point,
    pub priority: u8,
    pub id: usize,
    pub scale: [f32; 2],
}
impl Sprite {
    pub fn new(tilemap: TileMap, loc: Point, priority: u8, scale: [f32; 2]) -> Sprite {
        Sprite {
            tilemap,
            loc,
            priority,
            id: 0,
            scale,
        }
    }
    fn render(&self, global_offset: Point, buf: &mut Vec<u32>, buf_width: u32) {
        self.tilemap.render_scaled(
            [
                self.loc[0] + global_offset[0],
                self.loc[1] + global_offset[1],
            ],
            buf,
            buf_width,
            self.scale,
        );
    }
}
#[derive(Debug)]
pub(crate) struct Display {
    width: usize,
    height: usize,
    buffer: Vec<u32>, //[[u32;width];height]
    window: Window,
}
type Tile = [u32; 64]; //8x8 row order
pub type Point = [i32; 2];

#[derive(Debug, Clone)]
pub struct TileMap {
    atlas: Rc<RefCell<TileAtlas>>,
    width: usize,
    height: usize,
    tiles: Vec<usize>,
}
impl TileMap {
    fn new(atlas: Rc<RefCell<TileAtlas>>, width: usize, height: usize) -> TileMap {
        TileMap {
            atlas: atlas.clone(),
            width,
            height,
            tiles: vec![0; width * height],
        }
    }
    fn set_tile(&mut self, loc: Point, tileId: usize) {
        self.tiles[(self.width) * loc[1] as usize + loc[0] as usize] = tileId;
    }
    fn get_tile(&self, loc: Point) -> usize {
        self.tiles[(self.width) * loc[1] as usize + loc[0] as usize]
    }
    fn render(&self, loc: Point, buf: &mut Vec<u32>, buf_width: u32) {
        for (i, tile) in self.tiles.iter().enumerate() {
            let y = ((i / (self.width)) as i32 * 8) + loc[1];
            let x = ((i % (self.width)) as i32 * 8) + loc[0];
            self.atlas
                .borrow()
                .render_tile(*tile, [x, y], buf, buf_width);
        }
    }
    fn render_scaled(&self, loc: Point, buf: &mut Vec<u32>, buf_width: u32, scale: [f32; 2]) {
        let scale_x = if scale[0].is_finite() && scale[0] > 0.0 {
            scale[0]
        } else {
            1.0
        };
        let scale_y = if scale[1].is_finite() && scale[1] > 0.0 {
            scale[1]
        } else {
            1.0
        };
        if (scale_x - 1.0).abs() < f32::EPSILON && (scale_y - 1.0).abs() < f32::EPSILON {
            self.render(loc, buf, buf_width);
            return;
        }

        let src_width = (self.width * 8) as i32;
        let src_height = (self.height * 8) as i32;
        let mut src_buf = vec![0; (src_width * src_height) as usize];
        self.render([0, 0], &mut src_buf, src_width as u32);

        let dst_width = ((src_width as f32) * scale_x).round().max(1.0) as i32;
        let dst_height = ((src_height as f32) * scale_y).round().max(1.0) as i32;
        let buf_height = (buf.len() as u32 / buf_width) as i32;
        let buf_width_i32 = buf_width as i32;

        for y_dest in 0..dst_height {
            let out_y = loc[1] + y_dest;
            if out_y < 0 || out_y >= buf_height {
                continue;
            }
            let src_y = (y_dest as f32 / scale_y).floor() as i32;
            if src_y < 0 || src_y >= src_height {
                continue;
            }

            for x_dest in 0..dst_width {
                let out_x = loc[0] + x_dest;
                if out_x < 0 || out_x >= buf_width_i32 {
                    continue;
                }
                let src_x = (x_dest as f32 / scale_x).floor() as i32;
                if src_x < 0 || src_x >= src_width {
                    continue;
                }
                let pixel = src_buf[src_x as usize + src_y as usize * src_width as usize];
                if pixel != 0 {
                    buf[out_x as usize + out_y as usize * buf_width as usize] = pixel;
                }
            }
        }
    }
    //matrix: [[horizontal scale,horizontal rotation],[vertical rotation,vertical scale]]
    fn transform_render(
        &self,
        output_loc: Point,
        buf: &mut Vec<u32>,
        buf_width: u32,
        matrices: &Vec<Matrix>,
        cam_center: Point,
    ) {
        let buf_height = buf.len() as u32 / buf_width;
        let center_x = (buf_width / 2) as f32;
        let center_y = (buf_height / 2) as f32;

        // 1. Render the source texture once into a local scratchpad
        let mut src_buf = vec![0; buf.len()];
        self.render(cam_center, &mut src_buf, buf_width);

        // 2. Process each scanline
        for y_dest in 0..buf_height {
            let m = match matrices.get(y_dest as usize) {
                Some(m) => m,
                None => continue,
            };

            // Invert matrix for this row: M^-1 = (1/det) * [d, -b; -c, a]
            let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
            if det.abs() < f32::EPSILON {
                continue;
            }
            let inv_det = 1.0 / det;

            let im00 = m[1][1] * inv_det;
            let im01 = -m[0][1] * inv_det;
            let im10 = -m[1][0] * inv_det;
            let im11 = m[0][0] * inv_det;

            // Calculate target row bounds
            let out_y = y_dest as i32 + output_loc[1];
            if out_y < 0 || out_y >= buf_height as i32 {
                continue;
            }
            let out_row_idx = out_y as usize * buf_width as usize;

            // Relative Y coordinate once per row
            let ry = y_dest as f32 - center_y;

            // Setup starting source coordinates (x_dest = 0)
            let mut sx = im00 * (0.0 - center_x) + im01 * ry + center_x;
            let mut sy = im10 * (0.0 - center_x) + im11 * ry + center_y;

            for x_dest in 0..buf_width {
                let ix = sx as i32;
                let iy = sy as i32;

                // Sample source (check bounds)
                if ix >= 0 && ix < buf_width as i32 && iy >= 0 && iy < buf_height as i32 {
                    let pixel = src_buf[ix as usize + (iy as usize * buf_width as usize)];

                    if pixel != 0 {
                        let out_x = x_dest as i32 + output_loc[0];
                        if out_x >= 0 && out_x < buf_width as i32 {
                            buf[out_row_idx + out_x as usize] = pixel;
                        }
                    }
                }

                // Step source coordinates by the inverse matrix columns
                sx += im00;
                sy += im10;
            }
        }
    }
}
#[derive(Debug, Clone)]
struct TileAtlas {
    tiles: Vec<Tile>,
}
impl TileAtlas {
    fn new() -> TileAtlas {
        TileAtlas { tiles: Vec::new() }
    }
    fn _render_tile(&self, index: usize, loc: Point, buf: &mut Vec<u32>, buf_width: u32) {
        if index >= self.tiles.len() {
            return;
        }
        for (i, row) in self.tiles[index].chunks(8).enumerate() {
            let x = loc[0];
            let y = loc[1] + i as i32;
            let buf_height = buf.len() as i32 / buf_width as i32;
            if y >= 0 && y < buf_height {
                let buf_row_start = (y as usize) * buf_width as usize;
                let start_x = x.max(0) as usize;
                let end_x = (x + 8).min(buf_width as i32).max(0) as usize;
                if start_x < end_x {
                    let copy_len = end_x - start_x;
                    let buf_idx = buf_row_start + start_x;
                    let row_offset = (start_x as i32 - x) as usize;
                    for i in 0..copy_len {
                        if row[row_offset + i] != 0 {
                            buf[buf_idx + i] = row[row_offset + i];
                        }
                    }
                }
            }
        }
    }
    fn render_tile(&self, index: usize, loc: Point, buf: &mut Vec<u32>, buf_width: u32) {
        if index >= self.tiles.len() {
            return;
        }
        let tile = &self.tiles[index];
        let (target_x, target_y) = (loc[0], loc[1]);
        let buf_height = (buf.len() as u32 / buf_width) as i32;
        let start_row = (0).max(-target_y) as usize;
        let end_row = (8).min(buf_height - target_y).max(0) as usize;

        if start_row >= end_row {
            return;
        }
        let start_x = target_x.max(0);
        let end_x = (target_x + 8).min(buf_width as i32);

        if start_x >= end_x {
            return;
        }
        let copy_len = (end_x - start_x) as usize;
        let row_offset = (start_x - target_x) as usize;
        let start_x_usize = start_x as usize;
        let mut buf_idx =
            (target_y + start_row as i32) as usize * buf_width as usize + start_x_usize;
        for i in start_row..end_row {
            let tile_row_start = i * 8 + row_offset;
            for i in 0..copy_len {
                let tile_pixel = tile[tile_row_start + i];
                let alpha = tile_pixel & 0xff;
                if alpha == 0 {
                    continue;
                }
                let dst_idx = buf_idx + i;
                if alpha == 255 {
                    buf[dst_idx] = tile_pixel;
                    continue;
                }
                let base_pixel = buf[dst_idx];
                buf[dst_idx] = blend_over(base_pixel, tile_pixel, alpha);
            }
            //buf[buf_idx..buf_idx + copy_len].copy_from_slice(&tile[tile_row_start..tile_row_end]);
            buf_idx += buf_width as usize;
        }
    }
    fn add_tile(&mut self, tile: Tile) -> usize {
        self.tiles.push(tile);
        self.tiles.len() - 1
    }
}
impl Display {
    fn new(width: usize, height: usize, title: &str, scale: Scale) -> Self {
        let mut window = Window::new_with_scale(title, width as u32, height as u32, scale)
            .expect("Unable to open the window");
        window.set_pixel_format(PixelFormat::Rrggbbaa);
        window.set_resizable(true);
        Self {
            width,
            height,
            buffer: vec![0; width * height],
            window,
        }
    }
    fn render(&mut self) {
        if !self.window.is_open() {
            return;
        }
        self.window.update();
        self.window.render(&self.buffer).err();
    }
    fn pull_keys(&self) -> Vec<Key> {
        self.window.get_keys()
    }
    fn is_open(&self) -> bool {
        self.window.is_open()
    }
    fn clear(&mut self) {
        self.buffer.fill(0);
    }
    #[allow(dead_code)]
    fn update(&mut self) {
        self.window.update();
    }
}
