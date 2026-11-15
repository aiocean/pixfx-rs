//! The duel's rules and minds, driven step after step, ported from `site/tests/duel.test.js`, with the plan's
//! whole-tick and missing-effect cases.

mod common;

use common::{all_effects, config, fight, run};
use pixfx_duel::bar::skill_bar;
use pixfx_duel::data::{RAGE, school, spell};
use pixfx_duel::duel::Duel;
use pixfx_duel::fighter::Slot;
use pixfx_duel::out::Out;

const PLAN2: [&str; 16] = [
    "fire/candle", "fire/campfire", "fire/brazier", "impacts/parry", "explosions/plasma-explosion", "explosions/implosion",
    "explosions/small-explosion", "arcane/rune-circle", "lightning/ball-lightning", "projectiles/laser-bolt",
    "ice/icicle-fall", "shadow/void-orb", "holy/feather-burst", "spawn/poof", "fire/spirit-flame", "weather/fireflies",
];

fn plan2(seed: u32) -> Duel {
    Duel::with_brains(config(seed, PLAN2.iter().map(|s| s.to_string()).collect()))
}

/// Each round's wins as it ended, over `seconds` at 60 Hz.
fn rounds(duel: &mut Duel, seconds: f64) -> Vec<[u32; 2]> {
    let mut out = Vec::new();
    let mut round = duel.state.round;
    for _ in 0..(seconds * 60.0) as usize {
        duel.step(1.0 / 60.0);
        if duel.state.round != round {
            out.push(duel.state.wins);
            round = duel.state.round;
        }
    }
    out
}

#[test]
fn a_seed_replays_the_same_fight() {
    let a = run(&mut plan2(7), 20.0);
    let b = run(&mut plan2(7), 20.0);
    assert!(a.len() > 20);
    assert_eq!(a, b);
}

#[test]
fn fights_end_in_a_knockout_and_the_next_round_starts() {
    for seed in 1..=5 {
        let r = rounds(&mut plan2(seed), 120.0);
        assert!(!r.is_empty(), "seed {seed}");
        let last = r[r.len() - 1];
        assert_eq!((last[0] + last[1]) as usize, r.len());
    }
}

#[test]
fn a_heavy_blow_throws_its_target_which_lands_lies_and_gets_up() {
    let mut duel = plan2(2);
    duel.hold(0, 1e9);
    duel.hold(1, 1e9);
    for f in &mut duel.state.fighters {
        f.stats.dodge = 0.0;
    }
    duel.state.fighters[0].body.x = 100.0;
    duel.state.fighters[1].body.x = 116.0;
    duel.state.fighters[0].body.face = 1.0;
    duel.state.fighters[1].body.face = -1.0;
    let windup = spell("palm").map_or(0.0, |s| s.windup);
    duel.state.fighters[0].act = Some(pixfx_duel::fighter::Act {
        spell: "palm",
        phase: pixfx_duel::fighter::Phase::Windup,
        t: windup - 1.0 / 60.0,
        target: None,
        power: 1.0,
        stacks: 1.0,
        pierce: false,
        circle: None,
    });
    duel.step(1.0 / 60.0);
    assert!(duel.state.fighters[1].body.down.is_some());
    let mut lands = 0;
    let mut up = false;
    for _ in 0..240 {
        lands += duel.step(1.0 / 60.0).iter().filter(|o| matches!(o, Out::Land { .. })).count();
        if duel.state.fighters[1].body.down.is_none() {
            up = true;
            break;
        }
    }
    assert!(up);
    assert!(lands >= 1);
}

#[test]
fn fights_end_in_knockouts_and_each_school_wins_at_least_35_percent_of_rounds() {
    let mut wins = [0_u32; 2];
    for seed in 1..=40 {
        let r = rounds(&mut fight(seed), 120.0);
        assert!(!r.is_empty(), "seed {seed}");
        let last = r[r.len() - 1];
        wins[0] += last[0];
        wins[1] += last[1];
    }
    let total = f64::from(wins[0] + wins[1]);
    for w in wins {
        assert!(f64::from(w) / total >= 0.35, "{wins:?}");
    }
}

#[test]
fn a_cast_puts_its_spell_on_its_own_cooldown_and_a_bar_skills_one_cast_empties_its_slot() {
    let mut checked = 0;
    for seed in 1..=4 {
        let mut duel = plan2(seed);
        let mut was: [Option<&str>; 2] = [None, None];
        for _ in 0..90 * 60 {
            let before: Vec<Vec<Slot>> = duel.state.fighters.iter().map(|f| f.bar.clone()).collect();
            duel.step(1.0 / 60.0);
            for f in &duel.state.fighters {
                let name = f.act.map(|a| a.spell);
                let fresh = name.is_some() && name != was[f.index] && f.act.is_some_and(|a| a.t < 1.0 / 30.0);
                was[f.index] = name;
                let Some(name) = name.filter(|_| fresh) else { continue };
                let j = before[f.index].iter().position(|s| s.spell() == Some(name));
                let swift = j.is_some_and(|j| matches!(&before[f.index][j], Slot::Held { traits, .. } if traits.contains(&pixfx_duel::data::Trait::Swift)));
                let sp = spell(name).map_or((0.0, 0.0), |s| (s.cooldown, s.windup));
                let want = sp.0 * if swift { 0.6 } else { 1.0 } + sp.1;
                assert!((f.cool_full(name) - want).abs() < 1e-5, "{name}");
                let Some(j) = j else { continue };
                let (after, prior) = (&f.bar[j], &before[f.index][j]);
                let evolved = |s: &Slot| matches!(s, Slot::Held { evolved: true, .. });
                let refilled = evolved(after) && !evolved(prior);
                let again = f.lucky.is_some_and(|l| l.kind == pixfx_duel::data::Luck::Again && l.spell == name) && after.spell() == Some(name);
                assert!(matches!(after, Slot::Empty { .. }) || refilled || again);
                checked += 1;
            }
        }
    }
    assert!(checked > 30, "{checked}");
}

#[test]
fn the_skill_bar_shows_four_slots_the_last_one_the_ultimate_charged_by_rage() {
    let mut duel = plan2(3);
    for f in &mut duel.state.fighters {
        let ult: Vec<bool> = skill_bar(f).iter().map(|s| s.ultimate).collect();
        assert_eq!(ult, [false, false, false, true]);
        f.rage = RAGE - 1.0;
        let s = &skill_bar(f)[3];
        assert_eq!((s.charge, s.ready), ((RAGE - 1.0) / RAGE, false));
        f.rage = RAGE;
        let s = &skill_bar(f)[3];
        assert_eq!((s.charge, s.ready), (1.0, true));
    }
}

#[test]
fn a_dive_that_lands_cracks_the_ground_with_every_effect_in_the_engine() {
    let mut played = false;
    for seed in 1..=8 {
        played |= run(&mut fight(seed), 90.0).iter().any(|o| matches!(o, Out::Play { fx: "explosions/dirt-explosion", .. }));
    }
    assert!(played);
}

/// What decides a fight after `seconds` of seed 5 stepped at `hz`.
fn fought(hz: f64, seconds: f64) -> ([u32; 2], u32, Vec<(f64, f64)>) {
    let mut duel = plan2(5);
    for _ in 0..(seconds * hz) as usize {
        duel.step(1.0 / hz);
    }
    (duel.state.wins, duel.state.round, duel.state.fighters.iter().map(|f| (f.hp, f.body.x)).collect())
}

#[test]
fn a_seed_fights_the_same_at_60_hz_and_144_hz() {
    assert_eq!(fought(144.0, 60.0), fought(60.0, 60.0));
}

#[test]
fn step_0_moves_nothing() {
    let mut duel = plan2(5);
    run(&mut duel, 1.5);
    let before = (duel.state.time, duel.state.fighters.clone());
    assert!(duel.step(0.0).is_empty());
    assert_eq!((duel.state.time, duel.state.fighters.clone()), before);
}

#[test]
fn zero_dt_holds_and_big_dt_is_capped() {
    let mut duel = plan2(5);
    for _ in 0..100 {
        assert!(duel.step(0.0).is_empty());
    }
    assert_eq!(duel.state.time, 0.0);
    // one huge step runs at most a quarter second of whole ticks
    duel.step(10.0);
    assert!((duel.state.time - 0.25).abs() < 1e-9, "{}", duel.state.time);
    // NaN and negative steps hold too
    duel.step(f64::NAN);
    duel.step(-1.0);
    assert!((duel.state.time - 0.25).abs() < 1e-9, "{}", duel.state.time);
    // and the next real step still moves it on
    duel.step(1.0 / 60.0);
    assert!(duel.state.time > 0.26);
}

#[test]
fn missing_effects_leave_their_spells_out() {
    let effects: Vec<String> = all_effects().into_iter().filter(|e| e != "ice/icicle-fall" && e != "lightning/ball-lightning").collect();
    for seed in 1..=10 {
        let mut duel = Duel::with_brains(config(seed, effects.clone()));
        for f in &duel.state.fighters {
            assert!(!f.spells.contains(&"icicle") && !f.spells.contains(&"orb"));
            assert!(!f.pool.contains(&"icicle") && !f.pool.contains(&"orb"));
        }
        for o in run(&mut duel, 30.0) {
            if let Out::Play { fx, .. } = o {
                assert!(effects.iter().any(|e| e == fx), "{fx}");
            }
        }
    }
    // the blade and the dive need no effect: an empty registry still fights with them
    let mut bare = Duel::with_brains(config(1, Vec::new()));
    assert!(run(&mut bare, 20.0).iter().all(|o| !matches!(o, Out::Play { .. })));
    for f in &bare.state.fighters {
        assert!(f.spells.contains(&"slash") && f.spells.contains(&"dive"), "{:?}", f.spells);
        assert!(!f.spells.contains(&school(f.school).ultimate));
    }
}
