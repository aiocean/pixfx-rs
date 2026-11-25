//! Fire: ports of pixelfx `src/fx/fire`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "fire-brazier")]
mod brazier;
#[cfg(feature = "fire-brazier")]
pub use brazier::BRAZIER;

#[cfg(feature = "fire-campfire")]
mod campfire;
#[cfg(feature = "fire-campfire")]
pub use campfire::CAMPFIRE;

#[cfg(feature = "fire-candle")]
mod candle;
#[cfg(feature = "fire-candle")]
pub use candle::CANDLE;

#[cfg(feature = "fire-fire-breath")]
mod fire_breath;
#[cfg(feature = "fire-fire-breath")]
pub use fire_breath::FIRE_BREATH;

#[cfg(feature = "fire-fire-ring")]
mod fire_ring;
#[cfg(feature = "fire-fire-ring")]
pub use fire_ring::FIRE_RING;

#[cfg(feature = "fire-fire-tornado")]
mod fire_tornado;
#[cfg(feature = "fire-fire-tornado")]
pub use fire_tornado::FIRE_TORNADO;

#[cfg(feature = "fire-fireball")]
mod fireball;
#[cfg(feature = "fire-fireball")]
pub use fireball::FIREBALL;

#[cfg(feature = "fire-flame-pillar")]
mod flame_pillar;
#[cfg(feature = "fire-flame-pillar")]
pub use flame_pillar::FLAME_PILLAR;

#[cfg(feature = "fire-flamethrower")]
mod flamethrower;
#[cfg(feature = "fire-flamethrower")]
pub use flamethrower::FLAMETHROWER;

#[cfg(feature = "fire-meteor")]
mod meteor;
#[cfg(feature = "fire-meteor")]
pub use meteor::METEOR;

#[cfg(feature = "fire-spirit-flame")]
mod spirit_flame;
#[cfg(feature = "fire-spirit-flame")]
pub use spirit_flame::SPIRIT_FLAME;

#[cfg(feature = "fire-torch")]
mod torch;
#[cfg(feature = "fire-torch")]
pub use torch::TORCH;
