//! A volley of like shots aimed at an order (`Volley::aim`), whatever the shots are: onto the line through the
//! point and on past it, striking the point only if it lands there.

use pixfx_core::volley::{Aim, Order, Shot, Volley};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

struct Dart;

static DART: Dart = Dart;

const META: Meta = Meta {
    id: "test/dart",
    name: "Dart",
    desc: "A shot.",
    tags: &[],
    size: (4, 4),
    origin: (2, 2),
    duration: 0.2,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

struct Nothing;

impl EffectState for Nothing {
    fn draw(&self, _p: &mut Painter, _t: f64, _layer: Layer) {}
}

impl Effect for Dart {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(Nothing)
    }
}

static VOLLEY: Volley = Volley {
    piece: &DART,
    held: None,
    tail: None,
    hit: None,
    poise: 0.0,
    gap: 0.1,
    speed: 100.0,
    min_secs: 0.1,
    max_secs: 5.0,
    nose: 3.0,
    sink: 1.0,
    bend: 0.3,
    bend_min: 5.0,
    bend_max: 40.0,
    spread: (0.0, 0.0),
    ease: 1.0,
    beyond: 50.0,
    lands: false,
};

static THROWN: Volley = Volley {
    lands: true,
    ..VOLLEY
};

fn order(x: f64, y: f64, aim: Aim) -> Order {
    Order {
        x,
        y,
        aim,
        caster: None,
    }
}

fn shots() -> Vec<Shot> {
    // the far one listed first
    [(0.0, 0.0), (80.0, 0.0), (40.0, 0.0)]
        .map(|at| Shot { at, heading: 0.0 })
        .to_vec()
}

fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
}

#[test]
fn the_shot_nearest_the_target_goes_first_and_the_rest_a_gap_apart() {
    let launches = VOLLEY.aim(&shots(), &order(200.0, 0.0, Aim::Converge), 1.0);
    let from: Vec<f64> = launches.iter().map(|l| l.path.from.0).collect();
    assert_eq!(from, [80.0, 40.0, 0.0]);
    let after: Vec<f64> = launches.iter().map(|l| l.after).collect();
    assert!(
        after
            .iter()
            .zip([0.0, 0.1, 0.2])
            .all(|(a, b)| (a - b).abs() < 1e-9),
        "{after:?}"
    );
}

#[test]
fn converging_shots_curve_onto_the_point_and_fly_on_past_it_striking_only_what_they_meet() {
    for l in VOLLEY.aim(&shots(), &order(200.0, 30.0, Aim::Converge), 1.0) {
        // at the curve's end the tip, sunk one pixel, is on the point
        let (at, heading) = l.point(l.reach());
        let tip = l.tip(at, heading);
        let sunk = (tip.0 - libm::cos(heading), tip.1 - libm::sin(heading));
        assert!(near(sunk, (200.0, 30.0)), "{sunk:?}");
        // and it flies on the same way
        assert!((l.on - 50.0).abs() < 1e-9 && !l.through && l.mark.is_none());
        let (past, h2) = l.at(1.0);
        assert!((h2 - heading).abs() < 1e-9 && past.0 > at.0);
    }
}

#[test]
fn a_volley_that_lands_strikes_the_point_and_goes_there() {
    for l in THROWN.aim(&shots(), &order(200.0, 30.0, Aim::Converge), 1.0) {
        let mark = l.mark.unwrap_or(-1.0);
        // its tip on the point, a pixel short of sinking in
        let (at, heading) = l.point(mark);
        let tip = l.tip(at, heading);
        assert!(
            (tip.0 - 200.0).abs() < 0.5 && (tip.1 - 30.0).abs() < 0.5,
            "{tip:?}"
        );
        assert!(l.on == 0.0);
    }
}

#[test]
fn piercing_shots_fly_straight_through_the_point_and_on_at_one_speed() {
    for l in VOLLEY.aim(&shots(), &order(200.0, 0.0, Aim::Pierce), 1.0) {
        let (at, heading) = l.point(l.reach());
        assert!(near(l.tip(at, heading), (200.0, 0.0)));
        assert!(l.through && (l.on - 50.0).abs() < 1e-9);
        let (end, _) = l.at(1.0);
        assert!(near(end, (247.0, 0.0)), "{end:?}");
        let len = 247.0 - l.path.from.0;
        assert!((l.secs - len / 100.0).abs() < 1e-9);
    }
}
