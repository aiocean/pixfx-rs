//! Explosions: ports of pixelfx `src/fx/explosions`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "explosions-dirt-explosion")]
mod dirt_explosion;
#[cfg(feature = "explosions-implosion")]
mod implosion;
#[cfg(feature = "explosions-plasma-explosion")]
mod plasma_explosion;
#[cfg(feature = "explosions-small-explosion")]
mod small_explosion;

#[cfg(feature = "explosions-dirt-explosion")]
pub use dirt_explosion::DIRT_EXPLOSION;
#[cfg(feature = "explosions-implosion")]
pub use implosion::IMPLOSION;
#[cfg(feature = "explosions-plasma-explosion")]
pub use plasma_explosion::PLASMA_EXPLOSION;
#[cfg(feature = "explosions-small-explosion")]
pub use small_explosion::SMALL_EXPLOSION;
