//! Weather: ports of pixelfx `src/fx/weather`. [`lib`] is the shared `_lib.js`.

pub mod lib;

mod fireflies;
mod snow;

pub use fireflies::FIREFLIES;
pub use snow::SNOW;
