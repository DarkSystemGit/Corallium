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
}
struct Layer {
    id: i16,
    x: i16,
    y: i16,
    tilemap_height: i16,
    tilemap_width: i16,
    tilemap: [i16],
    transform: Transform,
    transformInfo: [f32],
}
fn registerAtlas(atlas: [i16]) -> void {}
fn registerSprite(sprite: Sprite) -> void {}
fn registerLayer(layer: Layer) -> void {}
fn render() -> void {}
fn pullControls(writeLoc: [bool; 11]) -> void {}
fn setPixel(x: i16, y: i16, color: i32) -> void {}
fn getPixel(x: i16, y: i16) -> i32 {}
