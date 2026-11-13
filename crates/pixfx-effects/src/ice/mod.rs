//! Ice: ports of pixelfx `src/fx/ice`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "ice-icicle-fall")]
mod icicle_fall;

#[cfg(feature = "ice-icicle-fall")]
pub use icicle_fall::ICICLE_FALL;
