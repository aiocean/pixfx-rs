//! What comes out of the mind: a choice, the posture it was made in, and why.

/// A lasting stance: what the fighter is trying to do for a while.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum Posture {
    /// At its range and height, probing.
    Neutral,
    /// Close in, blade after blade.
    Pressure,
    /// Keep the foe out at range.
    Zone,
    /// Land the hardest blow that fits the foe's opening.
    Punish,
    /// Something is about to land: block, step out, counter.
    Defend,
    /// Leave the danger, or the corner.
    Evade,
    /// Get the qi back.
    Recover,
    /// Go for the kill.
    Finish,
}

impl Posture {
    pub const ALL: [Posture; 8] = [
        Posture::Neutral,
        Posture::Pressure,
        Posture::Zone,
        Posture::Punish,
        Posture::Defend,
        Posture::Evade,
        Posture::Recover,
        Posture::Finish,
    ];
}

/// What the fighter does now.
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
pub enum Choice {
    /// Walk: `dir` -1 left, 1 right.
    Move {
        dir: f64,
    },
    Jump,
    Duck {
        hold: f64,
    },
    Guard {
        hold: f64,
    },
    /// Fly at `height` over the ground; 0 lands.
    Fly {
        height: f64,
    },
    Meditate,
    Wait,
    /// Use the skill of that name; `target` is an x for areas and blinks.
    Cast {
        skill: String,
        target: Option<f64>,
    },
}

/// What the game tells the mind about its skills, which the mind cannot see for itself.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum Outcome {
    /// The skill hurt the foe.
    Landed { skill: String },
    /// The foe hurt me while I recovered from the skill.
    Punished { skill: String },
}

/// Why the mind chose what it chose, for a debug view.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Why {
    /// Every posture's score this thought.
    pub postures: Vec<(Posture, f64)>,
    /// The best candidates and their scores, best first.
    pub top: Vec<(Choice, f64)>,
    /// The yomi level in force: 0 answers what the foe does, 1 what it will do, 2 its answer to me.
    pub level: u8,
    /// What it expects the foe to do next, if it has a read.
    pub predicted: Option<String>,
}

/// One thought.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Thought {
    pub choice: Choice,
    pub posture: Posture,
    pub why: Option<Why>,
}
