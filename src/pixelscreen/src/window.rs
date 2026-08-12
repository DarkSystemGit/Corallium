use std::sync::Arc;
use std::time::Duration;

use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::PhysicalKey;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window as WinitWindow, WindowBuilder};

use crate::input::{InputState, Key};
use crate::renderer::{BarColor, PixelFormat, Renderer, RendererError};
use crate::scale::ScaleMode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to create OS window: {0}")]
    Os(#[from] winit::error::OsError),
    #[error("failed to create event loop: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error(transparent)]
    Renderer(#[from] RendererError),
}

/// Initial window size, expressed as a multiple of the pixel buffer size
/// (minifb-style). Only affects the size the OS window opens at -- the
/// window remains resizable afterwards (see [`Window::set_resizable`]),
/// and the buffer is always rescaled/pillarboxed on the GPU to fit
/// whatever size the window ends up being, same as after any other
/// resize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    X1,
    X2,
    X4,
    X8,
    X16,
    X32,
    /// Opens the window maximized to fill the screen while keeping the
    /// standard OS window borders and title bar.
    FitScreen,
}

impl Scale {
    fn factor(self, buf_width: u32, buf_height: u32, event_loop: &EventLoop<()>) -> u32 {
        match self {
            Scale::X1 => 1,
            Scale::X2 => 2,
            Scale::X4 => 4,
            Scale::X8 => 8,
            Scale::X16 => 16,
            Scale::X32 => 32,
            Scale::FitScreen => {
                let Some(monitor) = event_loop.primary_monitor() else {
                    return 1;
                };
                let PhysicalSize {
                    width: mw,
                    height: mh,
                } = monitor.size();
                let fx = (mw / buf_width.max(1)).max(1);
                let fy = (mh / buf_height.max(1)).max(1);
                fx.min(fy).max(1) / 2
            }
        }
    }
}

/// A GPU-accelerated window that displays a fixed-size pixel buffer,
/// scaled up (integer by default) and centered with pillarbox bars to fit
/// the actual window size.
///
/// Typical usage:
///
/// ```no_run
/// use pixelscreen::{Window, Key};
///
/// let mut window = Window::new("demo", 320, 240).unwrap();
/// let mut buffer = vec![0xFF000000u32; 320 * 240]; // opaque black, 0xAARRGGBB... see docs
///
/// while window.is_open() {
///     window.update();
///
///     if window.is_key_down(Key::Escape) {
///         window.set_should_close(true);
///     }
///
///     // ... mutate `buffer` here ...
///
///     window.render(&buffer).unwrap();
/// }
/// ```
#[derive(Debug)]
pub struct Window {
    event_loop: EventLoop<()>,
    winit_window: Arc<WinitWindow>,
    renderer: Renderer,
    input: InputState,

    buf_width: u32,
    buf_height: u32,

    is_open: bool,
    should_close_on_escape: bool,
}

impl Window {
    /// Creates a new window titled `title`, opened at 1x the
    /// `buf_width` x `buf_height` pixel buffer size. Equivalent to
    /// `Window::new_with_scale(title, buf_width, buf_height, Scale::X1)`.
    /// See [`Window::new_with_scale`] to open larger initially.
    pub fn new(title: &str, buf_width: u32, buf_height: u32) -> Result<Self, Error> {
        Self::new_with_scale(title, buf_width, buf_height, Scale::X1)
    }

    /// Creates a new window titled `title`, opened at `scale` times the
    /// `buf_width` x `buf_height` pixel buffer size (minifb-style initial
    /// sizing). The window is resizable afterwards by default -- see
    /// [`Window::set_resizable`] -- and the buffer is always
    /// rescaled/pillarboxed on the GPU to fit whatever size the window
    /// ends up being, regardless of how it was initially opened.
    pub fn new_with_scale(
        title: &str,
        buf_width: u32,
        buf_height: u32,
        scale: Scale,
    ) -> Result<Self, Error> {
        let event_loop = EventLoop::new()?;

        let factor = scale.factor(buf_width, buf_height, &event_loop);
        let initial_size = LogicalSize::new(buf_width.max(1) * factor, buf_height.max(1) * factor);

        let mut builder = WindowBuilder::new()
            .with_title(title)
            .with_inner_size(initial_size)
            .with_resizable(true);

        if scale == Scale::FitScreen {
            // Suggest maximization to the OS during creation
            builder = builder.with_maximized(true);
        }

        let winit_window = Arc::new(builder.build(&event_loop)?);

        if scale == Scale::FitScreen {
            // Force maximization after creation to override OS quirks where
            // `with_inner_size` might cancel out the maximized state.
            winit_window.set_maximized(true);
        }

        let renderer = Renderer::new(winit_window.clone(), buf_width, buf_height)?;

        Ok(Self {
            event_loop,
            winit_window,
            renderer,
            input: InputState::new(),
            buf_width,
            buf_height,
            is_open: true,
            should_close_on_escape: false,
        })
    }

    /// Pumps pending OS events (resize, close, keyboard, focus, ...),
    /// updating internal state accordingly. Call this once per frame,
    /// before reading input or calling [`Window::render`].
    ///
    /// Returns `false` once the window has been closed (equivalent to
    /// [`Window::is_open`] immediately after this call).
    pub fn update(&mut self) -> bool {
        self.input.begin_frame();

        let input = &mut self.input;
        let renderer = &mut self.renderer;
        let should_close_on_escape = self.should_close_on_escape;
        let mut just_closed = false;

        let status = self
            .event_loop
            .pump_events(Some(Duration::ZERO), |event, elwt| {
                match event {
                    Event::WindowEvent { event, window_id }
                        if window_id == self.winit_window.id() =>
                    {
                        match event {
                            WindowEvent::CloseRequested => {
                                just_closed = true;
                                elwt.exit();
                            }
                            WindowEvent::Resized(size) => {
                                renderer.resize(size.width, size.height);
                            }
                            WindowEvent::ScaleFactorChanged { .. } => {
                                let size = self.winit_window.inner_size();
                                renderer.resize(size.width, size.height);
                            }
                            WindowEvent::KeyboardInput {
                                event: key_event, ..
                            } => {
                                if let PhysicalKey::Code(code) = key_event.physical_key {
                                    input.process_key_event(code, key_event.state);
                                    if should_close_on_escape
                                        && code == Key::Escape
                                        && key_event.state == ElementState::Pressed
                                    {
                                        just_closed = true;
                                        elwt.exit();
                                    }
                                }
                            }
                            WindowEvent::Focused(false) => {
                                // Avoid "stuck" keys if focus is lost mid-press.
                                input.clear();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            });

        if just_closed || matches!(status, PumpStatus::Exit(_)) {
            self.is_open = false;
        }

        self.is_open
    }

    /// Uploads `buffer` and draws it to the window, pillarboxed/letterboxed
    /// to fit per the current [`ScaleMode`]. `buffer` must contain exactly
    /// `width() * height()` pixels in 0xAABBGGRR (little-endian RGBA8)
    /// order -- i.e. each `u32` is `r | g << 8 | b << 16 | a << 24`.
    pub fn render(&mut self, buffer: &[u32]) -> Result<(), Error> {
        self.renderer
            .render(buffer, self.buf_width, self.buf_height)?;
        Ok(())
    }

    /// True until the window has been closed (close button, Escape if
    /// [`Window::set_close_on_escape`] is enabled, or
    /// [`Window::set_should_close`]).
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Request the window close. Takes effect immediately -- subsequent
    /// [`Window::is_open`] / [`Window::update`] calls return `false`.
    pub fn set_should_close(&mut self, val: bool) {
        if val {
            self.is_open = false;
        }
    }

    /// If enabled, pressing Escape will close the window (checked during
    /// [`Window::update`]). Disabled by default.
    pub fn set_close_on_escape(&mut self, enabled: bool) {
        self.should_close_on_escape = enabled;
    }

    /// True if `key` is currently held down.
    pub fn is_key_down(&self, key: Key) -> bool {
        self.input.is_key_down(key)
    }

    /// True if `key` transitioned from up to down since the last
    /// [`Window::update`] call.
    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.input.is_key_pressed(key)
    }

    /// True if `key` transitioned from down to up since the last
    /// [`Window::update`] call.
    pub fn is_key_released(&self, key: Key) -> bool {
        self.input.is_key_released(key)
    }

    /// All keys currently held down.
    pub fn held_keys(&self) -> impl Iterator<Item = &Key> {
        self.input.held_keys()
    }

    /// All keys currently held down, as an owned `Vec` (minifb-style
    /// `get_keys()`). Prefer [`Window::held_keys`] to avoid the allocation
    /// if you're just iterating.
    pub fn get_keys(&self) -> Vec<Key> {
        self.input.get_keys()
    }

    /// The fixed pixel-buffer width expected by [`Window::render`].
    pub fn width(&self) -> u32 {
        self.buf_width
    }

    /// The fixed pixel-buffer height expected by [`Window::render`].
    pub fn height(&self) -> u32 {
        self.buf_height
    }

    /// The current OS window size in physical pixels (changes on resize).
    pub fn window_size(&self) -> (u32, u32) {
        self.renderer.window_size()
    }

    /// Whether the user can resize the window by dragging its edges.
    /// `true` by default (unlike minifb, which defaults to `false`) --
    /// this crate's whole point is dynamically rescaling the buffer to fit
    /// whatever size the window is, so freely resizable is the natural
    /// default. Set `false` to lock the window to whatever size it was
    /// opened at.
    pub fn set_resizable(&mut self, resizable: bool) {
        self.winit_window.set_resizable(resizable);
    }

    /// How the buffer is scaled to fit the window. Default: [`ScaleMode::Integer`].
    pub fn set_scale_mode(&mut self, mode: ScaleMode) {
        self.renderer.scale_mode = mode;
    }

    pub fn scale_mode(&self) -> ScaleMode {
        self.renderer.scale_mode
    }

    /// Color used to fill the pillarbox/letterbox bars. Default: opaque black.
    pub fn set_bar_color(&mut self, color: BarColor) {
        self.renderer.bar_color = color;
    }

    /// How to interpret each `u32` passed to [`Window::render`]. Set this
    /// once to match your own buffer's byte order and never convert your
    /// buffer before rendering -- pixelscreen uploads it unmodified and
    /// unpacks it on the GPU. Default: [`PixelFormat::Aabbggrr`].
    pub fn set_pixel_format(&mut self, format: PixelFormat) {
        self.renderer.pixel_format = format;
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.renderer.pixel_format
    }
}
