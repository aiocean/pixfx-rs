//! pixfx as a wasm engine that JavaScript drives: list effects, read their meta, spawn instances, step them
//! and read their frames. The bindings are a thin layer over plain Rust ([`Player`], [`parse_options`]),
//! which other Rust crates use directly.
#![forbid(unsafe_code)]

mod feel;
mod hit_flash;
pub mod js;
mod options;
pub mod scene;
mod squash;

pub use feel::Feel;
pub use hit_flash::HitFlash;
use js::to_js;
pub use options::{MAX_MARGIN, parse_options};
pub use pixfx_core::Player;
pub use squash::{Blink, Squash};

use js_sys::{Array, Uint8ClampedArray, Uint32Array};
use pixfx_core::{Effect, Layer, Meta, OptKind, PixelBox, valid_dt};
use pixfx_draw::Rgba;
use pixfx_effects::{EFFECTS, effect};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const EFFECT_ID: &str = include_str!(concat!(env!("OUT_DIR"), "/effect_id.d.ts"));

/// Every effect id, in registry order.
#[wasm_bindgen(unchecked_return_type = "EffectId[]")]
pub fn effects() -> Array {
    EFFECTS
        .iter()
        .map(|e| JsValue::from_str(e.meta().id))
        .collect()
}

/// The palette presets every effect takes by name (`palette: "fire"`), in the engine's order.
#[wasm_bindgen(js_name = palettePresets)]
pub fn palette_presets() -> Vec<String> {
    pixfx_draw::palette::PRESETS
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect()
}

/// The outline presets every effect takes by name (`outline: "..."`), in the engine's order.
#[wasm_bindgen(js_name = outlinePresets)]
pub fn outline_presets() -> Vec<String> {
    pixfx_draw::outline::PRESETS
        .iter()
        .map(|(name, _)| (*name).to_string())
        .collect()
}

/// What an effect is, as JS reads it.
#[derive(Serialize, tsify::Tsify)]
#[serde(rename_all = "camelCase")]
#[tsify(rename = "EffectMeta")]
struct MetaJs {
    #[tsify(type = "EffectId")]
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    tags: &'static [&'static str],
    /// Canvas size in pixels, `[w, h]`.
    size: (u32, u32),
    /// Where the effect's local (0, 0) sits on its canvas, `[x, y]`.
    origin: (i32, i32),
    /// The margin that fits the effect's glow.
    glow_margin: u32,
    /// Seconds: one loop, or the whole one-shot.
    duration: f64,
    #[serde(rename = "loop")]
    looping: bool,
    fps: u32,
    /// Seeds are small; a JS number holds them exactly up to 2^53.
    seed: u64,
    /// The effect's own options, beyond the ones every effect takes.
    options: Vec<OptionJs>,
}

/// One of an effect's own options as JS reads it: its name, what it holds and its default.
#[derive(Serialize, tsify::Tsify)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[tsify(rename = "EffectOption")]
enum OptionJs {
    /// A number; `range` (`[lo, hi]`, or `null`) holds it where it sizes a loop. A `NaN` default means the effect
    /// picks the value itself.
    Num {
        name: &'static str,
        default: f64,
        range: Option<(f64, f64)>,
    },
    /// One colour, `"#rrggbb"`.
    Color { name: &'static str, default: Rgba },
    /// A colour ramp; a ramp of another length is resampled to the default's.
    Ramp {
        name: &'static str,
        default: &'static [Rgba],
    },
}

/// `{id, name, desc, tags, size: [w, h], origin: [x, y], glowMargin, duration, loop, fps, seed, options}` for one
/// effect; `options` lists its own options as `{name, kind: "num" | "color" | "ramp", default, range?}`.
#[wasm_bindgen(unchecked_return_type = "EffectMeta")]
pub fn meta(
    #[wasm_bindgen(unchecked_param_type = "EffectId")] id: &str,
) -> Result<JsValue, JsError> {
    meta_object(find(id)?.meta())
}

fn find(id: &str) -> Result<&'static dyn Effect, JsError> {
    effect(id).ok_or_else(|| JsError::new(&format!("unknown effect {id:?}")))
}

fn meta_object(m: &Meta) -> Result<JsValue, JsError> {
    js::to_js(&MetaJs {
        id: m.id,
        name: m.name,
        desc: m.desc,
        tags: m.tags,
        size: m.size,
        origin: m.origin,
        glow_margin: m.glow_margin(),
        duration: m.duration,
        looping: m.looping,
        fps: m.fps,
        seed: m.seed,
        options: m
            .options
            .iter()
            .map(|o| match o.kind {
                OptKind::Num { default, range } => OptionJs::Num {
                    name: o.name,
                    default,
                    range,
                },
                OptKind::Color { default } => OptionJs::Color {
                    name: o.name,
                    default,
                },
                OptKind::Ramp { default } => OptionJs::Ramp {
                    name: o.name,
                    default,
                },
            })
            .collect(),
    })
}

/// A live effect on its own canvas (`meta.size`, or the size its options give it (weather's `width` and
/// `height`), plus `margin` empty pixels on every side, origin at the canvas origin plus `margin`;
/// `meta(id).glowMargin` is the margin that fits an effect's glow). Each animation frame: `update(dt)`,
/// `draw(layer)`, then read `rgba()` (or `color()` and `light()`). Light is eased out at the canvas edge
/// unless the effect is full-screen or has a margin, so frames match the effect's sprite sheet.
#[wasm_bindgen]
pub struct Fx {
    player: Player,
}

#[wasm_bindgen]
impl Fx {
    /// Spawn effect `id`. `opts`: `undefined` or `{margin?, seed?, flip?, angle?, light?, palette?: string[], ...extra}`;
    /// an extra option is a number, a hex colour string (`"#ff8800"`) or an array of hex colours
    /// (see [`parse_options`]).
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "EffectId")] id: &str,
        #[wasm_bindgen(unchecked_optional_param_type = "FxOptions")] opts: JsValue,
    ) -> Result<Fx, JsError> {
        let fx = find(id)?;
        let (opts, margin) = parse_options(fx.meta(), &opts)?;
        Ok(Fx {
            player: Player::new(fx, opts, margin.unwrap_or(0)),
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

    /// The cues the effect fired since the last call, oldest first (`{t, pan, cue}`, `cue.type` `"shake"`,
    /// `"hitstop"`, `"ripple"`, `"flash"` or `"sound"`), and clears them. A ripple's or a sound's `x`/`y` are
    /// relative to the effect's origin: add where the host placed it. Hand them to a [`Feel`] with `take`.
    #[wasm_bindgen(js_name = takeCues, unchecked_return_type = "TimedCue[]")]
    pub fn take_cues(&mut self) -> Result<JsValue, JsError> {
        let origin = self.player.canvas().origin;
        let (ox, oy) = (f64::from(origin.0), f64::from(origin.1));
        let mut cues = self.player.take_cues();
        for c in &mut cues {
            match &mut c.cue {
                pixfx_core::Cue::Ripple { x, y, .. } => {
                    *x -= ox;
                    *y -= oy;
                }
                pixfx_core::Cue::Sound { x: Some(x), .. } => *x -= ox,
                _ => {}
            }
        }
        to_js(&cues)
    }

    /// The frame the effect shows: k while its clock is on frame k of `render_frames`, wrapped round a loop.
    /// Redraw when it changes.
    #[wasm_bindgen(js_name = frameIndex)]
    pub fn frame_index(&self) -> u32 {
        u32::try_from(self.player.frame_index()).unwrap_or(u32::MAX)
    }

    /// Frames of `1 / fps` its clock has passed: what it draws changes only when this does. A fading effect
    /// (no [`Fx::shown`]) redraws when it changes.
    #[wasm_bindgen(js_name = clockFrame)]
    pub fn clock_frame(&self) -> u32 {
        u32::try_from(self.player.clock_frame()).unwrap_or(u32::MAX)
    }

    /// The frame `draw` draws now, as `"{key}:{index}"`: instances showing the same one draw the same bytes,
    /// so a host draws it once and shows it for all of them. `undefined` while the effect fades after `stop()`
    /// (a fade frame is its own).
    pub fn shown(&self) -> Option<String> {
        self.player
            .shown(self.player.placement())
            .map(|id| format!("{}:{}", id.key, id.index))
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
        self.player.canvas().origin.0 + self.player.margin() as i32
    }

    #[wasm_bindgen(js_name = originY)]
    pub fn origin_y(&self) -> i32 {
        self.player.canvas().origin.1 + self.player.margin() as i32
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
        self.player.draw(layer, self.player.placement());
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

    /// The box holding every pixel the last `draw` showed (colour alpha or light), as `[x, y, w, h]` on the
    /// canvas; `[0, 0, 0, 0]` when it showed nothing.
    #[wasm_bindgen(js_name = litBox)]
    pub fn lit_box(&self) -> Vec<u32> {
        let b = self.player.lit_box().unwrap_or_default();
        vec![b.x, b.y, b.w, b.h]
    }

    /// As [`Fx::color_rgba_ptr`] for the box at (`x`, `y`) of `w` × `h` pixels alone (clipped to the canvas):
    /// `w * h * 4` bytes, row-major.
    #[wasm_bindgen(js_name = colorRgbaBoxPtr)]
    pub fn color_rgba_box_ptr(&mut self, x: u32, y: u32, w: u32, h: u32) -> usize {
        self.player.color_rgba_in(PixelBox { x, y, w, h }).as_ptr() as usize
    }

    /// As [`Fx::light_rgba_ptr`] for the box at (`x`, `y`) of `w` × `h` pixels alone (clipped to the canvas).
    #[wasm_bindgen(js_name = lightRgbaBoxPtr)]
    pub fn light_rgba_box_ptr(&mut self, x: u32, y: u32, w: u32, h: u32) -> usize {
        self.player.light_rgba_in(PixelBox { x, y, w, h }).as_ptr() as usize
    }

    /// The canvas composited over transparency: RGBA8 bytes, ready for `new ImageData(px, width, height)`.
    pub fn rgba(&mut self) -> Uint8ClampedArray {
        Uint8ClampedArray::from(self.player.rgba())
    }
}

/// The frames of a clip baked by `pixfx bake`, as live effects of that clip share them: `json` is the text of
/// `<name>.bake.json`, `pages` the bytes of the PNG files it names, in its order. One `{id, frame}` per frame,
/// `id` as [`Fx::shown`] gives it and `frame` `{back, front}`, each side `{box: {x, y, w, h}, color, light}`
/// cropped to its lit box as [`Fx::lit_box`] crops it (`color` and `light` as `ArrayBuffer`s of RGBA bytes, as
/// `colorRgbaBoxPtr` and `lightRgbaBoxPtr` give them).
#[wasm_bindgen(js_name = bakedFrames)]
pub fn baked_frames(json: &str, pages: Vec<js_sys::Uint8Array>) -> Result<Array, JsError> {
    let pages: Vec<Vec<u8>> = pages.iter().map(js_sys::Uint8Array::to_vec).collect();
    let refs: Vec<&[u8]> = pages.iter().map(Vec::as_slice).collect();
    let layers = pixfx_export::decode(json, &refs).map_err(|e| JsError::new(&e))?;
    let set = |o: &js_sys::Object, k: &str, v: &JsValue| {
        js_sys::Reflect::set(o, &JsValue::from_str(k), v)
            .map(drop)
            .map_err(|_| JsError::new("could not build a baked frame"))
    };
    let mut bytes = Vec::new();
    let mut side = |frame: &pixfx_draw::Frame| -> Result<js_sys::Object, JsError> {
        let b = PixelBox::lit(frame).unwrap_or_default();
        bytes.resize(b.w as usize * b.h as usize * 4, 0);
        let o = js_sys::Object::new();
        let bx = js_sys::Object::new();
        for (k, v) in [("x", b.x), ("y", b.y), ("w", b.w), ("h", b.h)] {
            set(&bx, k, &JsValue::from(v))?;
        }
        set(&o, "box", &bx)?;
        let color = js_sys::Uint8Array::from(pixfx_core::color_rgba_in(frame, b, &mut bytes));
        set(&o, "color", &color.buffer())?;
        let light = js_sys::Uint8Array::from(pixfx_core::light_rgba_in(frame, b, &mut bytes));
        set(&o, "light", &light.buffer())?;
        Ok(o)
    };
    let out = Array::new();
    // `decode` gives each frame's back, then its front
    for pair in layers.chunks(2) {
        let [back, front] = pair else {
            return Err(JsError::new("a baked frame lacks a layer"));
        };
        let frame = js_sys::Object::new();
        set(&frame, "back", &side(&back.frame)?.into())?;
        set(&frame, "front", &side(&front.frame)?.into())?;
        let entry = js_sys::Object::new();
        let id = format!("{}:{}", back.id.key, back.id.index);
        set(&entry, "id", &JsValue::from_str(&id))?;
        set(&entry, "frame", &frame)?;
        out.push(&entry);
    }
    Ok(out)
}
