//! One effect playing on its own canvas, in plain Rust: what [`crate::Fx`] wraps for JS and what other Rust
//! crates (a canvas renderer) use directly.

use pixfx_core::{Effect, Frame, Instance, Layer, Meta, Options, composite, edge_fade};

/// An effect instance and the canvas it draws into: the effect's own `meta.size`, with the effect's origin at
/// `meta.origin` (margin 0). After the n-th `update(1 / fps)`, `draw(Layer::Both)` leaves the frame equal to
/// [`pixfx_core::render_frames`] frame n, edge fade included, so a live effect shows what its sprite sheet
/// shows.
pub struct Player {
    inst: Instance,
    meta: &'static Meta,
    frame: Frame,
    rgba: Vec<u8>,
}

impl Player {
    /// Spawn `effect` with `opts`; `opts.x` and `opts.y` are replaced by the effect's origin.
    pub fn new(effect: &'static dyn Effect, opts: Options) -> Self {
        let meta = effect.meta();
        let (w, h) = meta.size;
        let at_origin = Options {
            x: f64::from(meta.origin.0),
            y: f64::from(meta.origin.1),
            ..opts
        };
        Self {
            inst: Instance::new(effect, at_origin),
            meta,
            frame: Frame::new(w, h, 0, 0),
            rgba: vec![0; w as usize * h as usize * 4],
        }
    }

    pub fn meta(&self) -> &'static Meta {
        self.meta
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

    /// Clear the frame and draw `layer` at the current time; light is eased out at the canvas edge unless the
    /// effect is full-screen (as [`pixfx_core::render_frames`] does).
    pub fn draw(&mut self, layer: Layer) {
        self.frame.clear();
        self.inst.draw(&mut self.frame, layer);
        if !self.meta.is_screen() {
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
