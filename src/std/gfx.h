enum Transform {
    Flat,
    SingleMatrixAffine,
    MultiMatrixAffine,
}
struct Sprite {
    id: i16,
    x: i16,
    y: i16,
    priority: i16,
    tilemap_height: i16,
    tilemap_width: i16,
    tilemap: [i16],
    scale_x: f32,
    scale_y: f32,
}
struct Layer {
    id: i16,
    x: i16,
    y: i16,
    tilemap_height: i16,
    tilemap_width: i16,
    tilemap: [i16],
    transform: Transform,
    transformInfo: [f32]?,
    loc: [i32;2]?
}
struct Bitmap {
    length: i16,
    width: i16,
    data: [i32],
}
fn registerAtlas(atlas: [i16]) -> void {}
fn registerSprite(sprite: Sprite) -> void {}
fn removeSprite(sprite: Sprite) -> void {}
fn registerLayer(layer: Layer) -> void {}
fn removeLayer(layer: Layer) -> void {}
fn render() -> void {}
fn pullControls(writeLoc: [bool; 11]) -> void {}
fn setPixel(x: i16, y: i16, color: i32) -> void {}
fn getPixel(x: i16, y: i16) -> i32 {}
fn registerBitmap(bitmap: Bitmap) -> void {}
fn removeBitmap(bitmap: Bitmap) -> void {}
