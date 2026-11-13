//! Holy: ports of pixelfx `src/fx/holy`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "holy-feather-burst")]
mod feather_burst;

#[cfg(feature = "holy-feather-burst")]
pub use feather_burst::FEATHER_BURST;
