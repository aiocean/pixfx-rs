//! Scifi: ports of pixelfx `src/fx/scifi`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "scifi-jetpack")]
mod jetpack;

#[cfg(feature = "scifi-jetpack")]
pub use jetpack::JETPACK;
