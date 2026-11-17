//! Projectiles: `trail`, the streak a mover leaves along the way it came.

#[cfg(feature = "projectiles-trail")]
mod trail;

#[cfg(feature = "projectiles-trail")]
pub use trail::TRAIL;
