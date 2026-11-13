//! Native and wasm32 renders of the same effect, seed and frame hash identically: both compare against one
//! golden file. Bless it on a native target only: `PIXFX_BLESS=1 cargo test -p pixfx-effects --test determinism -- --ignored`.

use pixfx_core::{Options, render_frames};
use pixfx_draw::frame_hash;
use pixfx_effects::EFFECTS;
use std::fmt::Write;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

const GOLDENS: &str = include_str!("goldens/frame_hashes.json");
const SEEDS: [u64; 3] = [1, 2, 7];

/// `{ id: { seed: { frame: "hash hex" } } }`, pretty-printed in a fixed order (hashes are strings: JSON
/// numbers cannot hold a u64 exactly in JS tooling).
fn current_hashes() -> Result<String, String> {
    let mut out = String::from("{\n");
    for (i, &fx) in EFFECTS.iter().enumerate() {
        let id = fx.meta().id;
        let _ = writeln!(out, "  \"{id}\": {{");
        for (j, seed) in SEEDS.iter().enumerate() {
            let opts = Options {
                seed: Some(*seed),
                ..Options::default()
            };
            let frames = render_frames(fx, &opts, 0).frames;
            let n = frames.len();
            let _ = writeln!(out, "    \"{seed}\": {{");
            // n / 4 as well: crt-off is still blank at frames 0, 1, n / 2 and n - 1
            let mut picks = vec![0, 1, n / 4, n / 2, n - 1];
            picks.dedup();
            for (k, idx) in picks.iter().enumerate() {
                let hash = frames.get(*idx).map(frame_hash).ok_or("no frame")?;
                let comma = if k + 1 < picks.len() { "," } else { "" };
                let _ = writeln!(out, "      \"{idx}\": \"{hash:016x}\"{comma}");
            }
            let _ = writeln!(out, "    }}{}", if j + 1 < SEEDS.len() { "," } else { "" });
        }
        let _ = writeln!(out, "  }}{}", if i + 1 < EFFECTS.len() { "," } else { "" });
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
#[ignore = "heavy: run before merging an effect"]
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
