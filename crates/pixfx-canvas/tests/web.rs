//! The stage's scheduling compiled to wasm and driven with JS options, no DOM needed
//! (run with `wasm-pack test --node crates/pixfx-canvas`). Blitting needs a canvas; the demo covers that.
#![cfg(target_arch = "wasm32")]

use std::num::NonZeroU32;

use js_sys::{Object, Reflect};
use pixfx_canvas::stage::{Scene, color_rgba, light_rgba, live_player, stage_options};
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

#[wasm_bindgen_test]
fn stage_rejects_a_margin_option() -> Result<(), JsValue> {
    for margin in [0, 8] {
        let o = Object::new();
        Reflect::set(&o, &"margin".into(), &JsValue::from(margin))?;
        let Err(e) = stage_options(&o) else {
            return Err("a margin option was accepted".into());
        };
        let text = Reflect::get(&JsValue::from(e), &"message".into())?
            .as_string()
            .unwrap_or_default();
        assert!(
            text.contains("margin"),
            "error does not name margin: {text}"
        );
    }
    let seeded = Object::new();
    Reflect::set(&seeded, &"seed".into(), &JsValue::from(7))?;
    assert!(stage_options(&seeded).is_ok());
    assert!(stage_options(&JsValue::UNDEFINED).is_ok());
    Ok(())
}

/// The message of the error a binding threw.
fn message(e: wasm_bindgen::JsError) -> String {
    Reflect::get(&JsValue::from(e), &"message".into())
        .ok()
        .and_then(|m| m.as_string())
        .unwrap_or_default()
}

/// An object from `(key, value)` pairs, as JS would pass it.
fn object(fields: &[(&str, JsValue)]) -> Result<JsValue, JsValue> {
    let o = Object::new();
    for (k, v) in fields {
        Reflect::set(&o, &JsValue::from_str(k), v)?;
    }
    Ok(o.into())
}

#[wasm_bindgen_test]
fn stage_set_names_a_misspelt_field() -> Result<(), JsValue> {
    use pixfx_canvas::stage::PlacementChange;
    use pixfx_wasm::js::parse;

    let Err(e) = parse::<PlacementChange>("set", &object(&[("flpi", JsValue::TRUE)])?) else {
        return Err("set({flpi}) was accepted".into());
    };
    let text = message(e);
    assert!(text.contains("flpi"), "error does not name flpi: {text}");
    Ok(())
}

#[wasm_bindgen_test]
fn body_spec_names_a_misspelt_field() -> Result<(), JsValue> {
    use pixfx_canvas::fighter::Body;

    let Err(e) = Body::new(0.0, object(&[("gravty", JsValue::from(400))])?) else {
        return Err("new Body(0, {gravty}) was accepted".into());
    };
    let text = message(e);
    assert!(
        text.contains("gravty"),
        "error does not name gravty: {text}"
    );
    Ok(())
}

#[wasm_bindgen_test]
fn light_names_a_misspelt_field() -> Result<(), JsValue> {
    use pixfx_light::Light;
    use pixfx_wasm::js::parse;

    let light = object(&[
        ("x", JsValue::from(1)),
        ("y", JsValue::from(2)),
        ("reach", JsValue::from(9)),
        ("color", JsValue::from_str("#ffcc00")),
        ("strenght", JsValue::from(0.5)),
    ])?;
    let Err(e) = parse::<Light>("light", &light) else {
        return Err("a light with strenght was accepted".into());
    };
    let text = message(e);
    assert!(
        text.contains("strenght"),
        "error does not name strenght: {text}"
    );
    Ok(())
}
