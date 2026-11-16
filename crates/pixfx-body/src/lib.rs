//! One rig for every pose, direction and animation. A figure ([`Body`]: bone lengths from a game's ruler) is
//! posed by joint angles ([`Pose`]) and seen from a facing ([`skeleton`]).
//!
//! Engine layer, pure: no I/O, no clock, `libm` maths, so the same pose gives the same joints natively and in
//! wasm. `pixfx-pose` (image-model guide sheets) builds on it.
#![forbid(unsafe_code)]

mod body;
mod pose;
mod rig;

pub use body::Body;
pub use pose::{Head, Limb, Pose, Side, Sword};
pub use rig::{Bone, LimbPoints, Point, Skeleton, facing, skeleton};
