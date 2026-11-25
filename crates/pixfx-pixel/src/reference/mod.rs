//! Effect references from an image model.
//!
//! An effect we write by hand starts as a picture an image model draws: a sheet of labelled cells, one frame each
//! (the effect forming, holding, taking a blow, ending), around a stick figure that shows where a character stands.
//! This module makes that sheet and its prompt from a [`spec::Spec`], builds the request to the model
//! ([`openrouter`]), and cuts what the model drew back into pixfx's two layers, colour and light, frame by frame,
//! with a review page ([`extract`], [`review`]), every frame as a pixel matrix with a shared palette
//! ([`matrix`]), and frames in between the drawn ones made by moving their pixels ([`tween`]). With a pixel grid and
//! a row of parts (the effect taken apart: main object, its pieces, small effects, light), the cells come back at
//! the effect's own resolution and the parts alone. The agent reads the parts and frames and writes the effect's
//! painter code, then checks it against the sheet ([`compare`]) and measures how much each step changes in both
//! ([`entropy`]); nothing in the engine paints from the sheet.
//!
//! The library is pure: no files, no network. The scripts in `examples/` do the I/O (`template`, `generate`,
//! `extract`, `compare`, `gallery`, `whole`), and the skill `.claude/skills/pixfx-ref/SKILL.md` walks through making an
//! effect with them.

pub mod base64;
pub mod compare;
pub mod entropy;
pub mod extract;
pub mod matrix;
pub mod openrouter;
pub mod prompt;
pub mod review;
pub mod spec;
pub mod template;
pub mod tween;
pub mod whole;

pub use pixfx_draw::RgbaImage;
pub use spec::{Layer, Part, Row, Spec};
