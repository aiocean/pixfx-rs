//! The pixel dissolve `disappear/fade` and `appear/fade` share: a sprite, read once from the actor it is handed,
//! drawn at a point of its fade-out. Each sprite pixel goes at its own moment, the edge's in the fade's first half and
//! the heart's in its second, a seeded hash of the pixel picking where in that half; it stays solid to the end,
//! lighting toward white over the last `LIT` of its time. The fade-out draws it forward; the fade-in draws the same
//! frames backward, so the heart arrives first and each pixel lands white and settles to its colour.

use std::sync::Arc;

use pixfx_core::{Actor, Canvas, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::hash;

/// Seconds from the whole sprite to nothing (or back).
pub(crate) const SECS: f64 = 0.8;

/// How much bigger the sprite is with none of it left, a share of its size (off by default: stretching a sprite
/// smears its pixels).
pub(crate) const GROW: NumOpt = NumOpt::new("grow", 0.0).within(0.0, 1.0);
/// How far toward white a pixel goes as it leaves or lands (0 keeps its colour).
pub(crate) const WHITEN: NumOpt = NumOpt::new("whiten", 0.5).within(0.0, 1.0);
/// The box the sprite and its swell fit in, centred on where the effect plays.
pub(crate) const W: NumOpt = NumOpt::new("w", 128.0).within(8.0, 512.0);
pub(crate) const H: NumOpt = NumOpt::new("h", 128.0).within(8.0, 512.0);

/// The share of the fade the heart waits for the edges: edge pixels go in the first half, the heart's in the second.
const HEART_WAIT: f64 = 0.5;
/// The share of the fade a pixel lights toward white before it is gone.
const LIT: f64 = 0.12;

/// The stand-in a preview dissolves: a disc this many pixels across, in the toolkit's gold.
const STAND_IN: i32 = 24;
const STAND_IN_COLOR: Rgba = Rgba::hex24(0xffd84a);

/// The canvas `w` x `h`, its origin in the middle.
pub(crate) fn canvas(opts: &Options) -> Canvas {
    let (w, h) = (opts.num(&W) as u32, opts.num(&H) as u32);
    Canvas {
        size: (w, h),
        origin: ((w / 2) as i32, (h / 2) as i32),
    }
}

/// One dissolve: its options and seed, and the sprite it dissolves (none: the stand-in disc).
pub(crate) struct Dissolve {
    seed: f64,
    grow: f64,
    whiten: f64,
    body: Option<Body>,
}

impl Dissolve {
    pub(crate) fn new(opts: &Options, seed: u64) -> Self {
        Self {
            seed: (seed & 0xffff_ffff) as f64,
            grow: opts.num(&GROW),
            whiten: opts.num(&WHITEN),
            body: None,
        }
    }

    pub(crate) fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.body = actor.and_then(|a| Body::of(a.opaque()));
    }

    /// The sprite `u` of the way through its fade-out (0 whole, 1 gone).
    pub(crate) fn draw(&self, p: &mut Painter, u: f64) {
        let u = u.clamp(0.0, 1.0);
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
                // each sprite pixel goes at its own moment: the edge's in the fade's first half, the heart's in its
                // second, a seeded hash of the pixel picking where in that half
                let r = (libm::hypot(sx - mx, sy - my) / body.reach).min(1.0);
                let from = (1.0 - r) * HEART_WAIT;
                let pick = f64::from(hash(&[libm::floor(sx), libm::floor(sy), self.seed]))
                    / 4_294_967_296.0;
                let when = from + (1.0 - HEART_WAIT) * pick;
                if u >= when {
                    continue;
                }
                // solid to the end, lighting toward white over the last `LIT` of its time
                let lit = ((u - (when - LIT)) / LIT).clamp(0.0, 1.0);
                let color = mix(c, WHITE, self.whiten * lit);
                let alpha = f64::from(c.3) / 255.0;
                p.each_game_cell(f64::from(x), f64::from(y), |p, sx, sy| {
                    p.blend_px(sx, sy, color, alpha);
                });
            }
        }
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
