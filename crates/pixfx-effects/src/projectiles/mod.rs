//! Projectiles: `trail`, the streak a mover leaves along the way it came.

#[cfg(feature = "projectiles-arcane-meteor")]
mod arcane_meteor;

#[cfg(feature = "projectiles-arcane-meteor")]
pub use arcane_meteor::ARCANE_METEOR;

#[cfg(feature = "projectiles-trail")]
mod trail;

#[cfg(feature = "projectiles-trail")]
pub use trail::TRAIL;

#[cfg(feature = "projectiles-violet-wake")]
mod violet_wake;

#[cfg(feature = "projectiles-violet-wake")]
pub use violet_wake::VIOLET_WAKE;

#[cfg(feature = "projectiles-aether-pierce")]
mod aether_pierce;
#[cfg(feature = "projectiles-aether-pierce")]
pub use aether_pierce::AETHER_PIERCE;

#[cfg(feature = "projectiles-corkscrew")]
mod corkscrew;
#[cfg(feature = "projectiles-corkscrew")]
pub use corkscrew::CORKSCREW;

#[cfg(feature = "projectiles-moon-windblade")]
mod moon_windblade;
#[cfg(feature = "projectiles-moon-windblade")]
pub use moon_windblade::MOON_WINDBLADE;

#[cfg(feature = "projectiles-pulse-rings")]
mod pulse_rings;
#[cfg(feature = "projectiles-pulse-rings")]
pub use pulse_rings::PULSE_RINGS;

#[cfg(feature = "projectiles-twin-sparks")]
mod twin_sparks;
#[cfg(feature = "projectiles-twin-sparks")]
pub use twin_sparks::TWIN_SPARKS;

#[cfg(any(
    feature = "projectiles-aether-pierce",
    feature = "projectiles-corkscrew",
    feature = "projectiles-moon-windblade",
    feature = "projectiles-twin-sparks",
))]
mod flight;

#[cfg(any(
    feature = "projectiles-aether-pierce",
    feature = "projectiles-corkscrew",
    feature = "projectiles-twin-sparks",
))]
mod ribbon;
