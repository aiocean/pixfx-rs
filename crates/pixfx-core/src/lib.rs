//! pixfx core: frames, colour, painter, light and simulation constants. A port of pixelfx's core (https://github.com/patch-gg/pixelfx)
//! that reproduces its JavaScript number semantics, so an effect renders the same pixels here as there.
#![forbid(unsafe_code)]

pub mod color;
pub mod frame;
pub mod math;
pub mod painter;
pub mod sim;

pub use color::{ColorError, Rgba, hex};
pub use frame::{Frame, composite};
pub use math::{Rng, rand};
pub use painter::Painter;
