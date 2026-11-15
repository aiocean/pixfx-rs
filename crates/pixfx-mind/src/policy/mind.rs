//! A policy that fights: it sees the foe through the same reaction delay as the utility mind (`Delayed`), thinks on
//! a jittered clock that hit-stop holds still, and samples its act with its own seeded dice.

use super::{ACTIONS, Act, Net, OBS, TAG, choice, legal, observe, sample};
use crate::choice::Choice;
use crate::rng::Rng;
use crate::sense::Sense;
use crate::skill::Skill;
use crate::view::Delayed;
use std::rc::Rc;

/// A trained policy with its clock, its view of the foe and its dice.
#[derive(Clone, Debug)]
pub struct PolicyMind {
    net: Rc<Net>,
    skills: Vec<Skill>,
    tag: [f32; TAG],
    reaction: f64,
    think: f64,
    rng: Rng,
    clock: f64,
    hold: f64,
    view: Delayed,
    last: Option<(usize, [f32; OBS], [bool; ACTIONS])>,
}

impl PolicyMind {
    /// A policy running `net` for a fighter with `skills` (the indexes `Sense` uses), tagged `tag`, seeing the foe
    /// `reaction` seconds late and thinking every `think` seconds (jittered 0.8-1.2), its dice seeded by `seed`.
    pub fn new(
        net: Rc<Net>,
        skills: Vec<Skill>,
        tag: [f32; TAG],
        reaction: f64,
        think: f64,
        seed: u32,
    ) -> PolicyMind {
        PolicyMind {
            net,
            skills,
            tag,
            reaction,
            think,
            rng: Rng::new(seed),
            clock: 0.0,
            hold: 0.0,
            view: Delayed::new(),
            last: None,
        }
    }

    /// What it does this tick, if it is time to think: its network's pick.
    pub fn think(&mut self, sense: &Sense, dt: f64) -> Option<Choice> {
        let net = Rc::clone(&self.net);
        self.decide(sense, dt, |o, l, u| sample(&net.logits(o), l, u))
    }

    /// The same clock and view as `think`, with the act picked by `pick` from what it sees and what is legal: the
    /// trainer's learner. The dice turn as in `think`, so a fight replays the same either way.
    pub fn decide_with(
        &mut self,
        sense: &Sense,
        dt: f64,
        pick: impl FnOnce(&[f32; OBS], &[bool; ACTIONS]) -> usize,
    ) -> Option<Choice> {
        self.decide(sense, dt, |o, l, _| pick(o, l))
    }

    fn decide(
        &mut self,
        sense: &Sense,
        dt: f64,
        pick: impl FnOnce(&[f32; OBS], &[bool; ACTIONS], f32) -> usize,
    ) -> Option<Choice> {
        if dt > 0.0 {
            self.clock -= dt;
            self.hold = (self.hold - dt).max(0.0);
        }
        self.view.push(sense);
        if !sense.me.can_act || self.clock > 0.0 || self.hold > 0.0 {
            return None;
        }
        self.clock = self.think * (0.8 + 0.4 * self.rng.next());
        let u = self.rng.next() as f32;
        let (foe, threats) = self.view.seen(sense.time, self.reaction);
        let obs = observe(sense, &foe, &threats, &self.skills, &self.tag);
        let ok = legal(sense, &self.skills);
        let i = pick(&obs, &ok, u);
        // an act it may not do now is a wait
        let act = Act::from_index(i)
            .filter(|_| ok.get(i).copied().unwrap_or(false))
            .unwrap_or(Act::Wait);
        self.last = Some((act.index(), obs, ok));
        Some(choice(act, sense, &foe, &self.skills))
    }

    /// The last act it chose, with what it saw and what was legal then.
    pub fn last(&self) -> Option<&(usize, [f32; OBS], [bool; ACTIONS])> {
        self.last.as_ref()
    }

    /// A new round: it forgets what it saw and thinks at once.
    pub fn reset(&mut self) {
        self.clock = 0.0;
        self.hold = 0.0;
        self.view.clear();
        self.last = None;
    }

    /// Think nothing for `seconds`.
    pub fn hold(&mut self, seconds: f64) {
        self.hold = seconds;
    }
}
