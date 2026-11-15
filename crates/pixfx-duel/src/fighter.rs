//! A fighter of the duel and the duel's whole state.

use crate::data::{Luck, Rarity, School, Stats, Trait, stats};
use crate::out::Out;
use crate::rng::Rng;
use crate::shot::Shot;
use crate::style::Style;
use pixfx_fighter::body::{Arena, Body};
use pixfx_mind::Outcome;

/// Where a cast is: gathering before it lands, or recovering after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Phase {
    Windup,
    Recover,
}

/// A cast under way: its spell, phase and time in it, where it aims, the power and stacks it strikes with, whether
/// it pierces a guard, and the rune circle it stands in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Act {
    pub spell: &'static str,
    pub phase: Phase,
    pub t: f64,
    pub target: Option<f64>,
    pub power: f64,
    pub stacks: f64,
    pub pierce: bool,
    pub circle: Option<u32>,
}

/// A slot of the rotating bar: counting down to a draw, or holding a skill drawn with its tier and traits.
#[derive(Clone, Debug, PartialEq)]
pub enum Slot {
    Empty { left: f64, full: f64 },
    Held { spell: &'static str, power: f64, evolved: bool, rarity: Rarity, traits: Vec<Trait>, idle: f64 },
}

impl Slot {
    /// The skill it holds.
    pub fn spell(&self) -> Option<&'static str> {
        match self {
            Slot::Held { spell, .. } => Some(spell),
            Slot::Empty { .. } => None,
        }
    }
}

/// A burn on a fighter: time left, time to the next tick, damage a tick, and who lit it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Burning {
    pub t: f64,
    pub tick: f64,
    pub per: f64,
    pub by: usize,
}

/// Shock stacks on a fighter, the time to the next twitch, and who put the last one there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shocked {
    pub stacks: f64,
    pub tick: f64,
    pub by: usize,
}

/// The last time luck smiled on a fighter's cast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LuckyCast {
    pub kind: Luck,
    pub spell: &'static str,
    pub slot: usize,
    pub at: f64,
}

/// One of the two fighters. Life (`hp`) and `qi` are shares, 0 to 100.
#[derive(Clone, Debug, PartialEq)]
pub struct Fighter {
    pub index: usize,
    pub school: School,
    /// Every spell it may cast this fight.
    pub spells: Vec<&'static str>,
    /// The school's skills its bar draws from this fight.
    pub pool: Vec<&'static str>,
    pub bar: Vec<Slot>,
    pub stats: Stats,
    pub hp: f64,
    pub qi: f64,
    pub act: Option<Act>,
    pub guard: f64,
    pub medit: bool,
    pub fly: bool,
    pub hover: f64,
    pub rage: f64,
    pub sealed: f64,
    /// A ward's charges, and the time it has left.
    pub charges: f64,
    pub shield_t: f64,
    /// A barrier's time left, the spell that raised it, and its effect.
    pub barrier: f64,
    pub barrier_spell: Option<&'static str>,
    pub barrier_key: Option<u32>,
    pub rang: f64,
    pub burn: Option<Burning>,
    pub def_down: f64,
    pub silence: f64,
    pub fury: f64,
    /// Stood up from a killing blow this round.
    pub rose: bool,
    pub dodged: f64,
    pub chill: f64,
    pub chill_t: f64,
    pub shock: Option<Shocked>,
    pub grit: f64,
    pub lucky: Option<LuckyCast>,
    /// Each spell's cooldown left and in full, in the order they were first cast.
    pub cool: Vec<(&'static str, f64)>,
    pub cool_full: Vec<(&'static str, f64)>,
    /// The way it walks: -1, 0 or 1.
    pub intent: f64,
    /// The flame it stands in while it burns.
    pub aura: Option<u32>,
    pub attune: f64,
    pub body: Body,
    /// How it fights, drawn once a fight (its seat reads it; the rules do not).
    pub style: Style,
}

impl Fighter {
    /// Fighter `index` of `school`, with the spells it may cast and the pool its bar draws from, fresh.
    pub fn new(index: usize, school: School, spells: Vec<&'static str>, pool: Vec<&'static str>) -> Fighter {
        Fighter {
            index,
            school,
            spells,
            pool,
            bar: Vec::new(),
            stats: *stats(school),
            hp: 100.0,
            qi: 100.0,
            act: None,
            guard: 0.0,
            medit: false,
            fly: false,
            hover: 0.0,
            rage: 0.0,
            sealed: 0.0,
            charges: 0.0,
            shield_t: 0.0,
            barrier: 0.0,
            barrier_spell: None,
            barrier_key: None,
            rang: 0.0,
            burn: None,
            def_down: 0.0,
            silence: 0.0,
            fury: 0.0,
            rose: false,
            dodged: 0.0,
            chill: 0.0,
            chill_t: 0.0,
            shock: None,
            grit: 0.0,
            lucky: None,
            cool: Vec::new(),
            cool_full: Vec::new(),
            intent: 0.0,
            aura: None,
            attune: 0.0,
            body: Body::new(0.0),
            style: Style::Balanced,
        }
    }

    /// The round's fresh state (`startRound`'s `Object.assign`): everything but the fighter's school, spells, body,
    /// bar and aura, which the round start handles itself.
    pub fn refresh(&mut self) {
        let fresh = Fighter::new(self.index, self.school, Vec::new(), Vec::new());
        *self = Fighter {
            spells: core::mem::take(&mut self.spells),
            pool: core::mem::take(&mut self.pool),
            bar: core::mem::take(&mut self.bar),
            stats: self.stats,
            aura: self.aura,
            attune: self.attune,
            body: self.body.clone(),
            style: self.style,
            ..fresh
        };
    }

    /// The height of its chest above the ground, where its events show.
    pub fn chest(&self) -> f64 {
        self.body.air + self.body.spec.chest
    }

    /// Cooldown left on `name` (JS `cool[name] ?? 0`).
    pub fn cool(&self, name: &str) -> f64 {
        self.cool.iter().find(|(n, _)| *n == name).map_or(0.0, |&(_, v)| v)
    }

    /// The full cooldown `name` was last set to.
    pub fn cool_full(&self, name: &str) -> f64 {
        self.cool_full.iter().find(|(n, _)| *n == name).map_or(0.0, |&(_, v)| v)
    }

    /// Set `name`'s cooldown and its full length; a new name joins at the end, as a JS object's key does.
    pub fn set_cool(&mut self, name: &'static str, left: f64, full: f64) {
        set(&mut self.cool, name, left);
        set(&mut self.cool_full, name, full);
    }

    /// Set only the cooldown left on `name`.
    pub fn set_cool_left(&mut self, name: &'static str, left: f64) {
        set(&mut self.cool, name, left);
    }
}

fn set(list: &mut Vec<(&'static str, f64)>, name: &'static str, v: f64) {
    match list.iter_mut().find(|(n, _)| *n == name) {
        Some(e) => e.1 = v,
        None => list.push((name, v)),
    }
}

/// What the rules tell a fighter's mind; the duel hands them over in order before that mind next thinks.
#[derive(Clone, Debug, PartialEq)]
pub enum Told {
    Outcome(Outcome),
    Reset,
}

/// The duel's whole state: the arena, the dice, the clock, the rounds, the fighters and what this step emitted.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub seed: u32,
    pub rng: Rng,
    pub left: f64,
    pub right: f64,
    pub ground: f64,
    /// How high above the ground a fighter may fly.
    pub top: f64,
    pub time: f64,
    /// Time left over from the last step, short of a whole tick.
    pub carry: f64,
    pub round: u32,
    pub wins: [u32; 2],
    pub winner: Option<usize>,
    pub pause: f64,
    /// The last effect key handed out.
    pub key: u32,
    /// Spells in flight, in the order they were cast.
    pub shots: Vec<Shot>,
    /// The last shot id handed out.
    pub shot_id: u32,
    pub fighters: [Fighter; 2],
    pub out: Vec<Out>,
    /// The effects in the registry: a spell whose effect is missing is never cast.
    pub effects: Vec<String>,
    /// The arena shows its lamps and night.
    pub decor: bool,
    /// What the rules told each mind since it last thought.
    pub told: Vec<(usize, Told)>,
}

impl State {
    pub fn new(seed: u32, left: f64, right: f64, ground: f64, top: f64, effects: Vec<String>, fighters: [Fighter; 2]) -> State {
        State {
            seed,
            rng: Rng::new(seed),
            left,
            right,
            ground,
            top,
            time: 0.0,
            carry: 0.0,
            round: 0,
            wins: [0, 0],
            winner: None,
            pause: 0.0,
            key: 0,
            shots: Vec::new(),
            shot_id: 0,
            fighters,
            out: Vec::new(),
            effects,
            decor: false,
            told: Vec::new(),
        }
    }

    /// Whether effect `fx` is in the registry.
    pub fn has(&self, fx: &str) -> bool {
        self.effects.iter().any(|e| e == fx)
    }

    /// The arena the bodies move in.
    pub fn arena(&self) -> Arena {
        Arena { left: self.left, right: self.right, top: self.top }
    }

    pub fn emit(&mut self, out: Out) {
        self.out.push(out);
    }

    /// Tell fighter `me`'s mind something.
    pub fn tell(&mut self, me: usize, told: Told) {
        self.told.push((me, told));
    }
}
