//! The JS-facing engine, exercised as JS would use it (run with `wasm-pack test --node crates/pixfx-wasm`).
#![cfg(target_arch = "wasm32")]

use js_sys::{Array, Object, Reflect};
use pixfx_core::{Options, render_frames};
use pixfx_effects::effect;
use pixfx_wasm::{Fx, effects, meta};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

fn obj(fields: &[(&str, JsValue)]) -> Result<JsValue, JsValue> {
    let o = Object::new();
    for (k, v) in fields {
        Reflect::set(&o, &JsValue::from_str(k), v)?;
    }
    Ok(o.into())
}

fn get(o: &JsValue, k: &str) -> Result<JsValue, JsValue> {
    Reflect::get(o, &JsValue::from_str(k))
}

#[wasm_bindgen_test]
fn lists_fire_effects() {
    let ids: Vec<String> = effects().iter().filter_map(|v| v.as_string()).collect();
    for id in ["fire/candle", "fire/campfire", "fire/brazier"] {
        assert!(ids.iter().any(|i| i == id), "{id} missing from {ids:?}");
    }
}

#[wasm_bindgen_test]
fn meta_matches() -> Result<(), JsValue> {
    let fx = effect("fire/campfire").ok_or("campfire not registered")?;
    let want = fx.meta();
    let m = meta("fire/campfire").map_err(JsValue::from)?;
    let size = Array::from(&get(&m, "size")?);
    assert_eq!(size.get(0).as_f64(), Some(f64::from(want.size.0)));
    assert_eq!(size.get(1).as_f64(), Some(f64::from(want.size.1)));
    assert_eq!(get(&m, "fps")?.as_f64(), Some(f64::from(want.fps)));
    assert_eq!(get(&m, "id")?.as_string().as_deref(), Some("fire/campfire"));
    assert_eq!(get(&m, "loop")?.as_bool(), Some(want.looping));
    Ok(())
}

#[wasm_bindgen_test]
fn frames_match_render_frames() -> Result<(), JsValue> {
    let fx = effect("fire/candle").ok_or("candle not registered")?;
    let opts = Options {
        seed: Some(1),
        ..Options::default()
    };
    let want = render_frames(fx, &opts, 0).frames;
    let fps = f64::from(fx.meta().fps);
    let mut live =
        Fx::new("fire/candle", obj(&[("seed", JsValue::from(1))])?).map_err(JsValue::from)?;
    assert_eq!((live.width(), live.height()), fx.meta().size);
    let mut stepped = 0;
    for n in [0usize, 1, 60] {
        while stepped < n {
            live.update(1.0 / fps);
            stepped += 1;
        }
        live.draw("both").map_err(JsValue::from)?;
        let frame = want.get(n).ok_or("render has too few frames")?;
        assert_eq!(
            live.color().to_vec(),
            frame.color,
            "colour differs at frame {n}"
        );
        assert_eq!(
            live.light().to_vec(),
            frame.light,
            "light differs at frame {n}"
        );
    }
    Ok(())
}

#[wasm_bindgen_test]
fn unknown_id_errors() {
    assert!(Fx::new("fire/nope", JsValue::UNDEFINED).is_err());
    assert!(meta("fire/nope").is_err());
}

#[wasm_bindgen_test]
fn wrong_option_type_errors() -> Result<(), JsValue> {
    let bad = obj(&[("flip", JsValue::from_str("yes"))])?;
    let Err(e) = Fx::new("fire/candle", bad) else {
        return Err("a string flip was accepted".into());
    };
    let msg = JsValue::from(e);
    let text = get(&msg, "message")?.as_string().unwrap_or_default();
    assert!(
        text.contains("flip"),
        "error does not name the field: {text}"
    );
    Ok(())
}

#[wasm_bindgen_test]
fn rgba_is_frame_sized() -> Result<(), JsValue> {
    let mut live = Fx::new("fire/campfire", JsValue::UNDEFINED).map_err(JsValue::from)?;
    live.update(0.1);
    live.draw("both").map_err(JsValue::from)?;
    let px = live.rgba();
    assert_eq!(px.length(), live.width() * live.height() * 4);
    assert!(
        px.to_vec().chunks(4).any(|p| p[3] > 0),
        "a lit campfire drew nothing"
    );
    Ok(())
}
