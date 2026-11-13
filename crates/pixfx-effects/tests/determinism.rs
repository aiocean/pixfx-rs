//! Native and wasm32 renders of the same effect, seed and frame hash identically: both compare against one
//! golden file. Bless it on a native target only: `PIXFX_BLESS=1 cargo test -p pixfx-effects --test determinism`.

use pixfx_core::{Options, frame_hash, render_frames};
use pixfx_effects::effect;
use std::fmt::Write;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

const GOLDENS: &str = include_str!("goldens/frame_hashes.json");
const IDS: [&str; 17] = [
    "fire/candle",
    "fire/campfire",
    "fire/brazier",
    "fire/spirit-flame",
    "impacts/parry",
    "explosions/dirt-explosion",
    "explosions/small-explosion",
    "explosions/implosion",
    "explosions/plasma-explosion",
    "arcane/rune-circle",
    "critters/wisp",
    "spawn/poof",
    "holy/feather-burst",
    "ice/icicle-fall",
    "scifi/jetpack",
    "lightning/ball-lightning",
    "nature/poison-cloud",
];
const SEEDS: [u64; 3] = [1, 2, 7];

/// `{ id: { seed: { frame: "hash hex" } } }`, pretty-printed in a fixed order (hashes are strings: JSON
/// numbers cannot hold a u64 exactly in JS tooling).
fn current_hashes() -> Result<String, String> {
    let mut out = String::from("{\n");
    for (i, id) in IDS.iter().enumerate() {
        let fx = effect(id).ok_or(format!("{id} is not registered"))?;
        let _ = writeln!(out, "  \"{id}\": {{");
        for (j, seed) in SEEDS.iter().enumerate() {
            let opts = Options {
                seed: Some(*seed),
                ..Options::default()
            };
            let frames = render_frames(fx, &opts, 0).frames;
            let n = frames.len();
            let _ = writeln!(out, "    \"{seed}\": {{");
            let picks = [0, 1, n / 2, n - 1];
            for (k, idx) in picks.iter().enumerate() {
                let hash = frames.get(*idx).map(frame_hash).ok_or("no frame")?;
                let comma = if k + 1 < picks.len() { "," } else { "" };
                let _ = writeln!(out, "      \"{idx}\": \"{hash:016x}\"{comma}");
            }
            let _ = writeln!(out, "    }}{}", if j + 1 < SEEDS.len() { "," } else { "" });
        }
        let _ = writeln!(out, "  }}{}", if i + 1 < IDS.len() { "," } else { "" });
    }
    out.push_str("}\n");
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn bless(json: &str) -> Result<(), String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/goldens/frame_hashes.json"
    );
    std::fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
fn bless(_json: &str) -> Result<(), String> {
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn frame_hashes_match_goldens() -> Result<(), String> {
    let json = current_hashes()?;
    if cfg!(not(target_arch = "wasm32")) && std::env::var_os("PIXFX_BLESS").is_some() {
        bless(&json)?;
        return Ok(());
    }
    if json == GOLDENS {
        return Ok(());
    }
    let first = json.lines().zip(GOLDENS.lines()).find(|(a, b)| a != b);
    Err(format!(
        "hashes differ from goldens; first differing line (got, golden): {first:?}"
    ))
}
