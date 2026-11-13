//! JS options to typed [`Options`], once, at the boundary.

use js_sys::{Array, Object};
use pixfx_core::{OptValue, Options, Rgba, hex};
use wasm_bindgen::{JsError, JsValue};

/// Largest integer a JS number holds exactly (`Number.MAX_SAFE_INTEGER`).
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Largest `margin`: keeps `size + 2 * margin` and the canvas's pixel count far from overflow.
pub const MAX_MARGIN: u32 = 4096;

/// `undefined`/`null` (all defaults) or a plain object: `margin` (whole number up to [`MAX_MARGIN`]: empty pixels round the
/// canvas, returned beside the options because it is the canvas's, not the effect's), `seed` (non-negative integer), `flip` (boolean),
/// `angle` (radians), `light` (glow strength), `palette` (array of `"#rgb"`/`"#rrggbb"` strings); any other key is
/// the effect's own option, a number, a hex colour or an array of hex colours. `x`/`y` are refused: an instance draws on its
/// own canvas at the effect's origin. A field of the wrong type is an error naming it.
pub fn parse_options(v: &JsValue) -> Result<(Options, u32), JsError> {
    let mut opts = Options::default();
    let mut margin = 0;
    if v.is_undefined() || v.is_null() {
        return Ok((opts, margin));
    }
    if !v.is_object() || Array::is_array(v) {
        return Err(JsError::new("options must be an object"));
    }
    for entry in Object::entries(&Object::from(v.clone())).iter() {
        let pair = Array::from(&entry);
        let key = pair.get(0).as_string().unwrap_or_default();
        let val = pair.get(1);
        if val.is_undefined() {
            continue;
        }
        match key.as_str() {
            "margin" => {
                margin = whole_number(&key, &val, MAX_MARGIN.into()).map_err(|_| {
                    JsError::new(&format!(
                        "option margin must be a whole number from 0 to {MAX_MARGIN}"
                    ))
                })? as u32;
            }
            "seed" => opts.seed = Some(whole_number(&key, &val, MAX_SAFE_INTEGER)? as u64),
            "flip" => {
                opts.flip = val
                    .as_bool()
                    .ok_or_else(|| JsError::new("option flip must be a boolean"))?;
            }
            "angle" => opts.angle = number(&key, &val)?,
            "light" => opts.light = number(&key, &val)?,
            "palette" => opts.palette = Some(ramp(&key, &val)?),
            "x" | "y" => {
                return Err(JsError::new(&format!(
                    "option {key} is not supported: an Fx draws on its own canvas at the effect's origin"
                )));
            }
            _ => {
                let extra = if Array::is_array(&val) {
                    OptValue::Ramp(ramp(&key, &val)?)
                } else if val.as_f64().is_some() {
                    OptValue::Num(number(&key, &val)?)
                } else if let Some(s) = val.as_string() {
                    OptValue::Color(
                        hex(&s).map_err(|e| JsError::new(&format!("option {key}: {e}")))?,
                    )
                } else {
                    return Err(JsError::new(&format!(
                        "option {key} must be a number, a hex colour or an array of hex colours"
                    )));
                };
                opts.extra.insert(key, extra);
            }
        }
    }
    Ok((opts, margin))
}

fn number(key: &str, v: &JsValue) -> Result<f64, JsError> {
    v.as_f64()
        .filter(|n| n.is_finite())
        .ok_or_else(|| JsError::new(&format!("option {key} must be a finite number")))
}

fn whole_number(key: &str, v: &JsValue, max: f64) -> Result<f64, JsError> {
    v.as_f64()
        .filter(|n| n.fract() == 0.0 && (0.0..=max).contains(n))
        .ok_or_else(|| JsError::new(&format!("option {key} must be a non-negative integer")))
}

fn ramp(key: &str, v: &JsValue) -> Result<Vec<Rgba>, JsError> {
    if !Array::is_array(v) {
        return Err(JsError::new(&format!(
            "option {key} must be an array of hex colours"
        )));
    }
    Array::from(v)
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let s = c.as_string().ok_or_else(|| {
                JsError::new(&format!("option {key}[{i}] must be a hex colour string"))
            })?;
            hex(&s).map_err(|e| JsError::new(&format!("option {key}[{i}]: {e}")))
        })
        .collect()
}
