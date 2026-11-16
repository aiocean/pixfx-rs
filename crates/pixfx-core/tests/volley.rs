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
    orbit_turns: 1.0,
    orbit_radius: 30.0,
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
    let from: Vec<f64> = launches.iter().map(|l| l.path.start().0).collect();
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
        let len = 247.0 - l.path.start().0;
        assert!((l.secs - len / 100.0).abs() < 1e-9);
    }
}

static STRAIGHT_IN: Volley = Volley {
    orbit_turns: 0.0,
    ..VOLLEY
};

/// The angle it turned round `c` along the launch's path, summed, and the sharpest turn of its heading between two
/// samples.
fn winding(l: &pixfx_core::volley::Launch, c: (f64, f64)) -> (f64, f64) {
    let n = 800;
    let (mut turned, mut sharpest) = (0.0, 0.0f64);
    let (mut last, mut last_h) = l.point(0.0);
    for k in 1..=n {
        let (at, h) = l.point(l.reach() * f64::from(k) / f64::from(n));
        let a = libm::atan2(at.1 - c.1, at.0 - c.0) - libm::atan2(last.1 - c.1, last.0 - c.0);
        turned += libm::atan2(libm::sin(a), libm::cos(a));
        let dh = h - last_h;
        sharpest = sharpest.max(libm::atan2(libm::sin(dh), libm::cos(dh)).abs());
        (last, last_h) = (at, h);
    }
    (turned, sharpest)
}

#[test]
fn orbiting_shots_go_round_the_point_on_a_ring_then_strike_it() {
    let target = (200.0, 30.0);
    for l in VOLLEY.aim(&shots(), &order(target.0, target.1, Aim::Orbit), 1.0) {
        let (turned, sharpest) = winding(&l, target);
        // the point lies to the right of the way they point: clockwise on screen, once round and more
        assert!(turned > core::f64::consts::TAU, "{turned}");
        // smoothly: no corner where the pieces join
        assert!(sharpest < 0.1, "{sharpest}");
        // on the ring it keeps its distance (30 pixels, 0.85 to 1.15 of it)
        let ring = l.path.pieces().nth(1).map(|p| match *p {
            pixfx_draw::path::Piece::Turn { r0, r1, .. } => (r0, r1),
            _ => (0.0, 0.0),
        });
        let (r0, r1) = ring.unwrap_or((0.0, -1.0));
        assert!(r0 == r1 && (25.5..=34.5).contains(&r0), "{r0}");
        // and ends as a converging shot: its tip, sunk one pixel, on the point, flying on
        let (at, heading) = l.point(l.reach());
        let tip = l.tip(at, heading);
        let sunk = (tip.0 - libm::cos(heading), tip.1 - libm::sin(heading));
        assert!(near(sunk, target), "{sunk:?}");
        assert!((l.on - 50.0).abs() < 1e-9 && !l.through);
    }
}

#[test]
fn a_volley_that_does_not_orbit_converges_when_told_to_orbit() {
    let o = STRAIGHT_IN.aim(&shots(), &order(200.0, 30.0, Aim::Orbit), 1.0);
    let c = STRAIGHT_IN.aim(&shots(), &order(200.0, 30.0, Aim::Converge), 1.0);
    assert_eq!(o.len(), c.len());
    for (a, b) in o.iter().zip(&c) {
        assert_eq!(a.path, b.path);
    }
}

#[test]
fn orbit_is_an_aim_a_game_names() {
    assert_eq!(Aim::parse("orbit"), Some(Aim::Orbit));
}
