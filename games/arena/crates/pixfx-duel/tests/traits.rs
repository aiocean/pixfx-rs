//! Chance on the bar, ported from `site/tests/traits.test.js`: the cases that need no cast. Those that cast (swift,
//! cheap's cast, piercing, lingering, lucky casts, a heal's power) come with casting and `Duel::step`.

mod common;

use common::{arena, empty};
use pixfx_duel::bar::{draw, ready};
use pixfx_duel::data::{RARITY, Rarity, TRAITS, Trait, fits, slot_power, spell};
use pixfx_duel::fighter::Slot;
use std::collections::HashSet;

#[test]
fn draws_are_common_70_rare_22_and_epic_8_percent_and_rarer_hits_harder() {
    let mut s = arena(3);
    let mut seen = [0_u32; 3];
    let n = 3000;
    for _ in 0..n {
        s.fighters[0].bar[1] = empty(1.0);
        draw(&mut s, 0, 1, false, None);
        let Slot::Held {
            rarity,
            traits,
            power,
            ..
        } = &s.fighters[0].bar[1]
        else {
            continue;
        };
        seen[*rarity as usize] += 1;
        assert!((power - slot_power(*rarity, traits, false)).abs() < 1e-9);
    }
    let share = |r: Rarity| f64::from(seen[r as usize]) / f64::from(n);
    assert!((share(Rarity::Common) - 0.7).abs() < 0.03);
    assert!((share(Rarity::Rare) - 0.22).abs() < 0.03);
    assert!((share(Rarity::Epic) - 0.08).abs() < 0.03);
    assert!((slot_power(Rarity::Rare, &[], false) - RARITY[1].power).abs() < 1e-9);
    assert!(
        (slot_power(Rarity::Epic, &[Trait::Heavy], true)
            - 1.5 * RARITY[2].power * TRAITS.heavy_power)
            .abs()
            < 1e-9
    );
}

#[test]
fn only_traits_that_fit_a_skill_are_rolled_a_common_has_at_most_one_a_rare_one_an_epic_two() {
    let mut s = arena(8);
    let (mut commons, mut common_with) = (0_u32, 0_u32);
    for f in 0..2 {
        for _ in 0..1500 {
            s.fighters[f].bar[2] = empty(1.0);
            draw(&mut s, f, 2, false, None);
            let Slot::Held {
                spell,
                rarity,
                traits,
                ..
            } = &s.fighters[f].bar[2]
            else {
                continue;
            };
            for t in traits {
                assert!(fits(*t, spell), "{spell} {t:?}");
            }
            assert_eq!(traits.iter().collect::<HashSet<_>>().len(), traits.len());
            let fitting = Trait::ALL.iter().filter(|t| fits(**t, spell)).count();
            match rarity {
                Rarity::Common => {
                    commons += 1;
                    common_with += u32::from(!traits.is_empty());
                    assert!(traits.len() <= 1);
                }
                Rarity::Rare => assert_eq!(traits.len(), fitting.min(1)),
                Rarity::Epic => assert_eq!(traits.len(), fitting.min(2)),
                Rarity::Legendary => {}
            }
        }
    }
    assert!((f64::from(common_with) / f64::from(commons) - 0.4).abs() < 0.05);
}

#[test]
fn heavy_and_piercing_fit_only_skills_that_hurt_lingering_only_skills_that_chill_or_shock() {
    assert!(!fits(Trait::Heavy, "heal"));
    assert!(!fits(Trait::Piercing, "ward"));
    assert!(fits(Trait::Heavy, "palm"));
    assert!(fits(Trait::Lingering, "jet"));
    assert!(!fits(Trait::Lingering, "lotus"));
    assert!(fits(Trait::Swift, "heal"));
    assert!(fits(Trait::Cheap, "hex"));
}

#[test]
fn piercing_fits_only_skills_whose_blow_a_guard_can_stop() {
    for name in [
        "icicle",
        "collapse",
        "quake",
        "frostfire",
        "void",
        "flurry",
        "seal",
    ] {
        assert!(!fits(Trait::Piercing, name), "{name}");
    }
    for name in ["palm", "burst", "spark", "shard", "jet", "orb", "bolt"] {
        assert!(fits(Trait::Piercing, name), "{name}");
    }
}

#[test]
fn a_cheap_skill_is_ready_on_half_the_qi() {
    let mut s = arena(1);
    let me = &mut s.fighters[0];
    if !me.pool.contains(&"burst") {
        me.pool.push("burst");
        me.spells.push("burst");
    }
    let cost = spell("burst").map_or(0.0, |sp| sp.cost);
    me.bar = vec![
        Slot::Held {
            spell: "burst",
            power: 1.0,
            evolved: false,
            rarity: Rarity::Common,
            traits: vec![Trait::Cheap],
            idle: 0.0,
        },
        empty(9.0),
        empty(9.0),
    ];
    me.qi = cost * 0.5 + 0.1;
    assert!(ready(me, "burst"));
    me.bar[0] = Slot::Held {
        spell: "burst",
        power: 1.0,
        evolved: false,
        rarity: Rarity::Common,
        traits: Vec::new(),
        idle: 0.0,
    };
    assert!(!ready(me, "burst"));
}
