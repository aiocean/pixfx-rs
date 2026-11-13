//! The screen's answer to the fight, driven the way a game drives it: what the fight did, then real time passing
//! (the site's `duel-feel.test.js`).

use pixfx_fighter::feel::{Cue, Feel, NumberKind};

const FRAME: f64 = 1.0 / 60.0;

/// Seconds of fight that `seconds` of real time buys.
fn fight_time(feel: &mut Feel, seconds: f64) -> f64 {
    let mut t = 0.0;
    let frames = pixfx_core::math::js_round(seconds / FRAME) as u32;
    for _ in 0..frames {
        t += feel.advance(FRAME);
    }
    t
}

/// A blow at the given weight, from fighter `by` at (`x`, `y`), struck up close.
fn hit(
    by: usize,
    (power, launch, crit): (f64, bool, bool),
    share: f64,
    damage: f64,
    (x, y): (f64, f64),
) -> Cue {
    Cue::Hit {
        by,
        power,
        launch,
        crit,
        ranged: false,
        share,
        damage,
        x,
        y,
    }
}

#[test]
fn a_blow_stops_the_fight_for_a_few_frames_then_it_runs_on_at_full_speed() {
    let mut feel = Feel::new();
    feel.react(&[hit(0, (5.0, false, false), 12.0, 300.0, (50.0, 20.0))]);
    assert_eq!(feel.advance(FRAME), 0.0);
    let stopped = 0.5 - fight_time(&mut feel, 0.5);
    assert!(stopped > 0.04, "stopped {stopped}");
    assert!(stopped < 0.13, "stopped {stopped}");
    assert_eq!(feel.advance(FRAME), FRAME);
}

#[test]
fn a_harder_blow_stops_the_fight_longer_and_shakes_the_screen_harder() {
    let lost = |power: f64, launch: bool, crit: bool, share: f64| {
        let mut feel = Feel::new();
        feel.react(&[hit(0, (power, launch, crit), share, 0.0, (0.0, 0.0))]);
        let trauma = feel.trauma();
        (1.0 - fight_time(&mut feel, 1.0), trauma)
    };
    let (light_stop, light_shake) = lost(5.0, false, false, 4.0);
    let (heavy_stop, heavy_shake) = lost(20.0, true, false, 20.0);
    assert!(heavy_stop > light_stop);
    assert!(heavy_shake > light_shake);
    // never past the cap, even a crit that throws
    assert!(lost(20.0, true, true, 30.0).0 <= 13.0 / 60.0 + 1e-9);
}

#[test]
fn a_knockout_plays_out_in_slow_motion_then_the_fight_runs_at_full_speed() {
    let mut feel = Feel::new();
    feel.react(&[Cue::Ko]);
    assert!(fight_time(&mut feel, 1.0) < 0.5);
    fight_time(&mut feel, 1.0);
    assert_eq!(feel.advance(FRAME), FRAME);
}

#[test]
fn the_screen_settles_after_a_shake() {
    let mut feel = Feel::new();
    feel.react(&[hit(0, (0.0, false, false), 25.0, 25.0, (0.0, 0.0))]);
    let mut moved = false;
    for _ in 0..10 {
        let (x, y) = feel.shake(1.0);
        moved |= x != 0 || y != 0;
        feel.advance(FRAME);
    }
    assert!(moved);
    fight_time(&mut feel, 1.0);
    assert_eq!(feel.shake(1.0), (0, 0));
}

#[test]
fn a_shake_sways_from_frame_to_frame_instead_of_jumping() {
    let mut feel = Feel::new();
    feel.react(&[hit(0, (20.0, true, true), 25.0, 25.0, (0.0, 0.0))]);
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
fn the_player_can_turn_the_shake_down_or_off() {
    let mut feel = Feel::new();
    feel.react(&[hit(0, (20.0, true, true), 25.0, 25.0, (0.0, 0.0))]);
    let mut full = 0;
    let mut half = 0;
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
}

#[test]
fn damage_floats_up_from_the_blow_and_fades_away() {
    let mut feel = Feel::new();
    feel.react(&[hit(1, (0.0, false, false), 9.4, 940.0, (80.0, 20.0))]);
    let first = feel.numbers();
    assert_eq!(first.len(), 1);
    let first = &first[0];
    assert_eq!(
        (first.x, first.text.as_str(), first.by, first.kind),
        (80.0, "940", Some(1), NumberKind::Hit)
    );
    fight_time(&mut feel, 0.4);
    let later = feel.numbers();
    assert_eq!(later.len(), 1);
    let later = &later[0];
    assert!(later.y > first.y);
    fight_time(&mut feel, 1.0);
    assert_eq!(feel.numbers(), vec![]);
}

mod effect_cues {
    use super::*;
    use pixfx_core::cue::{Cue as Fx, TimedCue};

    fn at(cue: Fx) -> TimedCue {
        TimedCue {
            t: 0.0,
            cue,
            pan: None,
        }
    }

    #[test]
    fn take_shake_caps_trauma_at_one() {
        let mut feel = Feel::new();
        feel.take(&[at(Fx::Shake { trauma: 0.7 }), at(Fx::Shake { trauma: 0.7 })]);
        assert_eq!(feel.trauma(), 1.0);
    }

    #[test]
    fn fight_and_effect_cues_share_one_freeze() {
        let mut feel = Feel::new();
        feel.react(&[hit(0, (5.0, false, false), 10.0, 5.0, (50.0, 10.0))]);
        feel.take(&[at(Fx::Hitstop { dur: 0.2 })]);
        // one freeze of min(0.2, cap / 60), not the two added together
        let want = 0.2_f64.min(feel.spec().stop.cap / 60.0);
        let mut frozen = 0.0;
        while feel.advance(FRAME) == 0.0 {
            frozen += FRAME;
            assert!(frozen < 1.0);
        }
        assert!((frozen - want).abs() < FRAME, "froze {frozen}, want {want}");
    }

    #[test]
    fn ripple_and_flash_age_during_hitstop() {
        let mut feel = Feel::new();
        feel.take(&[
            at(Fx::Ripple {
                x: 5.0,
                y: 5.0,
                r0: 0.0,
                r1: 20.0,
                dur: 0.05,
                amp: 1.0,
            }),
            at(Fx::Flash { k: 0.5, dur: 0.1 }),
            at(Fx::Hitstop { dur: 0.2 }),
        ]);
        assert_eq!(feel.ripples().len(), 1);
        assert_eq!(feel.advance(FRAME), 0.0);
        assert!(feel.ripples()[0].age > 0.0);
        assert!(feel.flash() < 0.5);
        for _ in 0..4 {
            assert_eq!(feel.advance(FRAME), 0.0);
        }
        assert!(feel.ripples().is_empty());
    }

    #[test]
    fn flash_keeps_the_strongest() {
        let mut feel = Feel::new();
        feel.take(&[
            at(Fx::Flash { k: 0.6, dur: 0.1 }),
            at(Fx::Flash { k: 0.3, dur: 0.1 }),
        ]);
        assert_eq!(feel.flash(), 0.6);
    }

    #[test]
    fn sound_is_ignored() {
        let mut feel = Feel::new();
        feel.take(&[at(Fx::Sound {
            name: "x",
            gain: 1.0,
            pitch: 1.0,
            len: None,
            x: None,
        })]);
        assert_eq!(feel.trauma(), 0.0);
        assert_eq!(feel.advance(FRAME), FRAME);
    }
}
