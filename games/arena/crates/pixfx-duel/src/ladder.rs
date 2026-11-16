//! The ladder: one seat against another over many seeds, each fought both ways (as Thunder, then as Frost) so the
//! schools' own balance cancels out, with what a watcher would judge: who wins as each school, how long rounds last,
//! whether fights stall, how varied and how aggressive the first seat plays, and the distance it keeps; and, for each
//! school whoever sits in it, what each skill did (telemetry), to see where one school's edge comes from.

use crate::bar::cost_of;
use crate::data::spell;
use crate::duel::{Duel, DuelConfig, TICK};
use crate::fighter::State;
use crate::out::{Note, Out, Stop};
use crate::seat::{MakeSeat, Seat};
use pixfx_mind::{Choice, Outcome, Posture};
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
    /// What each school (0 Thunder, 1 Frost) did, whichever seat sat in it.
    pub schools: [SchoolLog; 2],
}

/// What one skill of a school did over a match. Shares of life are of a full life (100 is one life).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkillLog {
    pub casts: u32,
    /// Blows of it that hurt the foe (a barrage's strikes and a bell's ring each count).
    pub landed: u32,
    /// The share of the foe's life it took: blows, what got through a guard, and a field's ticks.
    pub damage: f64,
    /// Blows the foe's guard held; `chip` is what still got through (in `damage` too).
    pub guarded: u32,
    pub chip: f64,
    /// Blows the foe's barrier turned aside or sent back.
    pub barrier: u32,
    /// Blows a ward's charge erased.
    pub ward: u32,
    pub dodged: u32,
    /// Blades that met the foe's blade.
    pub clashed: u32,
    /// Casts of it broken by a hit while they gathered.
    pub broken: u32,
    /// Times the foe struck while it recovered from it.
    pub punished: u32,
}

/// What one school did over a match.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SchoolLog {
    /// By skill: its own, and a foe's bolt it sent back.
    pub skills: BTreeMap<&'static str, SkillLog>,
    /// The share of the foe's life its burns and shocks took.
    pub statuses: BTreeMap<&'static str, f64>,
    /// What dealt the last blow of each round it won: a skill, "burn" or "shock".
    pub finals: BTreeMap<&'static str, u32>,
    /// The share of its own life it healed, and the share it lost, as its life shows.
    pub healed: f64,
    pub life_lost: f64,
    /// Qi paid for its casts, and rage spent on ultimates.
    pub qi_spent: f64,
    pub rage_spent: f64,
    /// Seconds of rounds in play, and of those, seconds spent meditating, guarding and flying.
    pub seconds: f64,
    pub meditating: f64,
    pub guarding: f64,
    pub flying: f64,
}

impl SchoolLog {
    /// The share of the foe's life it took, every skill and status together.
    pub fn dealt(&self) -> f64 {
        self.skills.values().map(|s| s.damage).sum::<f64>() + self.statuses.values().sum::<f64>()
    }

    fn skill(&mut self, name: &'static str) -> &mut SkillLog {
        self.skills.entry(name).or_default()
    }
}

impl Record {
    /// The share of finished rounds the first seat won.
    pub fn win_rate(&self) -> f64 {
        share(
            self.won[0] + self.won[1],
            self.won[0] + self.won[1] + self.lost[0] + self.lost[1],
        )
    }

    /// The share it won as school `i` (0 Thunder, 1 Frost).
    pub fn as_school(&self, i: usize) -> f64 {
        let (w, l) = (
            self.won.get(i).copied().unwrap_or(0),
            self.lost.get(i).copied().unwrap_or(0),
        );
        share(w, w + l)
    }

    /// The share of its decisions taken by its most frequent choice.
    pub fn most_used(&self) -> f64 {
        share(
            self.choices.values().copied().max().unwrap_or(0),
            self.decisions,
        )
    }

    /// The share of its decisions that cast a spell that hurts.
    pub fn attack_share(&self) -> f64 {
        share(self.attacks, self.decisions)
    }

    pub fn mean_length(&self) -> f64 {
        if self.lengths.is_empty() {
            0.0
        } else {
            self.lengths.iter().sum::<f64>() / self.lengths.len() as f64
        }
    }
}

fn share(a: u32, b: u32) -> f64 {
    if b == 0 {
        0.0
    } else {
        f64::from(a) / f64::from(b)
    }
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

/// A seat that writes its school's casts, what they cost, and the moves it was told were punished.
struct Watched {
    inner: Box<dyn Seat>,
    me: usize,
    logs: Rc<RefCell<[SchoolLog; 2]>>,
}

impl Seat for Watched {
    fn think(&mut self, state: &State, me: usize, dt: f64) -> Option<Choice> {
        let c = self.inner.think(state, me, dt);
        if let Some(Choice::Cast { skill, .. }) = &c
            && let Some(sp) = spell(skill)
        {
            let f = &state.fighters[me];
            let mut logs = self.logs.borrow_mut();
            let log = &mut logs[me];
            log.skill(sp.name).casts += 1;
            log.qi_spent += cost_of(f, sp.name);
            if sp.ultimate {
                log.rage_spent += f.rage;
            }
        }
        c
    }
    fn tell(&mut self, outcome: &Outcome) {
        if let Outcome::Punished { skill } = outcome
            && let Some(sp) = spell(skill)
        {
            self.logs.borrow_mut()[self.me].skill(sp.name).punished += 1;
        }
        self.inner.tell(outcome);
    }
    fn reset(&mut self) {
        self.inner.reset();
    }
    fn hold(&mut self, seconds: f64) {
        self.inner.hold(seconds);
    }
    fn posture(&self) -> Option<Posture> {
        self.inner.posture()
    }
}

fn watched(make: MakeSeat, logs: &Rc<RefCell<[SchoolLog; 2]>>) -> MakeSeat {
    let logs = Rc::clone(logs);
    Box::new(move |state, me| {
        Box::new(Watched {
            inner: make(state, me),
            me,
            logs,
        })
    })
}

/// What took life from a fighter: a skill of its foe's, or a status.
#[derive(Clone, Copy)]
enum Cause {
    Skill(&'static str),
    Status(&'static str),
}

/// Each fighter's life as a tick's blows land, and what last took some.
struct Lives {
    life: [f64; 2],
    last: [Option<Cause>; 2],
}

impl Lives {
    /// Fighter `f` loses `share` of its life to its foe, but nothing past what it has left; returns what it lost.
    fn took(&mut self, logs: &mut [SchoolLog; 2], f: usize, cause: Cause, share: f64) -> f64 {
        let taken = share.min(self.life[f]).max(0.0);
        self.life[f] -= taken;
        let by = &mut logs[1 - f];
        match cause {
            Cause::Skill(name) => by.skill(name).damage += taken,
            Cause::Status(name) => *by.statuses.entry(name).or_insert(0.0) += taken,
        }
        if taken > 0.0 {
            self.last[f] = Some(cause);
        }
        taken
    }
}

/// One tick's telemetry: what its events and notes say each blow did, from each fighter's life `before` the tick,
/// and what each fighter was doing after it.
fn account(logs: &mut [SchoolLog; 2], before: [f64; 2], out: &[Out], state: &State) {
    let mut lives = Lives {
        life: before,
        last: [None, None],
    };
    let mut healed = [0.0; 2];
    // a note stands where it was noted among the events: after the first `at` of them
    let mut notes = state.notes.iter().peekable();
    for i in 0..=out.len() {
        while let Some((_, note)) = notes.next_if(|(at, _)| *at <= i) {
            match *note {
                Note::Stopped {
                    by,
                    spell,
                    how,
                    chip,
                } => {
                    let s = logs[by].skill(spell);
                    match how {
                        Stop::Guard => s.guarded += 1,
                        Stop::Barrier => s.barrier += 1,
                        Stop::Ward => s.ward += 1,
                        Stop::Dodge => s.dodged += 1,
                        Stop::Clash => s.clashed += 1,
                    }
                    let taken = lives.took(logs, 1 - by, Cause::Skill(spell), chip);
                    logs[by].skill(spell).chip += taken;
                }
                Note::Ticked { by, cause, share } => {
                    let cause = match spell(cause) {
                        Some(sp) => Cause::Skill(sp.name),
                        None => Cause::Status(cause),
                    };
                    lives.took(logs, 1 - by, cause, share);
                }
                Note::Healed { by, share } => {
                    lives.life[by] += share;
                    healed[by] += share;
                    logs[by].healed += share;
                }
                Note::Broken { who, spell } => logs[who].skill(spell).broken += 1,
            }
        }
        if let Some(Out::Hit {
            by, spell, share, ..
        }) = out.get(i)
        {
            logs[*by].skill(spell).landed += 1;
            lives.took(logs, 1 - by, Cause::Skill(spell), *share);
        }
    }
    for (f, log) in logs.iter_mut().enumerate() {
        let me = &state.fighters[f];
        log.life_lost += before[f] - me.hp + healed[f];
        let on = |b: bool| if b { TICK } else { 0.0 };
        log.seconds += TICK;
        log.meditating += on(me.medit);
        log.guarding += on(me.guard > 0.0);
        log.flying += on(me.fly);
    }
    if let Some(w) = state.winner {
        let name = match lives.last[1 - w] {
            Some(Cause::Skill(n) | Cause::Status(n)) => n,
            None => "?",
        };
        *logs[w].finals.entry(name).or_insert(0) += 1;
    }
}

/// Seat `a` against seat `b` over `m`, each seed fought with `a` as Thunder and then as Frost.
pub fn match_up(a: &dyn Fn() -> MakeSeat, b: &dyn Fn() -> MakeSeat, m: &Match) -> Record {
    let mut r = Record::default();
    let tally = Rc::new(RefCell::new(Tally::default()));
    let logs = Rc::new(RefCell::new(<[SchoolLog; 2]>::default()));
    let (mut distance, mut ticks) = (0.0, 0_u64);
    for seed in m.seeds.clone() {
        for corner in 0..2 {
            let make = a();
            let t = Rc::clone(&tally);
            let counted: MakeSeat = Box::new(move |state, me| {
                Box::new(Counted {
                    inner: make(state, me),
                    tally: t,
                })
            });
            let seats = if corner == 0 {
                [counted, b()]
            } else {
                [b(), counted]
            }
            .map(|seat| watched(seat, &logs));
            let config = DuelConfig {
                seed,
                left: 0.0,
                right: 220.0,
                ground: 150.0,
                sky: 96.0,
                effects: m.effects.clone(),
            };
            let mut duel = Duel::new(config, seats);
            let mut started = duel.state.time;
            let mut round = duel.state.round;
            let mut ended = false;
            while duel.state.wins[0] + duel.state.wins[1] < m.rounds {
                let fighting = duel.state.winner.is_none();
                let before = duel.state.fighters.each_ref().map(|f| f.hp);
                let out = duel.step(TICK);
                if fighting {
                    account(&mut logs.borrow_mut(), before, &out, &duel.state);
                }
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
    r.schools = logs.borrow().clone();
    r.mean_distance = if ticks == 0 {
        0.0
    } else {
        distance / ticks as f64
    };
    r
}
