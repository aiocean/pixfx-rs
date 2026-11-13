//! The screen's answer to the fight, driven the way a game drives it: what the fight did, then real time passing
//! (the site's `duel-feel.test.js`).

use pixfx_fighter::feel::{Cue, Feel, NumberKind};

const FRAME: f64 = 1.0 / 60.0;

/// Seconds of fight that `seconds` of real time buys.
fn fight_time(feel: &mut Feel, seconds: f64) -> f64 {
    let mut t = 0.0;
    let frames = pixfx_draw::math::js_round(seconds / FRAME) as u32;
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
    fn fight_and_effect_cues_share_one_freeze() {
        let mut feel = Feel::new();
        feel.react(&[hit(0, (5.0, false, false), 10.0, 5.0, (50.0, 10.0))]);
        feel.take(&[at(Fx::Hitstop { dur: 0.2 })]);
        // one freeze of min(0.2, cap / 60), not the two added together
        let want = 0.2_f64.min(feel.spec().screen.freeze_cap);
        let mut frozen = 0.0;
        while feel.advance(FRAME) == 0.0 {
            frozen += FRAME;
            assert!(frozen < 1.0);
        }
        assert!((frozen - want).abs() < FRAME, "froze {frozen}, want {want}");
    }
}

#[test]
fn the_duel_shakes_by_its_own_screen_spec() {
    use pixfx_fighter::feel::FeelSpec;
    let most = FeelSpec::default().screen.shake;
    let mut worst = 0;
    for start in 0..20 {
        let mut feel = Feel::new();
        fight_time(&mut feel, f64::from(start) * 0.1);
        // the heaviest moment: a crit that takes a quarter of life, then the knockout
        feel.react(&[
            hit(0, (12.0, true, true), 25.0, 900.0, (50.0, 20.0)),
            Cue::Ko,
        ]);
        for _ in 0..90 {
            let (x, y) = feel.shake(1.0);
            worst = worst.max(x.abs().max(y.abs()));
            feel.advance(FRAME);
        }
    }
    assert!(
        f64::from(worst) <= most,
        "shook {worst} px, the duel's spec allows {most}"
    );
    assert!(worst >= 3, "a knockout reads clearly: {worst} px");
}
