//! The view shows the page each fighter as the rules hold it.

mod common;

use common::{fight, run};
use pixfx_duel::bar::{buffs_of, skill_bar};
use pixfx_duel::view::view;

#[test]
fn the_view_shows_each_fighter_as_the_rules_hold_it() {
    let mut duel = fight(3);
    let mut casts = 0;
    for _ in 0..30 * 60 {
        run(&mut duel, 1.0 / 60.0);
        let v = view(&duel.state);
        assert_eq!(
            (v.time, v.round, v.wins, v.winner),
            (
                duel.state.time,
                duel.state.round,
                duel.state.wins,
                duel.state.winner
            )
        );
        for (f, s) in v.fighters.iter().zip(&duel.state.fighters) {
            assert_eq!((f.index, f.hp, f.qi, f.rage), (s.index, s.hp, s.qi, s.rage));
            assert_eq!(
                (f.x, f.air, f.face, f.pose, f.pose_t),
                (
                    s.body.x,
                    s.body.air,
                    s.body.face,
                    s.body.pose,
                    s.body.pose_t
                )
            );
            assert_eq!(f.burn, s.burn.is_some());
            assert_eq!(
                f.act.map(|a| (a.spell, a.phase, a.t)),
                s.act.map(|a| (a.spell, a.phase, a.t))
            );
            assert_eq!((&f.skill_bar, &f.buffs), (&skill_bar(s), &buffs_of(s)));
            casts += usize::from(f.act.is_some());
        }
    }
    assert!(casts > 0);
}
