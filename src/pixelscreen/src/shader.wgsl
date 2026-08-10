// Draws the pixel buffer as a textured quad. Scaling/pillarbox placement
// is computed on the GPU in the vertex stage (see `FrameUniform` below).
// The buffer texture is uploaded as raw, unmodified `u32` texels (format
// `R32Uint`) -- the fragment shader unpacks each texel's R/G/B/A bytes
// itself, according to `pixel_format`, rather than relying on a fixed
// wgpu texture format + sampler. This means the Rust side never has to
// byte-shuffle your buffer before uploading it: whatever `u32` layout you
// already use, you set `pixel_format` once and hand the buffer over as-is
// every frame.
//
// Scaling always fits the buffer's height to the window's height baseline
// (modulo integer-mode rounding, see below). Width is derived from the same
// uniform scale factor so aspect ratio is always preserved, and the image is
// centered horizontally (side margins may be positive or negative).
//
// Area of the window outside the computed quad is left untouched by this
// draw call -- it shows whatever the render pass's clear color was, which
// is how the pillarbox bars appear.

struct FrameUniform {
    buf_size: vec2<f32>,
    win_size: vec2<f32>,
    // Legacy field kept for uniform layout compatibility.
    // Scaling in this shader always uses smooth height-fit behavior.
    scale_mode: f32,
    // 0.0 = PixelFormat::Aabbggrr: pixel = r | g<<8 | b<<16 | a<<24
    // 1.0 = PixelFormat::Rrggbbaa: pixel = r<<24 | g<<16 | b<<8 | a
    pixel_format: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(1)
var<uniform> frame: FrameUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Two triangles covering the unit square [0,1] x [0,1] in buffer-local
    // space, top-left origin (matches the row-major top-down pixel data
    // uploaded to the texture).
    var unit = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    let p = unit[vertex_index];

    let buf = max(frame.buf_size, vec2<f32>(1.0, 1.0));
    let win = max(frame.win_size, vec2<f32>(1.0, 1.0));

    // Always smooth/fractional scaling from height to avoid integer "popping"
    // and to ensure the content height matches the window height.
    let scale = win.y / buf.y;

    let draw_h = buf.y * scale;
    let draw_w = buf.x * scale;
    let draw_size = vec2<f32>(draw_w, draw_h);
    let origin = vec2<f32>((win.x - draw_w) * 0.5, (win.y - draw_h) * 0.5);

    // Pixel-space position (origin top-left, y-down) of this vertex.
    let pixel_pos = origin + p * draw_size;

    // Convert to clip space: x in [-1,1] left-to-right, y in [-1,1]
    // bottom-to-top (so we flip the y axis, since pixel space is y-down).
    let clip_x = (pixel_pos.x / win.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (pixel_pos.y / win.y) * 2.0;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.uv = p;
    return out;
}

@group(0) @binding(0)
var buffer_texture: texture_2d<u32>;

fn unpack_aabbggrr(p: u32) -> vec4<f32> {
    let r = f32(p & 0xffu);
    let g = f32((p >> 8u) & 0xffu);
    let b = f32((p >> 16u) & 0xffu);
    let a = f32((p >> 24u) & 0xffu);
    return vec4<f32>(r, g, b, a) / 255.0;
}

fn unpack_rrggbbaa(p: u32) -> vec4<f32> {
    let a = f32(p & 0xffu);
    let b = f32((p >> 8u) & 0xffu);
    let g = f32((p >> 16u) & 0xffu);
    let r = f32((p >> 24u) & 0xffu);
    return vec4<f32>(r, g, b, a) / 255.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(buffer_texture));
    // Nearest-neighbor: floor to the texel the uv falls in, no
    // interpolation, matching the previous sampler-based behavior exactly.
    let texel = vec2<i32>(min(in.uv * dims, dims - vec2<f32>(1.0, 1.0)));
    let packed = textureLoad(buffer_texture, texel, 0).r;

    if (frame.pixel_format < 0.5) {
        return unpack_aabbggrr(packed);
    } else {
        return unpack_rrggbbaa(packed);
    }
}
