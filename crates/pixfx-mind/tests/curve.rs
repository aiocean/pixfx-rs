//! Response curves and considerations: inputs in [0, 1] shaped and multiplied, a zero vetoes.

use pixfx_mind::{Consideration, Curve, Input, Inputs, score};
use std::cell::Cell;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn linear_clamps() {
    let c = Curve::Linear { m: 2.0, b: 0.0 };
    assert!(close(c.at(0.8), 1.0));
    assert!(close(c.at(-1.0), 0.0));
    assert!(close(c.at(0.25), 0.5));
}

#[test]
fn inverse_flips() {
    assert!(close(Curve::Inverse.at(0.25), 0.75));
}

#[test]
fn power_bends() {
    assert!(close(Curve::Power { k: 2.0 }.at(0.5), 0.25));
}

#[test]
fn logistic_is_half_at_mid() {
    let c = Curve::Logistic { k: 10.0, mid: 0.4 };
    assert!(close(c.at(0.4), 0.5));
    assert!(c.at(0.9) > 0.99);
    assert!(c.at(0.0) < 0.05);
}

#[test]
fn step_is_a_gate() {
    let c = Curve::Step { at: 0.5 };
    assert!(close(c.at(0.49), 0.0));
    assert!(close(c.at(0.5), 1.0));
}

#[test]
fn a_nan_input_scores_zero() {
    assert!(close(Curve::Linear { m: 1.0, b: 0.0 }.at(f64::NAN), 0.0));
}

/// Every input reads the same value, and counts how often it was read.
struct Fixed {
    value: f64,
    reads: Cell<u32>,
}

impl Inputs for Fixed {
    fn input(&self, _: Input) -> f64 {
        self.reads.set(self.reads.get() + 1);
        self.value
    }
}

#[test]
fn score_vetoes_on_zero() {
    let r = Fixed {
        value: 0.3,
        reads: Cell::new(0),
    };
    let cs = [
        Consideration {
            input: Input::Qi,
            curve: Curve::Step { at: 0.5 },
        },
        Consideration {
            input: Input::Hp,
            curve: Curve::Linear { m: 1.0, b: 0.0 },
        },
        Consideration {
            input: Input::Lead,
            curve: Curve::Linear { m: 1.0, b: 0.0 },
        },
    ];
    assert!(close(score(&cs, &r), 0.0));
    assert_eq!(r.reads.get(), 1);
}

#[test]
fn score_multiplies() {
    let r = Fixed {
        value: 0.5,
        reads: Cell::new(0),
    };
    let line = Curve::Linear { m: 1.0, b: 0.0 };
    let cs = [
        Consideration {
            input: Input::Qi,
            curve: line,
        },
        Consideration {
            input: Input::Hp,
            curve: line,
        },
    ];
    assert!(close(score(&cs, &r), 0.25));
}

#[test]
fn no_considerations_score_one() {
    let r = Fixed {
        value: 0.5,
        reads: Cell::new(0),
    };
    assert!(close(score(&[], &r), 1.0));
}
