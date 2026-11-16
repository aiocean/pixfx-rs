//! Every preset at every facing keeps the joints it had when the rig moved to pixfx-body. `PIXFX_BLESS=1` writes
//! the golden; otherwise each joint must match it.

use pixfx_pose::{Body, facing, presets, skeleton};
use serde_json::{Map, Value};

const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/skeletons.json");

fn now() -> Value {
    let body = Body::default();
    let mut all = Map::new();
    for name in presets::NAMES {
        let Some(pose) = presets::by_name(name, 0.25) else {
            continue;
        };
        for f in facing::ALL {
            let s = skeleton(&body, &pose, f);
            all.insert(
                format!("{name}@{f}"),
                serde_json::to_value(s).unwrap_or(Value::Null),
            );
        }
    }
    Value::Object(all)
}

/// Every number in `a` and `b` within 1e-9, keys of `b` only (new fields may join `a`).
fn same(a: &Value, b: &Value, at: &str) -> Result<(), String> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (
                x.as_f64().unwrap_or(f64::NAN),
                y.as_f64().unwrap_or(f64::NAN),
            );
            if (x - y).abs() <= 1e-9 {
                Ok(())
            } else {
                Err(format!("{at}: {x} != {y}"))
            }
        }
        (Value::Object(x), Value::Object(y)) => y.iter().try_for_each(|(k, v)| {
            x.get(k).map_or(Err(format!("{at}.{k} missing")), |w| {
                same(w, v, &format!("{at}.{k}"))
            })
        }),
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => x
            .iter()
            .zip(y)
            .enumerate()
            .try_for_each(|(i, (w, v))| same(w, v, &format!("{at}[{i}]"))),
        _ if a == b => Ok(()),
        _ => Err(format!("{at}: {a} != {b}")),
    }
}

#[test]
fn every_preset_at_every_facing_keeps_its_joints() {
    let now = now();
    if std::env::var("PIXFX_BLESS").is_ok() {
        let text = serde_json::to_string_pretty(&now).unwrap_or_default();
        assert!(std::fs::write(GOLDEN, text).is_ok());
        return;
    }
    let text = std::fs::read_to_string(GOLDEN).unwrap_or_default();
    let golden: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    assert!(
        golden.is_object(),
        "no golden at {GOLDEN}: run with PIXFX_BLESS=1"
    );
    let diff = same(&now, &golden, "");
    assert!(diff.is_ok(), "{diff:?}");
}
