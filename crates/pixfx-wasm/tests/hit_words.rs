//! The duel's hit event reaches `FightFeel.react` as JSON and is read there as a `Cue`: a counter or a punish must
//! arrive as one, or its word never floats over the blow (the two crates spell the kind each on their own).
#![cfg(all(feature = "duel", feature = "fighter"))]

use pixfx_duel::out::{How as Dealt, Out};
use pixfx_fighter::feel::{Cue, How as Felt};

fn felt(how: Dealt) -> Option<Felt> {
    let hit = Out::Hit {
        by: 0,
        spell: "slash",
        power: 6.0,
        launch: false,
        ranged: false,
        damage: 120.0,
        crit: false,
        how,
        share: 4.8,
        x: 100.0,
        y: 40.0,
    };
    let json = serde_json::to_value(&hit).ok()?;
    match serde_json::from_value::<Cue>(json).ok()? {
        Cue::Hit { how, .. } => Some(how),
        _ => None,
    }
}

#[test]
fn every_kind_of_blow_the_duel_deals_is_the_kind_the_feel_shows() {
    assert_eq!(felt(Dealt::Plain), Some(Felt::Plain));
    assert_eq!(felt(Dealt::Counter), Some(Felt::Counter));
    assert_eq!(felt(Dealt::Punish), Some(Felt::Punish));
}
