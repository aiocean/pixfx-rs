//! Spawn: ports of pixelfx `src/fx/spawn`.

#[cfg(feature = "spawn-poof")]
mod poof;

#[cfg(feature = "spawn-poof")]
pub use poof::POOF;

#[cfg(feature = "spawn-swirl-portal")]
mod swirl_portal;

#[cfg(feature = "spawn-swirl-portal")]
pub use swirl_portal::SWIRL_PORTAL;
