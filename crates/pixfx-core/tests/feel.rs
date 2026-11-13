//! The screen's answer to impacts, driven the way a game drives it: cues and impacts in, then real time passing.

use pixfx_core::cue::{Cue, TimedCue, impact};
use pixfx_core::effect::Options;
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
        reach >= Some(4),
        "a full shake reaches its pixels: {seen:?}"
    );
    // A fresh random spot each frame moves about 4/3 as far as the offset itself (2a/3 against a/2 for uniform
    // values in [-a, a]); a sway moves well under it.
    let offset: i32 = seen.iter().map(|&(x, y)| x.abs() + y.abs()).sum();
    let step: i32 = seen
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).abs() + (w[1].1 - w[0].1).abs())
        .sum();
    assert!(step * 4 < offset * 3, "jumped: {seen:?}");
    // the axes pass the centre at different moments: the screen does not snap back to rest
    assert!(
        seen.iter().skip(1).all(|&s| s != (0, 0)),
        "at rest mid-shake: {seen:?}"
    );
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

#[test]
fn trauma_drains_in_about_a_second() {
    let mut feel = Feel::new();
    feel.add_trauma(1.0);
    play_time(&mut feel, 30);
    assert!(
        (feel.trauma() - 0.5).abs() < 1e-9,
        "half a second: {}",
        feel.trauma()
    );
    play_time(&mut feel, 30);
    assert_eq!(feel.trauma(), 0.0);
}

#[test]
fn the_screen_rolls_with_the_shake_smoothly_and_settles() {
    let mut feel = Feel::new();
    let max = feel.spec().roll;
    assert_eq!(feel.roll(1.0), 0.0, "no trauma, no tilt");
    feel.add_trauma(1.0);
    let mut seen = Vec::new();
    for _ in 0..30 {
        let r = feel.roll(1.0);
        assert!(r.abs() <= max, "within the max roll: {r}");
        assert!(feel.roll(0.5).abs() <= r.abs());
        assert_eq!(feel.roll(0.0), 0.0);
        assert_eq!(feel.roll(f64::NAN), 0.0);
        seen.push(r);
        feel.advance(FRAME);
    }
    let reach = seen.iter().fold(0.0_f64, |m, r| m.max(r.abs()));
    assert!(reach > max * 0.2, "a full shake tilts the screen: {seen:?}");
    for w in seen.windows(2) {
        // fresh random angles would jump by up to twice the max
        assert!((w[1] - w[0]).abs() < max * 0.5, "jumped: {seen:?}");
    }
    play_time(&mut feel, 60);
    assert_eq!(feel.roll(1.0), 0.0);
}

#[test]
fn a_heavy_hit_shakes_clearly_and_a_light_one_stays_subtle() {
    // the most the screen moves after one hit, wherever in the noise it lands
    let peak = |weight: f64, start: u32| {
        let mut feel = Feel::new();
        play_time(&mut feel, start * 7);
        feel.take(&impact(0.0, weight, 0.0, 0.0, &Options::default()));
        (0..30)
            .map(|_| {
                let (x, y) = feel.shake(1.0);
                feel.advance(FRAME);
                x.abs().max(y.abs())
            })
            .max()
            .unwrap_or(0)
    };
    let blasts: Vec<i32> = (0..20).map(|start| peak(1.0, start)).collect();
    let pops: Vec<i32> = (0..20).map(|start| peak(0.0, start)).collect();
    // wherever it lands in the noise, a blast moves the screen; on average 2 to 4 px
    assert!(
        blasts.iter().all(|&p| p >= 1),
        "a blast always moves it: {blasts:?}"
    );
    assert!(
        blasts.iter().sum::<i32>() >= 50,
        "a blast moves it 2 to 4 px: {blasts:?}"
    );
    assert!(
        pops.iter().all(|&p| p <= 1),
        "a pop moves it 1 px at most: {pops:?}"
    );
}
