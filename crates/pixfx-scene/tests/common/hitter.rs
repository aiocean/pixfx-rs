//! A test effect that hits, for the scene's tests (`#[path = "common/hitter.rs"] mod hitter;`).

use pixfx_core::{Cue, Effect, EffectState, Layer, Meta, Options, TimedCue};
use pixfx_draw::Painter;

/// A one-shot that draws nothing and hits at 0.1 s: a stop and a full shake.
pub struct Hitter;

pub static HITTER: Hitter = Hitter;

const HITTER_META: Meta = Meta {
    id: "test/hitter",
    name: "Hitter",
    desc: "A one-shot that hits at 0.1 s.",
    tags: &[],
    size: (8, 6),
    origin: (4, 5),
    duration: 1.0,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

struct Blank;

impl EffectState for Blank {
    fn draw(&self, _p: &mut Painter, _t: f64, _layer: Layer) {}
}

impl Effect for Hitter {
    fn meta(&self) -> &Meta {
        &HITTER_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(Blank)
    }
    fn cues(&self, _opts: &Options) -> Vec<TimedCue> {
        let at = |cue| TimedCue {
            t: 0.1,
            cue,
            pan: None,
        };
        vec![
            at(Cue::Hitstop { dur: 0.1 }),
            at(Cue::Shake { trauma: 1.0 }),
        ]
    }
}
