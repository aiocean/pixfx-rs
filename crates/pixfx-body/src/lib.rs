//! One rig for every pose, direction and animation. A figure ([`Body`]: bone lengths from a game's ruler) is
//! posed by joint angles ([`Pose`]) and seen from a facing ([`skeleton`]).
//!
//! Engine layer, pure: no I/O, no clock, `libm` maths, so the same pose gives the same joints natively and in
//! wasm. `pixfx-pose` (image-model guide sheets) builds on it.
#![forbid(unsafe_code)]

mod body;
mod camera;
pub mod catalog;
mod clip;
mod hull;
mod paint;
mod part;
mod pose;
pub mod presets;
mod rig;
mod sprite;

pub use body::Body;
pub use camera::{Camera, Dir4, Dir8, ISO_PITCH, TOPDOWN_PITCH};
pub use clip::{Clip, Contact, Ease, Key, Mode};
pub use hull::{HullShape, Shape, hull};
pub use paint::{Layers, Look, PartMap, paint, rasterize};
pub use part::Part;
pub use pose::{Head, Limb, Pose, Side, Sword};
pub use rig::{Bone, Limb4, LimbPoints, Point, Skeleton, facing, skeleton, skeleton_seen};
pub use sprite::{Cell, Sprite, clip_cell, sprite};
