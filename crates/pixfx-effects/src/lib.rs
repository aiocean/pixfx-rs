//! pixfx effects: ports of pixelfx's effects (https://github.com/patch-gg/pixelfx), and of Sprite Fusion's pixel art
//! VFX (https://www.spritefusion.com/pixel-art-vfx) where pixelfx has no such effect, listed in [`EFFECTS`].
#![forbid(unsafe_code)]

use pixfx_core::effect::{Effect, find};

pub mod arcane;
pub mod critters;
pub mod explosions;
pub mod feedback;
pub mod fire;
pub mod holy;
pub mod ice;
pub mod impacts;
pub mod lightning;
pub mod nature;
pub mod pickups;
pub mod projectiles;
pub mod scifi;
pub mod shadow;
pub mod spawn;
pub mod transitions;
pub mod water;
pub mod weather;

/// Every effect in this crate.
pub static EFFECTS: &[&'static dyn Effect] = &[
    #[cfg(feature = "fire-candle")]
    &fire::CANDLE,
    #[cfg(feature = "fire-campfire")]
    &fire::CAMPFIRE,
    #[cfg(feature = "fire-brazier")]
    &fire::BRAZIER,
    #[cfg(feature = "fire-spirit-flame")]
    &fire::SPIRIT_FLAME,
    #[cfg(feature = "impacts-parry")]
    &impacts::PARRY,
    #[cfg(feature = "explosions-dirt-explosion")]
    &explosions::DIRT_EXPLOSION,
    #[cfg(feature = "explosions-small-explosion")]
    &explosions::SMALL_EXPLOSION,
    #[cfg(feature = "explosions-implosion")]
    &explosions::IMPLOSION,
    #[cfg(feature = "explosions-plasma-explosion")]
    &explosions::PLASMA_EXPLOSION,
    #[cfg(feature = "explosions-body-burst")]
    &explosions::BODY_BURST,
    #[cfg(feature = "arcane-rune-circle")]
    &arcane::RUNE_CIRCLE,
    #[cfg(feature = "arcane-hex-shield")]
    &arcane::HEX_SHIELD,
    #[cfg(feature = "critters-wisp")]
    &critters::WISP,
    #[cfg(feature = "feedback-outline")]
    &feedback::OUTLINE,
    #[cfg(feature = "feedback-power-up")]
    &feedback::POWER_UP,
    #[cfg(feature = "spawn-poof")]
    &spawn::POOF,
    #[cfg(feature = "holy-feather-burst")]
    &holy::FEATHER_BURST,
    #[cfg(feature = "ice-icicle-fall")]
    &ice::ICICLE_FALL,
    #[cfg(feature = "scifi-jetpack")]
    &scifi::JETPACK,
    #[cfg(feature = "lightning-ball-lightning")]
    &lightning::BALL_LIGHTNING,
    #[cfg(feature = "nature-poison-cloud")]
    &nature::POISON_CLOUD,
    #[cfg(feature = "pickups-coin-spin")]
    &pickups::COIN_SPIN,
    #[cfg(feature = "projectiles-laser-bolt")]
    &projectiles::LASER_BOLT,
    #[cfg(feature = "transitions-burn-away")]
    &transitions::BURN_AWAY,
    #[cfg(feature = "transitions-crt-off")]
    &transitions::CRT_OFF,
    #[cfg(feature = "shadow-void-orb")]
    &shadow::VOID_ORB,
    #[cfg(feature = "water-water-jet")]
    &water::WATER_JET,
    #[cfg(feature = "weather-snow")]
    &weather::SNOW,
    #[cfg(feature = "weather-fireflies")]
    &weather::FIREFLIES,
];

/// The effect with this id (`fire/candle`).
pub fn effect(id: &str) -> Option<&'static dyn Effect> {
    find(EFFECTS, id)
}
