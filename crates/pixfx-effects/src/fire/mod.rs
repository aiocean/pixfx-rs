//! Fire: ports of pixelfx `src/fx/fire`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "fire-brazier")]
mod brazier;
#[cfg(feature = "fire-campfire")]
mod campfire;
#[cfg(feature = "fire-candle")]
mod candle;
#[cfg(feature = "fire-spirit-flame")]
mod spirit_flame;

#[cfg(feature = "fire-brazier")]
pub use brazier::BRAZIER;
#[cfg(feature = "fire-campfire")]
pub use campfire::CAMPFIRE;
#[cfg(feature = "fire-candle")]
pub use candle::CANDLE;
#[cfg(feature = "fire-spirit-flame")]
pub use spirit_flame::SPIRIT_FLAME;
