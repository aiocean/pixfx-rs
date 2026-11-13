//! The marks and states a fighter carries, drawn the way a player sees them.

use pixfx_core::{Frame, Rgba};
use pixfx_fighter::status::{
    DomeStyle, Marks, draw_burning, draw_frozen, draw_marks, draw_shocked,
};

/// A frame covering the world box (-40, -70) to (40, 20): a fighter's feet go at (0, 0).
fn frame() -> Frame {
    Frame::new(80, 90, -40, -70)
}

fn at(f: &Frame, x: i32, y: i32) -> Rgba {
    let i = (y - f.oy) as usize * f.w as usize + (x - f.ox) as usize;
    Rgba::unpack(f.color.get(i).copied().unwrap_or(0))
}

fn drawn(f: &Frame) -> usize {
    f.color.iter().filter(|&&c| c != 0).count()
}

#[test]
fn nothing_on_a_fighter_draws_nothing() {
    let mut f = frame();
    draw_marks(&mut f, &Marks::default(), 0.0, 0.0);
    assert_eq!(f, frame());
}

#[test]
fn a_burn_hangs_a_flame_over_the_head() {
    let mut f = frame();
    let m = Marks {
        burning: true,
        ..Marks::default()
    };
    draw_marks(&mut f, &m, 0.0, 0.0);
    // one mark, 5 wide, centred: it starts 3 left of the feet, 48 above them
    assert_eq!(at(&f, -2, -47), Rgba::hex24(0xff9a3a));
    assert_eq!(at(&f, -1, -46), Rgba::hex24(0xfff0b0));
    assert_eq!(at(&f, 0, -10), Rgba::NONE);
}

#[test]
fn marks_line_up_left_to_right_burn_then_chill() {
    let mut f = frame();
    let m = Marks {
        burning: true,
        chilled: true,
        ..Marks::default()
    };
    draw_marks(&mut f, &m, 0.0, 0.0);
    // two marks, 11 wide, from x = -6 (5.5 rounds up); the flame's body at -5, the snowflake's stem at -6 + 6 + 2
    assert_eq!(at(&f, -5, -47), Rgba::hex24(0xff9a3a));
    assert_eq!(at(&f, 2, -49), Rgba::hex24(0xbff2ff));
}

#[test]
fn fury_pips_are_translucent_gold() {
    let mut f = frame();
    let m = Marks {
        fury: 2,
        ..Marks::default()
    };
    draw_marks(&mut f, &m, 0.0, 0.0);
    // alpha 0.9 is stored as 230 of 255, the canvas's rounding
    assert_eq!(at(&f, -7, -43), Rgba(0xff, 0xc2, 0x3d, 230));
    assert_eq!(at(&f, -4, -43), Rgba(0xff, 0xc2, 0x3d, 230));
    assert_eq!(at(&f, -1, -43), Rgba::NONE);
}

#[test]
fn a_barrier_flares_white_when_a_blow_rings_on_it() {
    let glow = Rgba::hex24(0xffe066);
    let calm = Marks {
        barrier: true,
        glow,
        dome: DomeStyle::Banded,
        ..Marks::default()
    };
    let rung = Marks { rang: true, ..calm };
    let (mut a, mut b) = (frame(), frame());
    draw_marks(&mut a, &calm, 0.0, 0.0);
    draw_marks(&mut b, &rung, 0.0, 0.0);
    // the rim at the dome's widest row, 15 left of centre, 20 above the feet
    let (ca, cb) = (at(&a, -15, -20), at(&b, -15, -20));
    // the glow, give or take the canvas's rounding through premultiplied bytes
    assert!(
        ca.0.abs_diff(0xff) <= 2 && ca.1.abs_diff(0xe0) <= 2 && ca.2.abs_diff(0x66) <= 2,
        "{ca:?}"
    );
    assert_eq!((cb.0, cb.1, cb.2), (0xff, 0xff, 0xff));
    assert!(cb.3 > ca.3, "a flare is brighter: {cb:?} vs {ca:?}");
}

type Overlay = fn(&mut Frame, f64, u32, f64, f64, f64);
const OVERLAYS: [(&str, Overlay); 3] = [
    ("burning", draw_burning),
    ("shocked", draw_shocked),
    ("frozen", draw_frozen),
];

#[test]
fn overlays_are_the_same_for_the_same_moment_and_lit() {
    for (name, draw) in OVERLAYS {
        let (mut a, mut b) = (frame(), frame());
        draw(&mut a, 0.73, 5, 0.0, 0.0, 1.0);
        draw(&mut b, 0.73, 5, 0.0, 0.0, 1.0);
        assert_eq!(a, b, "{name} is deterministic");
        assert!(drawn(&a) > 10, "{name} draws on the body");
        assert!(a.light.iter().any(|&l| l != 0), "{name} gives light");
    }
}

#[test]
fn overlays_stay_on_and_round_the_body() {
    for (name, draw) in OVERLAYS {
        for step in 0..40 {
            let mut f = frame();
            draw(&mut f, f64::from(step) * 0.05, 3, 0.0, 0.0, 1.0);
            for y in -70..20 {
                for x in -40..40 {
                    if at(&f, x, y) != Rgba::NONE {
                        assert!(
                            (-12..=12).contains(&x) && (-48..=1).contains(&y),
                            "{name} at {x},{y}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn no_intensity_draws_nothing() {
    for (name, draw) in OVERLAYS {
        let mut f = frame();
        draw(&mut f, 1.0, 1, 0.0, 0.0, 0.0);
        assert_eq!(f, frame(), "{name}");
    }
}

#[test]
fn fire_and_sparks_move_while_ice_holds_still() {
    let moved = |draw: Overlay, t0: f64, t1: f64| {
        let (mut a, mut b) = (frame(), frame());
        draw(&mut a, t0, 9, 0.0, 0.0, 1.0);
        draw(&mut b, t1, 9, 0.0, 0.0, 1.0);
        a.color != b.color
    };
    assert!(moved(draw_burning, 0.5, 0.6));
    assert!(moved(draw_shocked, 0.5, 0.6));
    // between glints the ice is the same
    let still = (0..20)
        .filter(|&i| !moved(draw_frozen, f64::from(i) * 0.1, f64::from(i) * 0.1 + 0.02))
        .count();
    assert!(still >= 10, "ice held still in {still} of 20 moments");
}
