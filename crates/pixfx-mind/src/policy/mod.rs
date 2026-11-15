//! A trained policy's runtime: what it sees (`observe`), what it can do (`Act`, `legal`, `choice`). The network and
//! the mind that runs it on a clock come with them. Game-neutral: a game hands in its `Sense`, its skills and a short
//! tag that says who the fighter is.

mod action;
mod observe;

pub use action::{ACTIONS, Act, Aim, SLOTS, choice, legal};
pub use observe::{OBS, TAG, observe};

use crate::sense::{Foe, Me};

/// The side the foe is on, as seen from `me`: 1 right, -1 left (its facing when they stand on one spot). The policy
/// sees and acts in a frame where the foe is always ahead.
pub(crate) fn ahead(me: &Me, foe: &Foe) -> f64 {
    let d = foe.x - me.x;
    let side = if d != 0.0 { d } else { me.face };
    if side < 0.0 { -1.0 } else { 1.0 }
}
