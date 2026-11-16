//! Seeing late (Rabin, Game AI Pro 2 ch. 5): a blow the foe winds up reaches the mind `reaction` seconds after it
//! starts, and a threat only once it has been there that long. That is the time to react to a danger. Everything
//! else (where the foe is, a cast it gathers, a recovery, a walk) is seen as it is: an opening is taken the moment
//! it shows.

use crate::reading::striking;
use crate::sense::{ActPhase, Doing, Foe, Sense, Threat};
use std::collections::VecDeque;

/// How far back the mind remembers what the foe did, in seconds.
const MEMORY: f64 = 1.0;

/// The foe's doings over the last second, and the latest sense.
#[derive(Clone, Debug, Default)]
pub struct Delayed {
    past: VecDeque<(f64, Doing)>,
    now: Option<(Foe, Vec<Threat>)>,
}

impl Delayed {
    pub fn new() -> Delayed {
        Delayed::default()
    }

    /// Take in this tick's sense. A sense at the same time as the last (hit-stop) replaces it.
    pub fn push(&mut self, sense: &Sense) {
        if self.past.back().is_some_and(|(t, _)| *t >= sense.time) {
            self.past.pop_back();
        }
        self.past.push_back((sense.time, sense.foe.doing.clone()));
        // keep one entry at or before the edge of memory, so a reaction that long still finds one
        while self
            .past
            .get(1)
            .is_some_and(|(t, _)| *t <= sense.time - MEMORY)
        {
            self.past.pop_front();
        }
        self.now = Some((sense.foe.clone(), sense.threats.clone()));
    }

    /// The foe as the mind sees it at `now`: as it is, but while it winds up a blow, doing what it did `reaction`
    /// seconds ago (aged to now); and the threats at least `reaction` old. Before anything is that old, the oldest
    /// doing it has.
    pub fn seen(&self, now: f64, reaction: f64) -> (Foe, Vec<Threat>) {
        let Some((foe, threats)) = &self.now else {
            return (empty_foe(), vec![]);
        };
        let edge = now - reaction;
        let past = self
            .past
            .iter()
            .rev()
            .find(|(t, _)| *t <= edge + 1e-9)
            .or(self.past.front());
        let mut seen = foe.clone();
        let blow =
            matches!(foe.doing, Doing::Act { kind, phase: ActPhase::Windup, .. } if striking(kind));
        if blow && let Some((t, doing)) = past {
            seen.doing = aged(doing, now - t);
        }
        let threats = threats
            .iter()
            .filter(|th| th.age >= reaction)
            .cloned()
            .collect();
        (seen, threats)
    }

    pub fn len(&self) -> usize {
        self.past.len()
    }

    pub fn is_empty(&self) -> bool {
        self.past.is_empty()
    }

    pub fn clear(&mut self) {
        self.past.clear();
        self.now = None;
    }
}

/// A doing seen `by` seconds ago, as it would stand now if it went on: a wind-up past its length is a blow struck
/// and recovering, a recovery past its length is over.
fn aged(doing: &Doing, by: f64) -> Doing {
    let Doing::Act {
        kind,
        phase,
        t,
        windup,
        recover,
        reach,
        damage,
        ultimate,
    } = *doing
    else {
        return doing.clone();
    };
    let mut t = t + by;
    let mut phase = phase;
    if phase == ActPhase::Windup && t >= windup {
        phase = ActPhase::Recover;
        t -= windup;
    }
    if phase == ActPhase::Recover && t >= recover {
        return Doing::Idle;
    }
    Doing::Act {
        kind,
        phase,
        t,
        windup,
        recover,
        reach,
        damage,
        ultimate,
    }
}

fn empty_foe() -> Foe {
    Foe {
        x: 0.0,
        air: 0.0,
        flying: false,
        hp: 100.0,
        qi: 100.0,
        doing: Doing::Idle,
        out_for: 0.0,
        open_in: 0.0,
        frozen: 0.0,
        shocked: 0.0,
        barrier: false,
        ready: vec![],
    }
}
