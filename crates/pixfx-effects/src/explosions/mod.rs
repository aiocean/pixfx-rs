//! Explosions: ports of pixelfx `src/fx/explosions`. [`lib`] is the shared `_lib.js`.

pub mod lib;

mod dirt_explosion;
mod implosion;
mod plasma_explosion;
mod small_explosion;

pub use dirt_explosion::DIRT_EXPLOSION;
pub use implosion::IMPLOSION;
pub use plasma_explosion::PLASMA_EXPLOSION;
pub use small_explosion::SMALL_EXPLOSION;
