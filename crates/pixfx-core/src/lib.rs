//! pixfx core: effects, particles, cues, game feel and the player, painted with pixfx-draw. A port of pixelfx's core (https://github.com/patch-gg/pixelfx)
//! that reproduces its JavaScript number semantics, so an effect renders the same pixels here as there.
#![forbid(unsafe_code)]

pub mod act;
pub mod cue;
pub mod effect;
pub mod feel;
pub mod frames;
pub mod hit_flash;
pub mod key;
pub mod particles;
pub mod player;
pub mod render;
pub mod screen;
pub mod sim;
pub mod squash;
pub mod state;

pub use act::{Act, ActEvent, Contact, Performance, Phase, Puff};
pub use cue::{Cue, FEEL, TimedCue, cues_between, cues_of, impact};
pub use effect::{
    Canvas, ColorOpt, Effect, EffectState, Instance, Layer, Meta, NumOpt, OptKind, OptSpec,
    OptValue, OptionError, Options, Placement, RampOpt, check_options, find, valid_dt,
};
pub use feel::{Feel, FeelSpec};
pub use frames::{FRAME_BUDGET, FrameStore};
pub use key::{ClipKey, FrameId};
pub use player::{PixelBox, Player, color_rgba_in, light_rgba_in};
pub use render::{Rendered, edge_fade, render_frames};
pub use screen::{Ripple, flash, ripple};
pub use sim::{
    Bit, BitKind, Bits, Blazes, CrystalOpts, Flags, Kind, Lights, Particle, Particles,
    ParticlesOpts, Piece, Spawn, band, crystal, flare, sparkle,
};
pub use squash::{Blink, Squash, SquashSpec};
pub use state::{Creature, StateError, StateMeta};
