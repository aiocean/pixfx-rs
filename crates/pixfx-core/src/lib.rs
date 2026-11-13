//! pixfx core: frames, colour, painter, light and simulation constants. A port of pixelfx's core (https://github.com/patch-gg/pixelfx)
//! that reproduces its JavaScript number semantics, so an effect renders the same pixels here as there.
#![forbid(unsafe_code)]

pub mod blend;
pub mod color;
pub mod effect;
pub mod frame;
pub mod math;
pub mod painter;
pub mod particles;
pub mod render;
pub mod shapes;
pub mod sim;

pub use color::{ColorError, Rgba, hex};
pub use effect::{Effect, EffectState, Instance, Layer, Meta, OptValue, Options, find, valid_dt};
pub use frame::{Frame, composite, composite_px, frame_hash};
pub use math::{Rng, rand};
pub use painter::Painter;
pub use render::{Rendered, edge_fade, render_frames};
pub use sim::{
    Bit, BitKind, Bits, Blazes, CrystalOpts, Flags, Kind, Lights, Particle, Particles,
    ParticlesOpts, Piece, Spawn, band, crystal, flare, sparkle,
};
