//! Creature states: an effect that is one state of a creature, and a pure player that follows the chain.
//!
//! A creature (a Bloater, say) is a set of effects, one per state. [`StateMeta`] on an effect's [`Meta`] says
//! which effect follows it and which options carry over; [`Creature`] plays one state at a time and moves on
//! when a one-shot ends. It holds no clock or host: callers step it.

use std::fmt;

use crate::cue::TimedCue;
use crate::effect::{Effect, Instance, Layer, Options, find};
use crate::frame::Frame;

/// What makes an effect one state of a creature (pixelfx `meta.state`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateMeta {
    /// The creature this state belongs to.
    pub set: &'static str,
    pub name: &'static str,
    /// Where the state sits in its creature's list.
    pub order: u32,
    /// The effect id that follows when this one-shot ends.
    pub next: Option<&'static str>,
    /// Option names copied from this state's options to the next state's.
    pub carry: &'static [&'static str],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateError {
    /// No effect in the list has this id.
    Unknown(String),
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(id) => write!(f, "unknown state: {id}"),
        }
    }
}

impl std::error::Error for StateError {}

/// A creature playing one state at a time.
pub struct Creature {
    list: &'static [&'static dyn Effect],
    effect: &'static dyn Effect,
    inst: Instance,
    opts: Options,
    cues: Vec<TimedCue>,
}

impl Creature {
    /// A creature in state `id` of `list`.
    pub fn new(
        list: &'static [&'static dyn Effect],
        id: &str,
        opts: Options,
    ) -> Result<Self, StateError> {
        let effect = lookup(list, id)?;
        Ok(Self {
            list,
            effect,
            inst: Instance::new(effect, opts.clone()),
            opts,
            cues: Vec::new(),
        })
    }

    /// Switch to state `id` now; its cues fire from time 0. Cues the old state already fired stay available.
    pub fn enter(&mut self, id: &str, opts: Options) -> Result<(), StateError> {
        let effect = lookup(self.list, id)?;
        self.drain();
        self.effect = effect;
        self.inst = Instance::new(effect, opts.clone());
        self.opts = opts;
        Ok(())
    }

    /// Advance by `dt` seconds; a one-shot that has ended moves on to its `next`, carrying options.
    pub fn step(&mut self, dt: f64) {
        self.inst.update(dt);
        self.drain();
        let meta = self.effect.meta();
        if meta.looping || self.inst.t() < meta.duration {
            return;
        }
        let Some(next) = meta.state.and_then(|s| s.next.map(|n| (n, s.carry))) else {
            return;
        };
        let opts = carried(&self.opts, next.1);
        // a `next` missing from the list leaves the creature on its last frame
        let _ = self.enter(next.0, opts);
    }

    pub fn draw(&self, frame: &mut Frame, layer: Layer) {
        self.inst.draw(frame, layer);
    }

    /// The id of the effect playing now.
    pub fn state(&self) -> &'static str {
        self.effect.meta().id
    }

    pub fn opts(&self) -> &Options {
        &self.opts
    }

    /// The cues fired since the last call, oldest first; clears them.
    pub fn take_cues(&mut self) -> Vec<TimedCue> {
        self.drain();
        std::mem::take(&mut self.cues)
    }

    /// Seconds since the current state began.
    pub fn t(&self) -> f64 {
        self.inst.t()
    }

    fn drain(&mut self) {
        let fired = self.inst.take_cues();
        self.cues.extend(fired);
    }
}

fn lookup(list: &[&'static dyn Effect], id: &str) -> Result<&'static dyn Effect, StateError> {
    find(list, id).ok_or_else(|| StateError::Unknown(id.to_string()))
}

/// The next state's options: the defaults, with place, flip and seed kept and the `carry` keys copied over.
fn carried(old: &Options, carry: &[&str]) -> Options {
    let mut o = Options {
        x: old.x,
        y: old.y,
        flip: old.flip,
        seed: old.seed,
        ..Options::default()
    };
    for key in carry {
        if let Some(v) = old.extra.get(*key) {
            o.extra.insert((*key).to_string(), v.clone());
        }
    }
    o
}
