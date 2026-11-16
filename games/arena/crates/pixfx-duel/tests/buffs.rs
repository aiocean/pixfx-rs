//! What the HUD shows under each fighter's bars, ported from `site/tests/buffs.test.js`.

mod common;

use common::arena;
use pixfx_duel::bar::{Buff, buffs_of};
use pixfx_duel::fighter::{Burning, Shocked};

fn b(id: &'static str, good: bool, stacks: f64) -> Buff {
    Buff { id, good, stacks }
}

#[test]
fn a_fresh_fighter_shows_its_schools_passive() {
    let s = arena(1);
    assert_eq!(buffs_of(&s.fighters[0]), [b("unyielding", true, 0.0)]);
    assert_eq!(buffs_of(&s.fighters[1]), [b("attune", true, 0.0)]);
}

#[test]
fn stacks_show_as_numbers_and_good_ones_come_before_bad_ones() {
    let mut s = arena(1);
    let t = &mut s.fighters[0];
    t.fury = 3.0;
    t.charges = 2.0;
    t.chill = 2.0;
    t.burn = Some(Burning {
        t: 1.0,
        tick: 0.5,
        per: 1.0,
        by: 1,
    });
    t.shock = Some(Shocked {
        stacks: 4.0,
        tick: 0.5,
        by: 1,
    });
    t.def_down = 1.0;
    t.silence = 1.0;
    t.sealed = 0.1;
    t.body.frozen = 0.5;
    t.grit = 1.6;
    t.barrier = 1.0;
    assert_eq!(
        buffs_of(t),
        [
            b("fury", true, 3.0),
            b("unyielding", true, 0.0),
            b("ward", true, 2.0),
            b("barrier", true, 0.0),
            b("grit", true, 1.0),
            b("burn", false, 0.0),
            b("chill", false, 2.0),
            b("shock", false, 4.0),
            b("frozen", false, 0.0),
            b("defDown", false, 0.0),
            b("silence", false, 0.0),
            b("sealed", false, 0.0),
        ]
    );
    s.fighters[1].attune = 3.0;
    assert_eq!(buffs_of(&s.fighters[1]), [b("attune", true, 1.0)]);
}

#[test]
fn an_unyielding_fighter_that_has_stood_up_shows_it_no_more_a_knocked_out_fighter_shows_nothing() {
    let mut s = arena(1);
    s.fighters[0].rose = true;
    assert!(
        buffs_of(&s.fighters[0])
            .iter()
            .all(|x| x.id != "unyielding")
    );
    s.fighters[1].hp = 0.0;
    s.fighters[1].chill = 2.0;
    assert!(buffs_of(&s.fighters[1]).is_empty());
}
