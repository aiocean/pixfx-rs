//! A body's squash, stretch and blink, driven the way a host drives them: it moves, lands, time passes.

use pixfx_core::squash::{Blink, Squash, SquashSpec, scale};

const FRAME: f64 = 1.0 / 60.0;

#[test]
fn a_body_stretches_with_its_speed_either_way_and_keeps_its_area() {
    let s = Squash::new(SquashSpec::default());
    assert_eq!(s.tall(0.0), 1.0, "at rest it is itself");
    assert!(s.tall(110.0) > 1.0 && s.tall(110.0) < s.tall(220.0));
    assert_eq!(
        s.tall(-220.0),
        s.tall(220.0),
        "falling stretches as rising does"
    );
    assert_eq!(s.tall(5000.0), 1.15, "no taller than the most stretch");
    let (sx, sy) = scale(s.tall(220.0));
    assert!((sx * sy - 1.0).abs() < 1e-12, "area kept: {sx} x {sy}");
    assert_eq!(scale(f64::NAN), (1.0, 1.0));
}

#[test]
fn a_landing_flattens_it_harder_the_harder_it_lands_then_it_eases_back() {
    let mut hard = Squash::new(SquashSpec::default());
    let mut soft = hard;
    hard.land(1.0);
    soft.land(0.3);
    assert_eq!(hard.tall(0.0), 0.8, "flattest on the frame it lands");
    assert!(soft.tall(0.0) > hard.tall(0.0));
    let mut last = hard.tall(0.0);
    for _ in 0..6 {
        hard.advance(FRAME);
        let now = hard.tall(0.0);
        assert!(now >= last, "eases back: {now} after {last}");
        last = now;
    }
    for _ in 0..3 {
        hard.advance(FRAME);
    }
    assert_eq!(hard.tall(0.0), 1.0, "itself once the squash is over");
    assert_eq!(hard.tall(220.0), 1.15, "and stretches again as it moves");
}

#[test]
fn a_blink_shows_faint_on_every_other_beat_while_it_lasts() {
    let mut b = Blink::new(SquashSpec::default());
    assert_eq!(b.alpha(), 1.0);
    b.start(0.5);
    let mut seen = Vec::new();
    for _ in 0..30 {
        b.advance(FRAME);
        seen.push(b.alpha());
    }
    assert!(seen.contains(&1.0) && seen.contains(&0.35), "{seen:?}");
    b.advance(0.3);
    assert!(!b.active());
    assert_eq!(b.alpha(), 1.0, "whole once it ends");
}
