//! The screen's answer to impacts, driven the way a game drives it: cues and impacts in, then real time passing.

use pixfx_core::cue::{Cue, TimedCue};
use pixfx_core::feel::Feel;

const FRAME: f64 = 1.0 / 60.0;

fn at(cue: Cue) -> TimedCue {
    TimedCue {
        t: 0.0,
        cue,
        pan: None,
    }
}

/// Seconds of play that `frames` frames of real time buy.
fn play_time(feel: &mut Feel, frames: u32) -> f64 {
    (0..frames).map(|_| feel.advance(FRAME)).sum()
}

#[test]
fn trauma_adds_up_to_one() {
    let mut feel = Feel::new();
    feel.take(&[
        at(Cue::Shake { trauma: 0.7 }),
        at(Cue::Shake { trauma: 0.7 }),
    ]);
    assert_eq!(feel.trauma(), 1.0);
}

#[test]
fn freezes_do_not_add_up_the_longest_wins_under_the_cap() {
    let mut feel = Feel::new();
    feel.freeze(0.1);
    feel.take(&[at(Cue::Hitstop { dur: 5.0 })]);
    feel.freeze(0.05);
    let cap = feel.spec().freeze_cap;
    let frozen = 1.0 - play_time(&mut feel, 60);
    assert!((frozen - cap).abs() < FRAME, "froze {frozen}, cap {cap}");
    assert!(!feel.frozen());
}

#[test]
fn slow_motion_runs_play_at_its_speed_after_a_freeze() {
    let mut feel = Feel::new();
    feel.freeze(0.1);
    feel.slow(0.5, 0.25);
    let first = play_time(&mut feel, 6);
    assert_eq!(first, 0.0, "frozen first");
    let slowed = play_time(&mut feel, 30);
    assert!((slowed - 0.5 * 0.25).abs() < 1e-9, "slowed {slowed}");
    assert_eq!(feel.advance(FRAME), FRAME, "then full speed");
}

#[test]
fn ripples_and_flash_run_on_through_a_freeze() {
    let mut feel = Feel::new();
    feel.take(&[
        at(Cue::Ripple {
            x: 5.0,
            y: 5.0,
            r0: 0.0,
            r1: 20.0,
            dur: 0.05,
            amp: 1.0,
        }),
        at(Cue::Flash { k: 0.5, dur: 0.1 }),
        at(Cue::Flash { k: 0.3, dur: 0.1 }),
        at(Cue::Hitstop { dur: 0.2 }),
    ]);
    assert_eq!(feel.flash(), 0.5, "the strongest flash");
    assert_eq!(feel.ripples().len(), 1);
    assert_eq!(play_time(&mut feel, 5), 0.0);
    assert!(feel.ripples().is_empty());
    assert!(feel.flash() < 0.5);
}

#[test]
fn a_sound_asks_nothing_of_the_screen() {
    let mut feel = Feel::new();
    feel.take(&[at(Cue::Sound {
        name: "x",
        gain: 1.0,
        pitch: 1.0,
        len: None,
        x: None,
    })]);
    assert_eq!(feel.trauma(), 0.0);
    assert_eq!(feel.advance(FRAME), FRAME);
}

#[test]
fn a_shake_sways_from_frame_to_frame_instead_of_jumping() {
    let mut feel = Feel::new();
    feel.add_trauma(1.0);
    let mut seen = Vec::new();
    for _ in 0..20 {
        seen.push(feel.shake(1.0));
        feel.advance(FRAME);
    }
    let reach = seen.iter().map(|&(x, y)| x.abs().max(y.abs())).max();
    assert!(
        reach >= Some(2),
        "a full shake reaches its pixels: {seen:?}"
    );
    for w in seen.windows(2) {
        let [(x0, y0), (x1, y1)] = [w[0], w[1]];
        // a fresh random spot each frame would jump up to twice the reach
        assert!(
            (x1 - x0).abs() <= 2 && (y1 - y0).abs() <= 2,
            "jumped: {seen:?}"
        );
    }
}

#[test]
fn the_shake_settles_and_the_player_can_turn_it_down_or_off() {
    let mut feel = Feel::new();
    feel.add_trauma(1.0);
    let (mut full, mut half) = (0, 0);
    for _ in 0..30 {
        let (x, y) = feel.shake(1.0);
        full = full.max(x.abs().max(y.abs()));
        let (x, y) = feel.shake(0.5);
        half = half.max(x.abs().max(y.abs()));
        assert_eq!(feel.shake(0.0), (0, 0));
        assert_eq!(feel.shake(f64::NAN), (0, 0));
        feel.advance(FRAME);
    }
    assert!(half < full, "half {half}, full {full}");
    play_time(&mut feel, 60);
    assert_eq!(feel.shake(1.0), (0, 0));
}
