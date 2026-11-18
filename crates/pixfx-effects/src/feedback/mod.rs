//! Feedback: effects that answer what the player did or picked, on the sprite they wrap: a buff (ported from Sprite
//! Fusion's pixel art VFX, https://www.spritefusion.com/pixel-art-vfx, whose category they keep) and an outline round a
//! sprite (made here).

#[cfg(feature = "feedback-cloak-shimmer")]
mod cloak_shimmer;
#[cfg(feature = "feedback-electrified")]
mod electrified;
#[cfg(feature = "feedback-encase")]
mod encase;
#[cfg(feature = "feedback-outline")]
mod outline;
#[cfg(feature = "feedback-power-up")]
mod power_up;
#[cfg(feature = "feedback-soaked")]
mod soaked;

#[cfg(feature = "feedback-cloak-shimmer")]
pub use cloak_shimmer::CLOAK_SHIMMER;
#[cfg(feature = "feedback-electrified")]
pub use electrified::ELECTRIFIED;
#[cfg(feature = "feedback-encase")]
pub use encase::ENCASE;
#[cfg(feature = "feedback-outline")]
pub use outline::OUTLINE;
#[cfg(feature = "feedback-power-up")]
pub use power_up::POWER_UP;
#[cfg(feature = "feedback-soaked")]
pub use soaked::SOAKED;
