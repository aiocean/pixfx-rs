//! The JS-facing engine, exercised as JS would use it (run with `wasm-pack test --node crates/pixfx-wasm`).
#![cfg(target_arch = "wasm32")]

use js_sys::{Array, Object, Reflect};
use pixfx_core::{Layer, Options, render_frames};
use pixfx_effects::effect;
use pixfx_wasm::{Fx, Player, effects, meta};
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
            live.update(1.0 / fps).map_err(JsValue::from)?;
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
    live.update(0.1).map_err(JsValue::from)?;
    live.draw("both").map_err(JsValue::from)?;
    let px = live.rgba();
    assert_eq!(px.length(), live.width() * live.height() * 4);
    assert!(
        px.to_vec().chunks(4).any(|p| p[3] > 0),
        "a lit campfire drew nothing"
    );
    Ok(())
}

#[wasm_bindgen_test]
fn bad_dt_errors_and_leaves_time_alone() -> Result<(), JsValue> {
    let mut live = Fx::new("fire/candle", JsValue::UNDEFINED).map_err(JsValue::from)?;
    live.update(0.1).map_err(JsValue::from)?;
    let before = live.t();
    let Err(e) = live.update(f64::NAN) else {
        return Err("update(NaN) was accepted".into());
    };
    let text = get(&JsValue::from(e), "message")?
        .as_string()
        .unwrap_or_default();
    assert!(text.contains("dt"), "error does not name dt: {text}");
    assert_eq!(live.t(), before, "update(NaN) moved time");
    assert!(
        live.update(f64::INFINITY).is_err(),
        "update(Infinity) was accepted"
    );
    assert!(live.update(-1.0).is_err(), "update(-1) was accepted");
    assert_eq!(live.t(), before, "a rejected update moved time");
    live.update(0.0).map_err(JsValue::from)?;
    assert_eq!(live.t(), before);
    Ok(())
}

fn error_text(e: impl Into<JsValue>) -> String {
    get(&e.into(), "message")
        .ok()
        .and_then(|m| m.as_string())
        .unwrap_or_default()
}

#[wasm_bindgen_test]
fn margin_option_grows_the_canvas() -> Result<(), JsValue> {
    let fx = effect("fire/campfire").ok_or("campfire not registered")?;
    let want = fx.meta();
    let m = get(&meta("fire/campfire").map_err(JsValue::from)?, "glowMargin")?
        .as_f64()
        .ok_or("meta has no glowMargin")? as u32;
    assert_eq!(m, want.glow_margin());
    let live =
        Fx::new("fire/campfire", obj(&[("margin", JsValue::from(m))])?).map_err(JsValue::from)?;
    assert_eq!(live.width(), want.size.0 + 2 * m);
    assert_eq!(live.height(), want.size.1 + 2 * m);
    assert_eq!(live.origin_x(), want.origin.0 + m as i32);
    assert_eq!(live.origin_y(), want.origin.1 + m as i32);
    Ok(())
}

#[wasm_bindgen_test]
fn margin_frames_match_player() -> Result<(), JsValue> {
    let fx = effect("fire/campfire").ok_or("campfire not registered")?;
    let m = fx.meta().glow_margin();
    let mut want = Player::new(fx, Options::default(), m);
    let mut live =
        Fx::new("fire/campfire", obj(&[("margin", JsValue::from(m))])?).map_err(JsValue::from)?;
    for _ in 0..3 {
        want.update(1.0 / 30.0);
        live.update(1.0 / 30.0).map_err(JsValue::from)?;
    }
    want.draw(Layer::Both);
    live.draw("both").map_err(JsValue::from)?;
    assert_eq!(live.color().to_vec(), want.frame().color);
    assert_eq!(live.light().to_vec(), want.frame().light);
    Ok(())
}

#[wasm_bindgen_test]
fn bad_margin_errors() -> Result<(), JsValue> {
    for bad in [
        JsValue::from(-1),
        JsValue::from(1.5),
        JsValue::from_str("x"),
        JsValue::from(4097),
        JsValue::from(4_294_967_295u32),
    ] {
        let Err(e) = Fx::new("fire/candle", obj(&[("margin", bad.clone())])?) else {
            return Err(format!("margin {bad:?} was accepted").into());
        };
        let text = error_text(e);
        assert!(
            text.contains("margin"),
            "error does not name margin: {text}"
        );
    }
    Ok(())
}

#[wasm_bindgen_test]
fn default_margin_unchanged() -> Result<(), JsValue> {
    let fx = effect("fire/candle").ok_or("candle not registered")?;
    let mut want = Player::new(fx, Options::default(), 0);
    let mut live = Fx::new("fire/candle", JsValue::UNDEFINED).map_err(JsValue::from)?;
    assert_eq!((live.width(), live.height()), fx.meta().size);
    assert_eq!(live.origin_x(), fx.meta().origin.0);
    want.update(0.1);
    live.update(0.1).map_err(JsValue::from)?;
    want.draw(Layer::Both);
    live.draw("both").map_err(JsValue::from)?;
    assert_eq!(live.color().to_vec(), want.frame().color);
    assert_eq!(live.light().to_vec(), want.frame().light);
    Ok(())
}

#[wasm_bindgen_test]
fn margin_cap_is_accepted() -> Result<(), JsValue> {
    let live =
        Fx::new("fire/candle", obj(&[("margin", JsValue::from(4096))])?).map_err(JsValue::from)?;
    assert_eq!(live.width(), 56 + 2 * 4096);
    Ok(())
}

#[wasm_bindgen_test]
fn hex_string_option_is_a_colour() -> Result<(), JsValue> {
    use pixfx_core::{OptValue, Rgba};
    use pixfx_wasm::parse_options;

    let (opts, _) =
        parse_options(&obj(&[("stone", JsValue::from_str("#ff0000"))])?).map_err(JsValue::from)?;
    assert_eq!(
        opts.extra.get("stone"),
        Some(&OptValue::Color(Rgba(255, 0, 0, 255)))
    );
    let Err(e) = parse_options(&obj(&[("stone", JsValue::from_str("#zz0000"))])?) else {
        return Err("a bad hex colour was accepted".into());
    };
    let text = error_text(e);
    assert!(
        text.contains("stone"),
        "error does not name the field: {text}"
    );
    let Err(e) = parse_options(&obj(&[("stone", JsValue::from(true))])?) else {
        return Err("a boolean option was accepted".into());
    };
    let text = error_text(e);
    assert!(
        text.contains("a hex colour"),
        "error does not list hex colours: {text}"
    );

    // the colour reaches the effect: the rocks and dust of small-explosion change
    let id = "explosions/small-explosion";
    let fx = effect(id).ok_or("small-explosion not registered")?;
    let mut want = Player::new(fx, opts, 0);
    let mut plain = Player::new(fx, Options::default(), 0);
    let mut live =
        Fx::new(id, obj(&[("stone", JsValue::from_str("#ff0000"))])?).map_err(JsValue::from)?;
    for _ in 0..30 {
        want.update(1.0 / 30.0);
        plain.update(1.0 / 30.0);
        live.update(1.0 / 30.0).map_err(JsValue::from)?;
    }
    want.draw(Layer::Both);
    plain.draw(Layer::Both);
    live.draw("both").map_err(JsValue::from)?;
    assert_eq!(live.color().to_vec(), want.frame().color);
    assert_ne!(
        want.frame().color,
        plain.frame().color,
        "stone changed nothing"
    );
    Ok(())
}
