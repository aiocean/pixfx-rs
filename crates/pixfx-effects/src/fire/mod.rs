//! Fire: ports of pixelfx `src/fx/fire`. [`lib`] is the shared `_lib.js`.

pub mod lib;

mod brazier;
mod campfire;
mod candle;

pub use brazier::BRAZIER;
pub use campfire::CAMPFIRE;
pub use candle::CANDLE;
