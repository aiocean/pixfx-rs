//! One effect playing on its own canvas: what every host plays through (the wasm `Fx`, the canvas stage, gpui).

use crate::cue::TimedCue;
use crate::effect::{Canvas, Effect, Instance, Layer, Meta, Options, Placement};
use crate::frame::{Frame, composite};
use crate::key::{ClipKey, FrameId};
use crate::render::edge_fade;

/// An effect instance and the canvas it draws into: the instance's canvas ([`Effect::canvas`]: `meta.size`
/// unless its options size it) plus `margin` empty pixels on every side, with the effect's origin at the
/// canvas origin plus `margin`. While its clock is on frame k ([`Player::frame_index`]), `draw(Layer::Both)`
/// leaves the frame equal to [`crate::render_frames`] frame k at the same margin, whatever steps the clock
/// took: margin 0 eases light out at the canvas edge (as the sprite sheet does), a margin leaves room for glow
/// that spills past the canvas and fades nothing.
pub struct Player {
    inst: Instance,
    /// The options the instance was spawned with (its origin on this player's frame): part of its frames' key.
    opts: Options,
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
            inst: Instance::new(effect, at_origin.clone()),
            opts: at_origin,
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

    /// Advance the clock by `dt` seconds ([`Instance::update`]: at most `MAX_DT`; a NaN, infinite or negative
    /// `dt` is ignored).
    pub fn update(&mut self, dt: f64) {
        self.inst.update(dt);
    }

    pub fn stop(&mut self) {
        self.inst.stop();
    }

    /// The cues playback passed since the last call ([`Instance::take_cues`]), in this player's frame space: `x`/`y`
    /// where [`Player::new`] put the effect's origin. Clears them.
    pub fn take_cues(&mut self) -> Vec<TimedCue> {
        self.inst.take_cues()
    }

    pub fn done(&self) -> bool {
        self.inst.done()
    }

    /// The frame the effect shows ([`Instance::frame`]): a host redraws when it changes.
    pub fn frame_index(&self) -> u64 {
        self.inst.frame()
    }

    /// Frames of `1 / fps` its clock has passed ([`Instance::clock_frame`]): what it draws changes only when this
    /// does (or the placement does), so a host redraws on a change and not on every display refresh.
    pub fn clock_frame(&self) -> u64 {
        self.inst.clock_frame()
    }

    /// The frame `draw(_, place)` would draw now, or none while the effect fades out after [`Player::stop`] (a
    /// fade frame is its own). Players showing the same [`FrameId`] draw byte-equal frames.
    pub fn shown(&self, place: Placement) -> Option<FrameId> {
        if self.inst.stopped() {
            return None;
        }
        Some(FrameId {
            key: ClipKey::new(self.meta.id, &self.opts, self.margin, place),
            index: self.inst.frame(),
        })
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
    /// [`crate::render_frames`] does).
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
