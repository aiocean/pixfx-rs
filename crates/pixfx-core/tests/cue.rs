//! Effect cues against game-factory `src/core/cues.js` (`cuesOf`, `cuesBetween`) and the `fire` hook of
//! `src/core/fx.js` (`createFx`).

use pixfx_core::cue::{Cue, TimedCue};
use pixfx_core::effect::{Effect, EffectState, Instance, Layer, Meta, Options};
use pixfx_core::painter::Painter;

/// A test effect with fixed cues.
struct Cued {
    meta: Meta,
    cues: &'static [TimedCue],
}

struct Blank;

impl EffectState for Blank {
    fn draw(&self, _p: &mut Painter, _t: f64, _layer: Layer) {}
}

impl Effect for Cued {
    fn meta(&self) -> &Meta {
        &self.meta
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(Blank)
    }
    fn cues(&self, _opts: &Options) -> Vec<TimedCue> {
        self.cues.to_vec()
    }
}

const fn meta(id: &'static str, looping: bool) -> Meta {
    Meta {
        id,
        name: "Cued",
        desc: "An effect with fixed cues.",
        tags: &["test"],
        size: (8, 8),
        origin: (0, 0),
        duration: 1.0,
        looping,
        fps: 30,
        seed: 1,
        state: None,
    }
}

const fn at(t: f64, cue: Cue) -> TimedCue {
    TimedCue { t, cue, pan: None }
}

const SHAKE: Cue = Cue::Shake { trauma: 0.5 };
const RIPPLE: Cue = Cue::Ripple {
    x: 10.0,
    y: 4.0,
    r0: 1.0,
    r1: 9.0,
    dur: 0.3,
    amp: 2.0,
};

static AT_ZERO: Cued = Cued {
    meta: meta("test/zero", false),
    cues: &[at(0.0, SHAKE)],
};
static LOOPED: Cued = Cued {
    meta: meta("test/loop", true),
    cues: &[at(0.5, SHAKE)],
};
static ONE_SHOT: Cued = Cued {
    meta: meta("test/oneshot", false),
    cues: &[at(0.5, SHAKE), at(1.5, Cue::Hitstop { dur: 0.1 })],
};
static RIPPLED: Cued = Cued {
    meta: meta("test/ripple", false),
    cues: &[at(0.5, RIPPLE)],
};

const SOUND: Cue = Cue::Sound {
    name: "hit",
    gain: 1.0,
    pitch: 1.0,
    len: None,
    x: Some(10.0),
};
static SOUNDED: Cued = Cued {
    meta: meta("test/sound", false),
    cues: &[at(0.5, SOUND)],
};
static UNPLACED: Cued = Cued {
    meta: meta("test/sound-unplaced", false),
    cues: &[at(
        0.5,
        Cue::Sound {
            name: "hit",
            gain: 1.0,
            pitch: 1.0,
            len: None,
            x: None,
        },
    )],
};

fn play(effect: &'static Cued, opts: Options, screen_w: f64, steps: u32) -> Vec<TimedCue> {
    let mut inst = Instance::new(effect, opts);
    inst.set_screen_w(screen_w);
    let mut out = Vec::new();
    for _ in 0..steps {
        inst.update(0.1);
        out.extend(inst.take_cues());
    }
    out
}

#[test]
fn cue_at_zero_fires_on_first_update() {
    let mut inst = Instance::new(&AT_ZERO, Options::default());
    assert!(inst.take_cues().is_empty());
    inst.update(0.1);
    let got = inst.take_cues();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].cue, SHAKE);
    assert_eq!(got[0].t, 0.0);
    assert!(inst.take_cues().is_empty(), "take_cues clears");
}

#[test]
fn loop_repeats_cues_each_period() {
    let got = play(&LOOPED, Options::default(), 0.0, 26);
    let times: Vec<f64> = got.iter().map(|c| c.t).collect();
    assert_eq!(times, vec![0.5, 1.5, 2.5]);
}

#[test]
fn one_shot_drops_cues_past_duration() {
    let got = play(&ONE_SHOT, Options::default(), 0.0, 30);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].cue, SHAKE);
}

#[test]
fn flip_mirrors_cue_x_once() {
    let opts = Options {
        x: 100.0,
        y: 20.0,
        flip: true,
        ..Options::default()
    };
    let got = play(&RIPPLED, opts, 0.0, 10);
    assert_eq!(got.len(), 1);
    assert!(matches!(
        got[0].cue,
        Cue::Ripple {
            x: 90.0,
            y: 24.0,
            ..
        }
    ));
}

#[test]
fn pan_spans_screen() {
    let pan_at = |x: f64| {
        let opts = Options {
            x,
            ..Options::default()
        };
        play(&LOOPED, opts, 200.0, 6)[0].pan
    };
    assert_eq!(pan_at(0.0), Some(-1.0));
    assert_eq!(pan_at(100.0), Some(0.0));
    assert_eq!(pan_at(200.0), Some(1.0));
    assert_eq!(pan_at(500.0), Some(1.0));
    assert_eq!(pan_at(-500.0), Some(-1.0));
    assert_eq!(play(&LOOPED, Options::default(), 0.0, 6)[0].pan, None);
}

#[test]
fn pan_follows_a_cues_own_world_x() {
    let opts = Options {
        x: 100.0,
        ..Options::default()
    };
    // ripple x 10 offset by opts.x 100 = 110 of 220 -> 0
    let got = play(&RIPPLED, opts, 220.0, 10);
    assert_eq!(got[0].pan, Some(0.0));
}

#[test]
fn a_sound_with_x_is_placed_and_panned_like_a_ripple() {
    let opts = Options {
        x: 100.0,
        flip: true,
        ..Options::default()
    };
    let got = play(&SOUNDED, opts, 180.0, 10);
    assert_eq!(got.len(), 1);
    // x 10 mirrored and offset by opts.x 100 = 90 of 180 -> 0
    assert!(matches!(got[0].cue, Cue::Sound { x: Some(90.0), .. }));
    assert_eq!(got[0].pan, Some(0.0));
}

#[test]
fn a_sound_without_x_pans_from_the_effect() {
    let opts = Options {
        x: 50.0,
        ..Options::default()
    };
    let got = play(&UNPLACED, opts, 200.0, 10);
    assert!(matches!(got[0].cue, Cue::Sound { x: None, .. }));
    assert_eq!(got[0].pan, Some(-0.5));
}

#[test]
fn stopping_instance_fires_no_cues() {
    let mut inst = Instance::new(&LOOPED, Options::default());
    inst.update(0.1);
    inst.stop();
    for _ in 0..10 {
        inst.update(0.1);
    }
    assert!(inst.take_cues().is_empty());
}
