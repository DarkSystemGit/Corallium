//! Move a square around with WASD / arrow keys. Demonstrates
//! `is_key_down` (held, for continuous movement) vs `is_key_pressed`
//! (edge-triggered, for one-shot actions).

use pixelscreen::{Key, Window};

const BUF_W: u32 = 200;
const BUF_H: u32 = 150;
const SPEED: f32 = 90.0; // pixels/sec
const SQUARE: i32 = 10;

fn main() {
    let mut window = Window::new("pixelscreen: keyboard sprite", BUF_W, BUF_H)
        .expect("failed to create window");
    window.set_close_on_escape(true);

    let mut buffer = vec![0xFF202020u32; (BUF_W * BUF_H) as usize];

    let mut x = BUF_W as f32 / 2.0;
    let mut y = BUF_H as f32 / 2.0;
    let mut color: u32 = 0xFF66CC66; // opaque green (R=0x66,G=0xCC,B=0x66)

    let mut last = std::time::Instant::now();

    while window.is_open() {
        window.update();

        let now = std::time::Instant::now();
        let dt = (now - last).as_secs_f32();
        last = now;

        // Held keys -> continuous movement.
        let left = window.is_key_down(Key::ArrowLeft) || window.is_key_down(Key::KeyA);
        let right = window.is_key_down(Key::ArrowRight) || window.is_key_down(Key::KeyD);
        let up = window.is_key_down(Key::ArrowUp) || window.is_key_down(Key::KeyW);
        let down = window.is_key_down(Key::ArrowDown) || window.is_key_down(Key::KeyS);
        // `get_keys()` (minifb-style) is handy for e.g. a debug HUD of everything held:
        let _held: Vec<Key> = window.get_keys();

        if left {
            x -= SPEED * dt;
        }
        if right {
            x += SPEED * dt;
        }
        if up {
            y -= SPEED * dt;
        }
        if down {
            y += SPEED * dt;
        }
        x = x.clamp(0.0, BUF_W as f32);
        y = y.clamp(0.0, BUF_H as f32);

        // Edge-triggered key -> one-shot action (cycle color), won't repeat
        // every frame while held.
        if window.is_key_pressed(Key::Space) {
            color = color.rotate_left(8);
        }

        buffer.fill(0xFF202020);
        let (cx, cy) = (x as i32, y as i32);
        for dy in -SQUARE..=SQUARE {
            for dx in -SQUARE..=SQUARE {
                let (px, py) = (cx + dx, cy + dy);
                if px >= 0 && py >= 0 && (px as u32) < BUF_W && (py as u32) < BUF_H {
                    buffer[(py as u32 * BUF_W + px as u32) as usize] = color;
                }
            }
        }

        window.render(&buffer).expect("render failed");
    }
}
