//! Every catalog clip's frames, side east and top-down south, pinned by frame_hash. `PIXFX_BLESS=1` writes them.

#![cfg(feature = "body")]

use std::collections::BTreeMap;

use pixfx_core::body::{Body, Camera, Dir4, Dir8, Layers, Look, catalog, clip_cell, sprite};

const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/body_goldens/frame_hashes.json"
);

fn now() -> BTreeMap<String, Vec<String>> {
    let b = Body::default();
    let mut all = BTreeMap::new();
    for n in catalog::NAMES {
        let Some(c) = catalog::by_name(n) else {
            continue;
        };
        for (tag, cam) in [
            ("side_e", Camera::side(Dir4::E)),
            ("topdown_s", Camera::topdown(Dir8::S)),
        ] {
            let cell = clip_cell(&b, &c, &[cam]);
            let hashes = (0..c.frames)
                .map(|f| {
                    let s = sprite(&b, &c, f, cam, cell, &Look::mannequin(), Layers::ALL);
                    format!("{:016x}", pixfx_draw::frame_hash(&s.frame))
                })
                .collect();
            all.insert(format!("{n}@{tag}"), hashes);
        }
    }
    all
}

#[test]
fn every_clip_paints_its_pinned_frames() {
    let now = now();
    if std::env::var("PIXFX_BLESS").is_ok() {
        let text = serde_json::to_string_pretty(&now).unwrap_or_default();
        assert!(std::fs::write(GOLDEN, text).is_ok());
        return;
    }
    let text = std::fs::read_to_string(GOLDEN).unwrap_or_default();
    let golden: BTreeMap<String, Vec<String>> = serde_json::from_str(&text).unwrap_or_default();
    assert!(
        !golden.is_empty(),
        "no golden at {GOLDEN}: run with PIXFX_BLESS=1"
    );
    for (k, v) in &now {
        assert_eq!(Some(v), golden.get(k), "{k}");
    }
    assert_eq!(now.len(), golden.len());
}
