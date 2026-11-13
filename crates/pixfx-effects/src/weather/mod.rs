//! Weather: ports of pixelfx `src/fx/weather`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "weather-fireflies")]
mod fireflies;
#[cfg(feature = "weather-snow")]
mod snow;

#[cfg(feature = "weather-fireflies")]
pub use fireflies::FIREFLIES;
#[cfg(feature = "weather-snow")]
pub use snow::SNOW;
