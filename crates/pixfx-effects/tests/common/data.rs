//! Compare an effect's game data (cues, creature state) with what pixelfx gives.
//!
//! A golden is `crates/pixfx-effects/tests/golden/data/<golden>.json`, written once by
//! `node tools/pixelfx-data.mjs <id> --pixelfx DIR --opts JSON` and committed; the test never runs node.
//! Name it `<id with / as __>__<key>=<value>[,<key>=<value>...]` in the order of the `opts` slice
//! (`critters__bloater-hurt__from=-1,hits=2`), or `<id as __>` alone when there are no options.
//!
//! Shape: `{cues: [{t, type, ...}], state: null | {set, name, order, next, carry}}`. Only these fields are
//! compared, and a field the golden leaves out is null (a sound's `x`). A golden's `act` stays out: pixfx has no
//! acting yet. Numbers match within 1e-9.

use pixfx_core::cue::{Cue, TimedCue, cues_of};
use pixfx_core::effect::{OptValue, Options};
use serde_json::{Map, Value, json};

const EPS: f64 = 1e-9;

/// The options a golden's `opts` mean: `seed`, `flip` (0 or 1) and the effect's own numbers.
fn options_of(opts: &[(&str, f64)]) -> Options {
    let mut o = Options::default();
    for (k, v) in opts {
        match *k {
            "seed" => o.seed = Some(*v as u64),
            "flip" => o.flip = *v != 0.0,
            _ => {
                o.extra.insert((*k).to_string(), OptValue::Num(*v));
            }
        }
    }
    o
}

fn cue_json(c: &TimedCue) -> Value {
    let mut m = match c.cue {
        Cue::Shake { trauma } => json!({"type": "shake", "trauma": trauma}),
        Cue::Hitstop { dur } => json!({"type": "hitstop", "dur": dur}),
        Cue::Ripple {
            x,
            y,
            r0,
            r1,
            dur,
            amp,
        } => {
            json!({"type": "ripple", "x": x, "y": y, "r0": r0, "r1": r1, "dur": dur, "amp": amp})
        }
        Cue::Flash { k, dur } => json!({"type": "flash", "k": k, "dur": dur}),
        Cue::Sound {
            name,
            gain,
            pitch,
            len,
            x,
        } => {
            json!({"type": "sound", "name": name, "gain": gain, "pitch": pitch, "len": len, "x": x})
        }
    };
    m["t"] = json!(c.t);
    m
}

/// What the port says for `id` at `opts`, in the golden's shape.
fn port_data(id: &str, opts: &[(&str, f64)]) -> Result<Value, String> {
    let effect = pixfx_effects::effect(id).ok_or_else(|| format!("no pixfx effect {id}"))?;
    let o = options_of(opts);
    let state = effect.meta().state.map(|s| {
        json!({"set": s.set, "name": s.name, "order": s.order, "next": s.next, "carry": s.carry})
    });
    Ok(json!({
        "cues": cues_of(effect, &o).iter().map(cue_json).collect::<Vec<_>>(),
        "state": state,
    }))
}

/// Keep of `v` only the keys the port models, so the golden's extras (cue `x`, hit `skin`) do not count.
/// A missing `at` is null, as the port's `None`.
fn project(golden: &Value) -> Value {
    let keep = |v: &Value, keys: &[&str]| {
        let mut m = Map::new();
        for k in keys {
            m.insert((*k).to_string(), v.get(*k).cloned().unwrap_or(Value::Null));
        }
        Value::Object(m)
    };
    let cue_keys = |t: &str| -> &'static [&'static str] {
        match t {
            "shake" => &["t", "type", "trauma"],
            "hitstop" => &["t", "type", "dur"],
            "ripple" => &["t", "type", "x", "y", "r0", "r1", "dur", "amp"],
            "flash" => &["t", "type", "k", "dur"],
            _ => &["t", "type", "name", "gain", "pitch", "len", "x"],
        }
    };
    let list = |v: &Value, f: &dyn Fn(&Value) -> Value| -> Value {
        Value::Array(
            v.as_array()
                .map(|a| a.iter().map(f).collect())
                .unwrap_or_default(),
        )
    };
    json!({
        "cues": list(&golden["cues"], &|c| keep(c, cue_keys(c["type"].as_str().unwrap_or("")))),
        "state": golden.get("state").filter(|s| !s.is_null()).map(|s| keep(s, &["set", "name", "order", "next", "carry"])),
    })
}

/// The first difference between `port` and `golden`, as `path: port vs golden`; numbers within [`EPS`].
fn first_diff(path: &str, port: &Value, golden: &Value) -> Option<String> {
    match (port, golden) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) if (a - b).abs() <= EPS => None,
            _ => Some(format!("{path}: {port} vs {golden}")),
        },
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .enumerate()
            .find_map(|(i, (x, y))| first_diff(&format!("{path}[{i}]"), x, y)),
        (Value::Object(a), Value::Object(b)) if a.len() == b.len() => {
            a.iter().find_map(|(k, x)| {
                b.get(k).map_or_else(
                    || Some(format!("{path}.{k}: missing in golden")),
                    |y| first_diff(&format!("{path}.{k}"), x, y),
                )
            })
        }
        _ if port == golden => None,
        _ => Some(format!("{path}: {port} vs {golden}")),
    }
}

/// `Ok` when the port's cues and state for `id` at `opts` equal the golden file `golden`.
pub fn check_data_matches(id: &str, opts: &[(&str, f64)], golden: &str) -> Result<(), String> {
    let path = format!(
        "{}/tests/golden/data/{golden}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).map_err(|e| format!("reading {path}: {e}"))?;
    let golden: Value = serde_json::from_str(&text).map_err(|e| format!("parsing {path}: {e}"))?;
    let port = port_data(id, opts)?;
    match first_diff("$", &port, &project(&golden)) {
        None => Ok(()),
        Some(d) => Err(format!("{id} {opts:?} differs from {path} at {d}")),
    }
}

/// Fail the test unless the port's data for `id` at `opts` matches the golden file `golden`.
pub fn assert_data_matches(id: &str, opts: &[(&str, f64)], golden: &str) {
    let r = check_data_matches(id, opts, golden);
    assert!(r.is_ok(), "{}", r.err().unwrap_or_default());
}
