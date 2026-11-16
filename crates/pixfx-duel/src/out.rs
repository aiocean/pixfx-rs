//! What a step of the duel hands the page: effect commands (`Play`, `Move`, `Stop`) and events to feel and show,
//! in the names and fields the page reads. Heights (`y`) are up from the ground for a fighter's
//! event, screen y for an effect.

use crate::data::{Luck, Rarity};

/// How an effect is played: a school's colours, its size, which way it faces, its seed. Left out when `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PlayOpts {
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub palette: Option<&'static [&'static str]>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub accent: Option<&'static [&'static str]>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub stars: Option<&'static [&'static str]>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub fire: Option<&'static [&'static str]>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub glow: Option<&'static str>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub width: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub height: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub flip: Option<bool>,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub seed: Option<u32>,
}

/// One command or event of a step, in the order the rules made it.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(
        tag = "type",
        rename_all = "camelCase",
        rename_all_fields = "camelCase"
    )
)]
pub enum Out {
    /// Start effect `fx` at (`x`, `y`) under `key`.
    Play {
        key: u32,
        fx: &'static str,
        x: f64,
        y: f64,
        opts: PlayOpts,
    },
    /// Move the effect under `key`.
    Move {
        key: u32,
        x: f64,
        y: f64,
    },
    /// Stop the effect under `key`.
    Stop {
        key: u32,
    },
    /// Fighter `by`'s `spell` struck.
    Hit {
        by: usize,
        spell: &'static str,
        power: f64,
        launch: bool,
        ranged: bool,
        damage: f64,
        crit: bool,
        share: f64,
        x: f64,
        y: f64,
    },
    /// A burn, shock or field ticked on its target.
    Tick {
        by: usize,
        damage: f64,
        share: f64,
        x: f64,
        y: f64,
    },
    /// Fighter `by` turned a blow aside.
    Block {
        by: usize,
        x: f64,
        y: f64,
    },
    /// Two blows met, or a ward's charge took one.
    Clash {
        x: f64,
        y: f64,
    },
    Dodge {
        x: f64,
        y: f64,
    },
    Heal {
        by: usize,
        amount: f64,
        x: f64,
        y: f64,
    },
    Ward {
        by: usize,
        amount: f64,
        x: f64,
        y: f64,
    },
    Barrier {
        by: usize,
        x: f64,
        y: f64,
    },
    Ko {
        x: f64,
        y: f64,
    },
    /// Feet or back met the floor.
    Land {
        x: f64,
        strength: f64,
    },
    /// Thrown into the arena's edge.
    Bump {
        x: f64,
        y: f64,
        strength: f64,
    },
    /// A streak of qi from (`x0`, `y0`) to (`x1`, `y1`).
    Streak {
        by: usize,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    },
    Lucky {
        by: usize,
        kind: Luck,
        x: f64,
        y: f64,
    },
    /// A bar skill rose a tier.
    Tier {
        by: usize,
        slot: usize,
        rarity: Rarity,
    },
    Ultimate {
        by: usize,
        x: f64,
        y: f64,
    },
    Unyielding {
        by: usize,
        x: f64,
        y: f64,
    },
}

/// What the rules note for whoever measures the duel (the ladder's telemetry), beside what the page reads: how a blow
/// that did not land ended, what a status took, what a heal gave, and a cast broken while it gathered. Shares are of
/// full life (0-100). The rules never read a note, and drawing one rolls no dice.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// The foe of fighter `by` stopped its blow of `spell`, `how`; a guard still lets `chip` through.
    Stopped {
        by: usize,
        spell: &'static str,
        how: Stop,
        chip: f64,
    },
    /// A burn, a shock or a field (`cause`: "burn", "shock" or the field's spell) of fighter `by`'s took `share` of
    /// its foe's life.
    Ticked {
        by: usize,
        cause: &'static str,
        share: f64,
    },
    /// Fighter `by` healed `share` of its life.
    Healed { by: usize, share: f64 },
    /// Fighter `who`'s cast of `spell` was broken while it gathered.
    Broken { who: usize, spell: &'static str },
}

/// How a blow was stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// Its guard held.
    Guard,
    /// Its barrier turned the blow aside (or sent a bolt back).
    Barrier,
    /// A ward's charge erased it.
    Ward,
    Dodge,
    /// Two blades met.
    Clash,
}

impl From<pixfx_fighter::body::Event> for Out {
    fn from(e: pixfx_fighter::body::Event) -> Out {
        match e {
            pixfx_fighter::body::Event::Land { x, strength } => Out::Land { x, strength },
            pixfx_fighter::body::Event::Bump { x, y, strength } => Out::Bump { x, y, strength },
        }
    }
}
