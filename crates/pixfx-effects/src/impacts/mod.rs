//! Impacts: ports of pixelfx `src/fx/impacts`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "impacts-parry")]
mod parry;

#[cfg(feature = "impacts-parry")]
pub use parry::PARRY;
