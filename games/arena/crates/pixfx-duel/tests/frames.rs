//! Frame data (`data::FRAMES`): a blow that catches a fighter gathering a cast is a COUNTER, one that catches it
//! recovering is a PUNISH, an unsafe skill whose blow is guarded recovers longer, a fighter casting is not guarding,
//! and an ultimate's opening cannot be hit. None of it draws a die of its own.

mod common;

use common::arena;
use pixfx_draw::math::{js_sign, or_one};
use pixfx_duel::act::{advance, begin, hurt};
use pixfx_duel::data::{FRAMES, spell};
use pixfx_duel::fighter::{Act, Phase, State, Told};
use pixfx_duel::out::{How, Out};
use pixfx_fighter::body::BodySpec;
use pixfx_mind::Outcome;

const DT: f64 = 1.0 / 60.0;

/// A duel where nobody dodges or crits, with every effect in the registry.
fn still(seed: u32) -> State {
    let mut s = arena(seed);
    s.effects = common::all_effects();
    for f in &mut s.fighters {
        f.stats.dodge = 0.0;
        f.stats.crit = 0.0;
    }
    s
}

fn act(name: &'static str, phase: Phase, t: f64) -> Act {
    Act {
        spell: name,
        phase,
        t,
        target: None,
        power: 1.0,
        stacks: 1.0,
        pierce: false,
        circle: None,
        blocked: false,
    }
}

/// Fighter `by` lands `name` on the other from 20 px: its cast reaches the end of its windup this tick. The one
/// struck faces it.
fn strike(s: &mut State, by: usize, name: &'static str) -> Vec<Out> {
    let f = 1 - by;
    let windup = spell(name).map_or(0.0, |sp| sp.windup);
    let fx = s.fighters[f].body.x;
    let bx = s.fighters[by].body.x;
    s.fighters[by].act = Some(Act {
        target: Some(fx),
        ..act(name, Phase::Windup, windup)
    });
    s.fighters[by].body.x = fx - 20.0 * js_sign(or_one(fx - bx));
    s.fighters[by].body.face = js_sign(fx - s.fighters[by].body.x);
    s.fighters[f].body.face = -s.fighters[by].body.face;
    s.out.clear();
    advance(s, by, DT);
    s.out.clone()
}

/// The `Hit` of `name` among `out`: its power and how it caught its target.
fn hit(out: &[Out], name: &str) -> Option<(f64, How)> {
    out.iter().find_map(|o| match o {
        Out::Hit {
            spell, power, how, ..
        } if *spell == name => Some((*power, *how)),
        _ => None,
    })
}

/// A slash on Frost standing idle, and the stagger it leaves.
fn plain_slash(seed: u32) -> (f64, How, f64) {
    let mut s = still(seed);
    let out = strike(&mut s, 0, "slash");
    let (power, how) = hit(&out, "slash").unwrap_or((0.0, How::Plain));
    (power, how, s.fighters[1].body.stun)
}

fn stagger(power: f64) -> f64 {
    let spec = BodySpec::default();
    spec.hitstun_base + power * spec.hitstun_per
}

#[test]
fn a_fighter_that_guards_then_casts_is_not_guarding_while_it_gathers() {
    let mut s = still(1);
    s.fighters[1].guard = 5.0;
    begin(&mut s, 1, "lotus", None);
    assert_eq!(s.fighters[1].guard, 0.0);
    // a slash is always turned aside by a guard: here it lands
    let out = strike(&mut s, 0, "slash");
    assert!(hit(&out, "slash").is_some(), "{out:?}");
    assert!(!out.iter().any(|o| matches!(o, Out::Clash { .. })));
}

#[test]
fn a_blow_on_a_fighter_standing_idle_is_plain() {
    let (power, how, stun) = plain_slash(3);
    assert_eq!(how, How::Plain);
    assert_eq!(power, 5.0);
    assert_eq!(stun, stagger(5.0));
}

#[test]
fn a_blow_on_a_fighter_gathering_a_cast_is_a_counter_harder_and_staggering_longer() {
    let mut s = still(3);
    s.fighters[1].act = Some(act("lotus", Phase::Windup, 0.1));
    let out = strike(&mut s, 0, "slash");
    let (power, how) = hit(&out, "slash").unwrap_or((0.0, How::Plain));
    assert_eq!(how, How::Counter);
    assert_eq!(power, 5.0 * FRAMES.counter.power);
    assert_eq!(
        s.fighters[1].body.stun,
        stagger(power) + FRAMES.counter.stagger
    );
    // and the cast it was gathering is lost
    assert!(s.fighters[1].act.is_none());
}

#[test]
fn a_blow_on_a_fighter_recovering_is_a_punish_harder_and_staggering_longer() {
    let mut s = still(3);
    s.fighters[1].act = Some(act("lotus", Phase::Recover, 0.1));
    let out = strike(&mut s, 0, "slash");
    let (power, how) = hit(&out, "slash").unwrap_or((0.0, How::Plain));
    assert_eq!(how, How::Punish);
    assert_eq!(power, 5.0 * FRAMES.punish.power);
    assert_eq!(
        s.fighters[1].body.stun,
        stagger(power) + FRAMES.punish.stagger
    );
    assert!(s.told.iter().any(|(f, t)| *f == 1
        && *t
            == Told::Outcome(Outcome::Punished {
                skill: "lotus".to_string()
            })));
}

#[test]
fn a_counter_draws_exactly_the_dice_a_plain_hit_draws() {
    for seed in 1..=20 {
        let mut plain = still(seed);
        let mut counter = still(seed);
        let mut punish = still(seed);
        counter.fighters[1].act = Some(act("lotus", Phase::Windup, 0.1));
        punish.fighters[1].act = Some(act("lotus", Phase::Recover, 0.1));
        for s in [&mut plain, &mut counter, &mut punish] {
            s.fighters[1].stats.crit = 0.2;
            s.fighters[0].stats.crit = 0.2;
            let out = strike(s, 0, "slash");
            assert!(hit(&out, "slash").is_some(), "seed {seed}");
        }
        assert_eq!(plain.rng, counter.rng, "seed {seed}");
        assert_eq!(plain.rng, punish.rng, "seed {seed}");
    }
}

#[test]
fn a_fields_strike_is_never_a_counter_or_a_punish() {
    for (name, phase) in [
        ("flurry", Phase::Windup),
        ("void", Phase::Recover),
        ("frostfire", Phase::Windup),
    ] {
        let mut s = still(5);
        s.fighters[0].act = Some(act("dive", phase, 0.1));
        s.out.clear();
        assert!(hurt(&mut s, 0, 10.0, 1.0, name, 1.0));
        assert_eq!(hit(&s.out, name), Some((10.0, How::Plain)), "{name}");
    }
}

#[test]
fn a_golden_bells_answer_counters_the_striker_still_in_its_windup() {
    let mut s = still(2);
    s.fighters[1].barrier = 2.0;
    s.fighters[1].barrier_spell = Some("bell");
    let out = strike(&mut s, 0, "slash");
    let (power, how) = hit(&out, "bell").unwrap_or((0.0, How::Plain));
    assert_eq!(how, How::Counter);
    assert_eq!(power, 9.0 * FRAMES.counter.power);
}

#[test]
fn an_ultimates_opening_cannot_be_hit_and_shows_as_a_dodge() {
    let mut s = still(4);
    s.fighters[1].act = Some(act("prison", Phase::Windup, 0.29));
    let rng = s.rng.clone();
    let hp = s.fighters[1].hp;
    let out = strike(&mut s, 0, "slash");
    assert!(hit(&out, "slash").is_none(), "{out:?}");
    let (x, y) = (s.fighters[1].body.x, s.fighters[1].chest());
    assert!(out.contains(&Out::Dodge { x, y }), "{out:?}");
    assert_eq!(s.fighters[1].hp, hp);
    assert_eq!(s.rng, rng, "the blow that passes through draws no dice");
    assert_eq!(s.fighters[1].act.map(|a| a.spell), Some("prison"));
}

#[test]
fn past_its_opening_an_ultimate_gathering_is_countered() {
    let mut s = still(4);
    s.fighters[1].act = Some(act("prison", Phase::Windup, 0.31));
    let out = strike(&mut s, 0, "slash");
    assert_eq!(
        hit(&out, "slash"),
        Some((5.0 * FRAMES.counter.power, How::Counter))
    );
    assert!(s.fighters[1].act.is_none());
}

#[test]
fn close_heavies_dive_dash_heal_and_ultimates_recover_slowly_and_pokes_stay_safe() {
    let recover = |name: &str| spell(name).map_or(0.0, |sp| sp.recover);
    for (name, want) in [
        ("slash", 0.32),
        ("dive", 0.5),
        ("step", 0.45),
        ("palm", 0.5),
        ("burst", 0.5),
        ("lotus", 0.45),
        ("jet", 0.35),
        ("heal", 0.45),
        ("tribulation", 0.8),
        ("prison", 0.8),
        ("bolt", 0.22),
    ] {
        assert_eq!(recover(name), want, "{name}");
    }
}

/// Fighter 0 casts `name` on a guarding Frost until a seed turns it aside; its act, just blocked.
fn blocked(name: &'static str) -> Option<State> {
    (1..=40).find_map(|seed| {
        let mut s = still(seed);
        s.fighters[1].guard = 5.0;
        let out = strike(&mut s, 0, name);
        (hit(&out, name).is_none() && out.iter().any(|o| matches!(o, Out::Clash { .. })))
            .then_some(s)
    })
}

/// How long fighter 0 stays in its recovery from here, in ticks.
fn recovery_ticks(s: &mut State) -> usize {
    let mut n = 0;
    while s.fighters[0].act.is_some() && n < 600 {
        advance(s, 0, DT);
        n += 1;
    }
    n
}

#[test]
fn an_unsafe_blow_turned_aside_leaves_its_striker_open_longer() {
    for name in ["palm", "step", "dive"] {
        let s = blocked(name);
        assert!(s.is_some(), "no seed blocks {name}");
        let Some(mut s) = s else { return };
        let a = s.fighters[0].act;
        assert_eq!(
            a.map(|a| (a.phase, a.blocked)),
            Some((Phase::Recover, true))
        );
        let want = spell(name).map_or(0.0, |sp| sp.recover) + FRAMES.blocked_recover;
        let ticks = recovery_ticks(&mut s) as f64;
        assert!(
            (ticks * DT - want).abs() <= DT,
            "{name}: {ticks} ticks, want {want} s"
        );
    }
}

#[test]
fn a_safe_blow_turned_aside_recovers_as_ever() {
    let s = blocked("slash");
    assert!(s.is_some(), "no seed blocks slash");
    let Some(mut s) = s else { return };
    let want = spell("slash").map_or(0.0, |sp| sp.recover);
    let ticks = recovery_ticks(&mut s) as f64;
    assert!(
        (ticks * DT - want).abs() <= DT,
        "{ticks} ticks, want {want} s"
    );
}
