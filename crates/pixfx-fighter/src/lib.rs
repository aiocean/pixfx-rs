//! Pixel fighters for pixfx, from the site's duel: a drawn fighter with a pose for each thing it can be doing
//! ([`paint`]), a body that takes hits the way fighting games do ([`body`]), the screen's answer to a blow
//! ([`feel`]) and the marks that hang on a fighter ([`status`]). The rules of a fight (moves, damage, AI) stay
//! with the game. Pure like `pixfx-core`: no I/O, no clock; randomness comes in from the caller.
#![forbid(unsafe_code)]

pub mod body;
pub mod feel;
pub mod look;
pub mod paint;
pub mod pose;
pub mod status;
