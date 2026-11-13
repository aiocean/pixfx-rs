//! Creature states: an effect that is one state of a creature, and the pure player that follows `next`.

use pixfx_core::cue::{Cue, TimedCue};
use pixfx_core::effect::{Effect, EffectState, Layer, Meta, OptValue, Options};
use pixfx_core::painter::Painter;
use pixfx_core::state::{Creature, StateError, StateMeta};

struct Blank;

impl EffectState for Blank {
    fn draw(&self, _p: &mut Painter, _t: f64, _layer: Layer) {}
}

struct State {
    meta: Meta,
    cues: &'static [TimedCue],
}

impl Effect for State {
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

const fn meta(
    id: &'static str,
    duration: f64,
    looping: bool,
    next: Option<&'static str>,
    carry: &'static [&'static str],
) -> Meta {
    Meta {
        id,
        name: "State",
        desc: "A creature state.",
        tags: &["test"],
        size: (8, 8),
        origin: (0, 0),
        duration,
        looping,
        fps: 30,
        seed: 1,
        state: Some(StateMeta {
            set: "critter",
            name: id,
            order: 0,
            next,
            carry,
        }),
    }
}

const SHAKE: TimedCue = TimedCue {
    t: 0.0,
    cue: Cue::Shake { trauma: 0.5 },
    pan: None,
};
const LATE: TimedCue = TimedCue {
    t: 0.4,
    cue: Cue::Hitstop { dur: 0.1 },
    pan: None,
};

static A: State = State {
    meta: meta("critter/a", 0.5, false, Some("critter/b"), &["hits"]),
    cues: &[SHAKE, LATE],
};
static B: State = State {
    meta: meta("critter/b", 1.0, true, None, &[]),
    cues: &[SHAKE],
};
static END: State = State {
    meta: meta("critter/end", 0.5, false, None, &[]),
    cues: &[],
};
static LIST: [&dyn Effect; 3] = [&A, &B, &END];

fn hits(n: f64) -> Options {
    let mut o = Options {
        x: 7.0,
        y: 9.0,
        flip: true,
        seed: Some(5),
        ..Options::default()
    };
    o.extra.insert("hits".into(), OptValue::Num(n));
    o.extra.insert("other".into(), OptValue::Num(3.0));
    o
}

fn run(c: &mut Creature, secs: f64) {
    for _ in 0..(secs * 60.0) as u32 {
        c.step(1.0 / 60.0);
    }
}

#[test]
fn follows_next_when_one_shot_ends() -> Result<(), StateError> {
    let mut c = Creature::new(&LIST, "critter/a", Options::default())?;
    run(&mut c, 0.3);
    assert_eq!(c.state(), "critter/a");
    run(&mut c, 0.4);
    assert_eq!(c.state(), "critter/b");
    Ok(())
}

#[test]
fn carry_survives_follow() -> Result<(), StateError> {
    let mut c = Creature::new(&LIST, "critter/a", hits(2.0))?;
    run(&mut c, 0.7);
    assert_eq!(c.state(), "critter/b");
    let o = c.opts();
    assert_eq!(o.num_or("hits", 0.0), 2.0);
    assert!(!o.extra.contains_key("other"));
    assert_eq!((o.x, o.y, o.flip, o.seed), (7.0, 9.0, true, Some(5)));
    Ok(())
}

#[test]
fn looping_state_does_not_follow() -> Result<(), StateError> {
    let mut c = Creature::new(&LIST, "critter/b", Options::default())?;
    run(&mut c, 3.0);
    assert_eq!(c.state(), "critter/b");
    Ok(())
}

#[test]
fn one_shot_without_next_stays_on_last_frame() -> Result<(), StateError> {
    let mut c = Creature::new(&LIST, "critter/end", Options::default())?;
    run(&mut c, 2.0);
    assert_eq!(c.state(), "critter/end");
    Ok(())
}

#[test]
fn enter_unknown_id_errors() -> Result<(), StateError> {
    let mut c = Creature::new(&LIST, "critter/a", Options::default())?;
    assert_eq!(
        c.enter("nope", Options::default()),
        Err(StateError::Unknown("nope".into()))
    );
    assert_eq!(c.state(), "critter/a");
    assert!(Creature::new(&LIST, "nope", Options::default()).is_err());
    assert_eq!(
        StateError::Unknown("nope".into()).to_string(),
        "unknown state: nope"
    );
    Ok(())
}

#[test]
fn enter_resets_time_and_fires_new_cues() -> Result<(), StateError> {
    let mut c = Creature::new(&LIST, "critter/a", Options::default())?;
    run(&mut c, 0.3);
    c.take_cues();
    c.enter("critter/b", Options::default())?;
    assert_eq!(c.t(), 0.0);
    c.step(1.0 / 60.0);
    assert_eq!(c.take_cues(), vec![SHAKE]);
    Ok(())
}

#[test]
fn cues_of_the_old_state_survive_the_switch() -> Result<(), StateError> {
    let mut c = Creature::new(&LIST, "critter/a", Options::default())?;
    run(&mut c, 0.45);
    c.enter("critter/b", Options::default())?;
    let got = c.take_cues();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].cue, SHAKE.cue);
    assert_eq!(got[1].cue, LATE.cue);
    assert!(c.take_cues().is_empty());
    Ok(())
}
