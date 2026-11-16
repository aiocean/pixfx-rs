//! Pose skeletons for image models. Build a figure ([`Body`]), pose it by joint angles ([`Pose`], or start from
//! one of the [`presets`]), see it from any [`facing`], and draw a sheet of captioned, colour-coded skeletons
//! ([`PoseSheet`]) for an image model to draw a character into, one pose a cell.
//!
//! Why this shape: a spike on 2026-11-14 found an image model follows a sheet of skeletons when each cell names
//! its pose in words and every limb has its own colour (left and right arm and leg, and the sword); a plain stick
//! figure left it guessing which arm holds the sword and which knee is down. [`legend`] gives the colour key as
//! prompt lines, so the prompt always says what the sheet shows.
//!
//! The skeleton is 3D: one pose seen at another facing turns the whole figure, so a turnaround of eight
//! directions is the same pose eight times ([`turnaround`]).
//!
//! Pure: a sheet comes out as an image and a layout; writing files is `examples/pose_sheet.rs`'s job. The layout
//! is `pixfx_pixel::sheet`'s, so `pixfx_pixel::sheet::cut` takes the model's drawing back apart cell by cell.
#![forbid(unsafe_code)]

mod font;
pub mod presets;
mod sheet;
pub mod spec;

// The rig lives in pixfx-body; its names stay here so every script and spec keeps working.
pub use pixfx_body::{
    Body, Bone, Head, Limb, LimbPoints, Point, Pose, Side, Skeleton, Sword, facing, skeleton,
};
pub use sheet::{Cell, Drawn, HEAD, PoseSheet, captions, colour, legend, turnaround};
