//! Native and wasm32 renders of the same effect, seed and frame hash identically: both compare against one
//! golden file, each effect its own entry, so a build of one effect checks that one. Bless it on a native target
//! only: `PIXFX_BLESS=1 cargo test -p pixfx-effects --test determinism -- --ignored`.

use pixfx_core::{Effect, Options, render_frames};
use pixfx_draw::frame_hash;
use pixfx_effects::EFFECTS;
use serde_json::{Map, Value};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

const GOLDENS: &str = include_str!("goldens/frame_hashes.json");
const SEEDS: [u64; 3] = [1, 2, 7];

/// An effect's hashes: `{ seed: { frame: "hash hex" } }` (hashes are strings: JSON numbers cannot hold a u64
/// exactly in JS tooling).
fn hashes(fx: &'static dyn Effect) -> Result<Value, String> {
    let mut seeds = Map::new();
    for seed in SEEDS {
        let opts = Options {
            seed: Some(seed),
            ..Options::default()
        };
        let frames = render_frames(fx, &opts, 0).frames;
        let n = frames.len();
        // n / 4 as well: crt-off is still blank at frames 0, 1, n / 2 and n - 1
        let mut picks = vec![0, 1, n / 4, n / 2, n.saturating_sub(1)];
        picks.sort_unstable();
        picks.dedup();
        // an effect with a single frame (feedback/outline) has no frame 1
        picks.retain(|&i| i < n);
        let mut by_frame = Map::new();
        for idx in picks {
            let hash = frames.get(idx).map(frame_hash).ok_or("no frame")?;
            by_frame.insert(idx.to_string(), Value::String(format!("{hash:016x}")));
        }
        seeds.insert(seed.to_string(), Value::Object(by_frame));
    }
    Ok(Value::Object(seeds))
}

/// The goldens: `{ id: hashes }`, one entry per effect.
fn goldens() -> Result<Map<String, Value>, String> {
    match serde_json::from_str(GOLDENS).map_err(|e| e.to_string())? {
        Value::Object(m) => Ok(m),
        _ => Err("frame_hashes.json is not an object".to_string()),
    }
}

/// Write the goldens back with the built effects' hashes in place: effects this build leaves out keep theirs.
#[cfg(not(target_arch = "wasm32"))]
fn bless(goldens: &Map<String, Value>) -> Result<(), String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/goldens/frame_hashes.json"
    );
    let json = serde_json::to_string_pretty(goldens).map_err(|e| e.to_string())?;
    std::fs::write(path, json + "\n").map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
fn bless(_goldens: &Map<String, Value>) -> Result<(), String> {
    Ok(())
}

/// Each effect this build has is checked against its own golden, so a build of one effect checks that one alone.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[ignore = "heavy: run before merging an effect"]
fn frame_hashes_match_goldens() -> Result<(), String> {
    let mut goldens = goldens()?;
    let blessing = cfg!(not(target_arch = "wasm32")) && std::env::var_os("PIXFX_BLESS").is_some();
    let mut differ = Vec::new();
    for &fx in EFFECTS {
        let id = fx.meta().id;
        let got = hashes(fx)?;
        if blessing {
            goldens.insert(id.to_string(), got);
        } else if goldens.get(id) != Some(&got) {
            differ.push(match goldens.get(id) {
                Some(want) => format!("{id}: got {got}, golden {want}"),
                None => format!("{id}: no golden (bless it: PIXFX_BLESS=1)"),
            });
        }
    }
    if blessing {
        return bless(&goldens);
    }
    if differ.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "hashes differ from goldens:\n{}",
            differ.join("\n")
        ))
    }
}
