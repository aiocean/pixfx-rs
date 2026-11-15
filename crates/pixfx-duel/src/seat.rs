//! Who decides for a fighter: a seat. The duel asks it each tick and tells it how its skills went; the rules never
//! call it themselves (they queue what to tell in `State::told`).

use crate::data::{TEMPERAMENT, school};
use crate::fighter::State;
use crate::sense::{sense_of, skill_of};
use pixfx_mind::{BodySize, Choice, Mind, Outcome, Posture, Profile, Thought};

/// A fighter's decider.
pub trait Seat {
    /// What fighter `me` does this tick, if it decides anything.
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice>;
    fn tell(&mut self, outcome: &Outcome);
    /// A new round.
    fn reset(&mut self);
    /// Think nothing for `seconds`.
    fn hold(&mut self, seconds: f64);
    fn posture(&self) -> Option<Posture> {
        None
    }
}

/// Makes fighter `me`'s seat once the fighters exist (a brain is told both fighters' spells).
pub type MakeSeat = Box<dyn FnOnce(&State, usize) -> Box<dyn Seat>>;

/// pixfx-mind's brain with its school's skills and temperament.
pub struct Brain {
    mind: Mind,
    /// Its last thought.
    pub thought: Option<Thought>,
}

impl Brain {
    pub fn new(state: &State, me: usize) -> Brain {
        let f = &state.fighters[me];
        let d = school(f.school);
        let t = TEMPERAMENT;
        let profile = Profile {
            aggression: d.aggression,
            tempo: d.tempo,
            spread: t.spread,
            range: d.range,
            dry: d.dry,
            sky: d.sky,
            risk: t.risk,
            floor: t.floor,
            lead: t.lead,
            prior: t.prior,
            reaction: t.reaction,
            think: t.think,
            posture_scale: d.postures,
        };
        let spec = &f.body.spec;
        let body = BodySize { half_width: spec.half_width, chest: spec.chest, body: spec.body, duck: spec.duck, walk: spec.walk };
        let skills = f.spells.iter().map(|n| skill_of(n)).collect();
        let mut mind = Mind::new(profile, body, skills, state.seed.wrapping_mul(2).wrapping_add(me as u32));
        mind.meet(state.fighters[1 - me].spells.iter().map(|n| skill_of(n)).collect());
        Brain { mind, thought: None }
    }

    /// A maker for `Duel::new`.
    pub fn make() -> MakeSeat {
        Box::new(|state, me| Box::new(Brain::new(state, me)))
    }
}

impl Seat for Brain {
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        let thought = self.mind.think(&sense_of(state, me), dt)?;
        let choice = thought.choice.clone();
        self.thought = Some(thought);
        Some(choice)
    }

    fn tell(&mut self, outcome: &Outcome) {
        self.mind.tell(outcome);
    }

    fn reset(&mut self) {
        self.mind.reset();
    }

    fn hold(&mut self, seconds: f64) {
        self.mind.hold(seconds);
    }

    fn posture(&self) -> Option<Posture> {
        Some(self.mind.posture())
    }
}

/// What a scripted seat runs: the state and the fighter it decides for, to a choice.
pub type Decide = Box<dyn FnMut(&State, usize) -> Option<Choice>>;

/// A seat run by a closure, for tests: told nothing, never held.
pub struct Script(pub Decide);

impl Seat for Script {
    fn think(&mut self, state: &State, me: usize, _dt: f64) -> Option<Choice> {
        (self.0)(state, me)
    }

    fn tell(&mut self, _outcome: &Outcome) {}

    fn reset(&mut self) {}

    fn hold(&mut self, _seconds: f64) {}
}
