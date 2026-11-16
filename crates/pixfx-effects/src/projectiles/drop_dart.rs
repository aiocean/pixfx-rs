//! Drop dart: a teardrop glints at the left, swells and darts right, swings up past the middle in a bright flash and
//! flies on as a thin blade that fades. Made here, by hand, from "Free pack, Part 1/05.png" (no source
//! code, so no parity check): its nine rows are the nine [`crate::tone::RAMPS`]. Its drop is one of
//! `impacts/twin-converge`'s, so it is drawn by [`crate::impacts::drops`].

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use crate::impacts::drops::{self, MOMENT, MOMENTS};
use crate::tone::{self, Ink, VARIANT, curve};

pub struct DropDart;

pub static DROP_DART: DropDart = DropDart;

const META: Meta = Meta {
    id: "projectiles/drop-dart",
    name: "Drop Dart",
    desc: "A teardrop glints, swells and darts right, swings up in a bright flash and flies on as a thin fading blade.",
    tags: &["projectile", "magic", "dart", "spark", "one-shot"],
    size: (96, 96),
    origin: (48, 48),
    duration: MOMENTS * MOMENT,
    looping: false,
    fps: 50,
    seed: 1,
    state: None,
    options: &[VARIANT.spec()],
};

/// Where the sheet draws the dart: the twin's left drop moved this far.
const SHIFT: (f64, f64) = (8.0, -4.0);
/// The moment the dart strikes: the sheet holds its flash (the drop's head, its tail behind it) for the whole moment,
/// then shows the dart flown on the far side, drawn from here as the twin's right drop.
const STRIKE: f64 = 9.0;
/// The faint wake of the strike, drawn as it comes on: specks behind the drop, then a streak ahead of its flash to
/// the right, each (from, to) in the sheet's pixels round its centre.
const WAKE_BEHIND: ((f64, f64), (f64, f64)) = ((-1.0, 2.0), (-14.0, 5.0));
const WAKE_AHEAD: ((f64, f64), (f64, f64)) = ((8.0, -3.0), (21.0, 1.0));
/// How much of the wake shows (0 none, 1 all of it, past 1 stretched on toward where the dart flies next) and how
/// bright its near end is, in ramp steps. It goes when the dart is past.
const WAKE: [(f64, f64); 4] = [(7.5, 0.0), (8.5, 0.6), (9.0, 1.0), (10.0, 1.4)];
const WAKE_TONE: f64 = 5.0;

impl Effect for DropDart {
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
        // the flash holds through its moment instead of easing toward the far side, then the dart is past
        let (at, quarters) = if s < STRIKE + 1.0 {
            (s.min(STRIKE), 3)
        } else {
            (s, 1)
        };
        let wake = if s < STRIKE + 1.0 {
            curve(&WAKE, s)
        } else {
            0.0
        };
        if wake > 0.0 {
            let mut ink = Ink::new(1);
            for (a, b) in [WAKE_BEHIND, WAKE_AHEAD] {
                let end = (a.0 + (b.0 - a.0) * wake, a.1 + (b.1 - a.1) * wake);
                ink.seg(a, end, (1.0, 0.6), (WAKE_TONE * wake, 0.0));
            }
            ink.paint(p, &self.ramp, 32.0, 0.0);
        }
        drops::paint(p, &self.ramp, at, 1, quarters, SHIFT);
    }
}
