//! `disappear/fade`: the sprite it wraps swells a little and fades out into light, its edges first and its heart
//! last: a soft exit for a spell that has spent itself, where `explosions/body-burst` blows a body apart. Made here,
//! the first disappear effect; its golden hashes pin it.
//!
//! It follows the sprite it is handed each frame (a shape still spinning keeps spinning as it fades); the swell and
//! the fade run from when it started. It does not replace its actor: the game hides the sprite when the fade starts
//! and may show it again later. With no actor it fades a stand-in disc.

use std::sync::Arc;

use pixfx_core::{Actor, Canvas, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};

pub struct Fade;

pub static FADE: Fade = Fade;

/// Seconds from the whole sprite to nothing.
const SECS: f64 = 0.45;

const META: Meta = Meta {
    id: "disappear/fade",
    name: "Fade",
    desc: "The sprite it wraps swells a little and fades out into light, its edges first and its heart last: a soft exit, nothing broken.",
    tags: &["disappear", "one-shot", "fade", "dissolve", "magic"],
    // `canvas` sizes it from `w` and `h`; this is their default
    size: (128, 128),
    origin: (64, 64),
    duration: SECS,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[GROW.spec(), WHITEN.spec(), W.spec(), H.spec()],
};

/// How much bigger the sprite is by the end, a share of its size.
const GROW: NumOpt = NumOpt::new("grow", 0.25).within(0.0, 1.0);
/// How far toward white a pixel goes as it fades (0 keeps its colour).
const WHITEN: NumOpt = NumOpt::new("whiten", 0.5).within(0.0, 1.0);
/// The box the sprite and its swell fit in, centred on where the effect plays.
const W: NumOpt = NumOpt::new("w", 128.0).within(8.0, 512.0);
const H: NumOpt = NumOpt::new("h", 128.0).within(8.0, 512.0);

/// The share of the fade the heart waits for the edges: the edge starts at once, the heart this far in.
const HEART_WAIT: f64 = 0.5;

/// The stand-in a preview fades: a disc this many pixels across, in the toolkit's gold.
const STAND_IN: i32 = 24;
const STAND_IN_COLOR: Rgba = Rgba::hex24(0xffd84a);

impl Effect for Fade {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn canvas(&self, opts: &Options) -> Canvas {
        let (w, h) = (opts.num(&W) as u32, opts.num(&H) as u32);
        Canvas {
            size: (w, h),
            origin: ((w / 2) as i32, (h / 2) as i32),
        }
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            grow: opts.num(&GROW),
            whiten: opts.num(&WHITEN),
            body: None,
        })
    }
}

/// The sprite as it showed when the fade started: its pixels on a grid, the grid's top-left in the effect's space,
/// its middle and how far its furthest pixel lies from it.
struct Body {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    px: Vec<Option<Rgba>>,
    mid: (f64, f64),
    reach: f64,
}

impl Body {
    fn of(pixels: impl IntoIterator<Item = (i32, i32, Rgba)>) -> Option<Body> {
        let pixels: Vec<(i32, i32, Rgba)> = pixels.into_iter().collect();
        let (x0, y0) = pixels
            .iter()
            .fold((i32::MAX, i32::MAX), |(a, b), &(x, y, _)| {
                (a.min(x), b.min(y))
            });
        let (x1, y1) = pixels
            .iter()
            .fold((i32::MIN, i32::MIN), |(a, b), &(x, y, _)| {
                (a.max(x), b.max(y))
            });
        if pixels.is_empty() {
            return None;
        }
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut px = vec![None; (w * h) as usize];
        for &(x, y, c) in &pixels {
            px[((y - y0) * w + (x - x0)) as usize] = Some(c);
        }
        let mid = (f64::from(x0 + x1 + 1) / 2.0, f64::from(y0 + y1 + 1) / 2.0);
        let reach = pixels
            .iter()
            .map(|&(x, y, _)| libm::hypot(f64::from(x) + 0.5 - mid.0, f64::from(y) + 0.5 - mid.1))
            .fold(1.0, f64::max);
        Some(Body {
            x: x0,
            y: y0,
            w,
            h,
            px,
            mid,
            reach,
        })
    }

    fn stand_in() -> Option<Body> {
        let r = STAND_IN / 2;
        Body::of(
            (-r..r)
                .flat_map(|y| (-r..r).map(move |x| (x, y)))
                .filter_map(|(x, y)| {
                    (libm::hypot(f64::from(x) + 0.5, f64::from(y) + 0.5) <= f64::from(r))
                        .then_some((x, y, STAND_IN_COLOR))
                }),
        )
    }

    /// Its pixel under (`x`, `y`) of the effect's space, none off it.
    fn at(&self, x: f64, y: f64) -> Option<Rgba> {
        let (i, j) = (
            libm::floor(x) as i32 - self.x,
            libm::floor(y) as i32 - self.y,
        );
        if i < 0 || j < 0 || i >= self.w || j >= self.h {
            return None;
        }
        self.px.get((j * self.w + i) as usize).copied().flatten()
    }
}

struct State {
    grow: f64,
    whiten: f64,
    body: Option<Body>,
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.body = actor.and_then(|a| Body::of(a.opaque()));
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let stand_in;
        let body = match &self.body {
            Some(b) => b,
            None => match Body::stand_in() {
                Some(b) => {
                    stand_in = b;
                    &stand_in
                }
                None => return,
            },
        };
        let u = (t / SECS).clamp(0.0, 1.0);
        // swelling fast, then slowing
        let scale = 1.0 + self.grow * (1.0 - (1.0 - u) * (1.0 - u));
        let (mx, my) = body.mid;
        let (hw, hh) = (
            f64::from(body.w) * scale / 2.0,
            f64::from(body.h) * scale / 2.0,
        );
        let (bx, by) = body_box(body, scale);
        for y in by.0..by.1 {
            for x in bx.0..bx.1 {
                // the pixel's centre, scaled back into the sprite about its middle
                let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                if (cx - mx).abs() > hw + 1.0 || (cy - my).abs() > hh + 1.0 {
                    continue;
                }
                let (sx, sy) = (mx + (cx - mx) / scale, my + (cy - my) / scale);
                let Some(c) = body.at(sx, sy) else { continue };
                // the edge fades from the start, the heart from `HEART_WAIT` in; each over the rest of the fade
                let r = (libm::hypot(sx - mx, sy - my) / body.reach).min(1.0);
                let from = (1.0 - r) * HEART_WAIT;
                let gone = ((u - from) / (1.0 - from)).clamp(0.0, 1.0);
                let a = f64::from(c.3) / 255.0 * (1.0 - gone);
                if a > 0.0 {
                    p.blend(
                        f64::from(x),
                        f64::from(y),
                        mix(c, WHITE, self.whiten * gone),
                        a,
                    );
                }
            }
        }
    }
}

/// The pixels the sprite covers at `scale` about its middle: columns and rows, ends exclusive.
fn body_box(body: &Body, scale: f64) -> ((i32, i32), (i32, i32)) {
    let (mx, my) = body.mid;
    let (hw, hh) = (
        f64::from(body.w) * scale / 2.0,
        f64::from(body.h) * scale / 2.0,
    );
    (
        (
            libm::floor(mx - hw) as i32 - 1,
            libm::ceil(mx + hw) as i32 + 1,
        ),
        (
            libm::floor(my - hh) as i32 - 1,
            libm::ceil(my + hh) as i32 + 1,
        ),
    )
}
