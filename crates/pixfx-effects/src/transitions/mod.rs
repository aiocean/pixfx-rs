//! Screen transitions: ports of pixelfx `src/fx/transitions`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "transitions-burn-away")]
mod burn_away;

#[cfg(feature = "transitions-burn-away")]
pub use burn_away::BURN_AWAY;
