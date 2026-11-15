//! Statuses that stack, grit and damage, ported from `site/tests/status.test.js` and the damage cases of
//! `site/tests/guard.test.js`. The cases that need a whole step of the duel (stacks fading, grit falling, payoffs,
//! cleanses, the orb) come with `Duel::step`.

use pixfx_duel::data::{Blow, School, shock_interval, stats};
use pixfx_duel::fighter::{Fighter, State};
use pixfx_duel::status::{ControlKind, add_chill, add_shock, control, damage_of, shocking, take};
use pixfx_draw::math::js_round;
use serde_json::Value;

/// A duel's state with two fresh fighters and no effects: only the rules under test move.
fn still() -> State {
    let fighters = [Fighter::new(0, School::Thunder, Vec::new(), Vec::new()), Fighter::new(1, School::Frost, Vec::new(), Vec::new())];
    State::new(1, 0.0, 220.0, 150.0, 96.0, Vec::new(), fighters)
}

#[test]
fn the_third_stack_freezes_and_clears_the_stacks() {
    let mut s = still();
    add_chill(&mut s, 0, 1.0);
    add_chill(&mut s, 0, 1.0);
    assert_eq!(s.fighters[0].chill, 2.0);
    assert_eq!(s.fighters[0].body.frozen, 0.0);
    add_chill(&mut s, 0, 1.0);
    assert_eq!(s.fighters[0].chill, 0.0);
    assert!(s.fighters[0].body.frozen > 0.0);
}

#[test]
fn ticks_come_faster_with_more_stacks() {
    assert!((shock_interval(1.0) - 0.6).abs() < 1e-9);
    assert!((shock_interval(5.0) - 0.6 * 0.854_f64 * 0.854 * 0.854 * 0.854).abs() < 1e-9);
}

#[test]
fn each_tick_spends_a_stack_costs_3_percent_of_the_casters_attack_and_holds_the_target() {
    let mut s = still();
    add_shock(&mut s, 1, 0, 3.0);
    assert_eq!(s.fighters[1].shock.map(|k| k.stacks), Some(3.0));
    let hp = s.fighters[1].hp;
    let ticks = js_round((shock_interval(3.0) + 1.0 / 60.0) * 60.0) as usize;
    for _ in 0..ticks {
        shocking(&mut s, 1, 1.0 / 60.0);
    }
    assert_eq!(s.fighters[1].shock.map(|k| k.stacks), Some(2.0));
    assert!(s.fighters[1].body.shocked > 0.0);
    let lost = (hp - s.fighters[1].hp) / 100.0 * stats(School::Frost).hp;
    assert!((lost - js_round(stats(School::Thunder).atk * 0.03)).abs() < 0.5, "{lost}");
    for _ in 0..180 {
        shocking(&mut s, 1, 1.0 / 60.0);
    }
    assert!(s.fighters[1].shock.is_none());
}

#[test]
fn stacks_stop_at_five() {
    let mut s = still();
    add_shock(&mut s, 1, 0, 9.0);
    assert_eq!(s.fighters[1].shock.map(|k| k.stacks), Some(5.0));
}

#[test]
fn a_second_freeze_at_once_is_half_as_long() {
    let mut s = still();
    assert!((control(&mut s, 0, ControlKind::Freeze, 0.9) - 0.9).abs() < 1e-9);
    s.fighters[0].body.frozen = 0.0;
    assert!((control(&mut s, 0, ControlKind::Freeze, 0.9) - 0.45).abs() < 1e-9);
}

#[test]
fn with_grit_2_a_freeze_is_only_a_chill_stack() {
    let mut s = still();
    s.fighters[0].grit = 2.0;
    assert_eq!(control(&mut s, 0, ControlKind::Freeze, 0.9), 0.0);
    assert_eq!(s.fighters[0].body.frozen, 0.0);
    assert_eq!(s.fighters[0].chill, 1.0);
}

#[test]
fn a_stun_goes_through_grit_too() {
    let mut s = still();
    control(&mut s, 0, ControlKind::Stun, 0.6);
    assert!((control(&mut s, 0, ControlKind::Stun, 0.6) - 0.3).abs() < 1e-9);
}

#[test]
fn a_knocked_out_fighter_takes_no_status() {
    let mut s = still();
    s.fighters[0].hp = 0.0;
    assert_eq!(control(&mut s, 0, ControlKind::Freeze, 0.9), 0.0);
    add_chill(&mut s, 0, 1.0);
    add_shock(&mut s, 0, 1, 1.0);
    assert_eq!(s.fighters[0].chill, 0.0);
    assert!(s.fighters[0].shock.is_none());
}

#[test]
fn an_unyielding_fighter_stands_up_once_a_round_from_a_killing_blow() {
    let mut s = still();
    let full = stats(School::Thunder).hp;
    take(&mut s, 0, full * 2.0);
    assert!((s.fighters[0].hp - 100.0 / full).abs() < 1e-9);
    assert!(s.out.iter().any(|o| matches!(o, pixfx_duel::out::Out::Unyielding { by: 0, .. })));
    take(&mut s, 0, full * 2.0);
    assert_eq!(s.fighters[0].hp, 0.0);
}

fn school(name: &str) -> School {
    if name == "frost" { School::Frost } else { School::Thunder }
}

#[test]
fn damage_matches_js_table() {
    let f: Value = serde_json::from_str(include_str!("fixtures/js_numbers.json")).unwrap_or(Value::Null);
    let rows = f["damage"].as_array().cloned().unwrap_or_default();
    assert_eq!(rows.len(), 12);
    for row in rows {
        let num = |i: usize| row[i].as_f64().unwrap_or(f64::NAN);
        let mut by = Fighter::new(0, school(row[0].as_str().unwrap_or("")), Vec::new(), Vec::new());
        by.fury = num(1);
        let mut target = Fighter::new(1, school(row[2].as_str().unwrap_or("")), Vec::new(), Vec::new());
        target.def_down = num(3);
        let blow = if row[5] == "physical" { Blow::Physical } else { Blow::Magic };
        let crit = row[6].as_bool().unwrap_or(false);
        assert_eq!(damage_of(&by, &target, num(4), blow, crit, num(7)), num(8), "{row}");
    }
}

#[test]
fn damage_stays_within_1_5_percent_of_its_middle() {
    let by = Fighter::new(0, School::Thunder, Vec::new(), Vec::new());
    let f = Fighter::new(1, School::Frost, Vec::new(), Vec::new());
    let mid = damage_of(&by, &f, 20.0, Blow::Magic, false, 0.5);
    for i in 0..20 {
        let roll = f64::from(i) / 20.0;
        assert!((damage_of(&by, &f, 20.0, Blow::Magic, false, roll) - mid).abs() / mid <= 0.016);
    }
    assert!(damage_of(&by, &f, 20.0, Blow::Magic, false, 0.0) < damage_of(&by, &f, 20.0, Blow::Magic, false, 0.999));
}
