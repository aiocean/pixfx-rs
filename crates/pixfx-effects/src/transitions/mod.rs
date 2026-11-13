//! Screen transitions: ports of pixelfx `src/fx/transitions`. [`lib`] is the shared `_lib.js`.

pub mod lib;

mod burn_away;
mod crt_off;

pub use burn_away::BURN_AWAY;
pub use crt_off::CRT_OFF;
