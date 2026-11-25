//! Feedback: effects that answer what the player did or picked, on the sprite they wrap: a buff (ported from Sprite
//! Fusion's pixel art VFX, https://www.spritefusion.com/pixel-art-vfx, whose category they keep) and an outline round a
//! sprite (made here).

#[cfg(feature = "feedback-amber-ward")]
mod amber_ward;
#[cfg(feature = "feedback-burning")]
mod burning;
#[cfg(feature = "feedback-cloak-shimmer")]
mod cloak_shimmer;
#[cfg(feature = "feedback-comet-weave")]
mod comet_weave;
#[cfg(feature = "feedback-electrified")]
mod electrified;
#[cfg(feature = "feedback-encase")]
mod encase;
#[cfg(feature = "feedback-frozen")]
mod frozen;
#[cfg(feature = "feedback-outline")]
mod outline;
#[cfg(feature = "feedback-petrified")]
mod petrified;
#[cfg(feature = "feedback-poisoned")]
mod poisoned;
#[cfg(feature = "feedback-power-up")]
mod power_up;
#[cfg(feature = "feedback-soaked")]
mod soaked;
#[cfg(any(
    feature = "feedback-burning",
    feature = "feedback-frozen",
    feature = "feedback-petrified",
    feature = "feedback-poisoned",
))]
mod status;
#[cfg(feature = "feedback-violet-portal")]
mod violet_portal;
#[cfg(feature = "feedback-volt-orbit")]
mod volt_orbit;

#[cfg(feature = "feedback-amber-ward")]
pub use amber_ward::AMBER_WARD;
#[cfg(feature = "feedback-burning")]
pub use burning::BURNING;
#[cfg(feature = "feedback-cloak-shimmer")]
pub use cloak_shimmer::CLOAK_SHIMMER;
#[cfg(feature = "feedback-comet-weave")]
pub use comet_weave::COMET_WEAVE;
#[cfg(feature = "feedback-electrified")]
pub use electrified::ELECTRIFIED;
#[cfg(feature = "feedback-encase")]
pub use encase::ENCASE;
#[cfg(feature = "feedback-frozen")]
pub use frozen::FROZEN;
#[cfg(feature = "feedback-outline")]
pub use outline::OUTLINE;
#[cfg(feature = "feedback-petrified")]
pub use petrified::PETRIFIED;
#[cfg(feature = "feedback-poisoned")]
pub use poisoned::POISONED;
#[cfg(feature = "feedback-power-up")]
pub use power_up::POWER_UP;
#[cfg(feature = "feedback-soaked")]
pub use soaked::SOAKED;
#[cfg(feature = "feedback-violet-portal")]
pub use violet_portal::VIOLET_PORTAL;
#[cfg(feature = "feedback-volt-orbit")]
pub use volt_orbit::VOLT_ORBIT;

#[cfg(feature = "feedback-afterimage")]
mod afterimage;
#[cfg(feature = "feedback-afterimage")]
pub use afterimage::AFTERIMAGE;

#[cfg(feature = "feedback-arc-link")]
mod arc_link;
#[cfg(feature = "feedback-arc-link")]
pub use arc_link::ARC_LINK;

#[cfg(feature = "feedback-elemental-charge")]
mod elemental_charge;
#[cfg(feature = "feedback-elemental-charge")]
pub use elemental_charge::ELEMENTAL_CHARGE;

#[cfg(feature = "feedback-foot-contact")]
mod foot_contact;
#[cfg(feature = "feedback-foot-contact")]
pub use foot_contact::FOOT_CONTACT;

#[cfg(feature = "feedback-hit-flash")]
mod hit_flash;
#[cfg(feature = "feedback-hit-flash")]
pub use hit_flash::HIT_FLASH;

#[cfg(feature = "feedback-mask-ignition")]
mod mask_ignition;
#[cfg(feature = "feedback-mask-ignition")]
pub use mask_ignition::MASK_IGNITION;

#[cfg(feature = "feedback-occlusion")]
mod occlusion;
#[cfg(feature = "feedback-occlusion")]
pub use occlusion::OCCLUSION;

#[cfg(feature = "feedback-ricochet")]
mod ricochet;
#[cfg(feature = "feedback-ricochet")]
pub use ricochet::RICOCHET;

#[cfg(feature = "feedback-rim-light")]
mod rim_light;
#[cfg(feature = "feedback-rim-light")]
pub use rim_light::RIM_LIGHT;

#[cfg(feature = "feedback-shield-absorb")]
mod shield_absorb;
#[cfg(feature = "feedback-shield-absorb")]
pub use shield_absorb::SHIELD_ABSORB;

#[cfg(feature = "feedback-shield-crack")]
mod shield_crack;
#[cfg(feature = "feedback-shield-crack")]
pub use shield_crack::SHIELD_CRACK;

#[cfg(feature = "feedback-sprite-shatter")]
mod sprite_shatter;
#[cfg(feature = "feedback-sprite-shatter")]
pub use sprite_shatter::SPRITE_SHATTER;

#[cfg(feature = "feedback-telegraph")]
mod telegraph;
#[cfg(feature = "feedback-telegraph")]
pub use telegraph::TELEGRAPH;

#[cfg(any(feature = "feedback-shield-crack", feature = "feedback-sprite-shatter",))]
mod lib;
