//! Comma seal: four commas grow into arcs that close a ring, curl inward into a bright square seal,
//! burst out as four forked blades and drift off as fading petals. Made here, by hand, from "Free pack, Part 1/16.png" (no source code,
//! so no parity check): its nine rows are the nine [`tone::RAMPS`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use crate::tone::{self, Ink, Knot, VARIANT, curve, pose};

pub struct CommaSeal;

pub static COMMA_SEAL: CommaSeal = CommaSeal;

/// Seconds a moment of the sheet lasts.
const MOMENT: f64 = 0.06;
const MOMENTS: f64 = 14.0;

const META: Meta = Meta {
    id: "arcane/comma-seal",
    name: "Comma Seal",
    desc: "Four commas close a ring, curl in to a bright square seal, then burst out as forked blades and fade as petals.",
    tags: &["arcane", "magic", "seal", "burst", "one-shot"],
    size: (96, 96),
    origin: (48, 48),
    duration: MOMENTS * MOMENT,
    looping: false,
    fps: 50,
    seed: 1,
    state: None,
    options: &[VARIANT.spec()],
};

/// How far out the whole seal sits, as a share of the drawn keys, by moment: the sheet's seal breathes, swelling as
/// it opens, drawing in to its tightest as the blades turn (moment 8), then flaring out again as it breaks up.
const BREATH: [(f64, f64); 14] = [
    (0.0, 1.0),
    (1.0, 1.03),
    (2.0, 1.07),
    (3.0, 1.11),
    (4.0, 1.11),
    (5.0, 1.04),
    (6.0, 0.965),
    (7.0, 0.93),
    (8.0, 0.86),
    (9.0, 1.0),
    (10.0, 1.1),
    (11.0, 1.1),
    (12.0, 1.0),
    (13.0, 0.97),
];

/// Degrees to radians.
const DEG: f64 = core::f64::consts::PI / 180.0;

/// The top comma (the others are it turned a quarter at a time) at each moment: a comma, an arc, the top of the
/// ring with its knob, curling in, the seal's edge, a forked blade, a fading petal.
const COMMA: [[Knot; 6]; 14] = [
    [
        (-15.5, -12.5, 0.6, 3.0),
        (-15.0, -11.5, 1.2, 6.0),
        (-14.3, -10.3, 1.5, 7.0),
        (-13.6, -9.1, 1.5, 7.0),
        (-12.9, -7.9, 1.2, 6.0),
        (-12.0, -7.0, 0.6, 3.0),
    ],
    [
        (-24.5, -17.0, 0.8, 3.0),
        (-21.0, -18.0, 1.5, 6.0),
        (-17.5, -16.0, 1.8, 7.0),
        (-15.0, -12.5, 1.8, 7.0),
        (-13.5, -9.0, 1.5, 6.0),
        (-13.0, -7.0, 0.8, 3.0),
    ],
    [
        (-11.0, -25.0, 1.2, 6.0),
        (-3.0, -27.0, 2.0, 8.0),
        (6.0, -27.0, 2.2, 9.0),
        (14.0, -25.0, 2.0, 8.0),
        (18.0, -19.0, 1.7, 7.0),
        (17.0, -9.0, 1.0, 5.0),
    ],
    [
        (-24.4, -8.9, 1.0, 5.0),
        (-18.4, -18.4, 1.2, 6.0),
        (-8.9, -24.4, 1.3, 6.0),
        (2.3, -25.9, 1.6, 8.0),
        (14.0, -22.0, 3.5, 15.0),
        (17.0, -18.0, 2.0, 12.0),
    ],
    [
        (13.0, -22.0, 0.8, 6.0),
        (9.0, -19.0, 3.5, 15.0),
        (4.0, -16.0, 3.0, 13.0),
        (0.0, -13.0, 2.0, 10.0),
        (-3.0, -10.0, 1.2, 7.0),
        (-4.0, -7.0, 0.5, 4.0),
    ],
    [
        (8.0, -19.0, 0.8, 6.0),
        (5.0, -16.0, 3.0, 15.0),
        (1.0, -13.0, 2.5, 13.0),
        (-2.0, -10.0, 1.8, 11.0),
        (-4.0, -7.0, 1.0, 8.0),
        (-4.5, -4.5, 0.5, 5.0),
    ],
    [
        (0.0, 0.0, 4.0, 15.0),
        (1.0, -4.0, 4.5, 15.0),
        (2.0, -8.0, 4.0, 15.0),
        (3.0, -11.0, 3.0, 14.0),
        (4.0, -13.0, 2.0, 12.0),
        (6.0, -15.0, 1.0, 9.0),
    ],
    [
        (-8.0, -7.0, 6.5, 15.0),
        (-4.0, -7.5, 6.5, 15.0),
        (0.0, -7.5, 6.5, 15.0),
        (4.0, -7.0, 6.5, 15.0),
        (7.0, -5.5, 6.5, 15.0),
        (8.0, -4.0, 6.0, 14.0),
    ],
    [
        (-13.0, -17.0, 1.5, 10.0),
        (-6.0, -17.0, 2.5, 15.0),
        (2.0, -16.0, 2.5, 15.0),
        (10.0, -14.0, 2.5, 14.0),
        (17.0, -11.0, 2.0, 12.0),
        (22.0, -7.0, 1.0, 8.0),
    ],
    [
        (-24.0, -15.0, 2.0, 13.0),
        (-19.0, -16.0, 3.0, 15.0),
        (-13.0, -16.0, 2.5, 14.0),
        (-5.0, -16.0, 1.8, 11.0),
        (2.0, -15.5, 1.2, 8.0),
        (8.0, -15.0, 0.8, 5.0),
    ],
    [
        (8.0, -29.0, 1.0, 8.0),
        (10.0, -25.0, 2.5, 12.0),
        (11.0, -21.0, 1.8, 9.0),
        (11.0, -17.0, 1.0, 5.0),
        (10.0, -14.0, 0.5, 2.0),
        (9.0, -12.0, 0.5, 1.0),
    ],
    [
        (9.0, -30.0, 0.5, 3.0),
        (10.5, -27.5, 1.5, 5.0),
        (11.0, -24.0, 1.0, 4.0),
        (11.0, -21.0, 0.5, 2.0),
        (10.5, -19.0, 0.5, 1.0),
        (10.0, -17.0, 0.5, 0.0),
    ],
    [
        (9.0, -30.0, 0.5, 1.0),
        (9.5, -29.0, 0.5, 1.0),
        (10.0, -28.0, 0.5, 1.0),
        (10.2, -27.0, 0.5, 1.0),
        (10.4, -26.0, 0.5, 1.0),
        (10.5, -25.0, 0.5, 0.0),
    ],
    [
        (10.0, -28.5, 0.5, 0.0),
        (10.0, -28.3, 0.5, 0.0),
        (10.0, -28.1, 0.5, 0.0),
        (10.0, -27.9, 0.5, 0.0),
        (10.0, -27.7, 0.5, 0.0),
        (10.0, -27.5, 0.5, 0.0),
    ],
];

/// The faint broken ring the commas leave behind: tone and radius by moment.
const HOOP: [(f64, f64); 7] = [
    (2.5, -1.0),
    (3.0, 5.0),
    (5.0, 4.0),
    (7.0, 3.0),
    (9.0, 2.0),
    (11.0, 1.0),
    (12.0, -1.0),
];
const HOOP_R: [(f64, f64); 3] = [(3.0, 27.0), (8.0, 28.0), (12.0, 30.0)];

impl Effect for CommaSeal {
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
        let mut ink = Ink::new(4).grain(0.2, 0.65, 16.0);
        let breath = curve(&BREATH, s);
        ink.path(&pose(&COMMA, s).map(|k| (k.0 * breath, k.1 * breath, k.2, k.3)));
        let hoop = curve(&HOOP, s);
        if hoop >= 0.0 {
            let (r, a) = (curve(&HOOP_R, s) * breath, (s * 6.0 - 150.0) * DEG);
            ink.arc((0.0, 0.0), r, a, a + 55.0 * DEG, (0.5, 0.5), (hoop, hoop));
        }
        // the seal flares (light on its own pixels only, nothing outside them)
        let flare = curve(&[(5.0, 0.0), (7.0, 1.0), (9.0, 0.0)], s);
        ink.paint(p, &self.ramp, 32.0, 0.08 * (1.0 + 2.0 * flare));
    }
}
