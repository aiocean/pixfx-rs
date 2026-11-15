//! Scripted fighters for the ladder and the trainer's league: simple, readable styles a trained policy must beat.
//! They see the foe as late as the brain does and think on the same clock.

use crate::bar::ready;
use crate::data::TEMPERAMENT;
use crate::fighter::State;
use crate::seat::{MakeSeat, Seat};
use crate::sense::{sense_of, skill_of};
use pixfx_mind::{ActPhase, Choice, Delayed, Doing, Outcome, SkillKind};

/// How a scripted fighter plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plan {
    /// Walks in and casts its hardest ready skill at the foe; never guards.
    Aggressive,
    /// Keeps its distance, guards every blow it sees wind up, casts only from range.
    Turtle,
}

/// A scripted seat.
pub struct Scripted {
    plan: Plan,
    view: Delayed,
    clock: f64,
    hold: f64,
}

/// How close the aggressive fighter walks before it strikes, and how far the turtle keeps.
const CLOSE: f64 = 22.0;
const FAR: f64 = 110.0;

impl Scripted {
    pub fn aggressive() -> MakeSeat {
        Box::new(|_, _| Box::new(Scripted::new(Plan::Aggressive)))
    }

    pub fn turtle() -> MakeSeat {
        Box::new(|_, _| Box::new(Scripted::new(Plan::Turtle)))
    }

    fn new(plan: Plan) -> Scripted {
        Scripted { plan, view: Delayed::new(), clock: 0.0, hold: 0.0 }
    }
}

impl Seat for Scripted {
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        let sense = sense_of(state, me);
        if dt > 0.0 {
            self.clock -= dt;
            self.hold = (self.hold - dt).max(0.0);
        }
        self.view.push(&sense);
        if !sense.me.can_act || self.clock > 0.0 || self.hold > 0.0 {
            return None;
        }
        self.clock = TEMPERAMENT.think;
        let (foe, _) = self.view.seen(sense.time, TEMPERAMENT.reaction);
        let f = &state.fighters[me];
        let gap = foe.x - sense.me.x;
        let dir = if gap < 0.0 { -1.0 } else { 1.0 };
        let dist = gap.abs();
        // its ready skills of the kinds it uses, hardest first
        let mut skills: Vec<(&'static str, f64)> = f
            .spells
            .iter()
            .copied()
            .filter(|n| ready(f, n))
            .filter_map(|n| {
                let s = skill_of(n);
                let fits = match self.plan {
                    Plan::Aggressive => s.damage > 0.0 && (s.reach.is_nan() || s.reach == 0.0 || dist <= s.reach.max(CLOSE)),
                    Plan::Turtle => matches!(s.kind, SkillKind::Bolt | SkillKind::Area | SkillKind::Seeker),
                };
                fits.then_some((n, s.damage))
            })
            .collect();
        skills.sort_by(|a, b| b.1.total_cmp(&a.1));
        let cast = skills.first().map(|(n, _)| Choice::Cast { skill: (*n).to_string(), target: Some(foe.x) });
        match self.plan {
            Plan::Aggressive => {
                if dist > CLOSE && cast.is_none() {
                    Some(Choice::Move { dir })
                } else {
                    cast.or(Some(Choice::Move { dir }))
                }
            }
            Plan::Turtle => {
                let blow = matches!(foe.doing, Doing::Act { phase: ActPhase::Windup, .. });
                if blow && dist < FAR + 40.0 {
                    Some(Choice::Guard { hold: 0.4 })
                } else if dist < FAR {
                    Some(Choice::Move { dir: -dir })
                } else if let Some(c) = cast {
                    Some(c)
                } else if sense.me.qi < 50.0 {
                    Some(Choice::Meditate)
                } else {
                    Some(Choice::Wait)
                }
            }
        }
    }

    fn tell(&mut self, _outcome: &Outcome) {}

    fn reset(&mut self) {
        self.view.clear();
        self.clock = 0.0;
        self.hold = 0.0;
    }

    fn hold(&mut self, seconds: f64) {
        self.hold = seconds;
    }
}
