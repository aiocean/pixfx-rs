//! A brush swept along poses (`pixfx_draw::sweep`): what a camera sees of a bright thing waved through one
//! exposure. A point leaves a line, a column a band, a bar turned about its end a fan, and the newest pass is on top.

use core::f64::consts::FRAC_PI_2;

use pixfx_draw::color::Rgba;
use pixfx_draw::frame::Frame;
use pixfx_draw::painter::Painter;
use pixfx_draw::sweep::{Brush, Cell, Look, Pose, Turn, sweep};

const RED: Rgba = Rgba(255, 0, 0, 255);
const BLUE: Rgba = Rgba(0, 0, 255, 255);

/// A plain look: no fading, no taper, no wave, no breakup, no light.
const PLAIN: Look = Look {
    secs: 1.0,
    fade: 0.0,
    taper: 1.0,
    wave: 0.0,
    phase: 0.0,
    breakup: 1.0,
    shift: 0.0,
    light: 0.0,
    skip: 0.0,
    soft: 1.0,
};

fn pose(x: f64, y: f64, angle: f64, age: f64) -> Pose {
    Pose { x, y, angle, age }
}

/// Sweep on a 64 x 64 frame with the origin in its middle; the lit pixels in local coordinates, with their colour.
fn swept(poses: &[Pose], brush: &Brush) -> Vec<(i32, i32, u32)> {
    let mut f = Frame::new(64, 64, 0, 0);
    let mut p = Painter::new(&mut f);
    p.place(32.0, 32.0, false, 0.0);
    sweep(&mut p, poses, brush, &[], &PLAIN);
    f.color
        .iter()
        .enumerate()
        .filter(|(_, c)| **c >> 24 != 0)
        .map(|(i, &c)| (i as i32 % 64 - 32, i as i32 / 64 - 32, c))
        .collect()
}

fn point(color: Rgba) -> Brush {
    Brush {
        cells: vec![Cell {
            across: 0.0,
            color,
            light: 0.0,
        }],
        turn: Turn::Heading,
    }
}

#[test]
fn a_point_swept_along_a_line_covers_the_line() {
    let lit = swept(
        &[pose(20.0, 0.0, 0.0, 0.0), pose(0.0, 0.0, 0.0, 0.5)],
        &point(RED),
    );
    let mut xs: Vec<i32> = lit
        .iter()
        .map(|&(x, y, _)| {
            assert_eq!(y, 0, "off the line at x {x}");
            x
        })
        .collect();
    xs.sort_unstable();
    assert_eq!(xs, (0..=20).collect::<Vec<_>>());
}

#[test]
fn a_column_swept_straight_covers_a_band() {
    let brush = Brush::column(&[RED, BLUE], 3.0);
    assert_eq!(brush.cells.len(), 3);
    let lit = swept(
        &[pose(20.0, 0.0, 0.0, 0.0), pose(0.0, 0.0, 0.0, 0.5)],
        &brush,
    );
    assert_eq!(lit.len(), 21 * 3, "{lit:?}");
    // the middle in the first colour, the edges a step on
    for &(x, y, c) in &lit {
        assert!((0..=20).contains(&x) && (-1..=1).contains(&y));
        assert_eq!(
            c,
            if y == 0 { RED.pack() } else { BLUE.pack() },
            "({x}, {y})"
        );
    }
}

#[test]
fn a_bar_turned_about_its_end_covers_a_fan() {
    // a bar 15 pixels long fixed to the mover, turned a quarter turn in one step without moving
    let bar = Brush {
        cells: (0..=15)
            .map(|k| Cell {
                across: f64::from(k),
                color: RED,
                light: 0.0,
            })
            .collect(),
        turn: Turn::Angle(0.0),
    };
    // across is square to the angle: at angle 0 the bar points down (+y), at -PI/2 it points right (+x)
    let lit = swept(
        &[pose(0.0, 0.0, -FRAC_PI_2, 0.0), pose(0.0, 0.0, 0.0, 0.5)],
        &bar,
    );
    for y in 0..=13 {
        for x in 0..=13 {
            if x * x + y * y <= 13 * 13 {
                assert!(
                    lit.iter().any(|&(a, b, _)| (a, b) == (x, y)),
                    "hole at ({x}, {y})"
                );
            }
        }
    }
}

#[test]
fn the_newest_pass_is_on_top() {
    // red crosses first (older), blue crosses later (newer), through the same pixel (10, 0)
    let mut f = Frame::new(64, 64, 0, 0);
    let mut p = Painter::new(&mut f);
    p.place(32.0, 32.0, false, 0.0);
    sweep(
        &mut p,
        &[pose(10.0, -10.0, 0.0, 0.5), pose(10.0, 10.0, 0.0, 0.9)],
        &point(RED),
        &[],
        &PLAIN,
    );
    sweep(
        &mut p,
        &[pose(20.0, 0.0, 0.0, 0.0), pose(0.0, 0.0, 0.0, 0.2)],
        &point(BLUE),
        &[],
        &PLAIN,
    );
    assert_eq!(f.color[32 * 64 + 42], BLUE.pack());
    // one sweep holding both passes: the younger wins too
    let mut g = Frame::new(64, 64, 0, 0);
    let mut q = Painter::new(&mut g);
    q.place(32.0, 32.0, false, 0.0);
    let looped = [
        pose(10.0, -10.0, 0.0, 0.0),
        pose(10.0, 10.0, 0.0, 0.2),
        pose(20.0, 0.0, 0.0, 0.4),
        pose(0.0, 0.0, 0.0, 0.6),
    ];
    let ramp = [RED, BLUE];
    sweep(
        &mut q,
        &looped,
        &point(RED),
        &ramp,
        &Look { fade: 2.0, ..PLAIN },
    );
    assert_eq!(
        g.color[32 * 64 + 42],
        RED.pack(),
        "the young pass is red, the old one blue"
    );
}

#[test]
fn an_empty_brush_one_pose_or_a_still_mover_draws_nothing() {
    assert!(
        swept(
            &[pose(1.0, 1.0, 0.0, 0.0), pose(9.0, 1.0, 0.0, 0.2)],
            &Brush {
                cells: vec![],
                turn: Turn::Heading
            }
        )
        .is_empty()
    );
    assert!(swept(&[pose(1.0, 1.0, 0.0, 0.0)], &point(RED)).is_empty());
    assert!(
        swept(
            &[pose(1.0, 1.0, 0.0, 0.0), pose(1.0, 1.0, 0.0, 0.2)],
            &point(RED)
        )
        .is_empty()
    );
}

#[test]
fn from_column_reads_a_frames_column() {
    let mut f = Frame::new(4, 6, 0, 0);
    f.color[4 + 2] = RED.pack();
    f.color[3 * 4 + 2] = BLUE.pack();
    f.light[3 * 4 + 2] = Rgba(200, 100, 0, 0).pack();
    let b = Brush::from_column(&f, 2, 2);
    assert_eq!(b.turn, Turn::Angle(0.0));
    assert_eq!(b.cells.len(), 2);
    assert_eq!(
        (b.cells[0].across, b.cells[0].color, b.cells[0].light),
        (-1.0, RED, 0.0)
    );
    assert_eq!(b.cells[1].across, 1.0);
    assert_eq!(b.cells[1].color, BLUE);
    assert!((b.cells[1].light - 200.0 / 255.0).abs() < 1e-9);
    assert!(Brush::from_column(&f, 0, 2).cells.is_empty());
    assert!(Brush::from_column(&f, 9, 2).cells.is_empty());
}

#[test]
fn edge_takes_each_rows_pixel_furthest_back() {
    // a 3-wide, 3-tall thing facing +x: its back column is red, the rest blue, the middle row starting a pixel later
    let mut px = Vec::new();
    for y in -1..=1 {
        for x in 0..3 {
            if y == 0 && x == 0 {
                continue;
            }
            let back = x == 0 || (y == 0 && x == 1);
            px.push((
                f64::from(x),
                f64::from(y),
                if back { RED } else { BLUE },
                0.5,
            ));
        }
    }
    assert_eq!(
        Brush::edge(px.clone(), 1)
            .cells
            .iter()
            .map(|c| c.color)
            .collect::<Vec<_>>(),
        vec![BLUE, BLUE, BLUE],
        "one in from the back is the body behind the outline"
    );
    let b = Brush::edge(px, 0);
    assert_eq!(b.turn, Turn::Heading);
    let cells: Vec<(f64, Rgba, f64)> = b
        .cells
        .iter()
        .map(|c| (c.across, c.color, c.light))
        .collect();
    assert_eq!(
        cells,
        vec![(-1.0, RED, 0.5), (0.0, RED, 0.5), (1.0, RED, 0.5)]
    );
    // swept straight along +x it smears a band as tall as the thing, in its edge's colour
    let lit = swept(&[pose(20.0, 0.0, 0.0, 0.0), pose(0.0, 0.0, 0.0, 0.5)], &b);
    for y in -1..=1 {
        assert!(
            lit.iter().any(|&(x, yy, _)| (x, yy) == (5, y)),
            "row {y} not smeared"
        );
    }
}

#[test]
fn a_soft_streak_turns_translucent_toward_its_end_and_stays_whole() {
    let mut f = Frame::new(64, 64, 0, 0);
    let mut p = Painter::new(&mut f);
    p.place(32.0, 32.0, false, 0.0);
    // newest at x 20, a second old at x 0: soft from halfway
    sweep(
        &mut p,
        &[pose(20.0, 0.0, 0.0, 0.0), pose(0.0, 0.0, 0.0, 1.0)],
        &point(RED),
        &[],
        &Look { soft: 0.5, ..PLAIN },
    );
    let alpha = |x: i32| f.color[32 * 64 + (32 + x) as usize] >> 24;
    assert_eq!(alpha(20), 255, "the head stays solid");
    assert_eq!(alpha(12), 255, "solid until halfway");
    assert!(
        alpha(5) > 0 && alpha(5) < 255,
        "translucent past halfway: {}",
        alpha(5)
    );
    assert!(alpha(2) < alpha(5), "clearer toward the end");
    assert!(
        (1..=20).all(|x| alpha(x) > 0),
        "no pixel dithered away: a blend, not a dither"
    );
}

#[test]
fn filtering_a_wake_keeps_its_wave_across_the_join() {
    use pixfx_draw::sweep::sweep_filtered;

    let poses = [
        pose(26.0, 0.0, 0.0, 0.0),
        pose(17.0, -4.0, 0.0, 0.25),
        pose(7.0, 1.0, 0.0, 0.50),
        pose(0.0, 0.0, 0.0, 0.75),
    ];
    let brush = point(RED);
    let look = Look { wave: 2.0, ..PLAIN };
    let mut whole = Frame::new(64, 64, 0, 0);
    let mut split = Frame::new(64, 64, 0, 0);
    let mut p = Painter::new(&mut whole);
    p.place(32.0, 32.0, false, 0.0);
    sweep(&mut p, &poses, &brush, &[], &look);
    let mut p = Painter::new(&mut split);
    p.place(32.0, 32.0, false, 0.0);
    sweep_filtered(&mut p, &poses, &brush, &[], &look, |age| age > 0.32);
    sweep_filtered(&mut p, &poses, &brush, &[], &look, |age| age <= 0.32);
    assert_eq!(whole, split);
}
