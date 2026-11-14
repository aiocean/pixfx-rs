//! Pixels for agents. An agent that draws pixel art in code works from a picture: an artist's, an image model's,
//! or a game's own sprite. This crate turns that picture into a grid it can read and reason about:
//!
//! - [`cut`]: the picture out of its background, stray specks and guide lines gone;
//! - [`nearest`] and [`colors`]: every pixel as an index into a palette (a given one, or the picture's own);
//! - [`shrink`]: a picture drawn big (an image model's "pixels" are many real pixels wide) down to its pixel grid;
//! - [`rects`]: equal cells merged into rects, the shape painter code fills;
//! - [`text`] and [`zoom`]: the grid shown back, as letters with a colour legend and as a big picture with grid
//!   lines, and [`Map::paint`]: at its true size;
//! - [`diff`]: how far two pictures are apart, to check what the engine paints against what was meant.
//!
//! It is a tool for the agent at work, not part of what a game ships: nothing in the engine paints from its output.
//! Pure: pictures come in as values and leave as values; reading and writing files is the CLI's job.
#![forbid(unsafe_code)]

mod cut;
mod diff;
mod grid;
mod show;

pub mod critique;
pub mod oklab;
pub mod parts;
pub mod recover;
pub mod reduce;
pub mod sheet;
pub mod view;

pub use cut::cut;
pub use diff::{Diff, diff};
pub use grid::{Map, Run, colors, nearest, rects, shrink};
pub use pixfx_draw::{Rgba, RgbaImage};
pub use show::{LEGEND, ParseError, TooManyColors, parse, text, text_named};
pub use view::zoom;

/// Alpha at and above which a pixel counts as painted.
pub(crate) const OPAQUE: u8 = 128;

/// Squared straight-line RGB distance between a pixel and a colour.
pub(crate) fn distance([r, g, b]: [u8; 3], c: Rgba) -> u32 {
    let d = |a: u8, b: u8| u32::from(a.abs_diff(b)).pow(2);
    d(r, c.0) + d(g, c.1) + d(b, c.2)
}
