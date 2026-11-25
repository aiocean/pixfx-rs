//! Native and wasm32 renders of the same effect, seed and frame hash identically: both compare against one
//! golden file, each effect its own entry, so a build of one effect checks that one. Bless it on a native target
//! only: `PIXFX_BLESS=1 cargo test -p pixfx-effects --test determinism -- --ignored`.

use std::sync::Arc;

use pixfx_core::{Actor, Clip, Effect, Layer, OptValue, Options};
use pixfx_draw::{PixelDensity, frame_hash};
use pixfx_effects::EFFECTS;
use serde_json::{Map, Value};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

const GOLDENS: &str = include_str!("goldens/frame_hashes.json");
const DENSE_GOLDENS: &str = include_str!("goldens/frame_hashes_dense.json");
const SEEDS: [u64; 3] = [1, 2, 7];

const SPRITE_EFFECTS: &[&str] = &[
    "feedback/afterimage",
    "feedback/foot-contact",
    "feedback/hit-flash",
    "feedback/occlusion",
    "feedback/rim-light",
    "feedback/ricochet",
    "feedback/shield-crack",
    "feedback/shield-absorb",
    "feedback/elemental-charge",
    "feedback/arc-link",
    "feedback/mask-ignition",
    "impacts/contact-pulse",
    "impacts/resonance",
    "impacts/aftershock",
    "impacts/directional-wave",
    "impacts/charged-pulse",
    "feedback/sprite-shatter",
    "feedback/telegraph",
    "projectiles/pulse-rings",
    "projectiles/twin-sparks",
    "projectiles/corkscrew",
    "projectiles/moon-windblade",
    "projectiles/aether-pierce",
];

fn test_actor() -> Result<Arc<Actor>, String> {
    let pixels = (0..40)
        .flat_map(|y| {
            (0..20).map(move |x| {
                let body = match y {
                    0..=3 => (6..14).contains(&x),
                    4..=10 => (5..15).contains(&x),
                    11..=19 => (1..19).contains(&x) && (x != 3 || y < 16),
                    20..=32 => (5..15).contains(&x),
                    _ => (5..10).contains(&x) || (11..16).contains(&x),
                };
                if body { 0xff835239 + 3 * x + y } else { 0 }
            })
        })
        .collect();
    Actor::new(-10, -40, 20, 40, pixels)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

/// An effect's hashes: `{ seed: { frame: "hash hex" } }` (hashes are strings: JSON numbers cannot hold a u64
/// exactly in JS tooling).
fn hashes(
    fx: &'static dyn Effect,
    pixel_density: PixelDensity,
    shatter: bool,
) -> Result<Value, String> {
    let mut seeds = Map::new();
    for seed in SEEDS {
        let mut opts = Options {
            seed: Some(seed),
            pixel_density,
            ..Options::default()
        };
        if shatter {
            opts.extra.insert("shatter".into(), OptValue::Num(1.0));
        }
        let clip = Clip::new(fx, opts, 0);
        let n = clip.meta().frames();
        let mut sim = clip.sim();
        if SPRITE_EFFECTS.contains(&fx.meta().id) {
            sim.set_actor(Some(test_actor()?));
        }
        let mut frame = clip.blank();
        let place = clip.placement();
        // n / 4 as well: crt-off is still blank at frames 0, 1, n / 2 and n - 1
        let mut picks = vec![0, 1, n / 4, n / 2, n.saturating_sub(1)];
        picks.sort_unstable();
        picks.dedup();
        // an effect with a single frame (feedback/outline) has no frame 1
        picks.retain(|&i| i < n);
        let mut by_frame = Map::new();
        for idx in picks {
            sim.seek(idx);
            clip.draw(&sim, &mut frame, Layer::Both, place, 1.0);
            let hash = frame_hash(&frame);
            by_frame.insert(idx.to_string(), Value::String(format!("{hash:016x}")));
        }
        seeds.insert(seed.to_string(), Value::Object(by_frame));
    }
    Ok(Value::Object(seeds))
}

/// The goldens: `{ id: hashes }`, one entry per effect.
fn goldens(source: &str) -> Result<Map<String, Value>, String> {
    match serde_json::from_str(source).map_err(|e| e.to_string())? {
        Value::Object(m) => Ok(m),
        _ => Err("frame_hashes.json is not an object".to_string()),
    }
}

/// Write the goldens back with the built effects' hashes in place: effects this build leaves out keep theirs.
#[cfg(not(target_arch = "wasm32"))]
fn bless(name: &str, goldens: &Map<String, Value>) -> Result<(), String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name);
    let json = serde_json::to_string_pretty(goldens).map_err(|e| e.to_string())?;
    std::fs::write(path, json + "\n").map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
fn bless(_name: &str, _goldens: &Map<String, Value>) -> Result<(), String> {
    Ok(())
}

fn check(source: &str, name: &str, density: PixelDensity) -> Result<(), String> {
    let mut goldens = goldens(source)?;
    let blessing = cfg!(not(target_arch = "wasm32")) && std::env::var_os("PIXFX_BLESS").is_some();
    let mut differ = Vec::new();
    for (fx, shatter) in EFFECTS.iter().flat_map(|&fx| {
        [(fx, false), (fx, true)]
            .into_iter()
            .filter(move |&(_, shatter)| !shatter || fx.meta().id == "feedback/shield-crack")
    }) {
        let id = if shatter {
            "feedback/shield-crack:shatter"
        } else {
            fx.meta().id
        };
        let got = hashes(fx, density, shatter)?;
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
        return bless(name, &goldens);
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

/// Each effect this build has is checked against its own golden, so a build of one effect checks that one alone.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[ignore = "heavy: run before merging an effect"]
fn frame_hashes_match_goldens() -> Result<(), String> {
    check(GOLDENS, "frame_hashes.json", PixelDensity::One)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[ignore = "heavy: run before merging an effect"]
fn dense_frame_hashes_match_goldens() -> Result<(), String> {
    check(DENSE_GOLDENS, "frame_hashes_dense.json", PixelDensity::Two)
}
