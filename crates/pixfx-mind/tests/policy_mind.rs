//! The policy on a clock: it thinks about eight times a second, sees the foe late, and holds still in hit-stop.

mod common;

use common::*;
use pixfx_mind::policy::{Net, OBS, PolicyMind};
use pixfx_mind::*;

const DT: f64 = 1.0 / 60.0;

thread_local! {
    static NET: Net = Net::seeded(5);
}

fn mind(seed: u32) -> PolicyMind {
    PolicyMind::new(kit(), [1.0, 0.0, 0.0, 1.0, 0.0], 0.16, 0.12, seed)
}

fn think(m: &mut PolicyMind, s: &Sense, dt: f64) -> Option<Choice> {
    NET.with(|n| m.think(n, s, dt))
}

fn at(t: f64) -> Sense {
    let mut s = sense(t, me(40.0), foe(150.0));
    s.slots = vec![Some(0), None, Some(2), Some(1), Some(3), None, Some(6)];
    s
}

/// Every choice over `seconds`, with the tick it came on.
fn run(m: &mut PolicyMind, seconds: f64) -> Vec<(usize, Choice)> {
    let mut out = vec![];
    for i in 0..(seconds / DT) as usize {
        if let Some(c) = think(m, &at(i as f64 * DT), DT) {
            out.push((i, c));
        }
    }
    out
}

#[test]
fn it_thinks_about_eight_times_a_second() {
    let n = run(&mut mind(1), 10.0).len();
    assert!((70..=100).contains(&n), "{n} choices in 10 s");
}

#[test]
fn the_same_seed_makes_the_same_choices() {
    assert_eq!(run(&mut mind(4), 5.0), run(&mut mind(4), 5.0));
    assert_ne!(run(&mut mind(4), 5.0), run(&mut mind(5), 5.0));
}

#[test]
fn hit_stop_holds_its_clock_and_dice_still() {
    let plain = run(&mut mind(2), 3.0);
    let mut m = mind(2);
    let mut stopped = vec![];
    for i in 0..(3.0 / DT) as usize {
        let t = i as f64 * DT;
        if let Some(c) = think(&mut m, &at(t), DT) {
            stopped.push((i, c));
        }
        // a few frames of hit-stop after each tick: the same time, no time passing
        for _ in 0..3 {
            assert_eq!(think(&mut m, &at(t), 0.0), None, "no thought in hit-stop");
        }
    }
    assert_eq!(plain, stopped);
}

#[test]
fn hold_keeps_it_quiet_for_that_long() {
    let mut m = mind(3);
    m.hold(1.0);
    let choices = run(&mut m, 2.0);
    let first = choices.first().map_or(usize::MAX, |(i, _)| *i);
    // decided once the hold ends (tick 60), acted on the tick after
    assert!((60..=62).contains(&first), "first choice at tick {first}");
}

#[test]
fn it_sees_a_windup_late_by_its_reaction() {
    // two minds with the same dice: one's foe starts a blade at 1 s, the other's stays idle
    let mut late = mind(6);
    let mut calm = mind(6);
    let mut seen: Vec<(f64, [f32; OBS], [f32; OBS])> = vec![];
    for i in 0..(2.0 / DT) as usize {
        let t = i as f64 * DT;
        let mut s = at(t);
        if t >= 1.0 {
            s.foe.doing = blade_windup(t - 1.0);
        }
        // nothing is queued (no act is answered), so only the decisions show
        let _ = late.begin(&s, DT);
        let a = late.due(&s).map(|d| d.obs);
        let _ = calm.begin(&at(t), DT);
        let b = calm.due(&at(t)).map(|d| d.obs);
        if let (Some(a), Some(b)) = (a, b) {
            seen.push((t, a, b));
        }
    }
    for (t, a, b) in &seen {
        if *t >= 1.0 && *t < 1.0 + 0.16 - 1e-9 {
            assert_eq!(a, b, "at {t} the blade is not seen yet");
        }
        if *t >= 1.0 + 0.17 && *t < 1.19 {
            assert_ne!(a, b, "at {t} the blade is seen");
        }
    }
    assert!(
        seen.iter().any(|(t, a, b)| *t >= 1.17 && a != b),
        "a decision saw the blade"
    );
}

#[test]
fn a_reset_forgets_what_it_saw_and_its_clock() {
    let mut m = mind(8);
    run(&mut m, 1.0);
    m.hold(5.0);
    m.reset();
    // nothing it meant to do survives, and it decides at once: its act comes the tick after
    assert_eq!(think(&mut m, &at(10.0), DT), None);
    assert!(think(&mut m, &at(10.0 + DT), DT).is_some());
}
