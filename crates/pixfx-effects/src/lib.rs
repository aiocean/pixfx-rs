//! Procedural, sprite-based and hybrid pixel effects, listed in [`EFFECTS`].
#![forbid(unsafe_code)]

use pixfx_core::effect::{Effect, find};

pub mod appear;
pub mod critters;
pub mod disappear;
#[cfg(any(feature = "appear-fade", feature = "disappear-fade"))]
mod dissolve;
#[cfg(any(feature = "appear-dither", feature = "disappear-dither"))]
mod dither;
pub mod explosions;
pub mod feedback;
pub mod fire;
pub mod ice;
pub mod impacts;
pub mod lightning;
#[cfg(any(
    feature = "feedback-mask-ignition",
    feature = "impacts-contact-pulse",
    feature = "impacts-aftershock",
))]
mod mask_heat;
pub mod melee;
pub mod projectiles;
#[cfg(any(
    feature = "slashes-slash-1",
    feature = "slashes-slash-2",
    feature = "slashes-slash-3",
))]
pub mod slashes;
pub mod spawn;
pub mod transitions;
pub mod water;

/// Every effect in this crate.
pub static EFFECTS: &[&'static dyn Effect] = &[
    #[cfg(feature = "appear-dither")]
    &appear::DITHER,
    #[cfg(feature = "appear-fade")]
    &appear::FADE,
    #[cfg(feature = "appear-teleport-beam")]
    &appear::TELEPORT_BEAM,
    #[cfg(feature = "critters-wisp")]
    &critters::WISP,
    #[cfg(feature = "disappear-blink-out")]
    &disappear::BLINK_OUT,
    #[cfg(feature = "disappear-dither")]
    &disappear::DITHER,
    #[cfg(feature = "disappear-fade")]
    &disappear::FADE,
    #[cfg(feature = "disappear-incinerate")]
    &disappear::INCINERATE,
    #[cfg(feature = "explosions-body-burst")]
    &explosions::BODY_BURST,
    #[cfg(feature = "feedback-amber-ward")]
    &feedback::AMBER_WARD,
    #[cfg(feature = "feedback-burning")]
    &feedback::BURNING,
    #[cfg(feature = "feedback-cloak-shimmer")]
    &feedback::CLOAK_SHIMMER,
    #[cfg(feature = "feedback-comet-weave")]
    &feedback::COMET_WEAVE,
    #[cfg(feature = "feedback-electrified")]
    &feedback::ELECTRIFIED,
    #[cfg(feature = "feedback-encase")]
    &feedback::ENCASE,
    #[cfg(feature = "feedback-frozen")]
    &feedback::FROZEN,
    #[cfg(feature = "feedback-outline")]
    &feedback::OUTLINE,
    #[cfg(feature = "feedback-petrified")]
    &feedback::PETRIFIED,
    #[cfg(feature = "feedback-poisoned")]
    &feedback::POISONED,
    #[cfg(feature = "feedback-power-up")]
    &feedback::POWER_UP,
    #[cfg(feature = "feedback-soaked")]
    &feedback::SOAKED,
    #[cfg(feature = "feedback-violet-portal")]
    &feedback::VIOLET_PORTAL,
    #[cfg(feature = "feedback-volt-orbit")]
    &feedback::VOLT_ORBIT,
    #[cfg(feature = "fire-brazier")]
    &fire::BRAZIER,
    #[cfg(feature = "fire-campfire")]
    &fire::CAMPFIRE,
    #[cfg(feature = "fire-candle")]
    &fire::CANDLE,
    #[cfg(feature = "fire-fire-breath")]
    &fire::FIRE_BREATH,
    #[cfg(feature = "fire-fire-ring")]
    &fire::FIRE_RING,
    #[cfg(feature = "fire-fire-tornado")]
    &fire::FIRE_TORNADO,
    #[cfg(feature = "fire-fireball")]
    &fire::FIREBALL,
    #[cfg(feature = "fire-flame-pillar")]
    &fire::FLAME_PILLAR,
    #[cfg(feature = "fire-flamethrower")]
    &fire::FLAMETHROWER,
    #[cfg(feature = "fire-meteor")]
    &fire::METEOR,
    #[cfg(feature = "fire-spirit-flame")]
    &fire::SPIRIT_FLAME,
    #[cfg(feature = "fire-torch")]
    &fire::TORCH,
    #[cfg(feature = "feedback-afterimage")]
    &feedback::AFTERIMAGE,
    #[cfg(feature = "feedback-arc-link")]
    &feedback::ARC_LINK,
    #[cfg(feature = "feedback-elemental-charge")]
    &feedback::ELEMENTAL_CHARGE,
    #[cfg(feature = "feedback-foot-contact")]
    &feedback::FOOT_CONTACT,
    #[cfg(feature = "feedback-hit-flash")]
    &feedback::HIT_FLASH,
    #[cfg(feature = "feedback-mask-ignition")]
    &feedback::MASK_IGNITION,
    #[cfg(feature = "feedback-occlusion")]
    &feedback::OCCLUSION,
    #[cfg(feature = "feedback-ricochet")]
    &feedback::RICOCHET,
    #[cfg(feature = "feedback-rim-light")]
    &feedback::RIM_LIGHT,
    #[cfg(feature = "feedback-shield-absorb")]
    &feedback::SHIELD_ABSORB,
    #[cfg(feature = "feedback-shield-crack")]
    &feedback::SHIELD_CRACK,
    #[cfg(feature = "feedback-sprite-shatter")]
    &feedback::SPRITE_SHATTER,
    #[cfg(feature = "feedback-telegraph")]
    &feedback::TELEGRAPH,
    #[cfg(feature = "ice-ice-crystal")]
    &ice::ICE_CRYSTAL,
    #[cfg(feature = "ice-ice-lance")]
    &ice::ICE_LANCE,
    #[cfg(feature = "ice-frost-breath")]
    &ice::FROST_BREATH,
    #[cfg(feature = "ice-blizzard-vortex")]
    &ice::BLIZZARD_VORTEX,
    #[cfg(feature = "ice-snowflake-burst")]
    &ice::SNOWFLAKE_BURST,
    #[cfg(feature = "ice-frost-nova")]
    &ice::FROST_NOVA,
    #[cfg(feature = "ice-icicle-fall")]
    &ice::ICICLE_FALL,
    #[cfg(feature = "ice-ice-spikes")]
    &ice::ICE_SPIKES,
    #[cfg(feature = "ice-ice-shatter")]
    &ice::ICE_SHATTER,
    #[cfg(feature = "ice-freeze")]
    &ice::FREEZE,
    #[cfg(feature = "impacts-parry")]
    &impacts::PARRY,
    #[cfg(feature = "impacts-rune-flash")]
    &impacts::RUNE_FLASH,
    #[cfg(feature = "impacts-thunderstrike")]
    &impacts::THUNDERSTRIKE,
    #[cfg(feature = "impacts-aftershock")]
    &impacts::AFTERSHOCK,
    #[cfg(feature = "impacts-charged-pulse")]
    &impacts::CHARGED_PULSE,
    #[cfg(feature = "impacts-contact-pulse")]
    &impacts::CONTACT_PULSE,
    #[cfg(feature = "impacts-directional-wave")]
    &impacts::DIRECTIONAL_WAVE,
    #[cfg(feature = "impacts-resonance")]
    &impacts::RESONANCE,
    #[cfg(feature = "lightning-ball-lightning")]
    &lightning::BALL_LIGHTNING,
    #[cfg(feature = "lightning-chain-lightning")]
    &lightning::CHAIN_LIGHTNING,
    #[cfg(feature = "lightning-charge-up")]
    &lightning::CHARGE_UP,
    #[cfg(feature = "lightning-electrified")]
    &lightning::ELECTRIFIED,
    #[cfg(feature = "lightning-lightning-strike")]
    &lightning::LIGHTNING_STRIKE,
    #[cfg(feature = "lightning-plasma-ball")]
    &lightning::PLASMA_BALL,
    #[cfg(feature = "lightning-spark-arc")]
    &lightning::SPARK_ARC,
    #[cfg(feature = "lightning-storm-cloud")]
    &lightning::STORM_CLOUD,
    #[cfg(feature = "lightning-tesla-coil")]
    &lightning::TESLA_COIL,
    #[cfg(feature = "lightning-thunderclap")]
    &lightning::THUNDERCLAP,
    #[cfg(feature = "lightning-zap-hit")]
    &lightning::ZAP_HIT,
    #[cfg(feature = "melee-sword-slash")]
    &melee::SWORD_SLASH,
    #[cfg(feature = "projectiles-arcane-meteor")]
    &projectiles::ARCANE_METEOR,
    #[cfg(feature = "projectiles-trail")]
    &projectiles::TRAIL,
    #[cfg(feature = "projectiles-violet-wake")]
    &projectiles::VIOLET_WAKE,
    #[cfg(feature = "projectiles-aether-pierce")]
    &projectiles::AETHER_PIERCE,
    #[cfg(feature = "projectiles-corkscrew")]
    &projectiles::CORKSCREW,
    #[cfg(feature = "projectiles-moon-windblade")]
    &projectiles::MOON_WINDBLADE,
    #[cfg(feature = "projectiles-pulse-rings")]
    &projectiles::PULSE_RINGS,
    #[cfg(feature = "projectiles-twin-sparks")]
    &projectiles::TWIN_SPARKS,
    #[cfg(feature = "slashes-slash-1")]
    &slashes::SLASH_1,
    #[cfg(feature = "slashes-slash-2")]
    &slashes::SLASH_2,
    #[cfg(feature = "slashes-slash-3")]
    &slashes::SLASH_3,
    #[cfg(feature = "spawn-poof")]
    &spawn::POOF,
    #[cfg(feature = "transitions-burn-away")]
    &transitions::BURN_AWAY,
    #[cfg(feature = "water-bubbles")]
    &water::BUBBLES,
    #[cfg(feature = "water-drip")]
    &water::DRIP,
    #[cfg(feature = "water-fountain")]
    &water::FOUNTAIN,
    #[cfg(feature = "water-geyser")]
    &water::GEYSER,
    #[cfg(feature = "water-rain-splash")]
    &water::RAIN_SPLASH,
    #[cfg(feature = "water-ripples")]
    &water::RIPPLES,
    #[cfg(feature = "water-splash")]
    &water::SPLASH,
    #[cfg(feature = "water-water-jet")]
    &water::WATER_JET,
    #[cfg(feature = "water-water-orb")]
    &water::WATER_ORB,
    #[cfg(feature = "water-waterfall")]
    &water::WATERFALL,
    #[cfg(feature = "water-wave-crash")]
    &water::WAVE_CRASH,
    #[cfg(feature = "water-whirlpool")]
    &water::WHIRLPOOL,
];

/// The effect with this id (`feedback/outline`).
pub fn effect(id: &str) -> Option<&'static dyn Effect> {
    find(EFFECTS, id)
}
