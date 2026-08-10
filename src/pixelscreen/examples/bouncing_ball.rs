//! A ball bouncing around a small buffer, upscaled into a bigger window.
//! Demonstrates the basic update/render loop and confirms pillarboxing
//! kicks in when the window doesn't cleanly match an integer multiple of
//! the buffer size (try resizing the window).

use pixelscreen::{Key, Scale, Window};

const BUF_W: u32 = 160;
const BUF_H: u32 = 120;

fn put_pixel(buf: &mut [u32], x: i32, y: i32, color: u32) {
    if x < 0 || y < 0 || x as u32 >= BUF_W || y as u32 >= BUF_H {
        return;
    }
    buf[(y as u32 * BUF_W + x as u32) as usize] = color;
}

fn draw_filled_circle(buf: &mut [u32], cx: i32, cy: i32, r: i32, color: u32) {
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r * r {
                put_pixel(buf, cx + dx, cy + dy, color);
            }
        }
    }
}

fn main() {
    // Opens at 2x the buffer size (minifb-style `Scale`); the window stays
    // freely resizable afterwards, and the buffer keeps rescaling to fit.
    let mut window = Window::new_with_scale("pixelscreen: bouncing ball", BUF_W, BUF_H, Scale::X2)
        .expect("failed to create window");
    window.set_close_on_escape(true);

    let mut buffer = vec![0xFF101010u32; (BUF_W * BUF_H) as usize];

    let (mut x, mut y): (f32, f32) = (BUF_W as f32 / 2.0, BUF_H as f32 / 2.0);
    let (mut vx, mut vy): (f32, f32) = (60.0, 45.0); // pixels/sec
    let radius = 6;
    let ball_color: u32 = 0xFF4DA6FF; // 0xAABBGGRR -> opaque orange (R=0xFF, G=0xA6, B=0x4D)

    let mut last = std::time::Instant::now();

    while window.is_open() {
        window.update();

        let now = std::time::Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;

        x += vx * dt;
        y += vy * dt;

        if x - radius as f32 <= 0.0 || x + radius as f32 >= BUF_W as f32 {
            vx = -vx;
            x = x.clamp(radius as f32, (BUF_W - radius as u32) as f32);
        }
        if y - radius as f32 <= 0.0 || y + radius as f32 >= BUF_H as f32 {
            vy = -vy;
            y = y.clamp(radius as f32, (BUF_H - radius as u32) as f32);
        }

        if window.is_key_pressed(Key::Space) {
            // Give it a little kick.
            vx *= 1.3;
            vy *= 1.3;
        }

        buffer.fill(0xFF101010);
        draw_filled_circle(&mut buffer, x as i32, y as i32, radius, ball_color);

        window.render(&buffer).expect("render failed");
    }
}
