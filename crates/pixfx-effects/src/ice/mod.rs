//! Ice: ports of pixelfx `src/fx/ice`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "ice-ice-crystal")]
mod ice_crystal;
#[cfg(feature = "ice-ice-crystal")]
pub use ice_crystal::ICE_CRYSTAL;
#[cfg(feature = "ice-ice-lance")]
mod ice_lance;
#[cfg(feature = "ice-ice-lance")]
pub use ice_lance::ICE_LANCE;
#[cfg(feature = "ice-frost-breath")]
mod frost_breath;
#[cfg(feature = "ice-frost-breath")]
pub use frost_breath::FROST_BREATH;
#[cfg(feature = "ice-blizzard-vortex")]
mod blizzard_vortex;
#[cfg(feature = "ice-blizzard-vortex")]
pub use blizzard_vortex::BLIZZARD_VORTEX;
#[cfg(feature = "ice-snowflake-burst")]
mod snowflake_burst;
#[cfg(feature = "ice-snowflake-burst")]
pub use snowflake_burst::SNOWFLAKE_BURST;
#[cfg(feature = "ice-frost-nova")]
mod frost_nova;
#[cfg(feature = "ice-frost-nova")]
pub use frost_nova::FROST_NOVA;
#[cfg(feature = "ice-icicle-fall")]
mod icicle_fall;
#[cfg(feature = "ice-icicle-fall")]
pub use icicle_fall::ICICLE_FALL;
#[cfg(feature = "ice-ice-spikes")]
mod ice_spikes;
#[cfg(feature = "ice-ice-spikes")]
pub use ice_spikes::ICE_SPIKES;
#[cfg(feature = "ice-ice-shatter")]
mod ice_shatter;
#[cfg(feature = "ice-ice-shatter")]
pub use ice_shatter::ICE_SHATTER;
#[cfg(feature = "ice-freeze")]
mod freeze;
#[cfg(feature = "ice-freeze")]
pub use freeze::FREEZE;
