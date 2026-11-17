//! pixfx effects: ports of pixelfx's effects (https://github.com/patch-gg/pixelfx), and of Sprite Fusion's pixel art
//! VFX (https://www.spritefusion.com/pixel-art-vfx) where pixelfx has no such effect, listed in [`EFFECTS`].
#![forbid(unsafe_code)]

use pixfx_core::effect::{Effect, find};

pub mod appear;
pub mod arcane;
pub mod critters;
pub mod disappear;
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
pub mod tone;
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
    #[cfg(feature = "impacts-hit")]
    &impacts::HIT,
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
    #[cfg(feature = "arcane-restriction")]
    &arcane::RESTRICTION,
    #[cfg(feature = "arcane-sword-formation")]
    &arcane::SWORD_FORMATION,
    #[cfg(any(feature = "arcane-flying-sword", feature = "arcane-sword-formation"))]
    &arcane::FLYING_SWORD,
    #[cfg(any(feature = "arcane-held-sword", feature = "arcane-sword-formation"))]
    &arcane::HELD_SWORD,
    #[cfg(any(feature = "arcane-sword-strike", feature = "arcane-sword-formation"))]
    &arcane::SWORD_STRIKE,
    #[cfg(feature = "arcane-sword-slash")]
    &arcane::SWORD_SLASH,
    #[cfg(feature = "arcane-bagua-mirror")]
    &arcane::BAGUA_MIRROR,
    #[cfg(feature = "arcane-bagua-mirror-strike")]
    &arcane::BAGUA_MIRROR_STRIKE,
    #[cfg(any(
        feature = "arcane-bagua-mirror-flying",
        feature = "arcane-bagua-mirror-strike"
    ))]
    &arcane::BAGUA_MIRROR_FLYING,
    #[cfg(any(
        feature = "arcane-bagua-mirror-burst",
        feature = "arcane-bagua-mirror-strike"
    ))]
    &arcane::BAGUA_MIRROR_BURST,
    #[cfg(feature = "critters-wisp")]
    &critters::WISP,
    #[cfg(feature = "feedback-heal-sparkle")]
    &feedback::HEAL_SPARKLE,
    #[cfg(feature = "feedback-outline")]
    &feedback::OUTLINE,
    #[cfg(feature = "feedback-power-up")]
    &feedback::POWER_UP,
    #[cfg(feature = "spawn-poof")]
    &spawn::POOF,
    #[cfg(feature = "holy-feather-burst")]
    &holy::FEATHER_BURST,
    #[cfg(feature = "holy-golden-bell")]
    &holy::GOLDEN_BELL,
    #[cfg(feature = "holy-lotus")]
    &holy::LOTUS,
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
    #[cfg(any(feature = "projectiles-trail", feature = "arcane-sword-formation"))]
    &projectiles::TRAIL,
    #[cfg(feature = "transitions-burn-away")]
    &transitions::BURN_AWAY,
    #[cfg(feature = "transitions-crt-off")]
    &transitions::CRT_OFF,
    #[cfg(feature = "shadow-blood-gush")]
    &shadow::BLOOD_GUSH,
    #[cfg(feature = "shadow-blood-shield")]
    &shadow::BLOOD_SHIELD,
    #[cfg(feature = "shadow-void-orb")]
    &shadow::VOID_ORB,
    #[cfg(feature = "shadow-winter-ordeal")]
    &shadow::WINTER_ORDEAL,
    #[cfg(feature = "water-splash")]
    &water::SPLASH,
    #[cfg(feature = "water-water-jet")]
    &water::WATER_JET,
    #[cfg(feature = "weather-snow")]
    &weather::SNOW,
    #[cfg(feature = "weather-fireflies")]
    &weather::FIREFLIES,
    #[cfg(feature = "arcane-blade-pinwheel")]
    &arcane::BLADE_PINWHEEL,
    #[cfg(feature = "impacts-star-converge")]
    &impacts::STAR_CONVERGE,
    #[cfg(feature = "impacts-twin-converge")]
    &impacts::TWIN_CONVERGE,
    #[cfg(feature = "projectiles-drop-dart")]
    &projectiles::DROP_DART,
    #[cfg(feature = "arcane-comet-rings")]
    &arcane::COMET_RINGS,
    #[cfg(feature = "arcane-comma-seal")]
    &arcane::COMMA_SEAL,
    #[cfg(feature = "arcane-shuriken-seal")]
    &arcane::SHURIKEN_SEAL,
    #[cfg(feature = "arcane-vortex-ring")]
    &arcane::VORTEX_RING,
    #[cfg(feature = "explosions-swirl-blast")]
    &explosions::SWIRL_BLAST,
    #[cfg(feature = "spawn-swirl-portal")]
    &spawn::SWIRL_PORTAL,
    #[cfg(feature = "arcane-twin-shuriken")]
    &arcane::TWIN_SHURIKEN,
    #[cfg(feature = "explosions-puff-burst")]
    &explosions::PUFF_BURST,
];

/// The effect with this id (`fire/candle`).
pub fn effect(id: &str) -> Option<&'static dyn Effect> {
    find(EFFECTS, id)
}
