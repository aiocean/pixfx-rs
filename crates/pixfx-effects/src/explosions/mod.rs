//! Explosions: `body-burst`, made here, which blows apart the sprite it wraps.

#[cfg(feature = "explosions-body-burst")]
mod body_burst;

#[cfg(feature = "explosions-body-burst")]
pub use body_burst::BODY_BURST;
