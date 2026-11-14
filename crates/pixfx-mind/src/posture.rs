//! Postures chosen by score (Unreal StateTree's utility selectors, the Infinite Axis Utility System): every
//! posture scores how well it fits the moment from its considerations, as data. The held posture gets a
//! commitment bonus that fades over its minimum time, and another must beat it by a margin, so the mind does not
//! flicker. Defend and Punish score in a band above the rest (Dragon Age: Inquisition's score bands): a blow about
//! to land, or a real opening, wins at once.

use crate::choice::Posture;
use crate::curve::{score, Consideration, Curve, Input};
use crate::reading::Reading;
use crate::skill::Profile;

/// What kind of thing an action is; each posture weighs each kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Approach,
    Retreat,
    Close,
    Ranged,
    Area,
    Defend,
    Evade,
    Buff,
    Meditate,
    Wait,
}

/// A posture as data. Its score is `base` times the best of its `options`, each the product of its
/// considerations (an option is one reason to be in the posture).
#[derive(Debug)]
pub struct PostureDef {
    pub posture: Posture,
    pub base: f64,
    /// Scores in the band above every other posture.
    pub interrupt: bool,
    /// Seconds over which the commitment bonus fades.
    pub min_time: f64,
    pub options: &'static [&'static [Consideration]],
    /// How much it likes each `Category`, in that order.
    pub weights: [f64; 10],
}

/// The interrupt band: an interrupt posture above this score is lifted over every other posture.
const BAND_FROM: f64 = 0.25;
const BAND: f64 = 1.0;
/// The commitment bonus at the moment a posture is entered.
const COMMIT: f64 = 0.3;
/// How much a new posture must beat the held one by.
const MARGIN: f64 = 0.1;

const fn c(input: Input, curve: Curve) -> Consideration {
    Consideration { input, curve }
}
const LINE: Curve = Curve::Linear { m: 1.0, b: 0.0 };
const READY: Curve = Curve::Step { at: 0.5 };
const HAS_QI: Curve = Curve::Logistic { k: 12.0, mid: 0.3 };
/// Half a score at 0, full at 1: a nudge, never a veto.
const NUDGE: Curve = Curve::Linear { m: 0.5, b: 0.5 };
/// Full at 0, 0.3 at 1: a breath after an attack holds back an attack, never vetoes it.
const NOT_CALM: Curve = Curve::Linear { m: -0.7, b: 1.0 };

pub static POSTURES: [PostureDef; 8] = [
    PostureDef {
        posture: Posture::Neutral,
        base: 0.3,
        interrupt: false,
        min_time: 0.4,
        options: &[&[]],
        weights: [1.2, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.8, 1.0],
    },
    PostureDef {
        posture: Posture::Pressure,
        base: 1.0,
        interrupt: false,
        min_time: 1.0,
        options: &[
            &[
                c(Input::Close, Curve::Power { k: 0.5 }),
                c(Input::CloseReady, READY),
                c(Input::Qi, HAS_QI),
                c(Input::Lead, NUDGE),
                c(Input::Calm, NOT_CALM),
            ],
            &[c(Input::FoeRecovering, LINE), c(Input::CloseReady, READY), c(Input::Calm, NOT_CALM)],
            &[c(Input::FoeCornered, Curve::Power { k: 2.0 }), c(Input::CloseReady, READY), c(Input::Close, LINE)],
        ],
        weights: [2.0, 0.3, 2.5, 0.8, 1.0, 0.8, 0.6, 0.6, 0.1, 0.3],
    },
    PostureDef {
        posture: Posture::Zone,
        base: 1.0,
        interrupt: false,
        min_time: 1.2,
        options: &[
            &[c(Input::RangedReady, READY), c(Input::Qi, HAS_QI), c(Input::Close, Curve::Inverse), c(Input::FoePressing, NUDGE)],
            &[c(Input::AreaReady, READY), c(Input::Qi, HAS_QI), c(Input::Close, Curve::Inverse), c(Input::FoePressing, NUDGE)],
        ],
        weights: [0.4, 1.8, 0.6, 2.5, 2.0, 1.0, 1.2, 1.0, 0.5, 0.6],
    },
    PostureDef {
        posture: Posture::Punish,
        base: 1.0,
        interrupt: true,
        min_time: 0.3,
        options: &[&[c(Input::Opening, LINE)]],
        weights: [1.2, 0.2, 3.0, 2.0, 2.0, 0.5, 0.4, 0.3, 0.1, 0.2],
    },
    PostureDef {
        posture: Posture::Defend,
        base: 1.0,
        interrupt: true,
        min_time: 0.2,
        options: &[&[c(Input::Threat, LINE)]],
        weights: [0.3, 1.5, 0.6, 0.6, 0.4, 3.0, 2.0, 0.8, 0.1, 0.3],
    },
    PostureDef {
        posture: Posture::Evade,
        base: 1.0,
        interrupt: false,
        min_time: 0.5,
        options: &[&[c(Input::Cornered, Curve::Logistic { k: 10.0, mid: 0.6 }), c(Input::Close, LINE)]],
        weights: [0.4, 2.0, 0.8, 0.8, 0.6, 1.5, 3.0, 0.6, 0.1, 0.3],
    },
    PostureDef {
        posture: Posture::Recover,
        base: 1.0,
        interrupt: false,
        min_time: 1.0,
        options: &[&[c(Input::Qi, Curve::Inverse), c(Input::Threat, Curve::Inverse), c(Input::Close, Curve::Inverse)]],
        weights: [0.2, 2.0, 0.4, 0.8, 0.5, 1.0, 1.5, 1.5, 3.0, 1.0],
    },
    PostureDef {
        posture: Posture::Finish,
        base: 1.0,
        interrupt: false,
        min_time: 1.5,
        options: &[
            &[c(Input::FoeHpLow, Curve::Linear { m: 1.6, b: -0.6 }), c(Input::Lead, NUDGE), c(Input::Calm, NOT_CALM)],
            &[c(Input::RageFull, Curve::Linear { m: 0.8, b: 0.0 })],
        ],
        weights: [2.0, 0.3, 2.5, 2.0, 2.5, 0.6, 0.5, 0.6, 0.1, 0.2],
    },
];

/// The posture the mind is in, since when.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    pub posture: Posture,
    pub since: f64,
}

/// The posture to be in now, and every posture's score (with band and commitment), in `Posture::ALL` order.
pub fn choose_posture(held: &Held, now: f64, r: &Reading, profile: &Profile) -> (Posture, [f64; 8]) {
    let mut scores = [0.0; 8];
    for (i, def) in POSTURES.iter().enumerate() {
        let fit = def.options.iter().map(|o| score(o, r)).fold(0.0, f64::max);
        let mut s = def.base * fit * profile.posture_scale[i];
        if def.interrupt && s > BAND_FROM {
            s += BAND;
        }
        if def.posture == held.posture {
            s += COMMIT * (1.0 - (now - held.since) / def.min_time).max(0.0);
        }
        scores[i] = s;
    }
    let held_i = held.posture as usize;
    let (best, top) = scores.iter().enumerate().fold((held_i, f64::MIN), |b, (i, s)| if *s > b.1 { (i, *s) } else { b });
    let pick = if best != held_i && top < scores[held_i] + MARGIN { held_i } else { best };
    (Posture::ALL[pick], scores)
}
