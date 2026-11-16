//! Puff burst: a small puff pops up from the ground, swells into a heart-shaped cloud and splits:
//! two lobes rise, two puffs roll out to the sides, and each hollows out from below until only its rim is left,
//! fading. Made here, by hand, from "Free pack, Part 1/25.png" (no source code,
//! so no parity check): its nine rows are the nine [`tone::RAMPS`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use crate::tone::{self, Knot, VARIANT, curve, pose};

pub struct PuffBurst;

pub static PUFF_BURST: PuffBurst = PuffBurst;

/// Seconds a moment of the sheet lasts.
const MOMENT: f64 = 0.06;
const MOMENTS: f64 = 14.0;

const META: Meta = Meta {
    id: "explosions/puff-burst",
    name: "Puff Burst",
    desc: "A puff pops from the ground, swells into a cloud and splits; the puffs rise and roll out, hollow out and fade.",
    tags: &["explosion", "smoke", "puff", "cloud", "one-shot"],
    size: (96, 96),
    origin: (48, 80),
    duration: MOMENTS * MOMENT,
    looping: false,
    fps: 50,
    seed: 1,
    state: None,
    options: &[VARIANT.spec()],
};

/// The sheet draws the puff 26 pixels above its foot (the origin).
const FOOT: f64 = 26.0;

/// The four puffs at each moment, relative to the sheet's centre (top-left lobe, top-right lobe, left, right):
/// centre, radius, tone. Radius 0: not yet there.
const PUFFS: [[Knot; 4]; 14] = [
    [
        (-1.0, 8.0, 0.0, 0.0),
        (1.5, 15.5, 5.0, 15.0),
        (-2.0, 12.0, 0.0, 0.0),
        (6.0, 10.0, 0.0, 0.0),
    ],
    [
        (-1.0, 8.0, 0.0, 0.0),
        (3.0, 14.0, 8.0, 15.0),
        (-2.0, 12.0, 0.0, 0.0),
        (6.0, 10.0, 0.0, 0.0),
    ],
    [
        (-3.0, 7.0, 6.0, 15.0),
        (4.0, 8.0, 9.0, 15.0),
        (-5.0, 10.0, 4.0, 13.0),
        (8.0, 8.0, 4.0, 13.0),
    ],
    [
        (-8.0, 1.0, 9.0, 15.0),
        (6.0, 0.0, 10.0, 15.0),
        (-8.0, 7.0, 8.0, 15.0),
        (12.0, 6.0, 9.0, 15.0),
    ],
    [
        (-6.0, -6.0, 7.5, 15.0),
        (3.0, -6.0, 7.0, 15.0),
        (-14.0, 5.0, 7.5, 13.0),
        (13.0, 8.0, 11.0, 15.0),
    ],
    [
        (-10.0, -6.0, 8.0, 15.0),
        (6.0, -8.0, 11.0, 15.0),
        (-18.5, 10.0, 7.0, 15.0),
        (18.0, 10.0, 8.0, 15.0),
    ],
    [
        (-10.0, -9.0, 7.0, 14.0),
        (6.0, -11.0, 10.0, 15.0),
        (-21.0, 11.0, 7.0, 11.0),
        (21.0, 12.0, 7.0, 12.0),
    ],
    [
        (-10.0, -12.0, 6.0, 8.0),
        (5.0, -14.0, 9.0, 14.0),
        (-24.0, 12.0, 6.0, 6.0),
        (24.0, 12.0, 6.0, 11.0),
    ],
    [
        (-10.0, -13.0, 7.0, 4.0),
        (5.0, -15.0, 9.0, 7.0),
        (-25.0, 12.0, 6.0, 3.0),
        (25.0, 12.0, 6.0, 6.0),
    ],
    [
        (-10.0, -14.0, 7.0, 2.5),
        (5.0, -16.0, 9.0, 3.5),
        (-26.0, 12.0, 6.0, 1.2),
        (26.5, 12.0, 6.0, 2.0),
    ],
    [
        (-10.0, -14.5, 7.0, 1.2),
        (5.0, -16.5, 9.0, 1.5),
        (-26.0, 12.0, 6.0, 0.5),
        (27.0, 12.0, 6.0, 0.8),
    ],
    [
        (-10.0, -15.0, 7.0, 0.5),
        (5.0, -17.0, 9.0, 0.7),
        (-26.0, 12.0, 6.0, 0.2),
        (27.0, 12.0, 6.0, 0.3),
    ],
    [
        (-10.0, -15.0, 7.0, 0.2),
        (5.0, -17.0, 9.0, 0.3),
        (-26.0, 12.0, 6.0, 0.0),
        (27.0, 12.0, 6.0, 0.0),
    ],
    [
        (-10.0, -15.0, 7.0, 0.0),
        (5.0, -17.0, 9.0, 0.0),
        (-26.0, 12.0, 6.0, 0.0),
        (27.0, 12.0, 6.0, 0.0),
    ],
];
/// Each puff's hollow at each moment, its radius (the same four, in order), and where it sits in its puff, in
/// shares of the puff's radius: low in the two lobes, so their tops stay longest; high and inward in the two side
/// puffs, so their outer, lower edges stay.
const HOLLOW_AT: [(f64, f64); 4] = [(0.0, 0.35), (0.0, 0.35), (0.3, -0.3), (-0.3, -0.3)];
const HOLES: [[Knot; 1]; 14] = [
    [(0.0, 0.0, 0.0, 0.0)],
    [(0.0, 0.0, 0.0, 0.0)],
    [(0.0, 0.0, 0.0, 0.0)],
    [(0.0, 0.0, 0.0, 0.0)],
    [(0.0, 0.0, 3.0, 0.0)],
    [(0.0, 0.0, 0.0, 0.0)],
    [(1.0, 1.0, 4.0, 3.0)],
    [(4.0, 4.0, 5.0, 3.0)],
    [(6.5, 7.0, 5.5, 4.5)],
    [(7.0, 8.5, 5.7, 5.3)],
    [(7.2, 9.0, 5.8, 5.6)],
    [(7.5, 9.3, 5.9, 5.8)],
    [(7.5, 9.5, 6.0, 6.0)],
    [(7.5, 9.5, 6.0, 6.0)],
];
/// The stalk under the first puff, from its top to the ground: top (x, y), half-width, tone.
const STALK: [[Knot; 1]; 5] = [
    [(1.5, 19.0, 1.2, 10.0)],
    [(3.0, 20.0, 1.2, 8.0)],
    [(3.0, 15.0, 1.0, 6.0)],
    [(2.0, 18.0, 0.6, 3.0)],
    [(2.0, 20.0, 0.0, 0.0)],
];
/// How deep a puff's dark rim is, and how dark.
const RIM: f64 = 1.5;
const RIM_KEEP: f64 = 0.62;

impl Effect for PuffBurst {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            ramp: tone::ramp(opts),
        })
    }
}

struct State {
    ramp: Vec<Rgba>,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let s = t / MOMENT;
        if layer == Layer::Back || s >= MOMENTS {
            return;
        }
        let puffs = pose(&PUFFS, s);
        let [hole] = pose(&HOLES, s);
        let holes = [hole.0, hole.1, hole.2, hole.3];
        let [stalk] = pose(&STALK, s);
        let ramp = &self.ramp;
        let flare = curve(&[(0.0, 0.5), (2.0, 1.0), (5.0, 0.6), (7.0, 0.0)], s);
        p.shade(-32.0, -32.0 - FOOT, 31.0, 31.0 - FOOT, |x, y, o| {
            let y = y + FOOT;
            let mut best: Option<f64> = None;
            for ((k, h), at) in puffs.iter().zip(holes).zip(HOLLOW_AT) {
                let d = libm::hypot(x - k.0, y - k.1);
                if k.2 <= 0.0 || d > k.2 {
                    continue;
                }
                let dh = libm::hypot(x - (k.0 + k.2 * at.0), y - (k.1 + k.2 * at.1));
                if h > 0.0 && dh < h {
                    continue;
                }
                let edge = (k.2 - d).min(if h > 0.0 { dh - h } else { f64::INFINITY });
                let v = if edge < RIM { k.3 * RIM_KEEP } else { k.3 };
                best = Some(best.map_or(v, |b: f64| b.max(v)));
            }
            if stalk.2 > 0.0 && libm::fabs(x - stalk.0) <= stalk.2 && y >= stalk.1 && y <= FOOT {
                // it thins and dims toward the ground
                let f = (y - stalk.1) / (FOOT - stalk.1).max(1.0);
                let v = stalk.3 * (1.0 - 0.7 * f);
                best = Some(best.map_or(v, |b: f64| b.max(v)));
            }
            let v = best?;
            o.k = 0.08 * flare * (v - 10.0).max(0.0);
            Some(tone::color(ramp, v / tone::LAST))
        });
    }
}
