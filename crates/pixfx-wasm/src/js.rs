//! The one way values cross from JS into Rust and back: [`parse`] reads a JS value strictly into a serde type
//! (an unknown, missing or mistyped field is an error naming it), [`to_js`] writes one out as plain JS data.

use js_sys::{Array, Object};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Number, Value};
use wasm_bindgen::{JsCast, JsError, JsValue};

/// Largest integer a JS number holds exactly (`Number.MAX_SAFE_INTEGER`).
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// `v` as a `T`, read strictly; `what` names the value in the error (`"body spec"`). A key whose value is
/// `undefined` counts as absent; numbers must be finite.
///
/// The value is first copied into plain data (every own key of every object, so `deny_unknown_fields` sees
/// misspelt keys), then deserialized with the path to any failing field in the error: `body spec.gravty:
/// unknown field ...`.
pub fn parse<T: DeserializeOwned>(what: &str, v: &JsValue) -> Result<T, JsError> {
    let mut path = String::new();
    let data = plain(v, &mut path).map_err(|e| JsError::new(&format!("{what}{path}: {e}")))?;
    serde_path_to_error::deserialize(data).map_err(|e| {
        let at = e.path().to_string();
        let inner = e.into_inner();
        if at == "." {
            JsError::new(&format!("{what}: {inner}"))
        } else {
            JsError::new(&format!("{what}.{at}: {inner}"))
        }
    })
}

/// `v` as plain JS data: objects for structs, `null` for `None`, numbers exactly as the `f64`s they are.
pub fn to_js<T: Serialize + ?Sized>(v: &T) -> Result<JsValue, JsError> {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsError::new(&format!("could not build a JS value: {e}")))
}

/// A JS value as plain data, `path` growing as it descends (`.motion.fly`, `[2]`) so an error says where.
/// Whole numbers JS holds exactly become integers (so they read as `u32` as well as `f64`); `-0` and fractions stay
/// `f64`, bit for bit.
fn plain(v: &JsValue, path: &mut String) -> Result<Value, String> {
    if v.is_undefined() || v.is_null() {
        return Ok(Value::Null);
    }
    if let Some(b) = v.as_bool() {
        return Ok(Value::Bool(b));
    }
    if let Some(n) = v.as_f64() {
        return number(n).map(Value::Number);
    }
    if let Some(s) = v.as_string() {
        return Ok(Value::String(s));
    }
    if Array::is_array(v) {
        let list = Array::from(v);
        let mut out = Vec::with_capacity(list.length() as usize);
        for (i, item) in list.iter().enumerate() {
            let len = path.len();
            path.push_str(&format!("[{i}]"));
            out.push(plain(&item, path)?);
            path.truncate(len);
        }
        return Ok(Value::Array(out));
    }
    if v.is_function() || !v.is_object() {
        return Err("expected data (a number, string, boolean, array or object)".to_owned());
    }
    let mut out = Map::new();
    for entry in Object::entries(v.unchecked_ref::<Object>()).iter() {
        let pair = Array::from(&entry);
        let value = pair.get(1);
        if value.is_undefined() {
            continue;
        }
        let key = pair.get(0).as_string().unwrap_or_default();
        let len = path.len();
        path.push('.');
        path.push_str(&key);
        let item = plain(&value, path)?;
        path.truncate(len);
        out.insert(key, item);
    }
    Ok(Value::Object(out))
}

fn number(n: f64) -> Result<Number, String> {
    if !n.is_finite() {
        return Err(format!("{n} is not a finite number"));
    }
    let whole =
        n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER && !(n == 0.0 && n.is_sign_negative());
    Ok(match whole {
        true if n >= 0.0 => Number::from(n as u64),
        true => Number::from(n as i64),
        false => Number::from_f64(n).ok_or_else(|| format!("{n} is not a finite number"))?,
    })
}
