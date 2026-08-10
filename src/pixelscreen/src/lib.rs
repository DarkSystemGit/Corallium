//! `pixelscreen` -- a small, GPU-accelerated library for displaying a raw
//! pixel buffer in a window.
//!
//! Create a [`Window`], hand it a `Vec<u32>` of RGBA pixels each frame via
//! [`Window::render`], and it takes care of scaling the buffer up to fit
//! the window (integer scaling by default) and centering it with
//! pillarbox/letterbox bars when the aspect ratios don't match exactly.
//!
//! ```no_run
//! use pixelscreen::{Window, Key};
//!
//! fn main() {
//!     let mut window = Window::new("demo", 320, 240).unwrap();
//!     let mut buffer = vec![0xFF000000u32; 320 * 240];
//!
//!     while window.is_open() {
//!         window.update();
//!
//!         if window.is_key_down(Key::Escape) {
//!             window.set_should_close(true);
//!         }
//!
//!         // draw into `buffer` here
//!
//!         window.render(&buffer).unwrap();
//!     }
//! }
//! ```

mod input;
mod renderer;
mod scale;
mod window;

pub use input::{InputState, Key};
pub use renderer::{BarColor, PixelFormat, RendererError};
pub use scale::{compute_viewport, ScaleMode, Viewport};
pub use window::{Error, Scale, Window};
