//! pixfx effects: ports of pixelfx's effects (https://github.com/patch-gg/pixelfx), listed in [`EFFECTS`].
#![forbid(unsafe_code)]

use pixfx_core::effect::{Effect, find};

pub mod arcane;
pub mod critters;
pub mod explosions;
pub mod fire;
pub mod holy;
pub mod impacts;
pub mod spawn;

/// Every effect in this crate.
pub static EFFECTS: &[&'static dyn Effect] = &[
    &fire::CANDLE,
    &fire::CAMPFIRE,
    &fire::BRAZIER,
    &fire::SPIRIT_FLAME,
    &impacts::PARRY,
    &explosions::DIRT_EXPLOSION,
    &explosions::SMALL_EXPLOSION,
    &explosions::IMPLOSION,
    &explosions::PLASMA_EXPLOSION,
    &arcane::RUNE_CIRCLE,
    &critters::WISP,
    &spawn::POOF,
    &holy::FEATHER_BURST,
];

/// The effect with this id (`fire/candle`).
pub fn effect(id: &str) -> Option<&'static dyn Effect> {
    find(EFFECTS, id)
}
