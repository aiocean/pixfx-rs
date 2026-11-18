//! pixfx effects: the effects that need the sprite they play on or the way it moved, listed in [`EFFECTS`]. An effect
//! that reads neither (a fire, an explosion, a shield) is a sprite the game plays, not an effect here.
#![forbid(unsafe_code)]

use pixfx_core::effect::{Effect, find};

pub mod appear;
pub mod disappear;
pub mod explosions;
pub mod feedback;
pub mod projectiles;

/// Every effect in this crate.
pub static EFFECTS: &[&'static dyn Effect] = &[
    #[cfg(feature = "appear-teleport-beam")]
    &appear::TELEPORT_BEAM,
    #[cfg(feature = "disappear-blink-out")]
    &disappear::BLINK_OUT,
    #[cfg(feature = "disappear-fade")]
    &disappear::FADE,
    #[cfg(feature = "disappear-incinerate")]
    &disappear::INCINERATE,
    #[cfg(feature = "explosions-body-burst")]
    &explosions::BODY_BURST,
    #[cfg(feature = "feedback-cloak-shimmer")]
    &feedback::CLOAK_SHIMMER,
    #[cfg(feature = "feedback-electrified")]
    &feedback::ELECTRIFIED,
    #[cfg(feature = "feedback-encase")]
    &feedback::ENCASE,
    #[cfg(feature = "feedback-outline")]
    &feedback::OUTLINE,
    #[cfg(feature = "feedback-power-up")]
    &feedback::POWER_UP,
    #[cfg(feature = "feedback-soaked")]
    &feedback::SOAKED,
    #[cfg(feature = "projectiles-trail")]
    &projectiles::TRAIL,
];

/// The effect with this id (`feedback/outline`).
pub fn effect(id: &str) -> Option<&'static dyn Effect> {
    find(EFFECTS, id)
}
