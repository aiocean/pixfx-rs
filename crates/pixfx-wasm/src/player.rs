//! One effect playing on its own canvas, in plain Rust: what [`crate::Fx`] wraps for JS and what other Rust
//! crates (a canvas renderer) use directly.

use pixfx_core::{
    Canvas, Effect, Frame, Instance, Layer, Meta, Options, Placement, composite, edge_fade,
};

/// An effect instance and the canvas it draws into: the instance's canvas ([`Effect::canvas`]: `meta.size`
/// unless its options size it) plus `margin` empty pixels on every side, with the effect's origin at the
/// canvas origin plus `margin`. After the n-th `update(1 / fps)`,
/// `draw(Layer::Both)` leaves the frame equal to [`pixfx_core::render_frames`] frame n at the same margin:
/// margin 0 eases light out at the canvas edge (as the sprite sheet does), a margin leaves room for glow that
/// spills past the canvas and fades nothing.
pub struct Player {
    inst: Instance,
    meta: &'static Meta,
    canvas: Canvas,
    margin: u32,
    frame: Frame,
    rgba: Vec<u8>,
}

impl Player {
    /// Spawn `effect` with `opts` on a canvas `margin` pixels bigger on every side; `opts.x` and `opts.y` are
    /// replaced by the effect's origin.
    pub fn new(effect: &'static dyn Effect, opts: Options, margin: u32) -> Self {
        let meta = effect.meta();
        let canvas = effect.canvas(&opts);
        let (w, h) = (canvas.size.0 + 2 * margin, canvas.size.1 + 2 * margin);
        let at_origin = Options {
            x: f64::from(canvas.origin.0),
            y: f64::from(canvas.origin.1),
            ..opts
        };
        // the frame's top-left sits `margin` up and left of the effect canvas's, as in `render_frames`
        let corner = -(margin as i32);
        Self {
            inst: Instance::new(effect, at_origin),
            meta,
            canvas,
            margin,
            frame: Frame::new(w, h, corner, corner),
            rgba: vec![0; w as usize * h as usize * 4],
        }
    }

    pub fn meta(&self) -> &'static Meta {
        self.meta
    }

    /// The canvas this instance draws on, margin not included.
    pub fn canvas(&self) -> Canvas {
        self.canvas
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

    /// Where its options place the instance on this player's frame: `x`/`y` where [`Player::new`] put the
    /// effect's origin ([`Instance::placement`]).
    pub fn placement(&self) -> Placement {
        self.inst.placement()
    }

    /// Clear the frame and draw `layer` at the current time, placed as `place` says (`x`/`y` in this player's
    /// frame). With margin 0, light is eased out at the canvas edge unless the effect is full-screen (as
    /// [`pixfx_core::render_frames`] does).
    pub fn draw(&mut self, layer: Layer, place: Placement) {
        self.frame.clear();
        self.inst.draw(&mut self.frame, layer, place);
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

    /// The last drawn colour layer alone as RGBA8 bytes (`w * h * 4`, straight alpha): each ABGR word unpacked.
    pub fn color_rgba(&mut self) -> &[u8] {
        for (out, word) in self
            .rgba
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(&self.frame.color)
        {
            *out = word.to_le_bytes();
        }
        &self.rgba
    }

    /// The last drawn light layer alone as RGBA8 bytes (`w * h * 4`): a lit pixel is `r, g, b, 255`, an unlit
    /// one `0, 0, 0, 0`, so drawn with additive blending it adds exactly the light and nothing elsewhere.
    pub fn light_rgba(&mut self) -> &[u8] {
        for (out, word) in self
            .rgba
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(&self.frame.light)
        {
            let rgb = word & 0x00ff_ffff;
            *out = if rgb == 0 { 0u32 } else { rgb | 0xff00_0000 }.to_le_bytes();
        }
        &self.rgba
    }
}

#[cfg(test)]
mod tests {
    use super::Player;
    use pixfx_core::{Layer, Options};
    use pixfx_effects::effect;

    /// A campfire drawn mid-loop, with glow room, so both layers hold many lit and many empty pixels.
    fn drawn() -> Result<Player, &'static str> {
        let fx = effect("fire/campfire").ok_or("campfire is not registered")?;
        let mut p = Player::new(fx, Options::default(), fx.meta().glow_margin());
        p.update(0.2);
        let place = p.placement();
        p.draw(Layer::Both, place);
        Ok(p)
    }

    #[test]
    fn color_rgba_unpacks_every_abgr_word() -> Result<(), &'static str> {
        let mut p = drawn()?;
        let words = p.frame().color.clone();
        let bytes = p.color_rgba().to_vec();
        assert_eq!(bytes.len(), words.len() * 4);
        for (i, w) in words.iter().enumerate() {
            let want = [*w as u8, (w >> 8) as u8, (w >> 16) as u8, (w >> 24) as u8];
            assert_eq!(bytes[i * 4..i * 4 + 4], want, "pixel {i}");
        }
        assert!(words.iter().any(|w| w >> 24 != 0) && words.contains(&0));
        Ok(())
    }

    #[test]
    fn light_rgba_is_opaque_where_lit_and_empty_elsewhere() -> Result<(), &'static str> {
        let mut p = drawn()?;
        let words = p.frame().light.clone();
        let bytes = p.light_rgba().to_vec();
        for (i, w) in words.iter().enumerate() {
            let rgb = w & 0x00ff_ffff;
            let want = if rgb == 0 {
                [0, 0, 0, 0]
            } else {
                [rgb as u8, (rgb >> 8) as u8, (rgb >> 16) as u8, 255]
            };
            assert_eq!(bytes[i * 4..i * 4 + 4], want, "pixel {i}");
        }
        assert!(
            words.iter().any(|w| w & 0x00ff_ffff == 0)
                && words.iter().any(|w| w & 0x00ff_ffff != 0)
        );
        Ok(())
    }
}
