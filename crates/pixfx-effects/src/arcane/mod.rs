//! Arcane: ports of pixelfx `src/fx/arcane`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "arcane-hex-shield")]
mod hex_shield;

#[cfg(feature = "arcane-rune-circle")]
mod rune_circle;

#[cfg(feature = "arcane-rune-circle")]
pub use rune_circle::RUNE_CIRCLE;

#[cfg(feature = "arcane-hex-shield")]
pub use hex_shield::HEX_SHIELD;
