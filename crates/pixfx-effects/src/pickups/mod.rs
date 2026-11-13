//! Pickups: ports of pixelfx `src/fx/pickups`. [`lib`] is the shared `_lib.js`.

pub mod lib;

mod coin_spin;

pub use coin_spin::COIN_SPIN;
