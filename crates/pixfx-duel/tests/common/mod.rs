//! A duel's state built as `createDuel` builds it, without its minds or its first round's bodies: each fighter's
//! pool drawn by the duel's dice (Thunder's, then Frost's), then three draws each. Every effect is in the registry.
//! Replaced by `Duel::new` once the duel loop lands.
#![allow(dead_code)]

use pixfx_duel::bar::{draw, fight_pool};
use pixfx_duel::data::{School, school};
use pixfx_duel::fighter::{Fighter, Slot, State};

pub fn arena(seed: u32) -> State {
    let blank = |i, s| Fighter::new(i, s, Vec::new(), Vec::new());
    let mut state = State::new(seed, 0.0, 220.0, 150.0, 96.0, Vec::new(), [blank(0, School::Thunder), blank(1, School::Frost)]);
    for (i, s) in [(0, School::Thunder), (1, School::Frost)] {
        let pool = fight_pool(&mut state, s, &|_| true);
        let d = school(s);
        let spells = d.basics.iter().copied().chain(pool.iter().copied()).chain([d.ultimate]).collect();
        state.fighters[i] = Fighter::new(i, s, spells, pool);
    }
    // startRound's places: 22% and 78% across the arena
    let span = state.right - state.left;
    for (i, at) in [(0, 0.22), (1, 0.78)] {
        state.fighters[i].body.start_round(state.left + span * at);
    }
    for f in 0..2 {
        for i in 0..3 {
            draw(&mut state, f, i, i == 0, None);
        }
    }
    state
}

/// A common, plain bar slot holding `spell`.
pub fn common(spell: &'static str) -> Slot {
    Slot::Held { spell, power: 1.0, evolved: false, rarity: pixfx_duel::data::Rarity::Common, traits: Vec::new(), idle: 0.0 }
}

pub fn empty(left: f64) -> Slot {
    Slot::Empty { left, full: left }
}

/// The skills on fighter `f`'s bar.
pub fn on_bar(f: &Fighter) -> Vec<&'static str> {
    f.bar.iter().filter_map(Slot::spell).collect()
}

/// Every number as an f64, so 1 and 1.0 compare equal.
pub fn norm(v: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Number(n) => serde_json::json!(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(a) => Value::Array(a.iter().map(norm).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), norm(v))).collect()),
        other => other.clone(),
    }
}

/// Every effect the duel's rules ask for, as if the registry held them all.
pub fn all_effects() -> Vec<String> {
    use pixfx_duel::data::{CAST_CIRCLE, CLASH, DECOR, spells};
    let mut out: Vec<String> = vec![CAST_CIRCLE.into(), CLASH.into(), "fire/spirit-flame".into()];
    out.extend(DECOR.iter().map(|d| d.to_string()));
    for s in spells() {
        out.extend([s.fx, s.hit_fx, s.end_fx, s.slam_fx].into_iter().flatten().map(String::from));
    }
    out.sort();
    out.dedup();
    out
}
