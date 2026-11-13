//! A sprite's hit flash, driven the way a host drives it: it is struck, play time passes (or stops for a hit-stop).

use pixfx_core::hit_flash::{HIT_FLASH, HitFlash};

const FRAME: f64 = 1.0 / 60.0;

#[test]
fn a_struck_sprite_is_filled_for_its_flash_then_drawn_as_itself() {
    let mut f = HitFlash::new();
    assert!(!f.active(), "unstruck it is itself");
    f.start(HIT_FLASH);
    let mut lit = 0;
    while f.active() {
        lit += 1;
        f.advance(FRAME);
    }
    assert_eq!(lit, 5, "filled for its 5 frames");
    assert_eq!(f.amount(), 0.0);
}

#[test]
fn the_fill_fades_from_full_as_struck() {
    let mut f = HitFlash::new();
    f.start(0.1);
    assert_eq!(f.amount(), 1.0);
    f.advance(0.05);
    assert!((f.amount() - 0.5).abs() < 1e-9, "half way: {}", f.amount());
}

#[test]
fn a_hit_stop_holds_the_flash_and_play_time_runs_it_down() {
    let mut f = HitFlash::new();
    f.start(HIT_FLASH);
    for _ in 0..30 {
        f.advance(0.0);
    }
    assert_eq!(f.amount(), 1.0, "frozen play holds it lit");
}

#[test]
fn a_second_hit_relights_a_fading_flash_but_a_shorter_one_does_not_cut_it() {
    let mut f = HitFlash::new();
    f.start(0.1);
    f.advance(0.08);
    f.start(0.1);
    assert_eq!(f.amount(), 1.0, "struck again: lit again");
    f.start(0.01);
    assert_eq!(f.amount(), 1.0, "a shorter flash leaves the longer one");
    f.start(f64::NAN);
    f.start(-1.0);
    assert!(f.active());
}
