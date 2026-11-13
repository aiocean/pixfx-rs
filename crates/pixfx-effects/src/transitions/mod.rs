//! Screen transitions: ports of pixelfx `src/fx/transitions`. [`lib`] is the shared `_lib.js`.

pub mod lib;

mod burn_away;

pub use burn_away::BURN_AWAY;
