//! Projectiles: ports of pixelfx `src/fx/projectiles`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "projectiles-laser-bolt")]
mod laser_bolt;

#[cfg(feature = "projectiles-laser-bolt")]
pub use laser_bolt::LASER_BOLT;

#[cfg(any(feature = "projectiles-trail", feature = "arcane-sword-formation"))]
mod trail;

#[cfg(any(feature = "projectiles-trail", feature = "arcane-sword-formation"))]
pub use trail::TRAIL;

#[cfg(feature = "projectiles-drop-dart")]
mod drop_dart;

#[cfg(feature = "projectiles-drop-dart")]
pub use drop_dart::DROP_DART;
