//! pixfx drawing: frames with colour and additive light, the painter, shapes, blending, palettes and outline rings.
//! A port of pixelfx's drawing that reproduces its JavaScript number semantics, so the same calls paint the same
//! pixels here as there. The lowest layer of the engine: it knows nothing of effects, scenes or hosts.
#![forbid(unsafe_code)]

pub mod blend;
pub mod color;
pub mod frame;
pub mod math;
pub mod outline;
pub mod painter;
pub mod palette;
pub mod shapes;

pub use color::{ColorError, Rgba, hex};
pub use frame::{Frame, composite, composite_px, frame_hash};
pub use math::{Rng, rand};
pub use outline::{Outline, OutlineError, Ring};
pub use painter::Painter;
