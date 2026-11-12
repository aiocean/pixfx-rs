//! The stage's scheduling compiled to wasm and driven with JS options, no DOM needed
//! (run with `wasm-pack test --node crates/pixfx-canvas`). Blitting needs a canvas; the demo covers that.
#![cfg(target_arch = "wasm32")]

use std::num::NonZeroU32;

use js_sys::{Object, Reflect};
use pixfx_canvas::stage::{Scene, color_rgba, light_rgba, live_player};
use pixfx_core::Layer;
use pixfx_effects::effect;
use pixfx_wasm::{Player, parse_options};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

fn spawn(id: &str, opts: &JsValue) -> Result<Player, JsValue> {
    let fx = effect(id).ok_or("not registered")?;
    Ok(live_player(
        fx,
        parse_options(opts).map_err(JsValue::from)?.0,
    ))
}

#[wasm_bindgen_test]
fn plays_fire_side_by_side_and_stops_them() -> Result<(), JsValue> {
    let mut scene = Scene::new(NonZeroU32::MIN.saturating_add(3));
    let seeded = Object::new();
    Reflect::set(&seeded, &"seed".into(), &JsValue::from(7))?;
    let ids = ["fire/candle", "fire/campfire", "fire/brazier"];
    let mut handles = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let h = scene
            .play(spawn(id, &seeded)?, 60.0 + 100.0 * i as f64, 150.0, ())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        handles.push(h);
    }
    let mut lit_fronts = 0;
    for _ in 0..30 {
        scene
            .update(1.0 / 30.0)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut passes = 0;
        let ok: Result<(), ()> = scene.draw(|pass, frame, _| {
            passes += 1;
            let mut bytes = vec![0u8; frame.w as usize * frame.h as usize * 4];
            let shown = color_rgba(frame, &mut bytes) | light_rgba(frame, &mut bytes);
            if pass.layer == Layer::Front && shown {
                lit_fronts += 1;
            }
            Ok(())
        });
        assert_eq!(ok, Ok(()));
        assert_eq!(passes, 6, "back and front of three effects");
    }
    assert_eq!(
        lit_fronts, 90,
        "every effect shows something in front every frame"
    );
    for h in &handles {
        assert!(scene.stop(*h));
    }
    scene
        .update(0.2)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    assert_eq!(scene.handles().collect::<Vec<_>>(), handles, "still fading");
    scene
        .update(0.2)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    assert_eq!(scene.handles().count(), 0);
    Ok(())
}

#[wasm_bindgen_test]
fn refuses_bad_steps_on_wasm() -> Result<(), JsValue> {
    let mut scene = Scene::new(NonZeroU32::MIN);
    let h = scene
        .play(spawn("fire/candle", &JsValue::UNDEFINED)?, 0.0, 0.0, ())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    assert!(scene.update(f64::NAN).is_err());
    assert!(scene.update(-1.0).is_err());
    assert_eq!(scene.t(h), Some(0.0));
    Ok(())
}
