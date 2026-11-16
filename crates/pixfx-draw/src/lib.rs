//! pixfx drawing: frames with colour and additive light, the painter, shapes, painted halos, blending and palettes.
//! A port of pixelfx's drawing that reproduces its JavaScript number semantics, so the same calls paint the same
//! pixels here as there. The lowest layer of the engine: it knows nothing of effects, scenes or hosts.
#![forbid(unsafe_code)]

pub mod blend;
pub mod color;
pub mod frame;
pub mod growth;
pub mod halo;
pub mod image;
pub mod math;
pub mod painter;
pub mod palette;
pub mod shapes;
pub mod sweep;

pub use color::{ColorError, Rgba, hex};
pub use frame::{Frame, composite, composite_px, frame_hash};
pub use image::RgbaImage;
pub use math::{Rng, rand};
pub use painter::Painter;
