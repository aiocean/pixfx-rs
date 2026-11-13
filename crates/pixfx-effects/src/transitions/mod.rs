//! Screen transitions: ports of pixelfx `src/fx/transitions`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "transitions-burn-away")]
mod burn_away;
#[cfg(feature = "transitions-crt-off")]
mod crt_off;

#[cfg(feature = "transitions-burn-away")]
pub use burn_away::BURN_AWAY;
#[cfg(feature = "transitions-crt-off")]
pub use crt_off::CRT_OFF;
