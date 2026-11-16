//! Comet rings: rings of broken arcs turn round a small square core; four bright comets race
//! round the outer ring, cut inward, and the arcs ripple out and fade. Made here, by hand, from "Free pack, Part 1/26.png" (no source code,
//! so no parity check): its nine rows are the nine [`tone::RAMPS`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use crate::tone::{self, Ink, VARIANT, curve};

pub struct CometRings;

pub static COMET_RINGS: CometRings = CometRings;

/// Seconds a moment of the sheet lasts.
const MOMENT: f64 = 0.06;
const MOMENTS: f64 = 14.0;

const META: Meta = Meta {
    id: "arcane/comet-rings",
    name: "Comet Rings",
    desc: "Rings of broken arcs turn round a small core; four comets race round, cut inward, and the arcs ripple out and fade.",
    tags: &["arcane", "magic", "ring", "comet", "one-shot"],
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

/// The outer ring: its thin full circle's tone and its four thick arcs' radius, tone and turn (degrees), by moment.
const HOOP: [(f64, f64); 5] = [(0.0, -1.0), (1.0, 2.0), (5.0, 3.0), (7.0, 2.0), (8.0, -1.0)];
const OUTER_R: [(f64, f64); 4] = [(0.0, 27.0), (2.0, 26.5), (8.0, 25.5), (12.0, 27.0)];
const OUTER_TONE: [(f64, f64); 8] = [
    (0.0, 4.0),
    (2.0, 7.0),
    (6.0, 6.0),
    (9.0, 4.0),
    (10.0, 3.0),
    (11.0, 1.5),
    (12.0, 0.8),
    (13.0, 0.3),
];
const OUTER_TURN: [(f64, f64); 2] = [(0.0, -70.0), (13.0, 60.0)];
/// The arcs that ripple out inside the outer ring once the comets have passed.
const RIPPLE: [(f64, f64); 5] = [
    (8.0, -1.0),
    (9.0, 3.0),
    (10.0, 2.5),
    (11.0, 1.5),
    (12.5, -1.0),
];
/// The inner rings and the diamond core.
const INNER_TONE: [(f64, f64); 4] = [(0.0, 2.0), (3.0, 3.0), (8.0, 3.0), (9.5, -1.0)];
const CORE_TONE: [(f64, f64); 4] = [(2.0, -1.0), (3.0, 2.0), (6.0, 2.0), (7.0, -1.0)];
/// The comets: the outer one rides the outer ring, the inner one cuts across inside it, each (radius, angle).
const COMET_OUT: [(f64, f64); 4] = [(3.5, -1.0), (4.5, 14.0), (8.0, 14.0), (9.0, -1.0)];
const COMET_OUT_A: [(f64, f64); 2] = [(3.5, -70.0), (8.5, -30.0)];
const COMET_IN: [(f64, f64); 4] = [(5.5, -1.0), (6.5, 14.0), (9.0, 14.0), (10.0, -1.0)];
const COMET_IN_R: [(f64, f64); 2] = [(5.5, 18.0), (9.5, 17.0)];
const COMET_IN_A: [(f64, f64); 2] = [(5.5, -60.0), (9.5, -45.0)];

impl Effect for CometRings {
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
        let mut ink = Ink::new(4).grain(0.2, 0.75, 26.0);
        let hoop = curve(&HOOP, s);
        if hoop >= 0.0 {
            ink.arc(
                (0.0, 0.0),
                24.0,
                -45.0 * DEG,
                45.0 * DEG,
                (0.5, 0.5),
                (hoop, hoop),
            );
        }
        let (or, ov) = (curve(&OUTER_R, s), curve(&OUTER_TONE, s));
        let turn = curve(&OUTER_TURN, s) * DEG;
        if ov >= 0.0 {
            ink.arc(
                (0.0, 0.0),
                or,
                turn - 22.0 * DEG,
                turn + 22.0 * DEG,
                (0.5, 1.2),
                (ov * 0.5, ov),
            );
        }
        let rip = curve(&RIPPLE, s);
        if rip >= 0.0 {
            for (k, dr) in [5.0, 10.0].iter().enumerate() {
                let a = turn - 35.0 * DEG - k as f64 * 15.0 * DEG;
                ink.arc(
                    (0.0, 0.0),
                    or - dr,
                    a - 18.0 * DEG,
                    a + 18.0 * DEG,
                    (0.5, 0.5),
                    (rip, rip),
                );
            }
        }
        let iv = curve(&INNER_TONE, s);
        if iv >= 0.0 {
            let a = -turn * 0.6 + 20.0 * DEG;
            ink.arc(
                (0.0, 0.0),
                13.5,
                a - 25.0 * DEG,
                a + 25.0 * DEG,
                (0.5, 0.5),
                (iv, iv),
            );
            ink.arc(
                (0.0, 0.0),
                18.5,
                -a - 20.0 * DEG,
                -a + 20.0 * DEG,
                (0.5, 0.5),
                (iv, iv),
            );
        }
        let cv = curve(&CORE_TONE, s);
        if cv >= 0.0 {
            ink.seg((0.0, -4.5), (4.5, 0.0), (0.5, 0.5), (cv, cv));
        }
        for (tone_keys, r, a) in [
            (&COMET_OUT[..], 25.0, curve(&COMET_OUT_A, s)),
            (&COMET_IN[..], curve(&COMET_IN_R, s), curve(&COMET_IN_A, s)),
        ] {
            let v = curve(tone_keys, s);
            if v < 0.0 {
                continue;
            }
            let a = a * DEG;
            // the head, then its tail trailing back along the ring
            ink.dot(r * libm::cos(a), r * libm::sin(a), 3.0, v);
            ink.arc((0.0, 0.0), r, a - 30.0 * DEG, a, (0.5, 2.5), (v * 0.4, v));
        }
        ink.paint(p, &self.ramp, 32.0, 0.08);
    }
}
