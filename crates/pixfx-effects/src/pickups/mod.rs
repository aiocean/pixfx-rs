//! Pickups: ports of pixelfx `src/fx/pickups`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "pickups-coin-spin")]
mod coin_spin;

#[cfg(feature = "pickups-coin-spin")]
pub use coin_spin::COIN_SPIN;
