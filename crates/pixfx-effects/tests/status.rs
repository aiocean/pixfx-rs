//! The status ailments a game plays on a character while it lasts: shocked, burning, frozen, poisoned, petrified.
//! Each is a looping effect that wraps its actor, is listed in [`EFFECTS`] under `feedback/`, and has a golden
//! entry pinning its pixels. Runs with the default features (`all`).

use std::sync::Arc;

use pixfx_core::{Actor, Clip, Layer, Options, render_frames};
use pixfx_draw::{PixelDensity, RULER, frame_hash};
use pixfx_effects::effect;
use serde_json::Value;

const GOLDENS: &str = include_str!("goldens/frame_hashes.json");

/// The ailments and the looping effect that shows each while it lasts.
const AILMENTS: [(&str, &str); 5] = [
    ("shocked", "feedback/electrified"),
    ("burning", "feedback/burning"),
    ("frozen", "feedback/frozen"),
    ("poisoned", "feedback/poisoned"),
    ("petrified", "feedback/petrified"),
];

#[test]
fn each_ailment_has_a_looping_effect_that_wraps_the_actor() {
    let mut missing = Vec::new();
    for (ailment, id) in AILMENTS {
        let Some(fx) = effect(id) else {
            missing.push(format!("{ailment}: no effect `{id}`"));
            continue;
        };
        let meta = fx.meta();
        if !meta.looping {
            missing.push(format!("{ailment}: `{id}` does not loop"));
        }
        if !fx.wraps_actor() {
            missing.push(format!("{ailment}: `{id}` does not wrap its actor"));
        }
        if !meta.tags.contains(&"status") {
            missing.push(format!("{ailment}: `{id}` is not tagged `status`"));
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

#[test]
fn each_ailment_effect_animates_and_has_a_golden() {
    let goldens: Value = serde_json::from_str(GOLDENS).unwrap_or(Value::Null);
    let mut wrong = Vec::new();
    for (ailment, id) in AILMENTS {
        let Some(fx) = effect(id) else {
            wrong.push(format!("{ailment}: no effect `{id}`"));
            continue;
        };
        if goldens.get(id).is_none() {
            wrong.push(format!(
                "{ailment}: `{id}` has no golden in frame_hashes.json"
            ));
        }
        let frames = render_frames(fx, &Options::default(), 0).frames;
        let hashes: Vec<u64> = frames.iter().map(frame_hash).collect();
        if hashes.len() < 2 || hashes.windows(2).all(|w| w[0] == w[1]) {
            wrong.push(format!(
                "{ailment}: `{id}` draws the same pixels on every frame"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_ailment_masks_cover_the_current_fighters_crown_at_both_densities() -> Result<(), String> {
    let height = RULER.height;
    let actor = Arc::new(
        Actor::new(
            -3,
            -(height as i32),
            6,
            height,
            vec![0xffff_ffff; 6 * height as usize],
        )
        .map_err(|e| e.to_string())?,
    );
    for (id, tick) in [
        ("feedback/burning", 0),
        ("feedback/frozen", 4),
        ("feedback/petrified", 12),
        ("feedback/poisoned", 0),
    ] {
        for density in [PixelDensity::One, PixelDensity::Two] {
            let fx = effect(id).ok_or_else(|| format!("{id} is missing"))?;
            let options = Options {
                pixel_density: density,
                ..Options::default()
            };
            let clip = Clip::new(fx, options, 0);
            let mut sim = clip.sim();
            sim.set_actor(Some(Arc::clone(&actor)));
            sim.seek(tick);
            let mut frame = clip.blank();
            clip.draw(&sim, &mut frame, Layer::Front, clip.placement(), 1.0);
            let origin = clip.canvas().origin;
            let factor = density.factor() as usize;
            let x = origin.0 as usize * factor;
            let y = (origin.1 as usize - height as usize + 1) * factor;
            for row in y..y + factor {
                for col in x..x + factor {
                    let pixel = frame
                        .color
                        .get(row * frame.w as usize + col)
                        .copied()
                        .unwrap_or(0);
                    assert_ne!(pixel >> 24, 0, "{id} crown at ({col}, {row}), {density:?}");
                }
            }
        }
    }
    Ok(())
}
