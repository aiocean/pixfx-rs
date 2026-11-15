//! The mind of one fighter for a round: it takes in every tick's sense, sees the foe late, counts its habits, and
//! every `think` seconds or so (when free to act) reads the moment, settles its posture, and picks what to do.

use crate::candidates::{Candidate, Ctx, candidates};
use crate::choice::{Choice, Outcome, Posture, Thought, Why};
use crate::habits::{Answers, FoeMove, Habits, foe_move};
use crate::pick::{Read, pick};
use crate::posture::{Category, Held, POSTURES, choose_posture};
use crate::reading::{Reading, read_posture};
use crate::rng::Rng;
use crate::sense::{Foe, Sense};
use crate::skill::{BodySize, Profile, Skill};
use crate::view::Delayed;
use crate::yomi::Yomi;

/// How long after my move the foe's next move counts as its answer to it, seconds.
const ANSWER_WINDOW: f64 = 1.0;
/// How many candidates a `Why` lists.
const WHY_TOP: usize = 5;

pub struct Mind {
    profile: Profile,
    body: BodySize,
    skills: Vec<Skill>,
    foe_skills: Vec<Skill>,
    rng: Rng,
    explain: bool,
    // the round
    clock: f64,
    calm: f64,
    hold: f64,
    held: Held,
    level: u8,
    now: f64,
    view: Delayed,
    seen: Option<Foe>,
    habits: Habits,
    answers: Answers,
    yomi: Yomi,
    read: Read,
    /// My last move: its category, when, and the yomi level it was made at.
    mine: Option<(Category, f64, u8)>,
}

impl Mind {
    pub fn new(profile: Profile, body: BodySize, skills: Vec<Skill>, seed: u32) -> Mind {
        let n = skills.len();
        let mut mind = Mind {
            profile,
            body,
            skills,
            foe_skills: Vec::new(),
            rng: Rng::new(seed),
            explain: false,
            clock: 0.0,
            calm: 0.0,
            hold: 0.0,
            held: Held {
                posture: Posture::Neutral,
                since: f64::NEG_INFINITY,
            },
            level: 0,
            now: 0.0,
            view: Delayed::new(),
            seen: None,
            habits: Habits::new(),
            answers: Answers::new(),
            yomi: Yomi::new(),
            read: Read::new(n),
            mine: None,
        };
        mind.reset();
        mind
    }

    /// A new round: everything it learnt and held is forgotten; its dice roll on. The first thought waits a moment.
    pub fn reset(&mut self) {
        self.clock = 0.4 + 0.3 * self.rng.next();
        self.calm = 0.0;
        self.hold = 0.0;
        self.held = Held {
            posture: Posture::Neutral,
            since: f64::NEG_INFINITY,
        };
        self.level = 0;
        self.view.clear();
        self.seen = None;
        self.habits.clear();
        self.answers.clear();
        self.yomi.clear();
        self.read.clear();
        self.mine = None;
    }

    /// The foe's skills, once: each sense then says only which of them are ready (`foe.ready`).
    pub fn meet(&mut self, foe_skills: Vec<Skill>) {
        self.foe_skills = foe_skills;
    }

    /// No thought for `seconds`.
    pub fn hold(&mut self, seconds: f64) {
        self.hold = seconds;
    }

    /// Fill every thought's `why`.
    pub fn explain(&mut self, on: bool) {
        self.explain = on;
    }

    pub fn posture(&self) -> Posture {
        self.held.posture
    }

    /// Take in this tick's sense, `dt` seconds after the last; a thought when it is time to think and free to act.
    pub fn think(&mut self, sense: &Sense, dt: f64) -> Option<Thought> {
        self.now = sense.time;
        if dt > 0.0 {
            self.clock -= dt;
            self.calm = (self.calm - dt).max(0.0);
            self.hold = (self.hold - dt).max(0.0);
        }
        self.view.push(sense);
        let (foe, threats) = self.view.seen(sense.time, self.profile.reaction);
        self.learn(sense, &foe);
        if !sense.me.can_act || self.clock > 0.0 || self.hold > 0.0 {
            return None;
        }
        self.clock = self.profile.think * (0.8 + 0.4 * self.rng.next());
        // each skill as hard as it strikes now
        let skills = powered(&self.skills, &sense.power);

        let reading = Reading::of(
            &sense.me,
            &foe,
            &threats,
            &sense.arena,
            &skills,
            &sense.ready,
            &self.body,
            &self.profile,
            self.calm,
        );
        let (posture, scores) = choose_posture(&self.held, sense.time, &reading, &self.profile);
        if posture != self.held.posture {
            self.held = Held {
                posture,
                since: sense.time,
            };
            self.level = self.yomi.draw(&mut self.rng);
        }
        let predicted = self.habits.predict(reading.foe_posture, sense.time);
        let answer = self
            .mine
            .and_then(|(c, _, _)| self.answers.predict(c, sense.time));
        let roll = self.rng.next();
        let cands = candidates(&Ctx {
            me: &sense.me,
            foe: &foe,
            foe_skills: &self.foe_skills,
            threats: &threats,
            arena: &sense.arena,
            skills: &skills,
            ready: &sense.ready,
            body: &self.body,
            profile: &self.profile,
            reading: &reading,
            read: &self.read,
            calm: self.calm,
            now: sense.time,
            predicted,
            answer,
            level: self.level,
            roll,
        });
        let weights = &POSTURES[posture as usize].weights;
        let picked = pick(&cands, weights, &mut self.rng);
        let choice = match &picked {
            Some((i, _)) => self.commit(&cands[*i], sense.time),
            None => Choice::Wait,
        };
        let why = self.explain.then(|| Why {
            postures: Posture::ALL.iter().copied().zip(scores).collect(),
            top: picked
                .as_ref()
                .map(|(_, ranked)| {
                    ranked
                        .iter()
                        .take(WHY_TOP)
                        .map(|(i, s)| (cands[*i].choice.clone(), *s))
                        .collect()
                })
                .unwrap_or_default(),
            level: self.level,
            predicted: predicted.map(|(m, sure)| format!("{m:?} {sure:.2}")),
        });
        Some(Thought {
            choice,
            posture,
            why,
        })
    }

    /// What the game saw happen to my skills.
    pub fn tell(&mut self, outcome: &Outcome) {
        let (name, payoff) = match outcome {
            Outcome::Landed { skill } => (skill, 1.0),
            Outcome::Punished { skill } => (skill, -1.0),
        };
        let Some(i) = self.skills.iter().position(|s| &s.name == name) else {
            return;
        };
        if payoff > 0.0 {
            self.read.landed(i);
        } else {
            self.read.punished(i, self.now);
        }
        if let Some((_, _, level)) = self.mine {
            self.yomi.paid(level, payoff);
        }
    }

    /// Count what the foe started since the last tick: a habit, and an answer to my last move.
    fn learn(&mut self, sense: &Sense, foe: &Foe) {
        if let Some(before) = &self.seen
            && let Some(m) = foe_move(before, foe, sense.me.x)
        {
            self.habits.see(read_posture(&sense.me, foe), m, sense.time);
            if let Some((mine, at, _)) = self.mine
                && sense.time - at <= ANSWER_WINDOW
                && m != FoeMove::Idle
            {
                self.answers.see(mine, m, sense.time);
            }
        }
        self.seen = Some(foe.clone());
    }

    /// Make the candidate the choice: an attack takes a breath after it and counts as tried.
    fn commit(&mut self, c: &Candidate, now: f64) -> Choice {
        if let Some(i) = c.skill
            && self.skills.get(i).is_some_and(|s| s.damage > 0.0)
        {
            let p = &self.profile;
            self.calm = p.tempo * (1.0 - p.spread / 2.0 + p.spread * self.rng.next());
            self.read.tried(i);
        }
        if !matches!(c.category, Category::Wait) {
            self.mine = Some((c.category, now, self.level));
        }
        c.choice.clone()
    }
}

/// `skills` with each one's damage scaled by its share in `power`; a skill past the end of `power` keeps its damage.
fn powered(skills: &[Skill], power: &[f64]) -> Vec<Skill> {
    skills
        .iter()
        .enumerate()
        .map(|(i, s)| Skill {
            damage: s.damage * power.get(i).copied().unwrap_or(1.0),
            ..s.clone()
        })
        .collect()
}
