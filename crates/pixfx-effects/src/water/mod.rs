//! Water: ports of pixelfx `src/fx/water`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "water-splash")]
mod splash;
#[cfg(feature = "water-water-jet")]
mod water_jet;

#[cfg(feature = "water-splash")]
pub use splash::SPLASH;
#[cfg(feature = "water-water-jet")]
pub use water_jet::WATER_JET;
