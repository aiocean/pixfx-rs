//! Scoring primitives (the Infinite Axis Utility System's considerations): an input of the moment, normalised to
//! `[0, 1]`, shaped by a response curve; a posture's considerations multiply, and a zero vetoes.

/// How an input in `[0, 1]` turns into a score in `[0, 1]`. Inputs and outputs are clamped; NaN reads as 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    /// `m * x + b`.
    Linear { m: f64, b: f64 },
    /// `x^k`: above 1 it only rises near the top, below 1 it rises early.
    Power { k: f64 },
    /// An S around `mid`, as steep as `k`.
    Logistic { k: f64, mid: f64 },
    /// 0 below `at`, 1 from it.
    Step { at: f64 },
    /// `1 - x`.
    Inverse,
}

fn unit(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

impl Curve {
    pub fn at(&self, x: f64) -> f64 {
        let x = unit(x);
        unit(match *self {
            Curve::Linear { m, b } => m * x + b,
            Curve::Power { k } => libm::pow(x, k),
            Curve::Logistic { k, mid } => 1.0 / (1.0 + libm::exp(-k * (x - mid))),
            Curve::Step { at } => {
                if x >= at { 1.0 } else { 0.0 }
            }
            Curve::Inverse => 1.0 - x,
        })
    }
}

/// What a consideration reads off the moment. Every input is in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// My qi share.
    Qi,
    /// My life share.
    Hp,
    /// How low the foe's life is: 1 at none left.
    FoeHpLow,
    /// How far ahead on life I am: 0.5 even, 1 far ahead.
    Lead,
    /// How long the foe stays open, against my quickest blow.
    Opening,
    /// How urgent the worst thing coming at me is.
    Threat,
    /// How near the distance is to the range I like.
    InRange,
    /// How close the foe is: 1 at arm's length.
    Close,
    /// A close skill (blade, blast, dash) is ready.
    CloseReady,
    /// A ranged skill (bolt, seeker) is ready.
    RangedReady,
    /// An area or field is ready.
    AreaReady,
    /// How near my back is to a wall.
    Cornered,
    /// How near the foe's back is to a wall.
    FoeCornered,
    /// How sure I am the foe is pressing me.
    FoePressing,
    /// How sure I am the foe is keeping me out.
    FoeZoning,
    /// How sure I am the foe is out of qi, or getting it back.
    FoeRecovering,
    /// My ultimate is ready.
    RageFull,
    /// How much of my breath after an attack is left.
    Calm,
}

/// Something that can read every input: the `Reading`, or a test's stand-in.
pub trait Inputs {
    fn input(&self, input: Input) -> f64;
}

/// One input through one curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Consideration {
    pub input: Input,
    pub curve: Curve,
}

/// The product of the considerations; 1 with none. Stops at the first zero.
pub fn score(considerations: &[Consideration], inputs: &impl Inputs) -> f64 {
    let mut total = 1.0;
    for c in considerations {
        total *= c.curve.at(inputs.input(c.input));
        if total == 0.0 {
            return 0.0;
        }
    }
    total
}
