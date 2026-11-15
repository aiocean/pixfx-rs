//! How a fighter fights: aggressive, balanced or defensive. Drawn per fighter from the seed on dice of its own, so the
//! rules' dice are untouched; the trained policy reads it in its tag and was paid by it in training.

use crate::data::School;
use crate::rng::Rng;
use pixfx_mind::policy::TAG;

/// A fighter's style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Style {
    Aggressive,
    Balanced,
    Defensive,
}

impl Style {
    pub const ALL: [Style; 3] = [Style::Aggressive, Style::Balanced, Style::Defensive];

    /// Its place in `ALL`.
    pub fn index(self) -> usize {
        match self {
            Style::Aggressive => 0,
            Style::Balanced => 1,
            Style::Defensive => 2,
        }
    }

    /// The style at `i` in `ALL` (balanced past the end).
    pub fn from_index(i: usize) -> Style {
        Style::ALL.get(i).copied().unwrap_or(Style::Balanced)
    }
}

/// The salt of the styles' dice, so they never share a stream with the rules' (`Rng::new(seed)`).
const SALT: u32 = 0x5717_1e00;

/// Both fighters' styles for a fight of `seed`.
pub fn draw(seed: u32) -> [Style; 2] {
    let mut rng = Rng::new(seed ^ SALT);
    let mut one = || Style::from_index((rng.next() * 3.0) as usize);
    [one(), one()]
}

/// Who a fighter is, as the policy reads it: its school (Thunder, Frost) and its style, one-hot.
pub fn tag(school: School, style: Style) -> [f32; TAG] {
    let mut t = [0.0; TAG];
    let s = match school {
        School::Thunder => 0,
        School::Frost => 1,
    };
    if let Some(v) = t.get_mut(s) {
        *v = 1.0;
    }
    if let Some(v) = t.get_mut(2 + style.index()) {
        *v = 1.0;
    }
    t
}
