use crate::util::convert_i16_to_u32;
use crate::vm::{DataType, Machine, unpack_dt};
use crate::{devices::RawDevice, util::unpack_float};
use minifb::{self, Key, Scale, Window, WindowOptions};
use std::{cell::RefCell, rc::Rc, vec};
pub fn driver(machine: &mut Machine, command: i16, device_id: usize) {
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
        3 => {
            //render()
            //render layers & sprites
            let (atlas_ptr, sprite_ptrs, layer_ptrs, scanlines) = {
                let gs = get_gs(machine, device_id);
                (
                    gs.ptrs.atlas,
                    &gs.ptrs.sprites,
                    &gs.ptrs.layers,
                    gs.display.height,
                )
            };
            //borrow checker pleasing dance
            let spl = sprite_ptrs.len();
            let lpl = layer_ptrs.len();
            load_atlas(atlas_ptr, machine, device_id);
            for spc in 0..spl {
                let sp = get_gs(machine, device_id).ptrs.sprites[spc];
                load_sprite(sp, machine, device_id);
            }
            for lpc in 0..lpl {
                let lp = get_gs(machine, device_id).ptrs.layers[lpc];
                load_layer(lp, machine, device_id, scanlines);
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
            let rkeys = gs
                .display
                .pull_keys()
                .iter()
                .map(|x| map_key_to_control(*x))
                .flatten()
                .collect::<Vec<Controls>>();
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
    let rsprite = machine.memory.read_range(ptr..ptr + 8, machine);
    let tilemapptr =
        convert_i16_to_u32(&[rsprite[6], rsprite[7]]).expect("Couldn't get tilemap ptr") as usize;
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
    match gs.sprite_exists(rsprite[0] as u8) {
        true => {
            let sprite = gs.get_sprite(rsprite[0] as u8);
            sprite.loc = [rsprite[1] as i32, rsprite[2] as i32];
            sprite.priority = rsprite[3] as u8;
            sprite.tilemap.height = rsprite[4] as usize;
            sprite.tilemap.width = rsprite[5] as usize;
            sprite.tilemap.tiles = tiles;
        }
        false => {
            let mut tilemap = gs.get_tilemap(rsprite[5] as usize, rsprite[4] as usize);
            tilemap.tiles = tiles;
            let mut sprite = Sprite::new(
                tilemap,
                [rsprite[1] as i32, rsprite[2] as i32],
                rsprite[3] as u8,
            );
            sprite.id = rsprite[0] as u8;
            gs.sprites.1.resize(rsprite[0] as usize + 1, None);
            gs.sprites.1[rsprite[0] as usize] = Some(sprite);
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
    let rdata = machine.memory.read_range(ptr..ptr + 10, machine);
    let (
        id,
        off_x,
        off_y,
        tilemap_height,
        tilemap_width,
        tilemap_ptr,
        transform_type,
        transform_ptr,
    ) = (
        rdata[0],
        rdata[1],
        rdata[2],
        rdata[3],
        rdata[4],
        convert_i16_to_u32(&[rdata[5], rdata[6]]).expect("Couldn't get tilemap") as usize,
        rdata[7],
        convert_i16_to_u32(&[rdata[8], rdata[9]]).expect("Couldn't get transform data") as usize,
    );
    let offset = [off_x as i32, off_y as i32];
    let render_type = match transform_type {
        0 => Some(RenderType::Regular),
        1 => {
            let rmatrix = machine.memory.read_range(
                transform_ptr as usize..transform_ptr as usize + (2 * 4) + 2,
                machine,
            ); //4 f32s and 2 i16
            let matrix = rmatrix[0..(4 * 2)]
                .chunks(2)
                .map(|x| unpack_float(x).expect("Couldn't parse floats"))
                .collect::<Vec<f32>>();
            let loc = [rmatrix[8] as i32, rmatrix[9] as i32];
            Some(RenderType::Matrix((
                [[matrix[0], matrix[1]], [matrix[2], matrix[3]]],
                loc,
            )))
        }
        2 => {
            //matracies: [matrix; scanlines]; loc: [i16,i16]
            let rmatrix = machine.memory.read_range(
                transform_ptr as usize..transform_ptr as usize + (4 * 2) * scanlines + 2,
                machine,
            );
            let matricies = rmatrix[0..(4 * 2) * scanlines]
                .chunks(2)
                .map(|x| unpack_float(x).expect("Couldn't parse floats"))
                .collect::<Vec<f32>>()
                .chunks(4)
                .map(|x| [[x[0] as f32, x[1] as f32], [x[2] as f32, x[3] as f32]])
                .collect::<Vec<Matrix>>();
            let loc = [
                rmatrix[(4 * 2) * scanlines] as i32,
                rmatrix[(4 * 2) * scanlines + 1] as i32,
            ];
            Some(RenderType::MultiMatrix((matricies, loc)))
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
                self.tilemap.transform_render(
                    self.offset,
                    buf,
                    buf_width,
                    &vec![*matrix; self.tilemap.height * 8],
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
    atlas: Rc<RefCell<TileAtlas>>,
    display: Display,
    controls: Vec<Controls>,
    ptrs: GraphicsPtrs,
    queuedPixels: Vec<(usize, usize, u32)>,
}
#[derive(Debug, Clone)]
struct GraphicsPtrs {
    sprites: Vec<usize>,
    layers: Vec<usize>,
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
}

fn map_key_to_control(key: Key) -> Option<Controls> {
    match key {
        Key::A => Some(Controls::A),
        Key::S => Some(Controls::B),
        Key::D => Some(Controls::X),
        Key::F => Some(Controls::Y),
        Key::Left => Some(Controls::Left),
        Key::Right => Some(Controls::Right),
        Key::Up => Some(Controls::Up),
        Key::Down => Some(Controls::Down),
        Key::Space => Some(Controls::Start),
        Key::Q => Some(Controls::LeftTrigger),
        Key::E => Some(Controls::RightTrigger),
        _ => None,
    }
}
impl GraphicsSystem {
    pub fn new(resolution: [u32; 2]) -> GraphicsSystem {
        let mut gs = GraphicsSystem {
            background_layers: vec![],
            sprites: ([0, 0], Vec::new()),
            atlas: Rc::new(RefCell::new(TileAtlas::new())),
            display: Display::new(
                resolution[0] as usize,
                resolution[1] as usize,
                "Corallium",
                65,
                Scale::X4,
            ),
            controls: Vec::new(),
            ptrs: GraphicsPtrs {
                sprites: vec![],
                layers: vec![],
                atlas: 0,
            },
            queuedPixels: vec![],
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
    pub fn get_pixel(&mut self, x: usize, y: usize) -> u32 {
        self.display.buffer[y * self.display.width + x]
    }
    pub fn add_sprite(&mut self, mut sprite: Sprite) -> u8 {
        sprite.id = self.sprites.1.len() as u8;
        self.sprites.1.push(Some(sprite));
        (self.sprites.1.len() - 1) as u8
    }
    pub fn get_sprite(&mut self, id: u8) -> &mut Sprite {
        self.sprites.1[id as usize]
            .as_mut()
            .expect("nonexistent sprite")
    }
    pub fn sprite_exists(&mut self, id: u8) -> bool {
        self.sprites.1.len() > id as usize
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
        self.display.clear();

        for layer in &mut self.background_layers {
            layer.render(&mut self.display.buffer, self.display.width as u32);
        }
        let mut sprites = self.sprites.1.clone();
        sprites.sort_by_key(|sprite| match sprite.is_some() {
            true => sprite.as_ref().unwrap().priority,
            false => 0,
        });
        for sprite in sprites {
            if let Some(sprite) = sprite {
                sprite.tilemap.render(
                    [
                        sprite.loc[0] + self.sprites.0[0],
                        sprite.loc[1] + self.sprites.0[1],
                    ],
                    &mut self.display.buffer,
                    self.display.width as u32,
                );
            }
        }
        for pix in self.queuedPixels.drain(..) {
            self.display.buffer[pix.1 * self.display.width + pix.0] = pix.2;
        }
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
}
#[derive(Debug, Clone)]
pub struct Sprite {
    pub tilemap: TileMap,
    pub loc: Point,
    pub priority: u8,
    pub id: u8,
}
impl Sprite {
    pub fn new(tilemap: TileMap, loc: Point, priority: u8) -> Sprite {
        Sprite {
            tilemap,
            loc,
            priority,
            id: 0,
        }
    }
    fn render(&self, buf: &mut Vec<u32>, buf_width: u32) {
        self.tilemap.render(self.loc, buf, buf_width);
    }
}
#[derive(Debug)]
struct Display {
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
                if tile[tile_row_start + i] != 0 {
                    let src = buf[buf_idx + i];
                    let dst = tile[tile_row_start + i];
                    let color = (0..4).fold(0, |acc, i| {
                        acc | ((((src >> (i * 8)) & 0xff) * (src & 0xff)
                            + ((dst >> (i * 8)) & 0xff) * (255 - (src & 0xff)))
                            / 255)
                            << (i * 8)
                    });
                    buf[buf_idx + i] = color;
                }
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
    fn new(width: usize, height: usize, title: &str, target_fps: usize, scale: Scale) -> Self {
        let mut window = Window::new(
            title,
            width,
            height,
            WindowOptions {
                scale_mode: minifb::ScaleMode::Pillarbox,
                scale,
                resize: true,
                ..WindowOptions::default()
            },
        )
        .expect("Unable to open the window");

        window.set_target_fps(target_fps);
        Self {
            width,
            height,
            buffer: vec![0; width * height],
            window,
        }
    }
    fn render(&mut self) {
        if self.window.is_open() {
            self.window
                .update_with_buffer(
                    self.buffer
                        .iter()
                        .map(|x| x >> 8)
                        .collect::<Vec<u32>>()
                        .as_slice(),
                    self.width,
                    self.height,
                )
                .err();
        } else {
            return;
        }
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
}
