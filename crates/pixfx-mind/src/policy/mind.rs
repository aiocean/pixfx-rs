//! A policy that fights: it sees the foe through the same reaction delay as the utility mind (`Delayed`), thinks on
//! a jittered clock that hit-stop holds still, and samples its act with its own seeded dice.
//!
//! A thought made on one tick is acted on the next: on each tick the mind first hands back the act it chose last
//! time (dropped if it cannot happen now), then, when its clock runs out, chooses the next one from what it senses
//! now. That split lets a trainer answer between ticks (`begin`, `due`, `queue`) with the same timing the game
//! plays (`think`): a 1/60 s wait on top of the reaction delay.

use super::{ACTIONS, Act, Net, OBS, TAG, choice, legal, observe, sample};
use crate::choice::Choice;
use crate::rng::Rng;
use crate::sense::Sense;
use crate::skill::Skill;
use crate::view::Delayed;

/// A decision that is due: what the policy sees, what it may do, and its draw for sampling.
#[derive(Clone, Debug, PartialEq)]
pub struct Due {
    pub obs: [f32; OBS],
    pub legal: [bool; ACTIONS],
    pub u: f32,
}

/// A trained policy's clock, view of the foe and dice. The network is handed in on each `think`, so many minds
/// share one.
#[derive(Clone, Debug)]
pub struct PolicyMind {
    skills: Vec<Skill>,
    tag: [f32; TAG],
    reaction: f64,
    think: f64,
    rng: Rng,
    clock: f64,
    hold: f64,
    view: Delayed,
    queued: Option<usize>,
}

impl PolicyMind {
    /// A policy for a fighter with `skills` (the indexes `Sense` uses), tagged `tag`, seeing the foe `reaction`
    /// seconds late and thinking every `think` seconds (jittered 0.8-1.2), its dice seeded by `seed`.
    pub fn new(
        skills: Vec<Skill>,
        tag: [f32; TAG],
        reaction: f64,
        think: f64,
        seed: u32,
    ) -> PolicyMind {
        PolicyMind {
            skills,
            tag,
            reaction,
            think,
            rng: Rng::new(seed),
            clock: 0.0,
            hold: 0.0,
            view: Delayed::new(),
            queued: None,
        }
    }

    /// One tick of the game: the act chosen last time, and, if it is time, the next one chosen by `net`.
    pub fn think(&mut self, net: &Net, sense: &Sense, dt: f64) -> Option<Choice> {
        let now = self.begin(sense, dt);
        if let Some(d) = self.due(sense) {
            self.queue(sample(&net.logits(&d.obs), &d.legal, d.u));
        }
        now
    }

    /// The first half of a tick: the clocks run, the foe is seen, and the act queued last time is handed back as a
    /// `Choice`, unless it cannot happen now. In hit-stop (`dt = 0`) nothing runs and the act waits.
    pub fn begin(&mut self, sense: &Sense, dt: f64) -> Option<Choice> {
        if dt > 0.0 {
            self.clock -= dt;
            self.hold = (self.hold - dt).max(0.0);
        }
        self.view.push(sense);
        // in hit-stop the act waits for the next tick where time passes
        if dt <= 0.0 {
            return None;
        }
        let i = self.queued.take()?;
        let act = Act::from_index(i)?;
        if !legal(sense, &self.skills).get(i).copied().unwrap_or(false) {
            return None;
        }
        let (foe, _) = self.view.seen(sense.time, self.reaction);
        Some(choice(act, sense, &foe, &self.skills))
    }

    /// The second half: when its clock has run out and it can act, a decision is due; the clock restarts and the
    /// dice turn. Answer it with `queue`.
    pub fn due(&mut self, sense: &Sense) -> Option<Due> {
        if !sense.me.can_act || self.clock > 0.0 || self.hold > 0.0 {
            return None;
        }
        self.clock = self.think * (0.8 + 0.4 * self.rng.next());
        let u = self.rng.next() as f32;
        let (foe, threats) = self.view.seen(sense.time, self.reaction);
        let obs = observe(sense, &foe, &threats, &self.skills, &self.tag);
        Some(Due {
            obs,
            legal: legal(sense, &self.skills),
            u,
        })
    }

    /// The act to hand back next tick.
    pub fn queue(&mut self, act: usize) {
        self.queued = Some(act);
    }

    /// A new round: it forgets what it saw and what it meant to do, and waits 0.4-0.7 s before it acts, as `Mind`
    /// does.
    pub fn reset(&mut self) {
        self.clock = 0.4 + 0.3 * self.rng.next();
        self.hold = 0.0;
        self.view.clear();
        self.queued = None;
    }

    /// Think nothing for `seconds`.
    pub fn hold(&mut self, seconds: f64) {
        self.hold = seconds;
    }
}
