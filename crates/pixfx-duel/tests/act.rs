//! Casting, blows and shots, ported from `site/tests/guard.test.js` and `site/tests/fx.test.js`: the cases a cast's
//! release decides. A strike here is the cast's last windup tick (`advance`), which is all `strike` in the JS tests
//! waits for; the whole-fight cases come with `Duel::step`.

mod common;

use common::arena;
use pixfx_draw::math::{js_sign, or_one};
use pixfx_duel::act::{advance, begin, clash, knock_out};
use pixfx_duel::data::{KNOCKBACK, spell};
use pixfx_duel::fighter::{Act, Phase, State};
use pixfx_duel::out::Out;
use pixfx_duel::shot::{ShotKind, shot};
use pixfx_duel::status::{add_shock, shocking};

/// A duel where nobody dodges or crits, as the JS tests' `still` sets it, with every effect in the registry.
fn still(seed: u32) -> State {
    let mut s = arena(seed);
    s.effects = common::all_effects();
    for f in &mut s.fighters {
        f.stats.dodge = 0.0;
        f.stats.crit = 0.0;
    }
    s
}

/// Fighter `by` lands `name` on the other from 20 px: its cast reaches the end of its windup this tick.
fn strike(s: &mut State, by: usize, name: &'static str) -> Vec<Out> {
    let f = 1 - by;
    let windup = spell(name).map_or(0.0, |sp| sp.windup);
    let fx = s.fighters[f].body.x;
    let bx = s.fighters[by].body.x;
    s.fighters[by].act = Some(Act {
        spell: name,
        phase: Phase::Windup,
        t: windup,
        target: Some(fx),
        power: 1.0,
        stacks: 1.0,
        pierce: false,
        circle: None,
    });
    s.fighters[by].body.x = fx - 20.0 * js_sign(or_one(fx - bx));
    s.fighters[by].body.face = js_sign(fx - s.fighters[by].body.x);
    s.out.clear();
    advance(s, by, 1.0 / 60.0);
    s.out.clone()
}

fn hits(out: &[Out], name: &str) -> bool {
    out.iter()
        .any(|o| matches!(o, Out::Hit { spell, .. } if *spell == name))
}

#[test]
fn a_failed_block_lets_the_whole_blow_through_and_the_guards_qi_is_paid_anyway() {
    let (mut through, mut held) = (0, 0);
    for seed in 1..=40 {
        let mut s = still(seed);
        s.fighters[1].body.face = js_sign(s.fighters[0].body.x - s.fighters[1].body.x);
        s.fighters[1].guard = 5.0;
        let qi = s.fighters[1].qi;
        let out = strike(&mut s, 0, "palm");
        assert!(s.fighters[1].qi < qi, "seed {seed}");
        if hits(&out, "palm") {
            through += 1
        } else {
            held += 1
        }
    }
    // palm (power 20): about half get through
    assert!(through > 8 && held > 8, "{through} {held}");
}

#[test]
fn a_guarded_blows_chip_is_cut_by_the_targets_defence_like_any_blow() {
    let chip = |pdef: f64| {
        let mut s = still(6);
        s.fighters[1].stats.pdef = pdef;
        s.fighters[1].body.face = js_sign(s.fighters[0].body.x - s.fighters[1].body.x);
        s.fighters[1].guard = 5.0;
        let hp = s.fighters[1].hp;
        strike(&mut s, 0, "slash");
        hp - s.fighters[1].hp
    };
    let soft = chip(20.0);
    let hard = chip(400.0);
    assert!(soft > 0.0);
    assert!(hard < soft * 0.7);
}

#[test]
fn each_charge_erases_one_whole_blow() {
    let mut s = still(2);
    s.fighters[1].charges = 2.0;
    s.fighters[1].shield_t = 6.0;
    let hp = s.fighters[1].hp;
    strike(&mut s, 0, "slash");
    strike(&mut s, 0, "slash");
    assert_eq!(s.fighters[1].hp, hp);
    assert_eq!(s.fighters[1].charges, 0.0);
    strike(&mut s, 0, "slash");
    assert!(s.fighters[1].hp < hp);
}

#[test]
fn a_shock_tick_passes_the_ward() {
    let mut s = still(2);
    s.fighters[1].charges = 3.0;
    s.fighters[1].shield_t = 6.0;
    add_shock(&mut s, 1, 0, 1.0);
    let hp = s.fighters[1].hp;
    for _ in 0..60 {
        shocking(&mut s, 1, 1.0 / 60.0);
    }
    assert!(s.fighters[1].hp < hp);
    assert_eq!(s.fighters[1].charges, 3.0);
}

#[test]
fn a_ward_holds_1_to_3_charges() {
    let mut seen = Vec::new();
    for seed in 1..=30 {
        let mut s = still(seed);
        let x = s.fighters[1].body.x;
        let windup = spell("ward").map_or(0.0, |sp| sp.windup);
        s.fighters[1].act = Some(Act {
            spell: "ward",
            phase: Phase::Windup,
            t: windup,
            target: Some(x),
            power: 1.0,
            stacks: 1.0,
            pierce: false,
            circle: None,
        });
        advance(&mut s, 1, 1.0 / 60.0);
        let c = s.fighters[1].charges;
        if !seen.contains(&c) {
            seen.push(c);
        }
    }
    seen.sort_by(f64::total_cmp);
    assert_eq!(seen, [1.0, 2.0, 3.0]);
}

#[test]
fn clash_at_equal_x_pushes_as_js() {
    let mut s = still(1);
    let x = s.fighters[0].body.x;
    s.fighters[1].body.x = x;
    let (v0, v1) = (s.fighters[0].body.vx, s.fighters[1].body.vx);
    s.out.clear();
    clash(&mut s, x, 100.0, &[0, 1]);
    // JS: Math.sign(f.x - x || 1) is 1 when they stand on the spot
    assert_eq!(s.fighters[0].body.vx, v0 + KNOCKBACK);
    assert_eq!(s.fighters[1].body.vx, v1 + KNOCKBACK);
    assert!(matches!(s.out.first(), Some(Out::Clash { y, .. }) if *y == s.ground - 100.0));
}

#[test]
fn a_cast_still_gathering_in_its_rune_circle_when_the_foe_falls_loses_its_circle() {
    let mut s = still(3);
    let target = s.fighters[0].body.x;
    begin(&mut s, 1, "collapse", Some(target));
    let circle = s.fighters[1].act.and_then(|a| a.circle);
    assert!(circle.is_some());
    let circle = circle.unwrap_or(0);
    s.out.clear();
    knock_out(&mut s, 0);
    assert!(s.out.contains(&Out::Stop { key: circle }));
    assert!(s.fighters[1].act.is_none());
    assert_eq!(s.winner, Some(1));
    assert_eq!(s.wins, [0, 1]);
}

#[test]
fn a_bolt_flies_strikes_and_ends_its_effect() {
    let mut s = still(4);
    s.fighters[1].body.x = s.fighters[0].body.x + 60.0;
    let windup = spell("bolt").map_or(0.0, |sp| sp.windup);
    s.fighters[0].body.face = 1.0;
    s.fighters[0].act = Some(Act {
        spell: "bolt",
        phase: Phase::Windup,
        t: windup,
        target: None,
        power: 1.0,
        stacks: 1.0,
        pierce: false,
        circle: None,
    });
    advance(&mut s, 0, 1.0 / 60.0);
    assert_eq!(s.shots.len(), 1);
    let key = s.out.iter().find_map(|o| match o {
        Out::Play {
            key,
            fx: "projectiles/laser-bolt",
            ..
        } => Some(*key),
        _ => None,
    });
    assert!(key.is_some(), "no bolt played");
    let key = key.unwrap_or(0);
    let mut landed = false;
    for _ in 0..60 {
        let ids: Vec<u32> = s.shots.iter().map(|x| x.id).collect();
        for id in ids {
            shot(&mut s, id, 1.0 / 60.0);
        }
        landed |= hits(&s.out, "bolt");
        if s.shots.is_empty() {
            break;
        }
    }
    assert!(landed);
    assert!(s.out.contains(&Out::Stop { key }));
    assert_eq!(s.fighters[1].shock.map(|k| k.stacks), Some(1.0));
}

#[test]
fn two_bolts_meeting_head_on_are_both_spent() {
    let mut s = still(5);
    let (x0, x1) = (60.0, 160.0);
    s.fighters[0].body.x = x0;
    s.fighters[1].body.x = x1;
    s.fighters[0].body.face = 1.0;
    s.fighters[1].body.face = -1.0;
    for (me, name) in [(0, "bolt"), (1, "shard")] {
        let windup = spell(name).map_or(0.0, |sp| sp.windup);
        s.fighters[me].act = Some(Act {
            spell: name,
            phase: Phase::Windup,
            t: windup,
            target: None,
            power: 1.0,
            stacks: 1.0,
            pierce: false,
            circle: None,
        });
        advance(&mut s, me, 1.0 / 60.0);
    }
    assert_eq!(s.shots.len(), 2);
    s.out.clear();
    for _ in 0..40 {
        let ids: Vec<u32> = s.shots.iter().map(|x| x.id).collect();
        for id in ids {
            shot(&mut s, id, 1.0 / 60.0);
        }
        if s.shots.is_empty() {
            break;
        }
    }
    assert!(s.shots.is_empty());
    assert!(s.out.iter().any(|o| matches!(o, Out::Clash { .. })));
    assert!(!s.out.iter().any(|o| matches!(o, Out::Hit { .. })));
    assert_eq!(
        s.out
            .iter()
            .filter(|o| matches!(o, Out::Stop { .. }))
            .count(),
        2
    );
}

#[test]
fn a_bolt_spent_in_a_clash_neither_lands_nor_takes_another_shot_with_it() {
    let mut s = still(5);
    s.fighters[0].body.x = 60.0;
    s.fighters[1].body.x = 160.0;
    s.fighters[0].body.face = 1.0;
    s.fighters[1].body.face = -1.0;
    for (me, name) in [(0, "bolt"), (1, "shard")] {
        let windup = spell(name).map_or(0.0, |sp| sp.windup);
        s.fighters[me].act = Some(Act {
            spell: name,
            phase: Phase::Windup,
            t: windup,
            target: None,
            power: 1.0,
            stacks: 1.0,
            pierce: false,
            circle: None,
        });
        advance(&mut s, me, 1.0 / 60.0);
    }
    assert_eq!(s.shots.len(), 2);
    // the two bolts meet right at Thunder's body, and a third shot flies last in the list, far above it all
    let y = s.fighters[0].body.spec.chest;
    s.shots[0].kind = ShotKind::Bolt {
        x: 100.0,
        y,
        vx: 60.0,
    };
    s.shots[1].kind = ShotKind::Bolt {
        x: 104.0,
        y,
        vx: -60.0,
    };
    s.fighters[0].body.x = 103.0;
    s.fighters[1].body.x = 200.0;
    let mut third = s.shots[0];
    third.id = 99;
    third.key = Some(999);
    third.kind = ShotKind::Bolt {
        x: 10.0,
        y: 500.0,
        vx: 0.0,
    };
    s.shots.push(third);
    s.out.clear();
    let ids: Vec<u32> = s.shots.iter().map(|x| x.id).collect();
    for id in ids {
        shot(&mut s, id, 1.0 / 60.0);
    }
    assert!(s.out.iter().any(|o| matches!(o, Out::Clash { .. })));
    // the spent bolt does not strike, and the third shot flies on
    assert!(
        !s.out
            .iter()
            .any(|o| matches!(o, Out::Hit { .. } | Out::Dodge { .. } | Out::Block { .. })),
        "{:?}",
        s.out
    );
    assert!(
        s.shots.iter().any(|x| x.id == 99),
        "the third shot left the list with its effect playing"
    );
}
