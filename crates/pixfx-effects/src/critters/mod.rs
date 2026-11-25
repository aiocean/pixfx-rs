//! Critters: ports of pixelfx `src/fx/critters`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "critters-wisp")]
mod wisp;

#[cfg(feature = "critters-wisp")]
pub use wisp::WISP;
