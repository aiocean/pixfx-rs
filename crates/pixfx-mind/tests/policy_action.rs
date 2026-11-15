//! What the policy can do: a fixed list of acts, masked to what can happen now, each a `Choice`.

mod common;

use common::*;
use pixfx_mind::policy::{ACTIONS, Act, Aim, SLOTS, choice, legal};
use pixfx_mind::*;

/// The kit's slots: slash, nothing, blink, bolt, icicle, nothing, prison.
fn slotted(mut s: Sense) -> Sense {
    s.slots = vec![Some(0), None, Some(2), Some(1), Some(3), None, Some(6)];
    s
}

#[test]
fn every_index_round_trips_through_act() {
    assert_eq!(ACTIONS, 9 + SLOTS * 3);
    for i in 0..ACTIONS {
        let a = Act::from_index(i).unwrap_or(Act::Wait);
        assert_eq!(a.index(), i, "{a:?}");
    }
    assert_eq!(Act::from_index(ACTIONS), None);
}

#[test]
fn wait_is_always_legal_and_casts_need_a_ready_filled_slot() {
    let kit = kit();
    let mut s = slotted(sense(1.0, me(40.0), foe(150.0)));
    s.ready[1] = false; // the bolt, in slot 3
    let l = legal(&s, &kit);
    assert!(l[Act::Wait.index()]);
    assert!(
        l[Act::Cast {
            slot: 0,
            aim: Aim::Foe
        }
        .index()],
        "slash"
    );
    assert!(
        !l[Act::Cast {
            slot: 1,
            aim: Aim::Foe
        }
        .index()],
        "an empty slot"
    );
    assert!(
        !l[Act::Cast {
            slot: 3,
            aim: Aim::Foe
        }
        .index()],
        "a bolt that is not ready"
    );
    s.me.can_act = false;
    let l = legal(&s, &kit);
    assert_eq!(
        l.iter().filter(|&&b| b).count(),
        1,
        "only waiting while it cannot act"
    );
    assert!(l[Act::Wait.index()]);
}

#[test]
fn flying_turns_fly_high_into_land() {
    let kit = kit();
    let mut s = slotted(sense(1.0, me(40.0), foe(150.0)));
    let l = legal(&s, &kit);
    assert!(l[Act::FlyHigh.index()] && !l[Act::Land.index()]);
    s.me.flying = true;
    s.me.air = 30.0;
    let l = legal(&s, &kit);
    assert!(!l[Act::FlyHigh.index()] && l[Act::Land.index()] && !l[Act::Jump.index()]);
}

#[test]
fn only_areas_and_blinks_take_ahead_or_behind() {
    let kit = kit();
    let s = slotted(sense(1.0, me(40.0), foe(150.0)));
    let l = legal(&s, &kit);
    for (slot, aims) in [(0, false), (2, true), (3, false), (4, true), (6, true)] {
        assert!(
            l[Act::Cast {
                slot,
                aim: Aim::Foe
            }
            .index()],
            "slot {slot}"
        );
        assert_eq!(
            l[Act::Cast {
                slot,
                aim: Aim::Ahead
            }
            .index()],
            aims,
            "slot {slot}"
        );
        assert_eq!(
            l[Act::Cast {
                slot,
                aim: Aim::Behind
            }
            .index()],
            aims,
            "slot {slot}"
        );
    }
}

#[test]
fn a_cast_maps_to_the_slot_skills_name_and_aim() {
    let kit = kit();
    let s = slotted(sense(1.0, me(40.0), foe(150.0)));
    let f = foe(150.0);
    let c = |a| choice(a, &s, &f, &kit);
    assert_eq!(
        c(Act::Cast {
            slot: 3,
            aim: Aim::Foe
        }),
        Choice::Cast {
            skill: "bolt".into(),
            target: None
        }
    );
    assert_eq!(
        c(Act::Cast {
            slot: 4,
            aim: Aim::Foe
        }),
        Choice::Cast {
            skill: "icicle".into(),
            target: Some(150.0)
        }
    );
    assert_eq!(
        c(Act::Cast {
            slot: 4,
            aim: Aim::Ahead
        }),
        Choice::Cast {
            skill: "icicle".into(),
            target: Some(180.0)
        }
    );
    assert_eq!(
        c(Act::Cast {
            slot: 4,
            aim: Aim::Behind
        }),
        Choice::Cast {
            skill: "icicle".into(),
            target: Some(120.0)
        }
    );
    assert_eq!(c(Act::Toward), Choice::Move { dir: 1.0 });
    assert_eq!(c(Act::Away), Choice::Move { dir: -1.0 });
    assert_eq!(c(Act::FlyHigh), Choice::Fly { height: 96.0 * 0.6 });
    assert_eq!(c(Act::Land), Choice::Fly { height: 0.0 });
    // the foe on the left: toward is left
    let s2 = slotted(sense(1.0, me(150.0), foe(40.0)));
    assert_eq!(
        choice(Act::Toward, &s2, &foe(40.0), &kit),
        Choice::Move { dir: -1.0 }
    );
    // an aim past the wall stays in the arena
    let s3 = slotted(sense(1.0, me(40.0), foe(215.0)));
    assert_eq!(
        choice(
            Act::Cast {
                slot: 4,
                aim: Aim::Ahead
            },
            &s3,
            &foe(215.0),
            &kit
        ),
        Choice::Cast {
            skill: "icicle".into(),
            target: Some(220.0)
        }
    );
}
