//! What a policy is told of its slots: the school's basics, the bar, the ultimate, as indexes into its spells.

mod common;

use common::{all_effects, config};
use pixfx_duel::data::school;
use pixfx_duel::duel::Duel;
use pixfx_duel::seat::Brain;
use pixfx_duel::sense::sense_of;

#[test]
fn slots_are_the_basics_the_bar_and_the_ultimate() {
    let mut duel = Duel::new(config(3, all_effects()), [Brain::make(), Brain::make()]);
    for _ in 0..120 {
        duel.step(1.0 / 60.0);
    }
    for me in 0..2 {
        let f = &duel.state.fighters[me];
        let s = sense_of(&duel.state, me);
        assert_eq!(s.slots.len(), 7);
        let name = |i: usize| s.slots[i].map(|k| f.spells[k]);
        let d = school(f.school);
        for (i, b) in d.basics.iter().enumerate() {
            assert_eq!(name(i), Some(*b));
        }
        for (i, slot) in f.bar.iter().enumerate() {
            assert_eq!(name(3 + i), slot.spell(), "bar slot {i}");
        }
        assert_eq!(f.bar.len(), 3);
        assert_eq!(name(6), Some(d.ultimate));
    }
}

#[test]
fn a_skill_says_what_it_inflicts_and_how_much() {
    use pixfx_duel::data::{BURN, CHILL, DEF_DOWN, SHOCK};
    use pixfx_duel::sense::skill_of;
    let bell = skill_of("bell").inflicts;
    assert_eq!(bell.stun, 0.6, "the bell stuns the one striking it");
    let frostfire = skill_of("frostfire").inflicts;
    assert_eq!(frostfire.chill, 1.0);
    assert_eq!(frostfire.chill_max, CHILL.max);
    assert!((frostfire.slow - (1.0 - CHILL.slow)).abs() < 1e-12);
    let orb = skill_of("orb").inflicts;
    assert_eq!(orb.burn, BURN.time);
    assert!((orb.burn_rate - BURN.share / BURN.every).abs() < 1e-12);
    assert_eq!(orb.shock, 1.0);
    assert_eq!(orb.shock_max, SHOCK.max);
    assert_eq!(orb.shock_hold, SHOCK.hold);
    assert_eq!(orb.shock_rate, SHOCK.share);
    let lotus = skill_of("lotus").inflicts;
    assert_eq!(lotus.def_down, 3.0);
    assert_eq!(lotus.def_loss, DEF_DOWN);
    assert_eq!(skill_of("palm").inflicts.steal_rage, 1.0);
    assert_eq!(skill_of("slash").inflicts, pixfx_mind::Inflicts::default());
}

#[test]
fn a_fighter_senses_its_own_act_and_the_foes_ultimate() {
    use pixfx_duel::fighter::{Act, Phase};
    use pixfx_mind::{ActPhase, Doing};
    let mut duel = Duel::new(config(3, all_effects()), [Brain::make(), Brain::make()]);
    duel.step(1.0 / 60.0);
    let ultimate = school(duel.state.fighters[1].school).ultimate;
    duel.state.fighters[1].act = Some(Act {
        spell: ultimate,
        phase: Phase::Recover,
        t: 0.1,
        target: None,
        power: 1.0,
        stacks: 1.0,
        pierce: false,
        circle: None,
        blocked: false,
    });
    let me = sense_of(&duel.state, 1).me;
    let a = me.act.unwrap_or_else(|| unreachable!("it is acting"));
    assert_eq!(a.phase, ActPhase::Recover);
    assert_eq!(a.t, 0.1);
    assert!(a.recover > 0.0);
    assert_eq!(sense_of(&duel.state, 0).me.act, None);
    let Doing::Act { ultimate, .. } = sense_of(&duel.state, 0).foe.doing else {
        unreachable!("the foe is acting")
    };
    assert!(ultimate);
}

#[test]
fn a_lingering_or_evolved_bar_skill_lands_more_stacks() {
    use pixfx_duel::fighter::Slot;
    let mut duel = Duel::new(config(3, all_effects()), [Brain::make(), Brain::make()]);
    duel.step(1.0 / 60.0);
    let f = &mut duel.state.fighters[0];
    let at = f.bar.iter().position(|s| s.spell().is_some());
    let at = at.unwrap_or_else(|| unreachable!("the bar holds a skill at the start"));
    if let Slot::Held {
        evolved, traits, ..
    } = &mut f.bar[at]
    {
        *evolved = true;
        traits.clear();
    }
    let name = f.bar[at].spell();
    let k = f.spells.iter().position(|n| Some(*n) == name);
    let s = sense_of(&duel.state, 0);
    let k = k.unwrap_or_else(|| unreachable!("a held skill is one of its spells"));
    assert_eq!(s.stacks.get(k), Some(&2.0));
    assert_eq!(s.stacks.len(), duel.state.fighters[0].spells.len());
}
