//! The ordered-dither dissolve `disappear/dither` and `appear/dither` share: a sprite, read once from the actor it is
//! handed, thinned through the painter's 4x4 Bayer threshold (`Painter::dot`) so it breaks into an even checker, not
//! grain. The thinning sweeps across the sprite along `angle`: the side it starts from thins first, the far side
//! last, `spread` saying how far ahead the first side runs. A pixel lights toward white as it thins. The fade-out
//! draws it forward; the fade-in draws the same frames backward.

use std::sync::Arc;

use pixfx_core::{Actor, Canvas, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};

/// Seconds from the whole sprite to nothing (or back).
pub(crate) const SECS: f64 = 0.8;

/// The way the thinning travels, in degrees, y down: 90 starts at the top and ends at the feet, 0 runs left to right.
pub(crate) const ANGLE: NumOpt = NumOpt::new("angle", 90.0).within(0.0, 360.0);
/// How far ahead the first side thins, a share of the fade: 0 thins the whole sprite evenly, 0.9 has the first side
/// gone before the last begins.
pub(crate) const SPREAD: NumOpt = NumOpt::new("spread", 0.6).within(0.0, 0.9);
/// How far toward white a pixel goes as it thins (0 keeps its colour).
pub(crate) const WHITEN: NumOpt = NumOpt::new("whiten", 0.3).within(0.0, 1.0);
/// The box the sprite fits in, centred on where the effect plays.
pub(crate) const W: NumOpt = NumOpt::new("w", 128.0).within(8.0, 512.0);
pub(crate) const H: NumOpt = NumOpt::new("h", 128.0).within(8.0, 512.0);

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

/// One sprite pixel: where it is in the effect's space, its colour, and when along the sweep it thins (0 first,
/// 1 last).
#[derive(Clone, Copy)]
struct Px {
    x: f64,
    y: f64,
    c: Rgba,
    key: f64,
}

/// One dither dissolve: its options and the sprite's pixels (none handed: the stand-in disc).
pub(crate) struct Dither {
    dir: (f64, f64),
    spread: f64,
    whiten: f64,
    pixels: Vec<Px>,
}

impl Dither {
    pub(crate) fn new(opts: &Options) -> Self {
        let a = opts.num(&ANGLE).to_radians();
        let mut d = Self {
            dir: (libm::cos(a), libm::sin(a)),
            spread: opts.num(&SPREAD),
            whiten: opts.num(&WHITEN),
            pixels: Vec::new(),
        };
        d.read(stand_in());
        d
    }

    pub(crate) fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        match actor {
            Some(a) => self.read(a.opaque().collect()),
            None => self.read(stand_in()),
        }
    }

    /// Keep the sprite's pixels, each keyed by how far along `dir` it lies across the sprite.
    fn read(&mut self, pixels: Vec<(i32, i32, Rgba)>) {
        let (dx, dy) = self.dir;
        let along = |x: i32, y: i32| (f64::from(x) + 0.5) * dx + (f64::from(y) + 0.5) * dy;
        let (lo, hi) = pixels
            .iter()
            .map(|&(x, y, _)| along(x, y))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        let span = hi - lo;
        self.pixels = pixels
            .into_iter()
            .map(|(x, y, c)| Px {
                x: f64::from(x),
                y: f64::from(y),
                c,
                key: if span > 0.0 {
                    (along(x, y) - lo) / span
                } else {
                    0.0
                },
            })
            .collect();
    }

    /// The sprite `u` of the way through its fade-out (0 whole, 1 gone).
    pub(crate) fn draw(&self, p: &mut Painter, u: f64) {
        let u = u.clamp(0.0, 1.0);
        let rest = 1.0 - self.spread;
        for px in &self.pixels {
            // how far this pixel has thinned: it starts once the sweep reaches it and takes the rest of the fade
            let gone = ((u - px.key * self.spread) / rest).clamp(0.0, 1.0);
            if gone >= 1.0 {
                continue;
            }
            p.dot(px.x, px.y, mix(px.c, WHITE, self.whiten * gone), 1.0 - gone);
        }
    }
}

/// The stand-in disc's pixels, centred on the origin.
fn stand_in() -> Vec<(i32, i32, Rgba)> {
    let r = STAND_IN / 2;
    (-r..r)
        .flat_map(|y| (-r..r).map(move |x| (x, y)))
        .filter(|&(x, y)| libm::hypot(f64::from(x) + 0.5, f64::from(y) + 0.5) <= f64::from(r))
        .map(|(x, y)| (x, y, STAND_IN_COLOR))
        .collect()
}
