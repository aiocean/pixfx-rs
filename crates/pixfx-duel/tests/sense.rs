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
