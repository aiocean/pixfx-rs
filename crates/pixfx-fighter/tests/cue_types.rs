//! `Cue::TYPES` is the list a host checks an event's `type` against before reading it as a cue: every name in it
//! must read as the cue of that name, fields other parts of the game read off the same event let be.
#![cfg(feature = "serde")]

use pixfx_fighter::feel::{Cue, How};

/// The `type` a cue is written with; a new variant fails to compile here until it is named (and listed in TYPES).
fn type_of(c: &Cue) -> &'static str {
    match c {
        Cue::Hit { .. } => "hit",
        Cue::Tick { .. } => "tick",
        Cue::Heal { .. } => "heal",
        Cue::Ward { .. } => "ward",
        Cue::Dodge { .. } => "dodge",
        Cue::Unyielding => "unyielding",
        Cue::Block => "block",
        Cue::Clash => "clash",
        Cue::Land { .. } => "land",
        Cue::Bump { .. } => "bump",
        Cue::Ko => "ko",
        Cue::Streak { .. } => "streak",
        Cue::Ultimate => "ultimate",
    }
}

#[test]
fn every_listed_type_reads_as_its_cue() -> Result<(), serde_json::Error> {
    for ty in Cue::TYPES {
        // every field any cue reads, and one only the game reads (`spell`)
        let event = serde_json::json!({
            "type": ty, "by": 1, "power": 9, "launch": false, "crit": true, "ranged": false, "share": 12.5,
            "damage": 7, "amount": 3, "x": 10.5, "y": 20, "strength": 0.5, "x0": 1, "y0": 2, "x1": 3, "y1": 4,
            "spell": "bolt",
        });
        let cue: Cue = serde_json::from_value(event)?;
        assert_eq!(type_of(&cue), ty);
    }
    Ok(())
}

/// The fight's hit says how it landed; a hit written before that (no `how`) reads as a plain one.
#[test]
fn a_hit_reads_how_it_landed_and_plain_when_it_does_not_say() -> Result<(), serde_json::Error> {
    let how = |extra: serde_json::Value| -> Result<Option<How>, serde_json::Error> {
        let mut event = serde_json::json!({
            "type": "hit", "by": 0, "power": 9, "launch": false, "crit": false, "ranged": false, "share": 10,
            "damage": 100, "x": 1, "y": 2,
        });
        if let (Some(e), Some(x)) = (event.as_object_mut(), extra.as_object()) {
            e.extend(x.clone());
        }
        let cue: Cue = serde_json::from_value(event)?;
        Ok(match cue {
            Cue::Hit { how, .. } => Some(how),
            _ => None,
        })
    };
    assert_eq!(how(serde_json::json!({}))?, Some(How::Plain));
    assert_eq!(how(serde_json::json!({"how": "plain"}))?, Some(How::Plain));
    assert_eq!(
        how(serde_json::json!({"how": "counter"}))?,
        Some(How::Counter)
    );
    assert_eq!(
        how(serde_json::json!({"how": "punish"}))?,
        Some(How::Punish)
    );
    assert!(how(serde_json::json!({"how": "sideways"})).is_err());
    Ok(())
}
