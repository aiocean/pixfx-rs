//! The landing duel's rules: two cultivators, Thunder and Frost, fight with skills drawn onto a rotating bar, with
//! statuses, wards and barriers, round after round. Everything is decided here and deterministic from a seed; a shell
//! plays the effect commands and feels the events `Duel::step` returns. Ported from `site/assets/duel.js`.
//! See `AGENTS.md` and `docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md`.

#![forbid(unsafe_code)]

pub mod act;
pub mod bar;
pub mod data;
pub mod duel;
pub mod fighter;
pub mod out;
pub mod rng;
pub mod seat;
pub mod sense;
pub mod shot;
pub mod status;
