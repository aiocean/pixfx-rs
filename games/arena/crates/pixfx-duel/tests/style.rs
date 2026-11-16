//! Each fighter's style: drawn from the seed on dice of its own, so the rules' dice and fights stay as they were.

mod common;

use common::fight;
use pixfx_duel::style::Style;
use pixfx_duel::view::view;

#[test]
fn styles_are_drawn_from_the_seed() {
    let styles = |seed| fight(seed).state.fighters.clone().map(|f| f.style);
    assert_eq!(styles(4), styles(4));
    let all: Vec<Style> = (1..=40).flat_map(styles).collect();
    for s in Style::ALL {
        assert!(all.contains(&s), "{s:?} drawn in 40 seeds");
    }
    let differs = (1..=40).filter(|&s| styles(s)[0] != styles(s)[1]).count();
    assert!(differs > 10, "the two corners draw on their own: {differs}");
}

#[test]
fn a_style_lasts_the_whole_fight_and_shows_in_the_view() {
    let mut duel = fight(9);
    let before = duel.state.fighters.clone().map(|f| f.style);
    for _ in 0..(90 * 60) {
        duel.step(1.0 / 60.0);
    }
    assert!(duel.state.round > 1, "a round ended");
    assert_eq!(duel.state.fighters.clone().map(|f| f.style), before);
    assert_eq!(view(&duel.state).fighters.map(|f| f.style), before);
}
