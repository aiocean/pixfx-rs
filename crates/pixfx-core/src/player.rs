//! One effect playing on its own canvas: what every host plays through (the wasm `Fx`, the canvas stage, gpui).

use crate::cue::TimedCue;
use crate::effect::{Canvas, Clock, Effect, Instance, Layer, Meta, Options, Placement, Sim};
use crate::key::{ClipKey, FrameId};
use crate::render::edge_fade;
use pixfx_draw::frame::{Frame, composite, composite_px};

/// What one effect draws and how big: the effect, its options with its origin where its canvas puts it, and the
/// margin of empty pixels round the canvas. Every instance of a clip with the same placement and frame draws a
/// byte-equal frame ([`Clip::key`]), so a host can draw it once and share it, or draw it on another thread.
#[derive(Clone)]
pub struct Clip {
    effect: &'static dyn Effect,
    /// The options the clip plays with, its origin on the clip's frame: part of its frames' key.
    opts: Options,
    canvas: Canvas,
    margin: u32,
}

impl Clip {
    /// `effect` with `opts` on a canvas `margin` pixels bigger on every side; `opts.x` and `opts.y` are replaced
    /// by the effect's origin.
    pub fn new(effect: &'static dyn Effect, opts: Options, margin: u32) -> Self {
        let canvas = effect.canvas(&opts);
        let at_origin = Options {
            x: f64::from(canvas.origin.0),
            y: f64::from(canvas.origin.1),
            ..opts
        };
        Self {
            effect,
            opts: at_origin,
            canvas,
            margin,
        }
    }

    /// `effect` with `opts` as live hosts play it: with [`Meta::glow_margin`] round its canvas, so nothing at
    /// the canvas edge is faded or cut. Rendered clips are drawn the same way, so they share its keys.
    pub fn live(effect: &'static dyn Effect, opts: Options) -> Self {
        Self::new(effect, opts, effect.meta().glow_margin())
    }

    pub fn meta(&self) -> &'static Meta {
        self.effect.meta()
    }

    /// The effect it plays.
    pub fn effect(&self) -> &'static dyn Effect {
        self.effect
    }

    /// The options it plays with, its origin where its canvas puts it.
    pub fn opts(&self) -> &Options {
        &self.opts
    }

    /// Whether its effect grips the character it plays on ([`Effect::wraps_actor`]).
    pub fn wraps_actor(&self) -> bool {
        self.effect.wraps_actor()
    }

    /// The canvas the effect draws on, margin not included.
    pub fn canvas(&self) -> Canvas {
        self.canvas
    }

    /// Empty pixels round the effect's canvas on every side.
    pub fn margin(&self) -> u32 {
        self.margin
    }

    /// The frame's size: the canvas plus the margin on every side.
    pub fn size(&self) -> (u32, u32) {
        (
            self.canvas.size.0 + 2 * self.margin,
            self.canvas.size.1 + 2 * self.margin,
        )
    }

    /// An empty frame of the clip's size, its top-left `margin` up and left of the effect canvas's (as in
    /// [`crate::render_frames`]).
    pub fn blank(&self) -> Frame {
        let (w, h) = self.size();
        let corner = -(self.margin as i32);
        Frame::new(w, h, corner, corner)
    }

    /// Where its options place it on the clip's frame ([`Instance::placement`]).
    pub fn placement(&self) -> Placement {
        let o = &self.opts;
        Placement {
            x: o.x,
            y: o.y,
            flip: o.flip,
            angle: o.angle,
            light: o.light,
        }
    }

    /// The key of every frame the clip draws placed as `place`.
    pub fn key(&self, place: Placement) -> ClipKey {
        ClipKey::new(self.effect.meta().id, &self.opts, self.margin, place)
    }

    /// A clock for a new instance.
    pub fn clock(&self) -> Clock {
        Clock::new(self.effect, &self.opts)
    }

    /// The state of a new instance, to draw from.
    pub fn sim(&self) -> Sim {
        Sim::new(self.effect, self.opts.clone())
    }

    /// Clear `frame` (made by [`Clip::blank`]) and draw `layer` of `sim`'s frame into it, placed as `place` says
    /// and at opacity `fade` ([`Clock::fade`]). With margin 0, light is eased out at the canvas edge unless the
    /// effect is full-screen (as [`crate::render_frames`] does).
    pub fn draw(&self, sim: &Sim, frame: &mut Frame, layer: Layer, place: Placement, fade: f64) {
        frame.clear();
        sim.draw(frame, layer, place, fade);
        if self.margin == 0 && !self.effect.meta().is_screen() {
            edge_fade(frame);
        }
    }
}

/// An effect instance and the canvas it draws into: the instance's canvas ([`Effect::canvas`]: `meta.size`
/// unless its options size it) plus `margin` empty pixels on every side, with the effect's origin at the
/// canvas origin plus `margin`. While its clock is on frame k ([`Player::frame_index`]), `draw(Layer::Both)`
/// leaves the frame equal to [`crate::render_frames`] frame k at the same margin, whatever steps the clock
/// took: margin 0 eases light out at the canvas edge (as the sprite sheet does), a margin leaves room for glow
/// that spills past the canvas and fades nothing.
pub struct Player {
    clip: Clip,
    inst: Instance,
    frame: Frame,
    rgba: Vec<u8>,
}

impl Player {
    /// Spawn `effect` with `opts` on a canvas `margin` pixels bigger on every side; `opts.x` and `opts.y` are
    /// replaced by the effect's origin.
    pub fn new(effect: &'static dyn Effect, opts: Options, margin: u32) -> Self {
        Self::of(Clip::new(effect, opts, margin))
    }

    /// `effect` with `opts` as live hosts play it ([`Clip::live`]).
    pub fn live(effect: &'static dyn Effect, opts: Options) -> Self {
        Self::of(Clip::live(effect, opts))
    }

    /// A new instance of `clip`.
    pub fn of(clip: Clip) -> Self {
        let (w, h) = clip.size();
        Self {
            inst: Instance::new(clip.effect, clip.opts.clone()),
            frame: clip.blank(),
            rgba: vec![0; w as usize * h as usize * 4],
            clip,
        }
    }

    /// What it plays.
    pub fn clip(&self) -> &Clip {
        &self.clip
    }

    pub fn meta(&self) -> &'static Meta {
        self.clip.meta()
    }

    /// The canvas this instance draws on, margin not included.
    pub fn canvas(&self) -> Canvas {
        self.clip.canvas
    }

    /// Empty pixels round the effect's canvas on every side.
    pub fn margin(&self) -> u32 {
        self.clip.margin
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
            key: self.clip.key(place),
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
        self.clip.draw(
            self.inst.sim(),
            &mut self.frame,
            layer,
            place,
            self.inst.clock().fade(),
        );
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
        let all = self.whole();
        self.color_rgba_in(all)
    }

    /// The last drawn light layer alone as RGBA8 bytes (`w * h * 4`), as [`light_rgba_in`] makes them: drawn added,
    /// they add the light, on an opaque canvas or a transparent one, and nothing elsewhere.
    pub fn light_rgba(&mut self) -> &[u8] {
        let all = self.whole();
        self.light_rgba_in(all)
    }

    /// The smallest box holding every pixel the last drawn frame shows (colour alpha or light rgb not 0), or
    /// none when it shows nothing: the part of the frame a host keeps.
    pub fn lit_box(&self) -> Option<PixelBox> {
        PixelBox::lit(&self.frame)
    }

    /// As [`Player::color_rgba`] for the pixels in `b` alone (`b.w * b.h * 4` bytes, row-major), `b` clipped to
    /// the frame.
    pub fn color_rgba_in(&mut self, b: PixelBox) -> &[u8] {
        color_rgba_in(&self.frame, b, &mut self.rgba)
    }

    /// As [`Player::light_rgba`] for the pixels in `b` alone (`b.w * b.h * 4` bytes, row-major), `b` clipped to
    /// the frame.
    pub fn light_rgba_in(&mut self, b: PixelBox) -> &[u8] {
        light_rgba_in(&self.frame, b, &mut self.rgba)
    }

    /// The whole frame as a box.
    fn whole(&self) -> PixelBox {
        PixelBox {
            x: 0,
            y: 0,
            w: self.frame.w,
            h: self.frame.h,
        }
    }
}

/// A box of whole pixels on a player's frame: its top-left at (`x`, `y`), `w` × `h` pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelBox {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl PixelBox {
    /// The part of the box on a `w` × `h` frame.
    /// The smallest box holding every pixel `f` shows (colour alpha or light rgb not 0), or none when it shows
    /// nothing.
    pub fn lit(f: &Frame) -> Option<Self> {
        let w = f.w as usize;
        if w == 0 {
            return None;
        }
        let lit = |(c, l): (&u32, &u32)| c >> 24 != 0 || l & 0x00ff_ffff != 0;
        let mut found: Option<(usize, usize, usize, usize)> = None;
        for (y, (colors, lights)) in f.color.chunks(w).zip(f.light.chunks(w)).enumerate() {
            let mut row = colors.iter().zip(lights);
            let Some(first) = row.position(lit) else {
                continue;
            };
            let last = colors.iter().zip(lights).rposition(lit).unwrap_or(first);
            found = Some(match found {
                None => (first, y, last, y),
                Some((x0, y0, x1, _)) => (x0.min(first), y0, x1.max(last), y),
            });
        }
        let (x0, y0, x1, y1) = found?;
        Some(Self {
            x: x0 as u32,
            y: y0 as u32,
            w: (x1 - x0 + 1) as u32,
            h: (y1 - y0 + 1) as u32,
        })
    }

    fn within(self, w: u32, h: u32) -> Self {
        let x = self.x.min(w);
        let y = self.y.min(h);
        Self {
            x,
            y,
            w: self.w.min(w - x),
            h: self.h.min(h - y),
        }
    }
}

/// The colour of `frame`'s pixels in `b` (clipped to the frame) as straight-alpha RGBA8 bytes, row-major, written
/// into `out` (at least `b.w * b.h * 4` bytes): each ABGR word unpacked.
pub fn color_rgba_in<'a>(frame: &Frame, b: PixelBox, out: &'a mut [u8]) -> &'a [u8] {
    let b = b.within(frame.w, frame.h);
    unpack(&frame.color, frame.w, b, out, u32::to_le_bytes)
}

/// As [`color_rgba_in`] for the light, to draw added (`"lighter"`, or a premultiplied additive blend): a lit pixel
/// is light over transparency as [`composite_px`] makes it (alpha its brightest channel, rgb `light * 255 / alpha`),
/// an unlit one `0, 0, 0, 0`. Premultiplied it is the light itself, so it adds that light to an opaque canvas and
/// shows as the light's own colour, glow fading to clear, on a transparent one.
pub fn light_rgba_in<'a>(frame: &Frame, b: PixelBox, out: &'a mut [u8]) -> &'a [u8] {
    let b = b.within(frame.w, frame.h);
    unpack(&frame.light, frame.w, b, out, |word| {
        // most of a frame is unlit, and unlit is transparent: skip the blend for it
        if word & 0x00ff_ffff == 0 {
            [0; 4]
        } else {
            composite_px(0, word, None)
        }
    })
}

/// Write the words of `words` (rows of `stride`) inside `b` into `out` as RGBA8 bytes, row-major, each word
/// turned into bytes by `px`; returns the bytes written.
fn unpack<'a>(
    words: &[u32],
    stride: u32,
    b: PixelBox,
    out: &'a mut [u8],
    px: impl Fn(u32) -> [u8; 4],
) -> &'a [u8] {
    let (stride, x, w) = (stride as usize, b.x as usize, b.w as usize);
    let mut dst = out.as_chunks_mut::<4>().0.iter_mut();
    for y in b.y as usize..(b.y + b.h) as usize {
        let start = y * stride + x;
        // the row first: `zip` takes from it before `dst`, so a row's end leaves `dst` where it was
        for (&word, o) in words
            .get(start..start + w)
            .unwrap_or(&[])
            .iter()
            .zip(dst.by_ref())
        {
            *o = px(word);
        }
    }
    let n = w * b.h as usize * 4;
    out.get(..n).unwrap_or(&[])
}
