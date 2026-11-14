//! Seeing late (Rabin, Game AI Pro 2 ch. 5): what the foe starts doing reaches the mind `reaction` seconds later,
//! and a threat only once it has been there that long. Where the foe is, how it stands and its countdowns are
//! seen as they are.

use crate::sense::{Doing, Foe, Sense, Threat};
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

    /// The foe as the mind sees it at `now`: as it is, but doing what it did `reaction` seconds ago (aged to now),
    /// and the threats at least `reaction` old. Before anything is that old, the oldest doing it has.
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
        if let Some((t, doing)) = past {
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

/// A doing seen `by` seconds ago, as it would stand now if it went on.
fn aged(doing: &Doing, by: f64) -> Doing {
    match doing {
        Doing::Act {
            kind,
            phase,
            t,
            windup,
            recover,
            reach,
            damage,
        } => Doing::Act {
            kind: *kind,
            phase: *phase,
            t: t + by,
            windup: *windup,
            recover: *recover,
            reach: *reach,
            damage: *damage,
        },
        other => other.clone(),
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
