//! The ladder: one seat against another over many seeds, each fought both ways (as Thunder, then as Frost) so the
//! schools' own balance cancels out, with what a watcher would judge: who wins as each school, how long rounds last,
//! whether fights stall, how varied and how aggressive the first seat plays, and the distance it keeps.

use crate::duel::{Duel, DuelConfig, TICK};
use crate::data::spell;
use crate::fighter::State;
use crate::seat::{MakeSeat, Seat};
use pixfx_mind::{Choice, Outcome};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Range;
use std::rc::Rc;

/// How a match is fought: its seeds, the rounds a fight lasts, the most seconds a round may last, and the effects
/// in the registry.
#[derive(Clone, Debug)]
pub struct Match {
    pub seeds: Range<u32>,
    pub rounds: u32,
    pub limit: f64,
    pub effects: Vec<String>,
}

/// A match's result, from the first seat's side.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Record {
    /// Rounds the first seat won and lost as Thunder (0) and as Frost (1).
    pub won: [u32; 2],
    pub lost: [u32; 2],
    /// Every finished round's length, seconds.
    pub lengths: Vec<f64>,
    /// Fights cut because a round passed the limit.
    pub timeouts: u32,
    /// The first seat's choices, by kind (a cast by its spell).
    pub choices: BTreeMap<String, u32>,
    pub decisions: u32,
    /// The first seat's casts of a spell that hurts.
    pub attacks: u32,
    /// The mean distance between the fighters, game pixels, over every tick of a round.
    pub mean_distance: f64,
}

impl Record {
    /// The share of finished rounds the first seat won.
    pub fn win_rate(&self) -> f64 {
        share(self.won[0] + self.won[1], self.won[0] + self.won[1] + self.lost[0] + self.lost[1])
    }

    /// The share it won as school `i` (0 Thunder, 1 Frost).
    pub fn as_school(&self, i: usize) -> f64 {
        let (w, l) = (self.won.get(i).copied().unwrap_or(0), self.lost.get(i).copied().unwrap_or(0));
        share(w, w + l)
    }

    /// The share of its decisions taken by its most frequent choice.
    pub fn most_used(&self) -> f64 {
        share(self.choices.values().copied().max().unwrap_or(0), self.decisions)
    }

    /// The share of its decisions that cast a spell that hurts.
    pub fn attack_share(&self) -> f64 {
        share(self.attacks, self.decisions)
    }

    pub fn mean_length(&self) -> f64 {
        if self.lengths.is_empty() { 0.0 } else { self.lengths.iter().sum::<f64>() / self.lengths.len() as f64 }
    }
}

fn share(a: u32, b: u32) -> f64 {
    if b == 0 { 0.0 } else { f64::from(a) / f64::from(b) }
}

/// What a counting seat writes.
#[derive(Default)]
struct Tally {
    choices: BTreeMap<String, u32>,
    decisions: u32,
    attacks: u32,
}

/// A seat that counts the choices of the seat inside it.
struct Counted {
    inner: Box<dyn Seat>,
    tally: Rc<RefCell<Tally>>,
}

impl Seat for Counted {
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        let c = self.inner.think(state, me, dt);
        if let Some(c) = &c {
            let mut t = self.tally.borrow_mut();
            t.decisions += 1;
            let key = match c {
                Choice::Move { dir } if *dir < 0.0 => "move left".to_string(),
                Choice::Move { .. } => "move right".to_string(),
                Choice::Jump => "jump".to_string(),
                Choice::Guard { .. } => "guard".to_string(),
                Choice::Duck { .. } => "duck".to_string(),
                Choice::Fly { height } if *height > 0.0 => "fly".to_string(),
                Choice::Fly { .. } => "land".to_string(),
                Choice::Meditate => "meditate".to_string(),
                Choice::Wait => "wait".to_string(),
                Choice::Cast { skill, .. } => {
                    if spell(skill).is_some_and(|s| s.damage.is_some_and(|d| d > 0.0)) {
                        t.attacks += 1;
                    }
                    format!("cast {skill}")
                }
            };
            *t.choices.entry(key).or_insert(0) += 1;
        }
        c
    }
    fn tell(&mut self, outcome: &Outcome) {
        self.inner.tell(outcome);
    }
    fn reset(&mut self) {
        self.inner.reset();
    }
    fn hold(&mut self, seconds: f64) {
        self.inner.hold(seconds);
    }
}

/// Seat `a` against seat `b` over `m`, each seed fought with `a` as Thunder and then as Frost.
pub fn match_up(a: &dyn Fn() -> MakeSeat, b: &dyn Fn() -> MakeSeat, m: &Match) -> Record {
    let mut r = Record::default();
    let tally = Rc::new(RefCell::new(Tally::default()));
    let (mut distance, mut ticks) = (0.0, 0_u64);
    for seed in m.seeds.clone() {
        for corner in 0..2 {
            let make = a();
            let t = Rc::clone(&tally);
            let counted: MakeSeat = Box::new(move |state, me| Box::new(Counted { inner: make(state, me), tally: t }));
            let seats = if corner == 0 { [counted, b()] } else { [b(), counted] };
            let config = DuelConfig { seed, left: 0.0, right: 220.0, ground: 150.0, sky: 96.0, effects: m.effects.clone() };
            let mut duel = Duel::new(config, seats);
            let mut started = duel.state.time;
            let mut round = duel.state.round;
            let mut ended = false;
            while duel.state.wins[0] + duel.state.wins[1] < m.rounds {
                duel.step(TICK);
                let s = &duel.state;
                if s.round != round {
                    // the next round has begun
                    started = s.time;
                    round = s.round;
                    ended = false;
                }
                if s.winner.is_some() {
                    if !ended {
                        r.lengths.push(s.time - started);
                        ended = true;
                    }
                    continue;
                }
                distance += (s.fighters[0].body.x - s.fighters[1].body.x).abs();
                ticks += 1;
                if s.time - started > m.limit {
                    r.timeouts += 1;
                    break;
                }
            }
            r.won[corner] += duel.state.wins[corner];
            r.lost[corner] += duel.state.wins[1 - corner];
        }
    }
    let t = tally.borrow();
    r.choices = t.choices.clone();
    r.decisions = t.decisions;
    r.attacks = t.attacks;
    r.mean_distance = if ticks == 0 { 0.0 } else { distance / ticks as f64 };
    r
}
