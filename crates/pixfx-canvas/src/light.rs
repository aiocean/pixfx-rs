//! `pixfx-light` for JS: the light a game's spells, blows and lamps throw on its scene. The lights live here;
//! the game adds them as plain objects, moves and puts them out by key, lets time pass, and draws them over its
//! canvas with `"lighter"`. Objects are read strictly: a missing, misspelt or mistyped field throws, naming it.

use js_sys::{Array, Object, Reflect, Uint8ClampedArray};
use pixfx_core::{Frame, hex, valid_dt};
use pixfx_light::{Glow, Light, Style};
use wasm_bindgen::prelude::*;
use web_sys::{
    CanvasRenderingContext2d, ImageData, OffscreenCanvas, OffscreenCanvasRenderingContext2d,
};

use crate::js_message;
use crate::stage::light_rgba;

/// The lights on a `w` x `h` game-pixel scene, in the duel's style (`pixfx_light::Style::default()`).
#[wasm_bindgen]
pub struct Lights {
    lights: Vec<Light>,
    style: Style,
    /// The light layer, redrawn each `draw`; reused, as are the buffers below.
    frame: Frame,
    canvas: OffscreenCanvas,
    ctx: OffscreenCanvasRenderingContext2d,
    bytes: Vec<u8>,
    js: Uint8ClampedArray,
    image: ImageData,
}

#[wasm_bindgen]
impl Lights {
    #[wasm_bindgen(constructor)]
    pub fn new(w: u32, h: u32) -> Result<Lights, JsError> {
        if w == 0 || h == 0 {
            return Err(JsError::new("the scene must be at least 1 x 1"));
        }
        let canvas = OffscreenCanvas::new(w, h)
            .map_err(|_| JsError::new("could not create an OffscreenCanvas"))?;
        let ctx = canvas
            .get_context("2d")
            .map_err(|_| JsError::new("could not get an OffscreenCanvas 2d context"))?
            .ok_or_else(|| JsError::new("OffscreenCanvas has no 2d context"))?
            .dyn_into::<OffscreenCanvasRenderingContext2d>()
            .map_err(|_| JsError::new("OffscreenCanvas 2d context has an unexpected type"))?;
        let len = w as usize * h as usize * 4;
        let js = Uint8ClampedArray::new_with_length(len as u32);
        let image = ImageData::new_with_js_u8_clamped_array_and_sh(&js, w, h)
            .map_err(|_| JsError::new("could not create ImageData"))?;
        Ok(Lights {
            lights: Vec::new(),
            style: Style::default(),
            frame: Frame::new(w, h, 0, 0),
            canvas,
            ctx,
            bytes: vec![0; len],
            js,
            image,
        })
    }

    /// Light one: `{key?, x, y, reach, color, strength, life?}`. `key` (a whole number, or null) finds it again;
    /// `color` is `"#rrggbb"`; `life` is seconds, or null/absent to burn until put out.
    pub fn add(&mut self, light: &JsValue) -> Result<(), JsError> {
        let o = Fields::new(
            light,
            "light",
            &["key", "x", "y", "reach", "color", "strength", "life"],
        )?;
        self.lights.push(Light {
            key: o.key("key")?,
            x: o.num("x")?,
            y: o.num("y")?,
            reach: o.num("reach")?,
            color: o.color("color")?,
            strength: o.num("strength")?,
            life: o.opt_num("life")?,
            age: 0.0,
        });
        Ok(())
    }

    /// Move every light tagged `key` to (`x`, `y`).
    #[wasm_bindgen(js_name = "move")]
    pub fn move_to(&mut self, key: u32, x: f64, y: f64) {
        for l in self.lights.iter_mut().filter(|l| l.key == Some(key)) {
            (l.x, l.y) = (x, y);
        }
    }

    /// Put out every light tagged `key` within `within` seconds.
    #[wasm_bindgen(js_name = putOut)]
    pub fn put_out(&mut self, key: u32, within: f64) {
        for l in self.lights.iter_mut().filter(|l| l.key == Some(key)) {
            *l = l.put_out(within);
        }
    }

    /// Let `dt` seconds (finite, 0 or more) pass: lights age, those whose life is over go out.
    pub fn step(&mut self, dt: f64) -> Result<(), JsError> {
        if !valid_dt(dt) {
            return Err(JsError::new(
                "dt must be a finite number of seconds, 0 or more",
            ));
        }
        self.lights = pixfx_light::step(std::mem::take(&mut self.lights), dt);
        Ok(())
    }

    /// The lights burning: `[{key, x, y, reach, color, strength, life, age}]`, `key` and `life` null when unset.
    pub fn list(&self) -> Result<Array, JsError> {
        self.lights
            .iter()
            .map(|l| {
                let opt = |v: Option<f64>| v.map_or(JsValue::NULL, JsValue::from);
                object(&[
                    ("key", opt(l.key.map(f64::from))),
                    ("x", l.x.into()),
                    ("y", l.y.into()),
                    ("reach", l.reach.into()),
                    ("color", pixfx_core::color::to_hex(l.color).into()),
                    ("strength", l.strength.into()),
                    ("life", opt(l.life)),
                    ("age", l.age.into()),
                ])
            })
            .collect()
    }

    /// Add the lights, and `glows` (this instant's extra lights, such as flickering lamps:
    /// `[{x, y, reach, color, strength}]`), over what is drawn on `ctx`, one game pixel as `scale` x `scale`
    /// canvas pixels at the context's current origin. The context's compositing and smoothing are restored.
    pub fn draw(
        &mut self,
        ctx: &CanvasRenderingContext2d,
        scale: u32,
        glows: &Array,
    ) -> Result<(), JsError> {
        let mut all = glows
            .iter()
            .map(|g| glow(&g))
            .collect::<Result<Vec<_>, _>>()?;
        all.extend(self.lights.iter().map(|l| l.glow(&self.style)));
        self.frame.clear();
        pixfx_light::draw(&mut self.frame, &all, &self.style);
        if !light_rgba(&self.frame, &mut self.bytes) {
            return Ok(());
        }
        self.js.copy_from(&self.bytes);
        self.ctx
            .put_image_data(&self.image, 0.0, 0.0)
            .map_err(|e| JsError::new(&js_message("putImageData failed", &e)))?;
        let s = f64::from(scale);
        let (w, h) = (f64::from(self.frame.w) * s, f64::from(self.frame.h) * s);
        ctx.save();
        ctx.set_image_smoothing_enabled(false);
        let drawn = ctx
            .set_global_composite_operation("lighter")
            .and_then(|()| {
                ctx.draw_image_with_offscreen_canvas_and_dw_and_dh(&self.canvas, 0.0, 0.0, w, h)
            });
        ctx.restore();
        drawn.map_err(|e| JsError::new(&js_message("drawing the light failed", &e)))
    }
}

/// One glow object `{x, y, reach, color, strength}`.
fn glow(v: &JsValue) -> Result<Glow, JsError> {
    let o = Fields::new(v, "glow", &["x", "y", "reach", "color", "strength"])?;
    Ok(Glow {
        x: o.num("x")?,
        y: o.num("y")?,
        reach: o.num("reach")?,
        color: o.color("color")?,
        strength: o.num("strength")?,
    })
}

/// A JS object read strictly: only the `known` fields, each of the right type.
struct Fields<'v> {
    v: &'v JsValue,
    what: &'static str,
}

impl<'v> Fields<'v> {
    fn new(v: &'v JsValue, what: &'static str, known: &[&str]) -> Result<Self, JsError> {
        if !v.is_object() {
            return Err(JsError::new(&format!("a {what} must be an object")));
        }
        for k in Object::keys(v.unchecked_ref::<Object>()).iter() {
            let k = k.as_string().unwrap_or_default();
            if !known.contains(&k.as_str()) {
                return Err(JsError::new(&format!(
                    "{what}: unknown field {k:?} (expected {})",
                    known.join(", ")
                )));
            }
        }
        Ok(Self { v, what })
    }

    fn get(&self, k: &str) -> Option<JsValue> {
        Reflect::get(self.v, &JsValue::from_str(k))
            .ok()
            .filter(|v| !v.is_undefined() && !v.is_null())
    }

    fn err(&self, k: &str, want: &str) -> JsError {
        JsError::new(&format!("{}.{k} must be {want}", self.what))
    }

    fn opt_num(&self, k: &str) -> Result<Option<f64>, JsError> {
        self.get(k)
            .map(|v| {
                v.as_f64()
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| self.err(k, "a finite number"))
            })
            .transpose()
    }

    fn num(&self, k: &str) -> Result<f64, JsError> {
        self.opt_num(k)?
            .ok_or_else(|| self.err(k, "a finite number"))
    }

    fn key(&self, k: &str) -> Result<Option<u32>, JsError> {
        self.opt_num(k)?
            .map(|n| {
                let whole = n >= 0.0 && n <= f64::from(u32::MAX) && n == n.trunc();
                whole
                    .then_some(n as u32)
                    .ok_or_else(|| self.err(k, "a whole number or null"))
            })
            .transpose()
    }

    fn color(&self, k: &str) -> Result<pixfx_core::Rgba, JsError> {
        let s = self
            .get(k)
            .and_then(|v| v.as_string())
            .ok_or_else(|| self.err(k, "a \"#rrggbb\" string"))?;
        hex(&s).map_err(|e| JsError::new(&format!("{}.{k}: {e}", self.what)))
    }
}

fn object(fields: &[(&str, JsValue)]) -> Result<JsValue, JsError> {
    let o = Object::new();
    for (k, v) in fields {
        Reflect::set(&o, &JsValue::from_str(k), v)
            .map_err(|_| JsError::new("could not build an object"))?;
    }
    Ok(o.into())
}
