//! JS options to typed [`Options`], once, at the boundary.

use std::collections::BTreeMap;
use std::fmt;

use pixfx_core::outline::{DARK, MAX_WIDTH, Outline, Ring};
use pixfx_core::{Meta, OptValue, Options, Rgba, check_options, hex};
use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use wasm_bindgen::{JsError, JsValue};

use crate::js::parse;

/// Largest `margin`: keeps `size + 2 * margin` and the canvas's pixel count far from overflow.
pub const MAX_MARGIN: u32 = 4096;

/// An effect's options as JS passes them: the ones every effect reads, and the effect's own by name.
#[derive(Default, tsify::Tsify)]
#[tsify(
    rename = "FxOptions",
    type = "{ margin?: number; seed?: number; flip?: boolean; angle?: number; light?: number; palette?: PaletteOption; outline?: OutlineOption; [option: string]: number | string | string[] | boolean | OutlineOption | undefined }"
)]
struct Wire {
    margin: Option<u32>,
    seed: Option<u64>,
    flip: Option<bool>,
    angle: Option<f64>,
    light: Option<f64>,
    palette: Option<PaletteIn>,
    outline: Option<OutlineIn>,
    extra: BTreeMap<String, Extra>,
}

/// Read key by key (not `#[serde(flatten)]`, which buffers the values and loses the key from the error's path).
impl<'de> Deserialize<'de> for Wire {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_map(WireVisitor)
    }
}

struct WireVisitor;

impl<'de> Visitor<'de> for WireVisitor {
    type Value = Wire;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an options object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Wire, A::Error> {
        let mut w = Wire::default();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "margin" => w.margin = map.next_value()?,
                "seed" => w.seed = map.next_value()?,
                "flip" => w.flip = map.next_value()?,
                "angle" => w.angle = map.next_value()?,
                "light" => w.light = map.next_value()?,
                "palette" => w.palette = map.next_value()?,
                "outline" => w.outline = map.next_value()?,
                _ => {
                    let v: Extra = map.next_value()?;
                    w.extra.insert(key, v);
                }
            }
        }
        Ok(w)
    }
}

/// The `palette` option as JS gives it: a preset's name ([`pixfx_core::palette::PRESETS`]) or a list of hex
/// colours, dark to light.
#[derive(tsify::Tsify)]
#[tsify(
    rename = "PaletteOption",
    type = "\"fire\" | \"ice\" | \"poison\" | \"holy\" | \"shadow\" | \"arcane\" | \"blood\" | \"electric\" | \"nature\" | \"gold\" | string[]"
)]
struct PaletteIn(Vec<Rgba>);

impl<'de> Deserialize<'de> for PaletteIn {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(PaletteVisitor)
    }
}

struct PaletteVisitor;

impl<'de> Visitor<'de> for PaletteVisitor {
    type Value = PaletteIn;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a palette preset's name or an array of hex colours")
    }

    fn visit_str<E: de::Error>(self, word: &str) -> Result<PaletteIn, E> {
        pixfx_core::palette::parse(word)
            .map(PaletteIn)
            .map_err(E::custom)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<PaletteIn, A::Error> {
        let mut ramp = Vec::new();
        while let Some(c) = seq.next_element::<Rgba>()? {
            ramp.push(c);
        }
        Ok(PaletteIn(ramp))
    }
}

/// The `outline` option as JS gives it: `false` (none), `true` (the dark preset), one word (`"off"`, a preset's
/// name or a hex colour), or `{color?, width?, corners?}` over the dark 1 px ring (`color` a preset's name or a hex
/// colour, `width` 1 to [`MAX_WIDTH`]).
#[derive(tsify::Tsify)]
#[tsify(
    rename = "OutlineOption",
    type = "boolean | \"off\" | \"dark\" | \"black\" | \"white\" | \"thick\" | (string & {}) | { color?: string; width?: number; corners?: boolean }"
)]
struct OutlineIn(Outline);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RingIn {
    color: Option<String>,
    width: Option<u32>,
    corners: Option<bool>,
}

impl<'de> Deserialize<'de> for OutlineIn {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(OutlineVisitor)
    }
}

struct OutlineVisitor;

impl<'de> Visitor<'de> for OutlineVisitor {
    type Value = OutlineIn;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("true, false, \"off\", a preset, a hex colour or {color, width, corners}")
    }

    fn visit_bool<E: de::Error>(self, on: bool) -> Result<OutlineIn, E> {
        Ok(OutlineIn(if on {
            Outline::On(Ring::thin(DARK))
        } else {
            Outline::Off
        }))
    }

    fn visit_str<E: de::Error>(self, word: &str) -> Result<OutlineIn, E> {
        Outline::parse(word).map(OutlineIn).map_err(E::custom)
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<OutlineIn, A::Error> {
        let r = RingIn::deserialize(de::value::MapAccessDeserializer::new(map))?;
        let base = Ring::thin(DARK);
        let color = match r.color.as_deref() {
            None => base.color(),
            Some(word) => match Outline::parse(word).map_err(de::Error::custom)? {
                Outline::On(ring) => ring.color(),
                Outline::Off => {
                    return Err(de::Error::custom(
                        "outline.color is a colour; turn the outline off with outline: false",
                    ));
                }
            },
        };
        let width = r.width.unwrap_or(base.width());
        let ring =
            Ring::new(color, width, r.corners.unwrap_or(base.corners())).ok_or_else(|| {
                de::Error::custom(format!(
                    "outline.width must be 1 to {MAX_WIDTH}, not {width}"
                ))
            })?;
        Ok(OutlineIn(Outline::On(ring)))
    }
}

/// One of the effect's own options: a number, a hex colour or a list of hex colours.
struct Extra(OptValue);

impl<'de> Deserialize<'de> for Extra {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(ExtraVisitor)
    }
}

struct ExtraVisitor;

impl<'de> Visitor<'de> for ExtraVisitor {
    type Value = Extra;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a number, a hex colour or an array of hex colours")
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Extra, E> {
        Ok(Extra(OptValue::Num(v)))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Extra, E> {
        // whole numbers arrive as integers only when a JS number holds them exactly
        Ok(Extra(OptValue::Num(v as f64)))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Extra, E> {
        Ok(Extra(OptValue::Num(v as f64)))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Extra, E> {
        hex(v).map(|c| Extra(OptValue::Color(c))).map_err(E::custom)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Extra, A::Error> {
        let mut ramp = Vec::new();
        while let Some(c) = seq.next_element::<Rgba>()? {
            ramp.push(c);
        }
        Ok(Extra(OptValue::Ramp(ramp)))
    }
}

/// `undefined`/`null` (all defaults) or a plain object: `margin` (whole number up to [`MAX_MARGIN`]: empty pixels
/// round the canvas, returned beside the options because it is the canvas's, not the effect's; `None` when not
/// given), `seed` (non-negative integer), `flip` (boolean), `angle` (radians), `light` (glow strength), `palette`
/// ([`PaletteIn`]: a preset's name or an array of `"#rgb"`/`"#rrggbb"` strings), `outline` ([`OutlineIn`]); any other key is one of the effect's own options ([`Meta::options`]):
/// a number, a hex colour or an array of hex colours, as the effect declares it. `x`/`y` are refused: an instance
/// draws on its own canvas at the effect's origin. A key the effect does not declare, or a field of the wrong
/// type, is an error naming it.
pub fn parse_options(meta: &Meta, v: &JsValue) -> Result<(Options, Option<u32>), JsError> {
    if v.is_undefined() || v.is_null() {
        return Ok((Options::default(), None));
    }
    let w: Wire = parse("options", v)?;
    if let Some(m) = w.margin.filter(|&m| m > MAX_MARGIN) {
        return Err(JsError::new(&format!(
            "options.margin must be a whole number from 0 to {MAX_MARGIN}, not {m}"
        )));
    }
    if let Some(key) = ["x", "y"].into_iter().find(|k| w.extra.contains_key(*k)) {
        return Err(JsError::new(&format!(
            "options.{key} is not supported: an Fx draws on its own canvas at the effect's origin"
        )));
    }
    let base = Options::default();
    let opts = Options {
        seed: w.seed,
        flip: w.flip.unwrap_or(base.flip),
        angle: w.angle.unwrap_or(base.angle),
        light: w.light.unwrap_or(base.light),
        palette: w.palette.map(|PaletteIn(p)| p),
        outline: w.outline.map(|OutlineIn(o)| o),
        extra: w.extra.into_iter().map(|(k, Extra(v))| (k, v)).collect(),
        ..base
    };
    check_options(meta, &opts).map_err(|e| JsError::new(&e.to_string()))?;
    Ok((opts, w.margin))
}
