//! The rule cases of `site/tests/{status,bar,traits,fx}.test.js` that need whole steps of a duel: statuses fading,
//! payoffs and cleanses, the bar turning over, casts with traits and luck, and the effects a fight starts and stops.

mod common;

use common::{all_effects, always, caster, config, fight, idle, loops, on_bar, run};
use pixfx_duel::bar::skill_bar;
use pixfx_duel::data::{LUCKY, Luck, Rarity, STALE, TRAITS, Trait, school, slot_power, spell};
use pixfx_duel::duel::Duel;
use pixfx_duel::fighter::{Act, Burning, Phase, Slot};
use pixfx_duel::out::Out;
use pixfx_duel::seat::MakeSeat;
use pixfx_duel::status::{add_chill, add_shock};
use std::collections::{HashMap, HashSet};

/// A duel whose two brains never act, so only the rules move.
fn still(seed: u32) -> Duel {
    let mut duel = fight(seed);
    duel.hold(0, 1e9);
    duel.hold(1, 1e9);
    duel
}

/// A cast of `name` at the end of its windup (or `before` it).
fn act(name: &'static str, target: f64, before: f64) -> Act {
    let windup = spell(name).map_or(0.0, |s| s.windup);
    Act {
        spell: name,
        phase: Phase::Windup,
        t: windup - before,
        target: Some(target),
        power: 1.0,
        stacks: 1.0,
        pierce: false,
        circle: None,
        blocked: false,
    }
}

fn held(spell: &'static str, rarity: Rarity, traits: Vec<Trait>, evolved: bool) -> Slot {
    let power = slot_power(rarity, &traits, evolved);
    Slot::Held {
        spell,
        power,
        evolved,
        rarity,
        traits,
        idle: 0.0,
    }
}

/// A duel where fighter `index` casts `name` once, from a bar slot set to `slot`, and the other stands still 30 px
/// away; nobody dodges or crits.
fn cast_once(seed: u32, index: usize, name: &'static str, slot: Slot) -> Duel {
    let seats: [MakeSeat; 2] = if index == 0 {
        [caster(name), idle()]
    } else {
        [idle(), caster(name)]
    };
    let mut duel = Duel::new(config(seed, all_effects()), seats);
    for f in &mut duel.state.fighters {
        f.stats.dodge = 0.0;
        f.stats.crit = 0.0;
    }
    let s = &mut duel.state;
    let me = &mut s.fighters[index];
    me.bar = vec![
        slot,
        Slot::Empty {
            left: 9.0,
            full: 9.0,
        },
        Slot::Empty {
            left: 9.0,
            full: 9.0,
        },
    ];
    if !me.pool.contains(&name) {
        me.pool.push(name);
    }
    if !me.spells.contains(&name) {
        me.spells.push(name);
    }
    let x = me.body.x;
    let foe = &mut s.fighters[1 - index];
    foe.body.x = x + if index == 0 { 30.0 } else { -30.0 };
    let fx = foe.body.x;
    s.fighters[index].body.face = pixfx_draw::math::js_sign(fx - x);
    duel
}

fn steps_until(duel: &mut Duel, ticks: usize, mut done: impl FnMut(&Duel, &[Out]) -> bool) -> bool {
    for _ in 0..ticks {
        let out = duel.step(1.0 / 60.0);
        if done(duel, &out) {
            return true;
        }
    }
    false
}

// ---------- statuses ----------

#[test]
fn stacks_fade_2_5_s_after_the_last_one() {
    let mut duel = still(1);
    add_chill(&mut duel.state, 0, 1.0);
    run(&mut duel, 2.3);
    assert_eq!(duel.state.fighters[0].chill, 1.0);
    run(&mut duel, 0.3);
    assert_eq!(duel.state.fighters[0].chill, 0.0);
}

#[test]
fn grit_falls_0_4_a_second_and_is_gone_next_round() {
    let mut duel = still(1);
    duel.state.fighters[0].grit = 1.0;
    run(&mut duel, 1.0);
    assert!((duel.state.fighters[0].grit - 0.6).abs() < 0.05);
    duel.state.fighters[0].grit = 3.0;
    duel.state.fighters[1].hp = 0.0;
    run(&mut duel, 6.0);
    assert_eq!(duel.state.fighters[0].grit, 0.0);
}

/// The hit `name` lands from fighter `by` 20 px off, with `set` run on the duel first.
fn payoff_blow(name: &'static str, set: impl Fn(&mut Duel)) -> (f64, Duel) {
    let mut duel = still(3);
    let by = if matches!(name, "palm" | "burst") {
        0
    } else {
        1
    };
    let f = 1 - by;
    for g in &mut duel.state.fighters {
        g.stats.dodge = 0.0;
        g.stats.crit = 0.0;
    }
    set(&mut duel);
    let fx = duel.state.fighters[f].body.x;
    duel.state.fighters[by].act = Some(act(name, fx, 0.0));
    duel.state.fighters[by].body.x = fx - if by == 0 { 20.0 } else { -20.0 };
    duel.state.fighters[by].body.face = if by == 0 { 1.0 } else { -1.0 };
    let hp = duel.state.fighters[f].hp;
    let mut power = f64::NAN;
    for _ in 0..90 {
        for o in duel.step(1.0 / 60.0) {
            if let Out::Hit {
                spell, power: p, ..
            } = o
                && spell == name
            {
                power = p;
            }
        }
        if duel.state.fighters[f].hp != hp {
            break;
        }
    }
    (power, duel)
}

#[test]
fn palm_hits_12_percent_harder_per_shock_stack_spends_them_and_leaves_its_own() {
    let (plain, _) = payoff_blow("palm", |_| {});
    let (shocked, duel) = payoff_blow("palm", |d| add_shock(&mut d.state, 1, 0, 2.0));
    assert!((shocked / plain - 1.24).abs() < 1e-5, "{shocked} {plain}");
    assert_eq!(duel.state.fighters[1].shock.map(|k| k.stacks), Some(1.0));
}

#[test]
fn lotus_hits_15_percent_harder_per_chill_stack_and_spends_them() {
    let (plain, _) = payoff_blow("lotus", |_| {});
    let (chilled, duel) = payoff_blow("lotus", |d| {
        add_chill(&mut d.state, 0, 1.0);
        add_chill(&mut d.state, 0, 1.0);
    });
    assert!((chilled / plain - 1.3).abs() < 1e-5, "{chilled} {plain}");
    assert_eq!(duel.state.fighters[0].chill, 0.0);
}

#[test]
fn heal_hex_and_ward_clear_burn_silence_defence_down_chill_and_shock() {
    for (name, index) in [("heal", 0), ("hex", 0), ("ward", 1)] {
        let mut duel = still(5);
        let foe = 1 - index;
        let s = &mut duel.state;
        s.fighters[index].burn = Some(Burning {
            t: 3.0,
            tick: 0.5,
            per: 2.0,
            by: foe,
        });
        s.fighters[index].def_down = 3.0;
        add_chill(s, index, 1.0);
        add_chill(s, index, 1.0);
        add_shock(s, index, foe, 3.0);
        let x = s.fighters[index].body.x;
        s.fighters[index].act = Some(act(name, x, 0.0));
        duel.step(1.0 / 60.0);
        let f = &duel.state.fighters[index];
        assert!(f.burn.is_none(), "{name}");
        assert_eq!((f.def_down, f.chill), (0.0, 0.0), "{name}");
        assert!(f.shock.is_none(), "{name}");
    }
}

#[test]
fn a_cleanse_lifts_a_silence_that_fell_during_the_cast() {
    let mut duel = still(5);
    let x = duel.state.fighters[0].body.x;
    duel.state.fighters[0].act = Some(act("heal", x, 0.0));
    duel.state.fighters[0].silence = 2.0;
    duel.step(1.0 / 60.0);
    assert_eq!(duel.state.fighters[0].silence, 0.0);
}

#[test]
fn an_orb_that_lands_leaves_a_shock_stack() {
    let mut duel = still(7);
    for g in &mut duel.state.fighters {
        g.stats.dodge = 0.0;
        g.stats.crit = 0.0;
    }
    let fx = duel.state.fighters[1].body.x;
    duel.state.fighters[0].act = Some(act("orb", fx, 0.0));
    duel.state.fighters[0].body.x = fx - 30.0;
    duel.state.fighters[0].body.face = 1.0;
    let landed = steps_until(&mut duel, 60 * 7, |_, out| {
        out.iter()
            .any(|o| matches!(o, Out::Hit { spell: "orb", .. }))
    });
    assert!(landed);
    assert!(duel.state.fighters[1].shock.map_or(0.0, |k| k.stacks) > 0.0);
}

// ---------- the bar ----------

#[test]
fn a_spent_skill_empties_its_slot_which_counts_down_and_draws_a_skill_not_on_the_bar() {
    for seed in 1..=6 {
        let mut duel = still(seed);
        let f = (seed % 2) as usize;
        let plain = if f == 0 {
            ["orb", "bell", "heal"]
        } else {
            ["mirror", "void", "frostfire"]
        };
        duel.state.fighters[f].bar = plain
            .iter()
            .map(|n| held(n, Rarity::Common, Vec::new(), false))
            .collect();
        pixfx_duel::bar::spend(&mut duel.state, f, 0);
        let Slot::Empty { left, .. } = duel.state.fighters[f].bar[0] else {
            return assert!(matches!(duel.state.fighters[f].bar[0], Slot::Empty { .. }));
        };
        assert!((1.5..=2.5).contains(&left));
        for _ in 0..libm::ceil(left * 60.0) as usize + 1 {
            duel.step(1.0 / 60.0);
        }
        let bar = &duel.state.fighters[f].bar;
        let new = bar[0].spell();
        assert!(new.is_some(), "seed {seed}");
        assert!(new != bar[1].spell() && new != bar[2].spell());
    }
}

#[test]
fn the_same_seed_plays_the_same_bar() {
    let bars = || {
        let mut duel = fight(7);
        let mut seen = Vec::new();
        for i in 0..60 * 60 {
            duel.step(1.0 / 60.0);
            if i % 30 == 0 {
                seen.push(
                    duel.state
                        .fighters
                        .iter()
                        .map(|f| f.bar.clone())
                        .collect::<Vec<_>>(),
                );
            }
        }
        seen
    };
    assert_eq!(bars(), bars());
}

#[test]
fn skill_bars_turn_over_in_a_fight() {
    let mut duel = fight(3);
    let mut seen = HashSet::new();
    for _ in 0..90 * 60 {
        duel.step(1.0 / 60.0);
        for f in &duel.state.fighters {
            for s in on_bar(f) {
                seen.insert((f.index, s));
            }
        }
    }
    assert!(seen.len() > 8);
}

#[test]
fn the_screens_bar_holds_no_nan_after_a_knockout() {
    let mut duel = fight(5);
    duel.state.fighters[0].hp = 0.0;
    duel.step(1.0 / 60.0);
    for f in &duel.state.fighters {
        for s in skill_bar(f) {
            assert!(
                [s.left, s.full, s.charge, s.power, s.stale]
                    .iter()
                    .all(|v| !v.is_nan())
            );
        }
    }
}

/// Still fighters, fighter 0's bar set to `spells`.
fn stale_bar(spells: [&'static str; 3]) -> Duel {
    let mut duel = still(9);
    let f = &mut duel.state.fighters[0];
    for n in spells {
        if !f.pool.contains(&n) {
            f.pool.push(n);
        }
    }
    f.bar = spells
        .iter()
        .map(|n| held(n, Rarity::Common, Vec::new(), false))
        .collect();
    duel
}

#[test]
fn a_skill_left_ready_and_uncast_for_stale_seconds_leaves_the_bar_for_another() {
    let mut duel = stale_bar(["orb", "bell", "heal"]);
    run(&mut duel, STALE - 0.5);
    assert_eq!(on_bar(&duel.state.fighters[0]), ["orb", "bell", "heal"]);
    run(&mut duel, 1.0);
    let bar = &duel.state.fighters[0].bar;
    for (i, was) in ["orb", "bell", "heal"].into_iter().enumerate() {
        assert!(bar[i].spell().is_some_and(|n| n != was));
    }
    assert_eq!(
        on_bar(&duel.state.fighters[0])
            .iter()
            .collect::<HashSet<_>>()
            .len(),
        3
    );
}

#[test]
fn a_cast_restarts_the_clock_and_a_skill_cooling_down_does_not_go_stale() {
    let mut duel = stale_bar(["orb", "bell", "heal"]);
    run(&mut duel, STALE - 1.0);
    let f = &mut duel.state.fighters[0];
    if let Slot::Held { idle, .. } = &mut f.bar[0] {
        *idle = 0.0;
    }
    f.set_cool_left("bell", STALE * 2.0);
    run(&mut duel, 2.0);
    let bar = &duel.state.fighters[0].bar;
    assert_eq!(bar[0].spell(), Some("orb"));
    assert_eq!(bar[1].spell(), Some("bell"));
    assert_ne!(bar[2].spell(), Some("heal"));
}

#[test]
fn the_screens_bar_shows_how_stale_each_skill_is() {
    let mut duel = stale_bar(["orb", "bell", "heal"]);
    run(&mut duel, STALE / 2.0);
    let bar = skill_bar(&duel.state.fighters[0]);
    assert!((bar[0].stale - 0.5).abs() < 0.05);
    assert_eq!(bar[3].stale, 0.0);
}

#[test]
fn every_pool_spell_is_cast_in_some_fight() {
    let mut cast = HashSet::new();
    for seed in 1..=24 {
        let mut duel = fight(seed);
        for _ in 0..120 * 60 {
            duel.step(1.0 / 60.0);
            for f in &duel.state.fighters {
                if let Some(a) = f.act.filter(|a| a.t == 0.0) {
                    cast.insert(a.spell);
                }
            }
        }
    }
    for s in [
        pixfx_duel::data::School::Thunder,
        pixfx_duel::data::School::Frost,
    ] {
        for name in school(s).pool {
            assert!(cast.contains(name), "{name}");
        }
    }
}

#[test]
fn a_new_rounds_draws_ignore_the_statuses_the_last_round_left_on_the_foe() {
    let next_bar = |seed: u32, leftover: bool| {
        let mut duel = still(seed);
        if leftover {
            add_shock(&mut duel.state, 1, 0, 3.0);
        }
        duel.state.fighters[1].hp = 0.0;
        let round = duel.state.round;
        for _ in 0..60 * 10 {
            if duel.state.round != round {
                break;
            }
            duel.step(1.0 / 60.0);
        }
        assert_eq!(duel.state.round, round + 1);
        on_bar(&duel.state.fighters[0])
    };
    for seed in 1..=20 {
        assert_eq!(next_bar(seed, true), next_bar(seed, false), "seed {seed}");
    }
}

#[test]
fn an_evolved_skill_inflicts_one_more_stack_of_what_it_inflicts() {
    let chill_of = |evolved: bool| {
        let mut duel = Duel::new(config(4, all_effects()), [idle(), caster("jet")]);
        for g in &mut duel.state.fighters {
            g.stats.dodge = 0.0;
            g.stats.crit = 0.0;
        }
        let tx = duel.state.fighters[0].body.x;
        let f = &mut duel.state.fighters[1];
        f.bar = vec![
            held("jet", Rarity::Common, Vec::new(), evolved),
            held("shard", Rarity::Common, Vec::new(), false),
            held("mirror", Rarity::Common, Vec::new(), false),
        ];
        if let Slot::Held { power, .. } = &mut f.bar[0] {
            *power = 1.0;
        }
        f.body.x = tx + 30.0;
        f.body.face = -1.0;
        run(&mut duel, 1.0);
        duel.state.fighters[0].chill
    };
    assert_eq!(chill_of(false), 1.0);
    assert_eq!(chill_of(true), 2.0);
}

// ---------- traits and luck ----------

#[test]
fn swift_cuts_the_skills_cooldown_to_60_percent() {
    let mut duel = cast_once(
        1,
        0,
        "spark",
        held("spark", Rarity::Common, vec![Trait::Swift], false),
    );
    duel.step(1.0 / 60.0);
    let sp = spell("spark").map_or((0.0, 0.0), |s| (s.cooldown, s.windup));
    assert!(
        (duel.state.fighters[0].cool_full("spark") - (sp.0 * TRAITS.swift_cooldown + sp.1)).abs()
            < 1e-9
    );
}

#[test]
fn cheap_halves_the_qi_a_cast_costs() {
    let mut duel = cast_once(
        1,
        0,
        "burst",
        held("burst", Rarity::Common, vec![Trait::Cheap], false),
    );
    duel.state.fighters[0].qi = spell("burst").map_or(0.0, |s| s.cost) * 0.5 + 0.1;
    assert!(pixfx_duel::bar::ready(&duel.state.fighters[0], "burst"));
    duel.step(1.0 / 60.0);
    assert_eq!(duel.state.fighters[0].act.map(|a| a.spell), Some("burst"));
    assert!(duel.state.fighters[0].qi < 1.0);
}

#[test]
fn piercing_goes_through_a_guard_every_time() {
    let landed = |traits: Vec<Trait>| {
        let mut n = 0;
        for seed in 1..=20 {
            let mut duel = cast_once(
                seed,
                0,
                "palm",
                held("palm", Rarity::Common, traits.clone(), false),
            );
            let (mx, fx) = (duel.state.fighters[0].body.x, duel.state.fighters[1].body.x);
            duel.state.fighters[1].body.face = pixfx_draw::math::js_sign(mx - fx);
            let mut hit = false;
            for _ in 0..60 {
                duel.state.fighters[1].guard = 5.0;
                hit = duel
                    .step(1.0 / 60.0)
                    .iter()
                    .any(|o| matches!(o, Out::Hit { spell: "palm", .. }));
                if hit {
                    break;
                }
            }
            n += usize::from(hit);
        }
        n
    };
    assert_eq!(landed(vec![Trait::Piercing]), 20);
    assert!(landed(Vec::new()) < 18);
}

#[test]
fn lingering_adds_a_stack_on_top_of_evolutions() {
    let mut duel = cast_once(
        4,
        1,
        "jet",
        held("jet", Rarity::Common, vec![Trait::Lingering], false),
    );
    steps_until(&mut duel, 60, |d, _| d.state.fighters[0].chill != 0.0);
    assert_eq!(duel.state.fighters[0].chill, 2.0);
    let mut duel = cast_once(
        4,
        0,
        "spark",
        held("spark", Rarity::Common, vec![Trait::Lingering], true),
    );
    steps_until(&mut duel, 60, |d, _| d.state.fighters[1].shock.is_some());
    assert_eq!(duel.state.fighters[1].shock.map(|k| k.stacks), Some(3.0));
}

struct Cast {
    kind: Option<Luck>,
    power: f64,
    qi: f64,
    cool: f64,
    kept: bool,
}

/// Fighter 0 casts `name` every tick it can, `n` times, with qi and the slot topped up; what each cast did.
fn casts(name: &'static str, n: usize) -> Vec<Cast> {
    let mut duel = Duel::new(config(12, all_effects()), [always(name), idle()]);
    let on_bar = spell(name).is_some_and(|s| s.kind != pixfx_duel::data::Kind::Melee);
    {
        let me = &mut duel.state.fighters[0];
        if !me.spells.contains(&name) {
            me.spells.push(name);
        }
        if on_bar && !me.pool.contains(&name) {
            me.pool.push(name);
        }
    }
    let mut out = Vec::new();
    while out.len() < n {
        let me = &mut duel.state.fighters[0];
        me.act = None;
        me.cool.clear();
        me.qi = 50.0;
        me.hp = 100.0;
        if on_bar {
            me.bar = vec![
                held(name, Rarity::Common, Vec::new(), false),
                Slot::Empty {
                    left: 9.0,
                    full: 9.0,
                },
                Slot::Empty {
                    left: 9.0,
                    full: 9.0,
                },
            ];
        }
        let lucky = duel.step(1.0 / 600.0).into_iter().find_map(|o| match o {
            Out::Lucky { kind, .. } => Some(kind),
            _ => None,
        });
        let me = &duel.state.fighters[0];
        if let Some(a) = me.act.filter(|a| a.spell == name) {
            out.push(Cast {
                kind: lucky,
                power: a.power,
                qi: me.qi,
                cool: me.cool(name),
                kept: me.bar.first().and_then(Slot::spell) == Some(name),
            });
        }
    }
    out
}

#[test]
fn about_one_bar_cast_in_twelve_is_lucky_and_each_kind_does_what_it_says() {
    let seen = casts("spark", 3000);
    let lucky: Vec<&Cast> = seen.iter().filter(|c| c.kind.is_some()).collect();
    assert!((lucky.len() as f64 / seen.len() as f64 - LUCKY.chance).abs() < 0.02);
    let kinds: HashSet<&str> = lucky
        .iter()
        .filter_map(|c| c.kind.map(Luck::name))
        .collect();
    assert_eq!(kinds, HashSet::from(["again", "double", "free"]));
    for c in &seen {
        let want_power = if c.kind == Some(Luck::Double) {
            2.0
        } else {
            1.0
        };
        assert!((c.power - want_power).abs() < 1e-9);
        if c.kind == Some(Luck::Free) {
            assert!(c.qi > 49.9);
        } else {
            assert!(c.qi < 45.0);
        }
        // Again keeps the skill on its slot for another cast, its cooldown over; any other cast empties the slot
        if c.kind == Some(Luck::Again) {
            assert_eq!((c.cool, c.kept), (0.0, true));
        } else {
            assert_eq!((c.cool > 0.0, c.kept), (true, false));
        }
    }
}

#[test]
fn the_blade_and_the_ultimate_are_never_lucky() {
    assert!(casts("slash", 400).iter().all(|c| c.kind.is_none()));
}

#[test]
fn a_lucky_cast_names_its_own_slot_and_again_puts_the_skill_back_on_it() {
    let mut duel = Duel::new(config(21, all_effects()), [always("spark"), idle()]);
    {
        let me = &mut duel.state.fighters[0];
        if !me.pool.contains(&"spark") {
            me.pool.push("spark");
        }
        if !me.spells.contains(&"spark") {
            me.spells.push("spark");
        }
    }
    let mut kinds = HashSet::new();
    for _ in 0..4000 {
        if kinds.len() == 3 {
            break;
        }
        let me = &mut duel.state.fighters[0];
        me.act = None;
        me.cool.clear();
        me.qi = 50.0;
        me.lucky = None;
        me.bar = vec![
            Slot::Empty {
                left: 9.0,
                full: 9.0,
            },
            held("spark", Rarity::Common, Vec::new(), false),
            Slot::Empty {
                left: 9.0,
                full: 9.0,
            },
        ];
        duel.step(1.0 / 600.0);
        let me = &duel.state.fighters[0];
        if let Some(l) = me.lucky {
            assert_eq!(l.slot, 1);
            assert_eq!(me.bar[1].spell() == Some("spark"), l.kind == Luck::Again);
            kinds.insert(l.kind.name());
        }
    }
    assert_eq!(kinds, HashSet::from(["again", "double", "free"]));
}

#[test]
fn a_heal_on_the_bar_heals_by_its_power_as_the_ai_is_told() {
    let mut duel = cast_once(
        2,
        0,
        "heal",
        held("heal", Rarity::Common, Vec::new(), false),
    );
    duel.state.fighters[0].hp = 40.0;
    duel.state.fighters[0].qi = 100.0;
    if let Slot::Held { power, .. } = &mut duel.state.fighters[0].bar[0] {
        *power = 1.4;
    }
    steps_until(&mut duel, 90, |d, _| d.state.fighters[0].hp != 40.0);
    let amount = spell("heal").and_then(|s| s.amount).unwrap_or(0.0);
    assert!((duel.state.fighters[0].hp - 40.0 - amount * 1.4).abs() < 1e-5);
}

// ---------- effects started and stopped ----------

#[test]
fn the_water_jet_sprays_for_a_moment_and_stops() {
    let mut duel = cast_once(3, 0, "jet", held("jet", Rarity::Common, Vec::new(), false));
    duel.state.fighters[0].qi = 100.0;
    let mut key = None;
    let stopped = steps_until(&mut duel, 3 * 60, |_, out| {
        for o in out {
            match o {
                Out::Play {
                    key: k,
                    fx: "water/water-jet",
                    ..
                } => key = Some(*k),
                Out::Stop { key: k } if Some(*k) == key => return true,
                _ => {}
            }
        }
        false
    });
    assert!(key.is_some());
    assert!(stopped);
}

#[test]
fn knockout_stops_every_effect_it_started() {
    for seed in 1..=8 {
        let mut duel = fight(seed);
        let mut live: HashMap<u32, (&str, f64)> = HashMap::new();
        for _ in 0..90 * 60 {
            let out = duel.step(1.0 / 60.0);
            for o in &out {
                match o {
                    Out::Play { key, fx, .. } if loops(fx) => {
                        live.insert(*key, (fx, duel.state.time));
                    }
                    Out::Stop { key } => {
                        live.remove(key);
                    }
                    _ => {}
                }
            }
            for (fx, at) in live.values() {
                assert!(
                    duel.state.time - at < 12.0,
                    "seed {seed}: {fx} left playing"
                );
            }
        }
    }
}

/// Fighter 0 casts `name`; once `fx` plays, the foe falls. Whether that effect is stopped by the next round.
fn stopped_after_knockout(name: &'static str, fx: &str) -> bool {
    let mut duel = cast_once(3, 0, name, held(name, Rarity::Common, Vec::new(), false));
    duel.state.fighters[0].qi = 100.0;
    let mut key = None;
    steps_until(&mut duel, 120, |_, out| {
        key = out.iter().find_map(|o| match o {
            Out::Play { key, fx: f, .. } if *f == fx => Some(*key),
            _ => None,
        });
        key.is_some()
    });
    assert!(key.is_some(), "{name}");
    duel.state.fighters[1].hp = 0.0;
    let round = duel.state.round;
    let mut stopped = false;
    for _ in 0..600 {
        if duel.state.round != round {
            break;
        }
        stopped |= duel
            .step(1.0 / 60.0)
            .iter()
            .any(|o| matches!(o, Out::Stop { key: k } if Some(*k) == key));
    }
    assert_eq!(duel.state.round, round + 1);
    stopped
}

#[test]
fn a_cast_still_gathering_in_its_rune_circle_when_the_foe_falls_loses_the_circle() {
    assert!(stopped_after_knockout("quake", "arcane/rune-circle"));
}

#[test]
fn a_barrier_standing_when_the_round_ends_loses_its_dome() {
    assert!(stopped_after_knockout("bell", "arcane/rune-circle"));
}

#[test]
fn the_hex_shields_dome_stands_on_the_casters_feet() {
    let mut duel = cast_once(3, 0, "hex", held("hex", Rarity::Common, Vec::new(), false));
    duel.state.fighters[0].qi = 100.0;
    let mut at = None;
    steps_until(&mut duel, 120, |d, out| {
        at = out.iter().find_map(|o| match o {
            Out::Play {
                fx: "arcane/hex-shield",
                y,
                ..
            } => Some((*y, d.state.ground - d.state.fighters[0].body.air)),
            _ => None,
        });
        at.is_some()
    });
    let (y, feet) = at.unwrap_or((f64::NAN, 0.0));
    assert_eq!(y, feet);
}
