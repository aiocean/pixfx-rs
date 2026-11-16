//! A fighter's brain for pixfx games. Each tick the game hands the mind what its fighter could perceive (a
//! [`Sense`]); when it is time to think, the mind answers with a [`Thought`]: a [`Choice`] made in a lasting
//! [`Posture`]. Skills are data ([`Skill`], known by [`SkillKind`]), so any game's fighters can use it.
//! See `AGENTS.md` and `docs/superpowers/specs/2026-11-14-pixfx-mind-design.md`.

#![forbid(unsafe_code)]

mod candidates;
mod choice;
mod curve;
mod habits;
mod mind;
mod pick;
pub mod policy;
mod posture;
mod reading;
mod rng;
mod sense;
mod skill;
mod view;
mod yomi;

pub use candidates::{Candidate, Ctx, candidates};
pub use choice::{Choice, Outcome, Posture, Thought, Why};
pub use curve::{Consideration, Curve, Input, Inputs, score};
pub use habits::{Answers, FoeMove, Habits, foe_move};
pub use mind::Mind;
pub use pick::{Read, pick};
pub use posture::{Category, Held, POSTURES, PostureDef, choose_posture};
pub use reading::{Reading, is_area, is_close, is_ranged, punisher, read_posture};
pub use rng::Rng;
pub use sense::{ActPhase, Acting, Arena, Doing, Foe, Me, Sense, Threat, ThreatShape};
pub use skill::{BodySize, Inflicts, Profile, Skill, SkillKind};
pub use view::Delayed;
pub use yomi::Yomi;
