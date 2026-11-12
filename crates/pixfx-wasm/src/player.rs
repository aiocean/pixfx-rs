//! One effect playing on its own canvas, in plain Rust: what [`crate::Fx`] wraps for JS and what other Rust
//! crates (a canvas renderer) use directly.

use pixfx_core::{Effect, Frame, Instance, Layer, Meta, Options, composite, edge_fade};

/// An effect instance and the canvas it draws into: the effect's `meta.size` plus `margin` empty pixels on
/// every side, with the effect's origin at `meta.origin + margin`. After the n-th `update(1 / fps)`,
/// `draw(Layer::Both)` leaves the frame equal to [`pixfx_core::render_frames`] frame n at the same margin:
/// margin 0 eases light out at the canvas edge (as the sprite sheet does), a margin leaves room for glow that
/// spills past the canvas and fades nothing.
pub struct Player {
    inst: Instance,
    meta: &'static Meta,
    margin: u32,
    frame: Frame,
    rgba: Vec<u8>,
}

impl Player {
    /// Spawn `effect` with `opts` on a canvas `margin` pixels bigger on every side; `opts.x` and `opts.y` are
    /// replaced by the effect's origin.
    pub fn new(effect: &'static dyn Effect, opts: Options, margin: u32) -> Self {
        let meta = effect.meta();
        let (w, h) = (meta.size.0 + 2 * margin, meta.size.1 + 2 * margin);
        let at_origin = Options {
            x: f64::from(meta.origin.0),
            y: f64::from(meta.origin.1),
            ..opts
        };
        // the frame's top-left sits `margin` up and left of the effect canvas's, as in `render_frames`
        let corner = -(margin as i32);
        Self {
            inst: Instance::new(effect, at_origin),
            meta,
            margin,
            frame: Frame::new(w, h, corner, corner),
            rgba: vec![0; w as usize * h as usize * 4],
        }
    }

    pub fn meta(&self) -> &'static Meta {
        self.meta
    }

    /// Empty pixels round the effect's canvas on every side.
    pub fn margin(&self) -> u32 {
        self.margin
    }

    /// Advance by `dt` seconds ([`Instance::update`]: at most `MAX_DT`, in 1/120 s sub-steps; a NaN, infinite or
    /// negative `dt` is ignored).
    pub fn update(&mut self, dt: f64) {
        self.inst.update(dt);
    }

    pub fn stop(&mut self) {
        self.inst.stop();
    }

    pub fn done(&self) -> bool {
        self.inst.done()
    }

    /// Seconds since the instance started.
    pub fn t(&self) -> f64 {
        self.inst.t()
    }

    /// Clear the frame and draw `layer` at the current time. With margin 0, light is eased out at the canvas
    /// edge unless the effect is full-screen (as [`pixfx_core::render_frames`] does).
    pub fn draw(&mut self, layer: Layer) {
        self.frame.clear();
        self.inst.draw(&mut self.frame, layer);
        if self.margin == 0 && !self.meta.is_screen() {
            edge_fade(&mut self.frame);
        }
    }

    /// The last drawn frame.
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// The last drawn frame composited over transparency, as RGBA8 bytes (`w * h * 4`).
    pub fn rgba(&mut self) -> &[u8] {
        composite(&self.frame, &mut self.rgba, None);
        &self.rgba
    }
}
