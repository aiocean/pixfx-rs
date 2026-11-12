//! pixfx as a wasm engine that JavaScript drives: list effects, read their meta, spawn instances, step them
//! and read their frames. The bindings are a thin layer over plain Rust ([`Player`], [`parse_options`]),
//! which other Rust crates use directly.
#![forbid(unsafe_code)]

mod options;
mod player;

pub use options::parse_options;
pub use player::Player;

use js_sys::{Array, Object, Reflect, Uint8ClampedArray, Uint32Array};
use pixfx_core::{Effect, Layer, Meta, valid_dt};
use pixfx_effects::{EFFECTS, effect};
use wasm_bindgen::prelude::*;

/// Every effect id, in registry order.
#[wasm_bindgen]
pub fn effects() -> Array {
    EFFECTS
        .iter()
        .map(|e| JsValue::from_str(e.meta().id))
        .collect()
}

/// `{id, name, desc, tags, size: [w, h], origin: [x, y], glowMargin, duration, loop, fps, seed}` for one effect.
#[wasm_bindgen]
pub fn meta(id: &str) -> Result<JsValue, JsError> {
    meta_object(find(id)?.meta())
}

fn find(id: &str) -> Result<&'static dyn Effect, JsError> {
    effect(id).ok_or_else(|| JsError::new(&format!("unknown effect {id:?}")))
}

fn meta_object(m: &Meta) -> Result<JsValue, JsError> {
    let o = Object::new();
    let pair = |a: f64, b: f64| Array::of2(&a.into(), &b.into());
    let tags: Array = m.tags.iter().map(|t| JsValue::from_str(t)).collect();
    let fields: [(&str, JsValue); 11] = [
        ("id", m.id.into()),
        ("name", m.name.into()),
        ("desc", m.desc.into()),
        ("tags", tags.into()),
        (
            "size",
            pair(f64::from(m.size.0), f64::from(m.size.1)).into(),
        ),
        (
            "origin",
            pair(f64::from(m.origin.0), f64::from(m.origin.1)).into(),
        ),
        ("glowMargin", m.glow_margin().into()),
        ("duration", m.duration.into()),
        ("loop", m.looping.into()),
        ("fps", m.fps.into()),
        // seeds are small; a JS number holds them exactly up to 2^53
        ("seed", (m.seed as f64).into()),
    ];
    for (k, v) in fields {
        Reflect::set(&o, &JsValue::from_str(k), &v)
            .map_err(|_| JsError::new("could not build the meta object"))?;
    }
    Ok(o.into())
}

/// A live effect on its own canvas (`meta.size` plus `margin` empty pixels on every side, origin at
/// `meta.origin + margin`; `meta(id).glowMargin` is the margin that fits an effect's glow). Each animation frame: `update(dt)`,
/// `draw(layer)`, then read `rgba()` (or `color()` and `light()`). Light is eased out at the canvas edge
/// unless the effect is full-screen or has a margin, so frames match the effect's sprite sheet.
#[wasm_bindgen]
pub struct Fx {
    player: Player,
}

#[wasm_bindgen]
impl Fx {
    /// Spawn effect `id`. `opts`: `undefined` or `{margin?, seed?, flip?, angle?, light?, palette?: string[], ...extra}`
    /// (see [`parse_options`]).
    #[wasm_bindgen(constructor)]
    pub fn new(id: &str, opts: JsValue) -> Result<Fx, JsError> {
        let fx = find(id)?;
        let (opts, margin) = parse_options(&opts)?;
        Ok(Fx {
            player: Player::new(fx, opts, margin),
        })
    }

    /// Advance by `dt` seconds (at most 0.25, in 1/120 s sub-steps). `dt` must be a finite number >= 0; any
    /// other value (NaN, a missing argument, a negative step) is an error and leaves the instance as it was.
    pub fn update(&mut self, dt: f64) -> Result<(), JsError> {
        if !valid_dt(dt) {
            return Err(JsError::new(&format!(
                "dt must be a finite number of seconds >= 0, not {dt}"
            )));
        }
        self.player.update(dt);
        Ok(())
    }

    /// Let it end: loops fade out, one-shots are cut short.
    pub fn stop(&mut self) {
        self.player.stop();
    }

    pub fn done(&self) -> bool {
        self.player.done()
    }

    /// Seconds since the instance started.
    pub fn t(&self) -> f64 {
        self.player.t()
    }

    pub fn width(&self) -> u32 {
        self.player.frame().w
    }

    pub fn height(&self) -> u32 {
        self.player.frame().h
    }

    /// Where the effect's origin sits on the canvas.
    #[wasm_bindgen(js_name = originX)]
    pub fn origin_x(&self) -> i32 {
        self.player.meta().origin.0 + self.player.margin() as i32
    }

    #[wasm_bindgen(js_name = originY)]
    pub fn origin_y(&self) -> i32 {
        self.player.meta().origin.1 + self.player.margin() as i32
    }

    /// Clear the canvas and draw `"back"`, `"front"` or `"both"` at the current time.
    pub fn draw(&mut self, layer: &str) -> Result<(), JsError> {
        let layer = match layer {
            "back" => Layer::Back,
            "front" => Layer::Front,
            "both" => Layer::Both,
            other => {
                return Err(JsError::new(&format!(
                    "layer must be \"back\", \"front\" or \"both\", not {other:?}"
                )));
            }
        };
        self.player.draw(layer);
        Ok(())
    }

    /// A copy of the colour layer: one ABGR word per pixel, row-major.
    pub fn color(&self) -> Uint32Array {
        Uint32Array::from(self.player.frame().color.as_slice())
    }

    /// A copy of the light layer: one xBGR word per pixel (top byte unused), row-major.
    pub fn light(&self) -> Uint32Array {
        Uint32Array::from(self.player.frame().light.as_slice())
    }

    /// Write the colour layer as RGBA8 bytes (straight alpha, `width * height * 4`) into this instance's byte
    /// buffer and return the buffer's address in wasm memory. Read it as
    /// `new Uint8ClampedArray(memory.buffer, ptr, width * height * 4)` before the next call on this instance:
    /// no copy leaves wasm.
    #[wasm_bindgen(js_name = colorRgbaPtr)]
    pub fn color_rgba_ptr(&mut self) -> usize {
        self.player.color_rgba().as_ptr() as usize
    }

    /// As [`Fx::color_rgba_ptr`] for the light layer: lit pixels `r, g, b, 255`, unlit `0, 0, 0, 0`.
    #[wasm_bindgen(js_name = lightRgbaPtr)]
    pub fn light_rgba_ptr(&mut self) -> usize {
        self.player.light_rgba().as_ptr() as usize
    }

    /// The canvas composited over transparency: RGBA8 bytes, ready for `new ImageData(px, width, height)`.
    pub fn rgba(&mut self) -> Uint8ClampedArray {
        Uint8ClampedArray::from(self.player.rgba())
    }
}
