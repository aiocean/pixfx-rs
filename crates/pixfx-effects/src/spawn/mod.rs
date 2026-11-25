//! Spawn: ports of pixelfx `src/fx/spawn`.

#[cfg(feature = "spawn-poof")]
mod poof;

#[cfg(feature = "spawn-poof")]
pub use poof::POOF;
