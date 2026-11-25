//! Water: ports of pixelfx `src/fx/water`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "water-bubbles")]
mod bubbles;
#[cfg(feature = "water-bubbles")]
pub use bubbles::BUBBLES;

#[cfg(feature = "water-drip")]
mod drip;
#[cfg(feature = "water-drip")]
pub use drip::DRIP;

#[cfg(feature = "water-fountain")]
mod fountain;
#[cfg(feature = "water-fountain")]
pub use fountain::FOUNTAIN;

#[cfg(feature = "water-geyser")]
mod geyser;
#[cfg(feature = "water-geyser")]
pub use geyser::GEYSER;

#[cfg(feature = "water-rain-splash")]
mod rain_splash;
#[cfg(feature = "water-rain-splash")]
pub use rain_splash::RAIN_SPLASH;

#[cfg(feature = "water-ripples")]
mod ripples;
#[cfg(feature = "water-ripples")]
pub use ripples::RIPPLES;

#[cfg(feature = "water-splash")]
mod splash;
#[cfg(feature = "water-splash")]
pub use splash::SPLASH;

#[cfg(feature = "water-water-jet")]
mod water_jet;
#[cfg(feature = "water-water-jet")]
pub use water_jet::WATER_JET;

#[cfg(feature = "water-water-orb")]
mod water_orb;
#[cfg(feature = "water-water-orb")]
pub use water_orb::WATER_ORB;

#[cfg(feature = "water-waterfall")]
mod waterfall;
#[cfg(feature = "water-waterfall")]
pub use waterfall::WATERFALL;

#[cfg(feature = "water-wave-crash")]
mod wave_crash;
#[cfg(feature = "water-wave-crash")]
pub use wave_crash::WAVE_CRASH;

#[cfg(feature = "water-whirlpool")]
mod whirlpool;
#[cfg(feature = "water-whirlpool")]
pub use whirlpool::WHIRLPOOL;
