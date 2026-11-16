//! What the tests share: a duel's state built as `Duel::new` builds it, without its minds or its first round's
//! bodies (each fighter's pool drawn by the duel's dice, Thunder's then Frost's, then three draws each), duels to run,
//! and `pinned`, which checks an output against its golden file.
#![allow(dead_code)]

use pixfx_duel::bar::{draw, fight_pool};
use pixfx_duel::data::{School, school};
use pixfx_duel::fighter::{Fighter, Slot, State};

pub fn arena(seed: u32) -> State {
    let blank = |i, s| Fighter::new(i, s, Vec::new(), Vec::new());
    let mut state = State::new(
        seed,
        0.0,
        220.0,
        150.0,
        96.0,
        Vec::new(),
        [blank(0, School::Thunder), blank(1, School::Frost)],
    );
    for (i, s) in [(0, School::Thunder), (1, School::Frost)] {
        let pool = fight_pool(&mut state, s, &|_| true);
        let d = school(s);
        let spells = d
            .basics
            .iter()
            .copied()
            .chain(pool.iter().copied())
            .chain([d.ultimate])
            .collect();
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
    Slot::Held {
        spell,
        power: 1.0,
        evolved: false,
        rarity: pixfx_duel::data::Rarity::Common,
        traits: Vec::new(),
        idle: 0.0,
    }
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

/// Every effect in the engine's registry, as the page has it.
pub fn all_effects() -> Vec<String> {
    pixfx_effects::EFFECTS
        .iter()
        .map(|e| e.meta().id.to_string())
        .collect()
}

/// Whether effect `fx` loops (and so must be stopped).
pub fn loops(fx: &str) -> bool {
    pixfx_effects::EFFECTS
        .iter()
        .any(|e| e.meta().id == fx && e.meta().looping)
}

/// The landing arena (0 to 220, ground 150) with effects `effects`.
pub fn config(seed: u32, effects: Vec<String>) -> pixfx_duel::duel::DuelConfig {
    pixfx_duel::duel::DuelConfig {
        seed,
        left: 0.0,
        right: 220.0,
        ground: 150.0,
        sky: 96.0,
        effects,
    }
}

/// A duel of two brains with the whole registry.
pub fn fight(seed: u32) -> pixfx_duel::duel::Duel {
    pixfx_duel::duel::Duel::with_brains(config(seed, all_effects()))
}

/// A seat that never acts.
pub fn idle() -> pixfx_duel::seat::MakeSeat {
    Box::new(|_, _| Box::new(pixfx_duel::seat::Script(Box::new(|_, _| None))))
}

/// A seat that casts `skill` once, then never acts again.
pub fn caster(skill: &'static str) -> pixfx_duel::seat::MakeSeat {
    Box::new(move |_, _| {
        let mut cast = false;
        Box::new(pixfx_duel::seat::Script(Box::new(move |_, _| {
            if cast {
                return None;
            }
            cast = true;
            Some(pixfx_mind::Choice::Cast {
                skill: skill.to_string(),
                target: None,
            })
        })))
    })
}

/// A seat that casts `skill` every tick it is asked.
pub fn always(skill: &'static str) -> pixfx_duel::seat::MakeSeat {
    Box::new(move |_, _| {
        Box::new(pixfx_duel::seat::Script(Box::new(move |_, _| {
            Some(pixfx_mind::Choice::Cast {
                skill: skill.to_string(),
                target: None,
            })
        })))
    })
}

/// Step `duel` `seconds` at 60 Hz; returns everything it emitted.
pub fn run(duel: &mut pixfx_duel::duel::Duel, seconds: f64) -> Vec<pixfx_duel::out::Out> {
    let mut out = Vec::new();
    for _ in 0..pixfx_draw::math::js_round(seconds * 60.0) as usize {
        out.extend(duel.step(1.0 / 60.0));
    }
    out
}

/// `got` against `tests/goldens/<name>.json`. A change that alters it fails; when the change is meant, delete the file
/// and run the test once: a missing file is written from `got`, and the test says so.
pub fn pinned(name: &str, got: &serde_json::Value) {
    let file = format!("{}/tests/goldens/{name}.json", env!("CARGO_MANIFEST_DIR"));
    match std::fs::read_to_string(&file) {
        Ok(text) => {
            let want: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
            assert_eq!(
                norm(got),
                norm(&want),
                "{name} changed: if that was meant, delete {file} and run again"
            );
        }
        Err(_) => {
            let text = serde_json::to_string_pretty(got).unwrap_or_default() + "\n";
            assert!(
                std::fs::write(&file, text).is_ok(),
                "could not write {file}"
            );
            eprintln!("wrote {file}: the first run pins it");
        }
    }
}
