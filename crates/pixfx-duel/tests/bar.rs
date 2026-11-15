//! The rotating skill bar, ported from `site/tests/bar.test.js`: the cases that need no step of the duel. Those that
//! do (countdowns, staleness, turnover, a round's redraw, an evolved cast's stacks) come with `Duel::step`.

mod common;

use common::{arena, common, empty, on_bar};
use pixfx_duel::bar::{draw, skill_bar, spend};
use pixfx_duel::data::{POOL_SIZE, RARITY, Rarity, Trait, payoff, school, slot_power, spell};
use pixfx_duel::fighter::Slot;
use pixfx_duel::status::{add_chill, add_shock};
use std::collections::HashSet;

fn held(slot: &Slot) -> (&'static str, f64, bool, Rarity, Vec<Trait>) {
    match slot {
        Slot::Held { spell, power, evolved, rarity, traits, .. } => (spell, *power, *evolved, *rarity, traits.clone()),
        Slot::Empty { .. } => ("", 0.0, false, Rarity::Common, Vec::new()),
    }
}

fn wait(slot: &Slot) -> Option<f64> {
    match slot {
        Slot::Empty { left, .. } => Some(*left),
        Slot::Held { .. } => None,
    }
}

#[test]
fn a_slot_holds_one_cast_casting_it_empties_the_slot() {
    let mut s = arena(6);
    s.fighters[0].bar = vec![common("orb"), common("bell"), common("heal")];
    spend(&mut s, 0, 1);
    assert!(wait(&s.fighters[0].bar[1]).is_some_and(|w| w > 0.0));
    assert_eq!(s.fighters[0].bar[1].spell(), None);
}

#[test]
fn a_fresh_round_draws_three_different_skills_from_the_schools_pool() {
    for seed in 1..=10 {
        for f in &arena(seed).fighters {
            assert_eq!(f.bar.len(), 3);
            let spells = on_bar(f);
            assert_eq!(spells.len(), 3);
            assert_eq!(spells.iter().collect::<HashSet<_>>().len(), 3);
            for slot in &f.bar {
                let (name, power, _, rarity, traits) = held(slot);
                assert!(school(f.school).pool.contains(&name));
                assert!((power - slot_power(rarity, &traits, false)).abs() < 1e-9);
            }
        }
    }
}

#[test]
fn a_spent_skill_empties_its_slot_for_1_5_to_2_5_s() {
    for seed in 1..=6 {
        let mut s = arena(seed);
        let f = (seed % 2) as usize;
        let plain = if f == 0 { ["orb", "bell", "heal"] } else { ["mirror", "void", "frostfire"] };
        s.fighters[f].bar = plain.iter().map(|n| common(n)).collect();
        spend(&mut s, f, 0);
        let w = wait(&s.fighters[f].bar[0]).unwrap_or(0.0);
        assert!((1.5..=2.5).contains(&w), "{w}");
    }
}

#[test]
fn a_draw_fills_an_empty_slot_with_a_skill_not_on_the_bar() {
    let mut s = arena(4);
    s.fighters[0].bar[1] = empty(1.0);
    draw(&mut s, 0, 1, false, None);
    assert!(s.fighters[0].bar[1].spell().is_some());
    assert_eq!(on_bar(&s.fighters[0]).iter().collect::<HashSet<_>>().len(), 3);
}

#[test]
fn the_blade_the_dive_the_blink_and_the_ultimate_are_always_at_hand() {
    for f in &arena(2).fighters {
        for name in ["slash", "dive", "blink", school(f.school).ultimate] {
            assert!(f.spells.contains(&name), "{name}");
        }
    }
}

#[test]
fn the_screens_bar_holds_no_nan_before_a_fight() {
    for f in &arena(5).fighters {
        for slot in skill_bar(f) {
            for v in [slot.left, slot.full, slot.charge, slot.power, slot.stale] {
                assert!(!v.is_nan());
            }
        }
    }
}

#[test]
fn a_skills_tier_is_its_rarity_and_a_legendary_is_never_drawn_only_reached() {
    let names: Vec<&str> = Rarity::ALL.iter().map(|r| r.name()).collect();
    assert_eq!(names, ["common", "rare", "epic", "legendary"]);
    assert_eq!(RARITY[3].odds, 0.0);
    assert!(RARITY[3].power > RARITY[2].power);
    assert_eq!(RARITY[3].traits, 3);
}

#[test]
fn drawing_a_skill_already_on_the_bar_raises_its_tier_and_the_slot_draws_another_skill() {
    let mut s = arena(11);
    s.fighters[0].pool = vec!["orb", "heal"];
    let mut ups = 0;
    for _ in 0..200 {
        s.fighters[0].bar = vec![common("orb"), empty(1.0), empty(1.0)];
        draw(&mut s, 0, 1, false, None);
        assert_eq!(s.fighters[0].bar[1].spell(), Some("heal"));
        let (_, power, _, rarity, traits) = held(&s.fighters[0].bar[0]);
        if rarity == Rarity::Rare {
            ups += 1;
            assert_eq!(traits.len(), 1);
            assert!((power - slot_power(Rarity::Rare, &traits, false)).abs() < 1e-9);
        } else {
            assert_eq!(rarity, Rarity::Common);
        }
    }
    assert!(ups > 60 && ups < 140, "{ups}");
}

#[test]
fn tiers_climb_one_step_a_duplicate_up_to_legendary_keeping_the_traits_they_had() {
    let mut s = arena(12);
    s.fighters[0].pool = vec!["palm"];
    let swift = Slot::Held { spell: "palm", power: 1.0, evolved: false, rarity: Rarity::Common, traits: vec![Trait::Swift], idle: 0.0 };
    s.fighters[0].bar = vec![swift, empty(1.0), empty(1.0)];
    let mut seen = Vec::new();
    for _ in 0..4 {
        draw(&mut s, 0, 1, false, None);
        let (_, _, _, rarity, traits) = held(&s.fighters[0].bar[0]);
        seen.push(rarity);
        assert!(traits.contains(&Trait::Swift));
        assert_eq!(traits.len(), RARITY[rarity as usize].traits.clamp(1, 5));
        // nothing else to draw: the slot waits again
        assert!(wait(&s.fighters[0].bar[1]).is_some_and(|w| w > 0.0));
    }
    assert_eq!(seen, [Rarity::Rare, Rarity::Epic, Rarity::Legendary, Rarity::Legendary]);
}

#[test]
fn an_evolved_skill_keeps_its_evolution_when_a_duplicate_raises_its_tier() {
    let mut s = arena(13);
    s.fighters[0].pool = vec!["palm"];
    let evolved = Slot::Held { spell: "palm", power: 1.5, evolved: true, rarity: Rarity::Common, traits: Vec::new(), idle: 0.0 };
    s.fighters[0].bar = vec![evolved, empty(1.0), empty(1.0)];
    draw(&mut s, 0, 2, false, None);
    let (_, power, evolved, rarity, traits) = held(&s.fighters[0].bar[0]);
    assert!(evolved);
    assert_eq!(rarity, Rarity::Rare);
    assert!((power - slot_power(Rarity::Rare, &traits, true)).abs() < 1e-9);
}

#[test]
fn the_new_spells_are_in_their_schools_pools() {
    let thunder = school(pixfx_duel::data::School::Thunder);
    let frost = school(pixfx_duel::data::School::Frost);
    for name in ["spark", "burst", "quake", "hex"] {
        assert!(thunder.pool.contains(&name));
    }
    for name in ["shard", "jet", "flurry"] {
        assert!(frost.pool.contains(&name));
    }
    for d in [thunder, frost] {
        for name in d.pool.iter().chain([&d.ultimate]) {
            let cost = spell(name).map_or(-1.0, |s| s.cost);
            assert!(cost > if *name == d.ultimate { -1.0 } else { 0.0 }, "{name}");
        }
    }
}

#[test]
fn each_fight_plays_8_of_its_schools_skills_with_its_signature_kind_and_a_skill_that_pays_off_its_status() {
    assert_eq!(POOL_SIZE, 8);
    for seed in 1..=30 {
        for f in &arena(seed).fighters {
            let d = school(f.school);
            assert_eq!(f.pool.len(), 8);
            assert_eq!(f.pool.iter().collect::<HashSet<_>>().len(), 8);
            assert!(f.pool.iter().all(|n| d.pool.contains(n)));
            assert!(f.pool.iter().any(|n| spell(n).is_some_and(|s| s.kind == d.signature)));
            assert!(f.pool.iter().any(|n| payoff(n).is_some()));
            let mut mine: Vec<_> = f.spells.iter().filter(|n| d.pool.contains(n)).copied().collect();
            let mut pool = f.pool.clone();
            mine.sort_unstable();
            pool.sort_unstable();
            assert_eq!(mine, pool);
        }
    }
}

#[test]
fn a_pools_payoff_skill_always_has_a_skill_in_the_pool_that_lays_its_status() {
    for seed in 1..=200 {
        for f in &arena(seed).fighters {
            for name in f.pool.iter().filter(|n| payoff(n).is_some()) {
                let Some((status, _)) = payoff(name) else { continue };
                let lays = f.pool.iter().any(|n| {
                    n != name
                        && spell(n).is_some_and(|s| if status == pixfx_duel::data::Status::Shock { s.shock } else { s.chill })
                });
                assert!(lays, "{seed} {name}");
            }
        }
    }
}

#[test]
fn the_same_seed_plays_the_same_pools_and_seeds_differ() {
    let pools = |seed| {
        arena(seed)
            .fighters
            .iter()
            .map(|f| {
                let mut p = f.pool.clone();
                p.sort_unstable();
                p.join(",")
            })
            .collect::<Vec<_>>()
            .join("|")
    };
    assert_eq!(pools(5), pools(5));
    let seen: HashSet<String> = (1..=10).map(pools).collect();
    assert!(seen.len() >= 5);
}

#[test]
fn a_rounds_first_slot_is_the_schools_signature_kind() {
    for seed in 1..=15 {
        for f in &arena(seed).fighters {
            let first = f.bar[0].spell().and_then(spell).map(|s| s.kind);
            assert_eq!(first, Some(school(f.school).signature), "{seed}");
        }
    }
}

#[test]
fn with_a_status_on_the_foe_the_skills_that_spend_it_are_drawn_far_more_often() {
    let rate = |shocked: bool| {
        let mut s = arena(9);
        s.fighters[0].pool = school(pixfx_duel::data::School::Thunder).pool.to_vec();
        s.fighters[0].bar[0] = common("orb");
        s.fighters[0].bar[2] = common("bell");
        let n = 600;
        let mut pays = 0;
        for _ in 0..n {
            s.fighters[1].shock = None;
            if shocked {
                add_shock(&mut s, 1, 0, 2.0);
            }
            s.fighters[0].bar[1] = empty(1.0);
            draw(&mut s, 0, 1, false, None);
            if matches!(s.fighters[0].bar[1].spell(), Some("palm" | "burst")) {
                pays += 1;
            }
        }
        f64::from(pays) / f64::from(n)
    };
    assert!(rate(true) > rate(false) + 0.15);
}

#[test]
fn a_draw_never_repeats_a_skill_on_the_bar() {
    let mut s = arena(11);
    for k in 0..300 {
        add_chill(&mut s, 0, 1.0);
        let i = k % 3;
        s.fighters[1].bar[i] = empty(1.0);
        draw(&mut s, 1, i, false, None);
        assert_eq!(on_bar(&s.fighters[1]).iter().collect::<HashSet<_>>().len(), 3);
    }
}

fn bar_of(seed: u32, f: usize, spells: [&'static str; 3]) -> pixfx_duel::fighter::State {
    let mut s = arena(seed);
    s.fighters[f].bar = spells.iter().map(|n| common(n)).collect();
    s
}

#[test]
fn a_spent_skill_whose_partner_is_on_the_bar_refills_at_once_evolved_and_half_again_as_strong() {
    for seed in 1..=8 {
        let mut s = bar_of(seed, 0, ["palm", "step", "orb"]);
        spend(&mut s, 0, 0);
        let (name, power, evolved, _, _) = held(&s.fighters[0].bar[0]);
        assert_eq!(name, "palm");
        assert!(evolved);
        assert!((power - 1.5).abs() < 1e-9);
    }
}

#[test]
fn each_school_has_its_own_pairs() {
    assert!(!school(pixfx_duel::data::School::Frost).pool.contains(&"bolt"));
    let mut frost = bar_of(1, 1, ["shard", "icicle", "lotus"]);
    spend(&mut frost, 1, 0);
    assert!(held(&frost.fighters[1].bar[0]).2);
    let mut thunder = bar_of(1, 0, ["bolt", "seal", "orb"]);
    spend(&mut thunder, 0, 0);
    assert!(held(&thunder.fighters[0].bar[0]).2);
}

#[test]
fn without_its_partner_on_the_bar_a_spent_skill_counts_down_as_usual() {
    let mut s = bar_of(2, 0, ["palm", "orb", "bell"]);
    spend(&mut s, 0, 0);
    assert!(wait(&s.fighters[0].bar[0]).is_some_and(|w| w >= 1.5));
}

#[test]
fn an_evolved_skill_once_spent_empties_its_slot() {
    let mut s = bar_of(3, 0, ["palm", "step", "orb"]);
    spend(&mut s, 0, 0);
    spend(&mut s, 0, 0);
    assert!(wait(&s.fighters[0].bar[0]).is_some_and(|w| w >= 1.5));
}

#[test]
fn pools_and_opening_bars_are_pinned_for_each_seed() {
    let mut arenas = serde_json::Map::new();
    for seed in 1..=8_u32 {
        let s = arena(seed);
        let fighters: Vec<serde_json::Value> = s
            .fighters
            .iter()
            .map(|me| {
                let bar: Vec<serde_json::Value> = me
                    .bar
                    .iter()
                    .map(|slot| {
                        let (spell, power, _, rarity, traits) = held(slot);
                        let traits: Vec<&str> = traits.iter().map(|t| t.name()).collect();
                        serde_json::json!({ "spell": spell, "rarity": rarity.name(), "traits": traits, "power": power })
                    })
                    .collect();
                serde_json::json!({ "pool": me.pool, "bar": bar })
            })
            .collect();
        arenas.insert(seed.to_string(), serde_json::Value::Array(fighters));
    }
    common::pinned("arenas", &serde_json::Value::Object(arenas));
}
