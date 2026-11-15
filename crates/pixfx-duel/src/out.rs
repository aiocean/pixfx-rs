//! What a step of the duel hands the page: effect commands (`Play`, `Move`, `Stop`) and events to feel and show,
//! with `duel.js`'s names and fields. Heights (`y`) are as `duel.js` gives them: up from the ground for a fighter's
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
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase"))]
pub enum Out {
    /// Start effect `fx` at (`x`, `y`) under `key`.
    Play { key: u32, fx: &'static str, x: f64, y: f64, opts: PlayOpts },
    /// Move the effect under `key`.
    Move { key: u32, x: f64, y: f64 },
    /// Stop the effect under `key`.
    Stop { key: u32 },
    /// Fighter `by`'s `spell` struck.
    Hit { by: usize, spell: &'static str, power: f64, launch: bool, ranged: bool, damage: f64, crit: bool, share: f64, x: f64, y: f64 },
    /// A burn, shock or field ticked on its target.
    Tick { by: usize, damage: f64, share: f64, x: f64, y: f64 },
    /// Fighter `by` turned a blow aside.
    Block { by: usize, x: f64, y: f64 },
    /// Two blows met, or a ward's charge took one.
    Clash { x: f64, y: f64 },
    Dodge { x: f64, y: f64 },
    Heal { by: usize, amount: f64, x: f64, y: f64 },
    Ward { by: usize, amount: f64, x: f64, y: f64 },
    Barrier { by: usize, x: f64, y: f64 },
    Ko { x: f64, y: f64 },
    /// Feet or back met the floor.
    Land { x: f64, strength: f64 },
    /// Thrown into the arena's edge.
    Bump { x: f64, y: f64, strength: f64 },
    /// A streak of qi from (`x0`, `y0`) to (`x1`, `y1`).
    Streak { by: usize, x0: f64, y0: f64, x1: f64, y1: f64 },
    Lucky { by: usize, kind: Luck, x: f64, y: f64 },
    /// A bar skill rose a tier.
    Tier { by: usize, slot: usize, rarity: Rarity },
    Ultimate { by: usize, x: f64, y: f64 },
    Unyielding { by: usize, x: f64, y: f64 },
}

impl From<pixfx_fighter::body::Event> for Out {
    fn from(e: pixfx_fighter::body::Event) -> Out {
        match e {
            pixfx_fighter::body::Event::Land { x, strength } => Out::Land { x, strength },
            pixfx_fighter::body::Event::Bump { x, y, strength } => Out::Bump { x, y, strength },
        }
    }
}
