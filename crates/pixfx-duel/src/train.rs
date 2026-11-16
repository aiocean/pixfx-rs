//! The training environment: the trainer's learner in one corner of a fresh duel, an opponent from the league in the
//! other, one round a episode. The learner decides through a `PolicyMind` with the timing the page plays, so what
//! it learns is what it does on the page.
//!
//! Reward, per decision: +1 for the round won and -1 for the round lost (a round cut at `limit` seconds is lost too,
//! so stalling never pays); `shaping` x (dealt - taken) / 100, which the trainer anneals to 0; minus
//! `STALL` a decision once `QUIET` seconds pass with no damage either way; minus `REPEAT` when one act (other than
//! waiting) is chosen `RUN` times in a row; minus `HABIT` when the act chosen is already `SHARE` or more of the last
//! `HABITS` decisions.

use crate::duel::{Duel, DuelConfig, TICK};
use crate::first_mind::FirstMind;
use crate::fighter::State;
use crate::scripted::Scripted;
use crate::seat::{Brain, MakeSeat, PolicySeat, Seat, policy_mind};
use crate::sense::sense_of;
use crate::style::Style;
use pixfx_mind::policy::{ACTIONS, Due, Net, OBS, PolicyMind};
use pixfx_mind::{Choice, Outcome};
use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;

/// The penalty a decision for a fight gone quiet, and how long it must be quiet.
const STALL: f32 = 0.002;
const QUIET: f64 = 6.0;
/// The penalty for choosing one act `RUN` times in a row.
const REPEAT: f32 = 0.002;
const RUN: u32 = 6;
/// What choosing an act that already fills `SHARE` of the last `HABITS` decisions (about 5 s of play) costs.
const HABIT: f32 = 0.004;
const HABITS: usize = 40;
const SHARE: f64 = 0.4;

/// Who the learner fights.
#[derive(Clone, Debug)]
pub enum Opponent {
    Brain,
    First,
    Aggressive,
    Turtle,
    /// A policy (the learner's past or present self) in `EnvConfig::foe_style`.
    Policy(Rc<Net>),
}

/// One episode: the fight's seed, the learner's corner (0 Thunder, 1 Frost) and style, the opponent and its style,
/// the seconds a round may last, and the effects in the registry (the page's whole registry: some rules play only
/// when their effect is there).
#[derive(Clone, Debug)]
pub struct EnvConfig {
    pub seed: u32,
    pub learner: usize,
    pub style: Style,
    pub foe_style: Style,
    pub opponent: Opponent,
    pub limit: f64,
    pub effects: Vec<String>,
}

/// What one decision earned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub reward: f32,
    pub done: bool,
    /// At a knockout, whether the learner won; `None` while the round goes on or when it was cut.
    pub won: Option<bool>,
    /// The round's length so far, seconds.
    pub length: f64,
}

/// What passes between the env and its learner's seat.
#[derive(Default)]
struct Wire {
    ask: Option<Due>,
    answer: Option<usize>,
}

/// The learner's seat: a policy mind whose decisions the trainer answers.
struct Learner {
    mind: PolicyMind,
    wire: Rc<RefCell<Wire>>,
}

impl Seat for Learner {
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        let sense = sense_of(state, me);
        let mut wire = self.wire.borrow_mut();
        if let Some(a) = wire.answer.take() {
            self.mind.queue(a);
        }
        let now = self.mind.begin(&sense, dt);
        if let Some(due) = self.mind.due(&sense) {
            wire.ask = Some(due);
        }
        now
    }

    fn tell(&mut self, _outcome: &Outcome) {}

    fn reset(&mut self) {
        self.mind.reset();
    }

    fn hold(&mut self, seconds: f64) {
        self.mind.hold(seconds);
    }
}

/// A training episode.
pub struct Env {
    duel: Duel,
    me: usize,
    limit: f64,
    wire: Rc<RefCell<Wire>>,
    /// The decision the trainer answers next.
    due: Due,
    quiet: f64,
    last: Option<usize>,
    run: u32,
    /// The last `HABITS` acts it chose.
    recent: VecDeque<usize>,
    done: bool,
}

impl Env {
    /// A fresh duel, run to the learner's first decision.
    pub fn new(cfg: EnvConfig) -> Env {
        let wire = Rc::new(RefCell::new(Wire::default()));
        let (style, w) = (cfg.style, Rc::clone(&wire));
        let learner: MakeSeat = Box::new(move |state, me| Box::new(Learner { mind: policy_mind(state, me, style), wire: w }));
        let foe: MakeSeat = match cfg.opponent {
            Opponent::Brain => Brain::make(),
            Opponent::First => FirstMind::make(),
            Opponent::Aggressive => Scripted::aggressive(),
            Opponent::Turtle => Scripted::turtle(),
            Opponent::Policy(net) => PolicySeat::with_style(net, cfg.foe_style),
        };
        let me = cfg.learner.min(1);
        let seats = if me == 0 { [learner, foe] } else { [foe, learner] };
        let config = DuelConfig { seed: cfg.seed, left: 0.0, right: 220.0, ground: 150.0, sky: 96.0, effects: cfg.effects };
        let blank = Due { obs: [0.0; OBS], legal: [false; ACTIONS], u: 0.0 };
        let mut env = Env { duel: Duel::new(config, seats), me, limit: cfg.limit, wire, due: blank, quiet: 0.0, last: None, run: 0, recent: VecDeque::with_capacity(HABITS + 1), done: false };
        env.advance();
        env
    }

    /// What the learner sees now and what it may do.
    pub fn observe(&self) -> (&[f32; OBS], &[bool; ACTIONS]) {
        (&self.due.obs, &self.due.legal)
    }

    /// Answer the learner's decision with `action` (an act it may not do is a wait), run the duel to its next
    /// decision or the round's end, and pay for it. `shaping` weighs the damage term (the trainer anneals it to 0).
    pub fn step(&mut self, action: usize, shaping: f32) -> Step {
        let length = self.duel.state.time;
        if self.done {
            return Step { reward: 0.0, done: true, won: None, length };
        }
        let act = if self.due.legal.get(action).copied().unwrap_or(false) { action } else { 0 };
        let mut reward = 0.0;
        if act != 0 && self.last == Some(act) {
            self.run += 1;
        } else {
            self.run = 1;
        }
        self.last = Some(act);
        if act != 0 && self.run >= RUN {
            reward -= REPEAT;
        }
        if self.quiet >= QUIET {
            reward -= STALL;
        }
        let same = self.recent.iter().filter(|&&a| a == act).count();
        if self.recent.len() == HABITS && same as f64 >= SHARE * HABITS as f64 {
            reward -= HABIT;
        }
        self.recent.push_back(act);
        if self.recent.len() > HABITS {
            self.recent.pop_front();
        }
        self.wire.borrow_mut().answer = Some(act);
        let (dealt, taken, won) = self.advance();
        reward += shaping * ((dealt - taken) / 100.0) as f32;
        let length = self.duel.state.time;
        let cut = won.is_none() && length >= self.limit;
        if won.is_some() || cut {
            self.done = true;
            reward += if won == Some(true) { 1.0 } else { -1.0 };
        }
        Step { reward, done: self.done, won, length }
    }

    /// Run whole ticks until the learner's next decision, the round's end or the limit: the damage it dealt and
    /// took on the way, and, at a knockout, whether it won.
    fn advance(&mut self) -> (f64, f64, Option<bool>) {
        let (me, foe) = (self.me, 1 - self.me);
        let (mut dealt, mut taken) = (0.0, 0.0);
        loop {
            let before = [self.duel.state.fighters[0].hp, self.duel.state.fighters[1].hp];
            self.duel.step(TICK);
            let f = &self.duel.state.fighters;
            let (d, t) = ((before[foe] - f[foe].hp).max(0.0), (before[me] - f[me].hp).max(0.0));
            dealt += d;
            taken += t;
            self.quiet = if d > 0.0 || t > 0.0 { 0.0 } else { self.quiet + TICK };
            if let Some(w) = self.duel.state.winner {
                return (dealt, taken, Some(w == me));
            }
            if self.duel.state.time >= self.limit {
                return (dealt, taken, None);
            }
            if let Some(due) = self.wire.borrow_mut().ask.take() {
                self.due = due;
                return (dealt, taken, None);
            }
        }
    }
}
