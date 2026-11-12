//! pixfx effects: ports of pixelfx's effects (https://github.com/patch-gg/pixelfx), listed in [`EFFECTS`].
#![forbid(unsafe_code)]

use pixfx_core::effect::{Effect, find};

pub mod fire;

/// Every effect in this crate.
pub static EFFECTS: &[&'static dyn Effect] = &[&fire::CANDLE, &fire::CAMPFIRE, &fire::BRAZIER];

/// The effect with this id (`fire/candle`).
pub fn effect(id: &str) -> Option<&'static dyn Effect> {
    find(EFFECTS, id)
}
