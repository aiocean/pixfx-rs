//! Blade pinwheel: four thin streaks fly in from the edges, bend into a pinwheel, wind into a bright ring and burst
//! back out as four shards that fly off and fade. Made here, by hand, from "Free pack, Part 1/13.png"
//! (no source code, so no parity check): its nine rows are the nine [`tone::RAMPS`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use crate::tone::{self, Ink, Knot, VARIANT, curve, pose};

pub struct BladePinwheel;

pub static BLADE_PINWHEEL: BladePinwheel = BladePinwheel;

/// Seconds a moment of the sheet lasts.
const MOMENT: f64 = 0.06;
const MOMENTS: f64 = 13.0;

const META: Meta = Meta {
    id: "arcane/blade-pinwheel",
    name: "Blade Pinwheel",
    desc: "Four streaks fly in, twist into a pinwheel and wind into a bright ring, then four shards burst out and fade.",
    tags: &["arcane", "magic", "burst", "pinwheel", "one-shot"],
    size: (96, 96),
    origin: (48, 48),
    duration: MOMENTS * MOMENT,
    looping: false,
    fps: 50,
    seed: 1,
    state: None,
    options: &[VARIANT.spec()],
};

/// The top blade (the others are it turned a quarter at a time) at moments 0 to 6: a streak falling in, bending,
/// a spiral arm, wound tight, a side of the bright ring, a quarter of the opened ring, an arm flung back out.
const BLADE: [[Knot; 6]; 7] = [
    [
        (-3.5, -31.5, 0.5, 1.0),
        (-3.5, -28.9, 0.5, 1.0),
        (-3.5, -26.3, 0.5, 1.0),
        (-3.5, -23.7, 0.5, 1.0),
        (-3.5, -21.1, 0.5, 1.0),
        (-3.5, -18.5, 0.5, 1.0),
    ],
    [
        (-3.0, -26.5, 0.5, 1.0),
        (-3.0, -21.5, 1.0, 3.0),
        (-3.0, -17.5, 1.0, 4.0),
        (-4.5, -13.5, 1.0, 4.0),
        (-6.5, -10.5, 1.0, 3.0),
        (-8.5, -6.5, 0.5, 1.0),
    ],
    [
        (-3.5, -18.5, 0.5, 2.0),
        (-4.5, -13.0, 1.2, 6.0),
        (-6.5, -7.0, 1.6, 8.0),
        (-6.5, -1.0, 1.6, 8.0),
        (-4.0, 3.0, 1.3, 6.0),
        (0.0, 4.5, 0.8, 5.0),
    ],
    [
        (-1.5, -8.5, 0.6, 4.0),
        (-4.0, -7.0, 1.2, 11.0),
        (-5.5, -4.5, 1.6, 13.0),
        (-5.5, -1.0, 1.6, 13.0),
        (-4.0, 2.0, 1.3, 12.0),
        (-1.5, 3.0, 0.8, 8.0),
    ],
    [
        (-4.5, -4.5, 2.0, 13.0),
        (-2.7, -4.5, 2.0, 13.0),
        (-0.9, -4.5, 2.0, 13.0),
        (0.9, -4.5, 2.0, 13.0),
        (2.7, -4.5, 2.0, 13.0),
        (4.5, -4.5, 2.0, 13.0),
    ],
    [
        (-1.7, -9.8, 0.5, 6.0),
        (1.35, -7.7, 1.3, 11.0),
        (3.75, -6.5, 1.3, 11.0),
        (5.75, -4.8, 1.3, 11.0),
        (7.05, -2.6, 1.3, 11.0),
        (7.5, 0.0, 0.8, 8.0),
    ],
    [
        (-1.5, -18.5, 0.5, 1.0),
        (-1.0, -15.0, 1.0, 4.0),
        (0.0, -11.0, 1.0, 5.0),
        (1.5, -8.0, 1.0, 6.0),
        (3.5, -5.0, 1.0, 5.0),
        (5.5, -2.0, 0.5, 1.0),
    ],
];

/// The moment the blades break into shards.
const BREAK: f64 = 6.5;
/// A shard's distance from the centre, its length and its tone, by moment.
const SHARD_OUT: [(f64, f64); 5] = [
    (BREAK, 11.0),
    (7.0, 14.0),
    (9.0, 22.0),
    (11.0, 26.5),
    (12.0, 29.0),
];
const SHARD_LEN: [(f64, f64); 4] = [(BREAK, 13.0), (9.0, 8.0), (11.0, 3.0), (12.0, 1.5)];
const SHARD_TONE: [(f64, f64); 5] = [
    (BREAK, 6.0),
    (7.0, 5.0),
    (9.0, 4.0),
    (11.0, 1.2),
    (12.0, 0.5),
];
/// How far a shard leans off the line from the centre (its top end leads to the left).
const SHARD_LEAN: f64 = 0.27;

impl Effect for BladePinwheel {
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
        if layer == Layer::Back {
            return;
        }
        let s = t / MOMENT;
        if s >= MOMENTS {
            return;
        }
        let mut ink = Ink::new(4).grain(0.3, 0.55, 13.0);
        if s < BREAK {
            ink.path(&pose(&BLADE, s));
        } else {
            let (d, len) = (curve(&SHARD_OUT, s), curve(&SHARD_LEN, s));
            let v = curve(&SHARD_TONE, s);
            let (r, h) = (if len > 4.0 { 1.0 } else { 0.5 }, len / 2.0);
            let x = -0.04 * d;
            ink.path(&[
                (x - SHARD_LEAN * h, -d - h, 0.5, v * 0.5),
                (x, -d, r, v),
                (x + SHARD_LEAN * h, -d + h, 0.5, v * 0.5),
            ]);
        }
        // the wound ring flares (light on its own pixels only, nothing outside them)
        let flare = curve(&[(2.0, 0.0), (3.5, 1.0), (5.0, 0.6), (6.0, 0.0)], s);
        ink.paint(p, &self.ramp, 32.0, 0.08 * (1.0 + 2.0 * flare));
    }
}
