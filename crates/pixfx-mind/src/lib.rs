//! A fighter's brain for pixfx games. Each tick the game hands the mind what its fighter could perceive (a
//! [`Sense`]); when it is time to think, the mind answers with a [`Thought`]: a [`Choice`] made in a lasting
//! [`Posture`]. Skills are data ([`Skill`], known by [`SkillKind`]), so any game's fighters can use it.
//! See `AGENTS.md` and `docs/superpowers/specs/2026-11-14-pixfx-mind-design.md`.

#![forbid(unsafe_code)]

mod choice;
mod curve;
mod habits;
mod reading;
mod rng;
mod sense;
mod skill;
mod view;
mod yomi;

pub use choice::{Choice, Outcome, Posture, Thought, Why};
pub use habits::{foe_move, FoeMove, Habits};
pub use curve::{score, Consideration, Curve, Input, Inputs};
pub use reading::{is_area, is_close, is_ranged, punisher, Reading};
pub use rng::Rng;
pub use sense::{Arena, Doing, Foe, Me, Phase, Sense, Shape, Threat};
pub use skill::{BodySize, Inflicts, Profile, Skill, SkillKind};
pub use view::Delayed;
pub use yomi::Yomi;
