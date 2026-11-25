//! Rendering an effect's frames: port of pixelfx `tools/lib/fxlib.mjs` `renderFrames` (the bare render: no
//! backdrop, no stand-in actor) and `edgeFade`.

use std::sync::Arc;

use crate::actor::Actor;
use crate::effect::{Effect, Instance, Layer, Options};
use pixfx_draw::Painter;
use pixfx_draw::frame::Frame;
use pixfx_draw::math::js_round;

/// An effect's frames, each `w` x `h` raster texels. Size, origin and margin use the requested raster density.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    pub w: u32,
    pub h: u32,
    pub fps: u32,
    /// Where the effect's origin sits on its canvas (the canvas the options gave it).
    pub origin: (i32, i32),
    /// Empty pixels added round the effect's canvas on every side; the pivot is `origin + margin`.
    pub margin: u32,
    pub frames: Vec<Frame>,
}

/// Pixels over which [`edge_fade`] eases light out at the canvas edge.
const EDGE: u32 = 8;

/// Render every frame of one loop or one-shot ([`crate::effect::Meta::frames`]): frame k is the effect after k
/// ticks of `1 / fps`, the frame a live [`Instance`] shows while its clock is on it.
/// The effect sits at its origin on the canvas `opts` give it ([`Effect::canvas`]) whatever `opts.x` and
/// `opts.y` say. Without a margin, light is eased out at the canvas edge (full-screen effects excepted) so a
/// glow cut off by the edge does not show as a box.
pub fn render_frames(effect: &'static dyn Effect, opts: &Options, margin: u32) -> Rendered {
    render_frames_with_actor(effect, opts, margin, |_| None)
}

/// [`render_frames`] of an effect playing on a character: frame k is drawn after the state has stepped to it with
/// `actor(k)`, the character's sprite as it shows on frame k, handed over before frame k's steps.
pub fn render_frames_with_actor(
    effect: &'static dyn Effect,
    opts: &Options,
    margin: u32,
    actor: impl Fn(u64) -> Option<Arc<Actor>>,
) -> Rendered {
    let meta = effect.meta();
    let canvas = effect.canvas(opts);
    let (w, h) = (canvas.size.0 + margin * 2, canvas.size.1 + margin * 2);
    let density = opts.pixel_density.factor();
    let n = meta.frames();
    let at_origin = Options {
        x: f64::from(canvas.origin.0),
        y: f64::from(canvas.origin.1),
        ..opts.clone()
    };
    let mut inst = Instance::new(effect, at_origin);
    let fade = margin == 0 && !meta.is_screen();
    let corner = -(margin as i32);
    let mut frames = Vec::with_capacity(n as usize);
    for k in 0..n {
        // unclamped by MAX_DT: renders step exactly as pixelfx, whatever the fps
        inst.set_actor(actor(k));
        inst.seek(k);
        let mut frame = Frame::with_density(w, h, corner, corner, opts.pixel_density);
        inst.draw(&mut frame, Layer::Both, inst.placement());
        if fade {
            edge_fade(&mut frame);
        }
        frames.push(frame);
    }
    Rendered {
        w: w * density,
        h: h * density,
        fps: meta.fps,
        origin: (
            canvas.origin.0 * density as i32,
            canvas.origin.1 * density as i32,
        ),
        margin: margin * density,
        frames,
    }
}

/// Every frame of `effect` on its own canvas with the sprite it wraps drawn too: its back layer, the sprite
/// (`actor(k)` at frame `k`, placed in the effect's space and handed to it), then its front layer, as a game shows
/// them. For previews of a sprite-aware effect, which [`render_frames_with_actor`] leaves with a hole where the
/// sprite stands.
pub fn render_frames_on_actor(
    effect: &'static dyn Effect,
    opts: &Options,
    actor: impl Fn(u64) -> Option<Arc<Actor>>,
) -> Rendered {
    render_frames_on_actor_with(effect, opts, actor, |p, a| {
        for (x, y, c) in a.opaque() {
            p.paint_cell(f64::from(x), f64::from(y), c, 1.0);
        }
    })
}

/// Render around an actor with a separate visual raster. The effect still reads the actor in game pixels.
pub fn render_frames_on_actor_with(
    effect: &'static dyn Effect,
    opts: &Options,
    actor: impl Fn(u64) -> Option<Arc<Actor>>,
    draw_actor: impl Fn(&mut Painter<'_>, &Actor),
) -> Rendered {
    let meta = effect.meta();
    let canvas = effect.canvas(opts);
    let (w, h) = canvas.size;
    let density = opts.pixel_density.factor();
    let (ox, oy) = (f64::from(canvas.origin.0), f64::from(canvas.origin.1));
    let at_origin = Options {
        x: ox,
        y: oy,
        ..opts.clone()
    };
    let mut inst = Instance::new(effect, at_origin);
    let fade = !meta.is_screen();
    let n = meta.frames();
    let mut frames = Vec::with_capacity(n as usize);
    for k in 0..n {
        let sprite = actor(k);
        inst.set_actor(sprite.clone());
        inst.seek(k);
        let mut frame = Frame::with_density(w, h, 0, 0, opts.pixel_density);
        inst.draw(&mut frame, Layer::Back, inst.placement());
        if let Some(a) = &sprite {
            let mut p = Painter::new(&mut frame);
            p.place(ox, oy, false, 0.0);
            draw_actor(&mut p, a);
        }
        inst.draw(&mut frame, Layer::Front, inst.placement());
        if fade {
            edge_fade(&mut frame);
        }
        frames.push(frame);
    }
    Rendered {
        w: w * density,
        h: h * density,
        fps: meta.fps,
        origin: (
            canvas.origin.0 * density as i32,
            canvas.origin.1 * density as i32,
        ),
        margin: 0,
        frames,
    }
}

/// Ease the light out over the last [`EDGE`] pixels of the frame, in place; colour is left alone
/// (pixelfx `edgeFade`). [`render_frames`] applies it to margin-0 renders of effects that are not full-screen;
/// a live host showing an effect on its own canvas applies it the same way to match them.
pub fn edge_fade(f: &mut Frame) {
    let (w, h) = (i64::from(f.w), i64::from(f.h));
    let e_max = i64::from(EDGE * f.density().factor());
    for y in 0..h {
        let ey = y.min(h - 1 - y);
        let mut x = 0;
        while x < w {
            let e = ey.min(x).min(w - 1 - x);
            if e >= e_max {
                // the rest of the row up to the far edge band is untouched
                x = w - e_max;
                continue;
            }
            let i = (y * w + x) as usize;
            x += 1;
            let Some(px) = f.light.get_mut(i) else {
                continue;
            };
            let l = *px;
            if l & 0x00ff_ffff == 0 {
                continue;
            }
            let u = (e as f64 + 0.5) / e_max as f64;
            let k = u * u * (3.0 - 2.0 * u);
            let ch = |s: u32| js_round(f64::from((l >> s) & 255) * k) as u32;
            *px = (l & 0xff00_0000) | ch(0) | (ch(8) << 8) | (ch(16) << 16);
        }
    }
}
