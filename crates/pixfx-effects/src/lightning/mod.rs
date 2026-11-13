//! Lightning: ports of pixelfx `src/fx/lightning`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "lightning-ball-lightning")]
mod ball_lightning;

#[cfg(feature = "lightning-ball-lightning")]
pub use ball_lightning::BALL_LIGHTNING;
