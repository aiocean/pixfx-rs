//! One rig for every pose, direction and animation. A figure ([`Body`]: bone lengths from a game's ruler) is
//! posed by joint angles ([`Pose`]) and seen from a facing ([`skeleton`]).
//!
//! Engine layer, pure: no I/O, no clock, `libm` maths, so the same pose gives the same joints natively and in
//! wasm. `pixfx-pose` (image-model guide sheets) builds on it.
#![forbid(unsafe_code)]

mod body;
mod camera;
mod clip;
mod pose;
mod rig;

pub use body::Body;
pub use camera::{Camera, Dir4, Dir8, ISO_PITCH, TOPDOWN_PITCH};
pub use clip::{Clip, Contact, Ease, Key, Mode};
pub use pose::{Head, Limb, Pose, Side, Sword};
pub use rig::{Bone, Limb4, LimbPoints, Point, Skeleton, facing, skeleton, skeleton_seen};
