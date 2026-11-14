//! What a fighter could perceive this tick, as the game hands it in: the true state. The mind applies its own
//! reaction time to the foe and the threats.

use crate::skill::{Skill, SkillKind};

/// The arena's walls and ceiling, in game pixels.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Arena {
    pub left: f64,
    pub right: f64,
    pub top: f64,
}

/// Which half of a skill a fighter is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum ActPhase {
    Windup,
    Recover,
}

/// What the foe is doing.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        tag = "type",
        rename_all = "camelCase",
        rename_all_fields = "camelCase",
        deny_unknown_fields
    )
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum Doing {
    Idle,
    /// Walking: `dir` -1 left, 1 right.
    Move {
        dir: f64,
    },
    Guard,
    Meditate,
    Fly,
    /// Using a skill: `t` seconds into its `phase`.
    Act {
        kind: SkillKind,
        phase: ActPhase,
        t: f64,
        windup: f64,
        recover: f64,
        reach: f64,
        damage: f64,
    },
}

/// The fighter itself.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Me {
    pub x: f64,
    /// Height of the feet over the ground.
    pub air: f64,
    /// -1 facing left, 1 right.
    pub face: f64,
    /// Life, 0 to 100.
    pub hp: f64,
    /// Qi, 0 to 100.
    pub qi: f64,
    pub rage: f64,
    /// Rage an ultimate needs.
    pub rage_full: f64,
    pub flying: bool,
    pub hover: f64,
    pub vx: f64,
    /// Free to act: not in a skill, reeling, crouching to jump, ducking, frozen, shocked or knocked out.
    pub can_act: bool,
    pub silenced: bool,
    /// Blink is sealed.
    pub sealed: bool,
    pub barrier: bool,
    pub shield: bool,
    /// A seeker of its own still hunts the foe.
    pub seeker_out: bool,
}

/// The foe.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Foe {
    pub x: f64,
    pub air: f64,
    pub flying: bool,
    pub hp: f64,
    pub qi: f64,
    pub doing: Doing,
    /// Seconds until it can act again (staggered, falling, lying, getting up).
    pub out_for: f64,
    /// Seconds until a blow can touch it.
    pub open_in: f64,
    pub frozen: f64,
    pub shocked: f64,
    pub barrier: bool,
    /// Its damaging skills ready now: what it could answer with.
    pub ready: Vec<Skill>,
}

/// The shape of a threat in flight, as the mind reasons about it. Heights from the ground, lengths in game pixels.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        tag = "type",
        rename_all = "camelCase",
        rename_all_fields = "camelCase",
        deny_unknown_fields
    )
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum ThreatShape {
    /// Flies level at height `y`.
    Projectile { x: f64, y: f64, vx: f64 },
    /// Lands on `x` in `lands_in` seconds, hurting within `radius` (and drawing in from `reach`).
    Area {
        x: f64,
        radius: f64,
        reach: f64,
        lands_in: f64,
        grounded: bool,
    },
    /// Hunts the foe; at `x`, `y` now.
    Seeker { x: f64, y: f64, radius: f64 },
    /// Stays on `x`, hurting within `reach`.
    Field { x: f64, reach: f64 },
    /// Strikes that follow the one they fall on; the next lands in `lands_in`.
    Barrage { lands_in: f64 },
}

/// A foe's shot that could reach me.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Threat {
    /// Seconds it has been there to see.
    pub age: f64,
    pub damage: f64,
    pub shape: ThreatShape,
}

/// Everything a fighter could perceive this tick.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Sense {
    /// The fight's clock, seconds.
    pub time: f64,
    pub me: Me,
    pub foe: Foe,
    pub arena: Arena,
    pub threats: Vec<Threat>,
    /// Whether each skill handed to `Mind::new` is ready now, in that order.
    pub ready: Vec<bool>,
}
