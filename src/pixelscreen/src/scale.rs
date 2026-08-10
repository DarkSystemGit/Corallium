//! CPU-side reference implementation of the same scaling and pillarbox
//! centering math that `shader.wgsl`'s vertex stage computes on the GPU
//! for the actual draw. [`Renderer`](crate) does not use this module to
//! render -- it's kept public as a utility, e.g. for converting a
//! mouse/window coordinate into buffer-space coordinates (hit-testing),
//! and as a spec the shader math is tested against.
//!
//! Scaling always fits the buffer's height to the window's height baseline
//! (modulo integer-mode rounding, see [`ScaleMode::Integer`]). The
//! resulting width is centered horizontally: narrower than the window ->
//! pillarbox bars on the sides; wider -> width is clamped to the window
//! and the image is squeezed horizontally (matching the CPU pillarbox
//! helper behavior).

/// How the buffer is scaled to fit the window's height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleMode {
    /// Scale by the largest whole-number factor that fits the window's
    /// height. Crisp, blocky, no interpolation artifacts. If the height
    /// isn't an exact multiple of the buffer height, a thin leftover strip
    /// may remain top/bottom (unlike `Fit`, which always fills exactly).
    /// This is the default.
    Integer,
    /// Scale by the exact fractional factor that fills the window's height
    /// precisely (top and bottom always perfectly flush).
    Fit,
}

impl Default for ScaleMode {
    fn default() -> Self {
        ScaleMode::Integer
    }
}

/// A viewport rectangle in physical pixel coordinates, origin top-left,
/// describing where the scaled buffer image should be drawn within the
/// window. Window area not covered by this rect (only ever on the
/// left/right) is bar/background color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// The effective scale factor that was applied to the buffer.
    pub scale: f32,
}

/// Computes the centered, height-fit viewport for drawing a `buf_w`x`buf_h`
/// buffer inside a `win_w`x`win_h` window, per `mode`. The buffer's height
/// always fits the window's height baseline (see module docs); width is
/// centered, producing pillarbox bars on the sides if narrower than the
/// window. If the computed width is wider than the window, it is clamped
/// to the window width.
///
/// Returns a viewport with zero size if the window or buffer has a zero
/// dimension (nothing should be drawn).
pub fn compute_viewport(
    buf_w: u32,
    buf_h: u32,
    win_w: u32,
    win_h: u32,
    mode: ScaleMode,
) -> Viewport {
    if buf_w == 0 || buf_h == 0 || win_w == 0 || win_h == 0 {
        return Viewport { x: 0.0, y: 0.0, width: 0.0, height: 0.0, scale: 0.0 };
    }

    let (buf_w_f, buf_h_f, win_w_f, win_h_f) =
        (buf_w as f32, buf_h as f32, win_w as f32, win_h as f32);

    // Scale is always derived from height alone -- this is what makes the
    // top/bottom edges fit and the sides pillarbox.
    let height_scale = win_h_f / buf_h_f;
    let scale = match mode {
        ScaleMode::Integer => height_scale.floor().max(1.0),
        ScaleMode::Fit => height_scale,
    };

    let draw_w = (buf_w_f * scale).min(win_w_f);
    let draw_h = buf_h_f * scale;

    let x = ((win_w_f - draw_w) * 0.5).round();
    let y = ((win_h_f - draw_h) * 0.5).round();

    Viewport { x, y, width: draw_w.round(), height: draw_h.round(), scale }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_fit_integer_has_no_bars() {
        let vp = compute_viewport(320, 240, 640, 480, ScaleMode::Integer);
        assert_eq!(vp.scale, 2.0);
        assert_eq!(vp.width, 640.0);
        assert_eq!(vp.height, 480.0);
        assert_eq!(vp.x, 0.0);
        assert_eq!(vp.y, 0.0);
    }

    #[test]
    fn narrower_content_pillarboxes_sides() {
        // buffer 320x240 (4:3) in a 1000x480 window -> height fits exactly,
        // scaled width (640) is narrower than the window -> bars on sides.
        let vp = compute_viewport(320, 240, 1000, 480, ScaleMode::Integer);
        assert_eq!(vp.scale, 2.0);
        assert_eq!(vp.width, 640.0);
        assert_eq!(vp.height, 480.0); // fits top/bottom exactly
        assert_eq!(vp.x, (1000.0 - 640.0) / 2.0);
        assert_eq!(vp.y, 0.0); // never any letterbox bar in Fit-driven height
    }

    #[test]
    fn wider_content_clamps_to_window_width() {
        // buffer 320x240 in a narrow 200x480 window: height scale is still
        // 2, draw_h stays at 480, but width is clamped to the window width.
        let vp = compute_viewport(320, 240, 200, 480, ScaleMode::Integer);
        assert_eq!(vp.scale, 2.0);
        assert_eq!(vp.width, 200.0);
        assert_eq!(vp.height, 480.0);
        assert_eq!(vp.x, 0.0);
        assert_eq!(vp.y, 0.0);
    }

    #[test]
    fn height_alone_drives_scale_regardless_of_width() {
        // A very wide window shouldn't change the scale at all -- only
        // height matters, so this still scales by exactly 2 and pillarboxes.
        let vp = compute_viewport(320, 240, 5000, 480, ScaleMode::Integer);
        assert_eq!(vp.scale, 2.0);
        assert_eq!(vp.height, 480.0);
    }

    #[test]
    fn too_small_window_still_scales_at_least_one() {
        let vp = compute_viewport(320, 240, 100, 100, ScaleMode::Integer);
        assert_eq!(vp.scale, 1.0);
    }

    #[test]
    fn fit_mode_matches_window_height_exactly() {
        let vp = compute_viewport(320, 240, 500, 481, ScaleMode::Fit);
        let expected: f32 = 481.0 / 240.0;
        assert!((vp.scale - expected).abs() < f32::EPSILON);
        assert!((vp.height - 481.0).abs() < 1.0); // exact modulo rounding to whole px
    }

    #[test]
    fn zero_dims_are_safe() {
        let vp = compute_viewport(0, 240, 640, 480, ScaleMode::Integer);
        assert_eq!(vp.width, 0.0);
        let vp = compute_viewport(320, 240, 0, 480, ScaleMode::Integer);
        assert_eq!(vp.width, 0.0);
    }
}
