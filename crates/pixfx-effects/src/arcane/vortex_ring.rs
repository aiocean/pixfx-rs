//! Vortex ring: a small four-point spark turns into a pinwheel, knots into loops, whirls out
//! into a vortex and flashes into a full bright ring round a square of loops; the ring then shrinks, breaks into
//! shards that scatter outward and fade. Made here, by hand, from "Free pack, Part 1/03.png" (no source code,
//! so no parity check): its nine rows are the nine [`tone::RAMPS`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use crate::tone::{self, Ink, Knot, VARIANT, curve, pose};

pub struct VortexRing;

pub static VORTEX_RING: VortexRing = VortexRing;

/// Seconds a moment of the sheet lasts.
const MOMENT: f64 = 0.06;
const MOMENTS: f64 = 13.0;

const META: Meta = Meta {
    id: "arcane/vortex-ring",
    name: "Vortex Ring",
    desc: "A spark winds into a pinwheel and a whirling vortex, flashes into a bright ring, then breaks into shards that scatter.",
    tags: &["arcane", "magic", "vortex", "ring", "burst", "one-shot"],
    size: (96, 96),
    origin: (48, 48),
    duration: MOMENTS * MOMENT,
    looping: false,
    fps: 50,
    seed: 1,
    state: None,
    options: &[VARIANT.spec()],
};

/// Degrees to radians.
const DEG: f64 = core::f64::consts::PI / 180.0;

/// One arm (the others are it turned a quarter at a time) at moments 0 to 4: a point of the spark, a comma of
/// the pinwheel, a loop of the knot, an arm of the vortex.
const ARM: [[Knot; 6]; 5] = [
    [
        (-1.5, -5.5, 0.5, 0.0),
        (-1.5, -4.5, 0.5, 0.0),
        (-1.5, -3.5, 0.5, 0.0),
        (-1.5, -2.5, 0.5, 0.0),
        (-2.0, -2.0, 0.5, 0.0),
        (-2.5, -1.5, 0.5, 0.0),
    ],
    [
        (-9.0, -9.5, 0.5, 1.0),
        (-6.0, -9.0, 0.6, 1.0),
        (-3.5, -7.5, 0.6, 1.0),
        (-2.0, -5.5, 0.6, 1.0),
        (-1.5, -3.5, 0.5, 1.0),
        (-1.5, -2.0, 0.5, 0.5),
    ],
    [
        (-13.0, -5.0, 0.6, 2.0),
        (-12.0, -8.0, 1.0, 3.0),
        (-10.0, -10.5, 1.3, 3.0),
        (-7.0, -11.0, 1.2, 3.0),
        (-4.0, -9.5, 0.8, 2.0),
        (-2.0, -8.0, 0.5, 1.0),
    ],
    [
        (-10.0, -6.0, 1.8, 6.0),
        (-12.0, -11.0, 2.0, 6.0),
        (-7.0, -15.0, 2.0, 6.0),
        (-1.0, -13.0, 2.0, 6.0),
        (1.0, -7.0, 1.8, 6.0),
        (-2.0, -3.0, 1.2, 5.0),
    ],
    [
        (-22.0, -10.0, 1.5, 6.0),
        (-15.0, -17.0, 3.0, 11.0),
        (-5.0, -19.0, 3.5, 13.0),
        (4.0, -16.0, 3.5, 15.0),
        (9.0, -9.0, 3.0, 15.0),
        (8.0, -3.0, 2.0, 13.0),
    ],
];
/// The moment the vortex becomes the ring.
const RING_FROM: f64 = 4.4;

/// The ring by moment: radius, half-width and tone; it closes whole, then shrinks and breaks.
const RING_R: [(f64, f64); 5] = [
    (4.0, 22.0),
    (5.0, 20.5),
    (6.0, 21.0),
    (8.0, 21.0),
    (9.0, 22.0),
];
const RING_W: [(f64, f64); 6] = [
    (4.0, 1.0),
    (5.0, 6.0),
    (6.0, 5.0),
    (7.0, 4.0),
    (8.0, 2.5),
    (8.8, 0.0),
];
const RING_TONE: [(f64, f64); 6] = [
    (4.0, 10.0),
    (5.0, 15.0),
    (6.0, 13.0),
    (7.0, 11.0),
    (8.0, 8.0),
    (8.8, 6.0),
];
/// The square of loops inside the ring.
const LOOPS: [(f64, f64); 5] = [(4.5, -1.0), (5.0, 8.0), (6.0, 6.0), (7.0, 4.0), (8.0, -1.0)];

/// The shards the ring breaks into, six a quarter: distance, length and tone by moment.
const SHARDS_FROM: f64 = 6.5;
const BREAK: f64 = 7.5;
const SHARD_R: [(f64, f64); 3] = [(6.5, 20.0), (9.0, 19.5), (12.0, 17.5)];
const SHARD_LEN: [(f64, f64); 4] = [(6.5, 6.0), (9.0, 5.0), (11.0, 3.0), (12.0, 1.5)];
const SHARD_TONE: [(f64, f64); 6] = [
    (6.5, 9.0),
    (8.0, 7.0),
    (9.0, 5.0),
    (10.0, 3.0),
    (11.0, 1.5),
    (12.0, 0.7),
];

impl Effect for VortexRing {
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
        let mut ink = Ink::new(4).grain(0.3, 0.6, 3.0);
        if s < RING_FROM {
            ink.path(&pose(&ARM, s));
        }
        let w = curve(&RING_W, s);
        if s >= RING_FROM && w > 0.0 {
            let (r, v) = (curve(&RING_R, s), curve(&RING_TONE, s));
            // whole until it breaks, then gaps open between the shards
            let span = curve(&[(BREAK - 0.5, 46.0), (BREAK + 0.5, 30.0)], s);
            ink.arc(
                (0.0, 0.0),
                r,
                (-90.0 - span) * DEG,
                (-90.0 + span) * DEG,
                (w, w),
                (v, v),
            );
        }
        let loops = curve(&LOOPS, s);
        if loops >= 0.0 {
            ink.seg((-10.0, -10.0), (10.0, -10.0), (0.8, 0.8), (loops, loops));
            ink.arc(
                (0.0, 0.0),
                7.0,
                -150.0 * DEG,
                -30.0 * DEG,
                (0.6, 0.6),
                (loops, loops),
            );
        }
        if s >= SHARDS_FROM {
            let (len, v) = (curve(&SHARD_LEN, s), curve(&SHARD_TONE, s));
            for j in 0..6 {
                let a = (-125.0 + 15.0 * j as f64 + 12.0 * (s - 7.0)) * DEG;
                // in and out of the ring by turns, so they read as a scattered band
                let r = curve(&SHARD_R, s) + [-5.0, 3.0, -1.0, 5.0, -4.0, 1.0][j];
                let (cx, cy) = (r * libm::cos(a), r * libm::sin(a));
                // along the ring, leaning outward
                let d = a + 60.0 * DEG;
                let (dx, dy) = (libm::cos(d) * len / 2.0, libm::sin(d) * len / 2.0);
                ink.path(&[
                    (cx - dx, cy - dy, 0.5, v * 0.6),
                    (cx, cy, 1.0, v),
                    (cx + dx, cy + dy, 0.5, v * 0.6),
                ]);
            }
        }
        // the ring flares (light on its own pixels only, nothing outside them)
        let flare = curve(&[(3.5, 0.0), (5.0, 1.0), (6.5, 0.4), (7.5, 0.0)], s);
        ink.paint(p, &self.ramp, 32.0, 0.08 * (1.0 + 2.0 * flare));
    }
}
