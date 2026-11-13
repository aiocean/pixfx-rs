//! The screen transitions are registered under their pixelfx ids with pixelfx's meta
//! (`src/fx/transitions/*.js`).

use pixfx_core::{Meta, OptValue, Options, frame_hash, render_frames};
use pixfx_effects::effect;

fn meta(id: &str) -> Option<Meta> {
    effect(id).map(|e| *e.meta())
}

#[test]
fn burn_away_meta_matches_pixelfx() {
    let m = meta("transitions/burn-away");
    assert!(m.is_some(), "transitions/burn-away is not registered");
    assert_eq!(m.map(|m| m.size), Some((320, 180)));
    assert_eq!(m.map(|m| m.origin), Some((0, 0)));
    assert_eq!(m.map(|m| m.duration), Some(3.0));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(3));
    assert_eq!(m.map(|m| m.is_screen()), Some(true));
}

/// Frame hashes of burn-away rendered with these hole options (none given: pixelfx's own hole).
fn burn_away_hashes(hole: &[(&str, f64)]) -> Result<Vec<u64>, String> {
    let fx = effect("transitions/burn-away").ok_or("transitions/burn-away is not registered")?;
    let mut opts = Options::default();
    for (k, v) in hole {
        opts.extra.insert((*k).to_string(), OptValue::Num(*v));
    }
    Ok(render_frames(fx, &opts, 0)
        .frames
        .iter()
        .map(frame_hash)
        .collect())
}

#[test]
fn burn_away_hole_follows_hole_options() -> Result<(), String> {
    let default = burn_away_hashes(&[])?;
    // the hole opens elsewhere: by frame 60 (2 s) it is burning open round (40, 30)
    let moved = burn_away_hashes(&[("holeX", 40.0), ("holeY", 30.0)])?;
    assert_ne!(moved.get(60), default.get(60));
    // the cover burns in the same way: the hole options only move where it opens
    assert_eq!(moved.get(30), default.get(30));
    // pixelfx's own hole centre for seed 3 (outField: new Rng((3 * 13 + 1) * 31 + 5), node at 044e12a)
    let same = burn_away_hashes(&[("holeX", 172.76643117144704), ("holeY", 87.36016465723515)])?;
    assert_eq!(same, default);
    Ok(())
}
