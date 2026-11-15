//! The duel as the page reads it: each fighter's body, life, statuses, cast, skill bar and buffs, and the round.

use crate::style::Style;
use crate::bar::{Buff, SlotView, buffs_of, skill_bar};
use crate::data::{Luck, School, Stats};
use crate::fighter::{Fighter, Phase, State};
use pixfx_fighter::pose::Pose;

/// The duel this tick.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct DuelView {
    pub time: f64,
    pub round: u32,
    pub wins: [u32; 2],
    pub winner: Option<usize>,
    pub left: f64,
    pub right: f64,
    pub ground: f64,
    pub fighters: [FighterView; 2],
}

/// A fighter this tick.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FighterView {
    pub index: usize,
    pub school: School,
    /// How it fights this fight, shown before the fight so a watcher can study it.
    pub style: Style,
    pub hp: f64,
    pub qi: f64,
    pub rage: f64,
    pub x: f64,
    pub air: f64,
    pub vx: f64,
    pub vy: f64,
    pub face: f64,
    pub pose: Pose,
    pub pose_t: f64,
    pub flash: f64,
    pub hit_t: f64,
    pub hit_power: f64,
    pub frozen: f64,
    pub shocked: f64,
    pub dodged: f64,
    pub rang: f64,
    pub charges: f64,
    pub barrier: f64,
    /// It is burning.
    pub burn: bool,
    pub lucky: Option<LuckView>,
    pub act: Option<ActView>,
    pub stats: Stats,
    pub untouchable: bool,
    pub skill_bar: Vec<SlotView>,
    pub buffs: Vec<Buff>,
}

/// A cast under way: its spell, phase and time in it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ActView {
    pub spell: &'static str,
    pub phase: Phase,
    pub t: f64,
}

/// The last lucky cast: which luck, the spell, its slot, and when.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct LuckView {
    pub kind: Luck,
    pub spell: &'static str,
    pub slot: usize,
    pub at: f64,
}

/// The duel in `state` as the page reads it.
pub fn view(state: &State) -> DuelView {
    DuelView {
        time: state.time,
        round: state.round,
        wins: state.wins,
        winner: state.winner,
        left: state.left,
        right: state.right,
        ground: state.ground,
        fighters: [fighter_view(&state.fighters[0]), fighter_view(&state.fighters[1])],
    }
}

fn fighter_view(f: &Fighter) -> FighterView {
    let b = &f.body;
    FighterView {
        index: f.index,
        school: f.school,
        style: f.style,
        hp: f.hp,
        qi: f.qi,
        rage: f.rage,
        x: b.x,
        air: b.air,
        vx: b.vx,
        vy: b.vy,
        face: b.face,
        pose: b.pose,
        pose_t: b.pose_t,
        flash: b.flash,
        hit_t: b.hit_t,
        hit_power: b.hit_power,
        frozen: b.frozen,
        shocked: b.shocked,
        dodged: f.dodged,
        rang: f.rang,
        charges: f.charges,
        barrier: f.barrier,
        burn: f.burn.is_some(),
        lucky: f.lucky.map(|l| LuckView { kind: l.kind, spell: l.spell, slot: l.slot, at: l.at }),
        act: f.act.map(|a| ActView { spell: a.spell, phase: a.phase, t: a.t }),
        stats: f.stats,
        untouchable: b.untouchable(),
        skill_bar: skill_bar(f),
        buffs: buffs_of(f),
    }
}
