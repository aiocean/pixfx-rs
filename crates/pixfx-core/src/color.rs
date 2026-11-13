//! Colours: port of pixelfx `src/core/color.js`. A colour packs into a `u32` laid out like ImageData bytes
//! (R, G, B, A in memory, so `A << 24 | B << 16 | G << 8 | R`); packed 0 means "nothing" to the painter.

use crate::math::{clamp, js_round, to_int32};
use core::fmt;

/// An RGBA8 colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    /// Fully transparent black, which packs to 0: the painter draws nothing for it.
    pub const NONE: Rgba = Rgba(0, 0, 0, 0);

    /// ABGR word as pixelfx stores colours in `Uint32Array`s.
    pub const fn pack(self) -> u32 {
        (self.3 as u32) << 24 | (self.2 as u32) << 16 | (self.1 as u32) << 8 | self.0 as u32
    }

    pub const fn unpack(c: u32) -> Rgba {
        Rgba(c as u8, (c >> 8) as u8, (c >> 16) as u8, (c >> 24) as u8)
    }

    /// The opaque colour written `#rrggbb` as the literal `0xrrggbb`: [`hex`] for constants (palette defaults).
    pub const fn hex24(rgb: u32) -> Rgba {
        Rgba((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255)
    }
}

/// pixelfx `rgb(r, g, b)`: each channel taken as `v & 255` (JS ToInt32, so fractions truncate), alpha 255.
pub fn rgb(r: f64, g: f64, b: f64) -> Rgba {
    Rgba(to_int32(r) as u8, to_int32(g) as u8, to_int32(b) as u8, 255)
}

pub const WHITE: Rgba = Rgba(255, 255, 255, 255);
pub const BLACK: Rgba = Rgba(0, 0, 0, 255);
pub const INK: Rgba = Rgba(0x0b, 0x0a, 0x10, 255);
pub const PATCH: Rgba = Rgba(0xf5, 0x3c, 0xd1, 255);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColorError(pub String);

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a #rgb, #rrggbb or #rrggbbaa colour: {:?}", self.0)
    }
}

impl std::error::Error for ColorError {}

/// `"#rgb"`, `"#rrggbb"` or `"#rrggbbaa"` (the `#` optional) to a colour. As pixelfx, the result is always
/// opaque: an 8-digit string's alpha digits are ignored (pixelfx parses only the first six).
pub fn hex(s: &str) -> Result<Rgba, ColorError> {
    let err = || ColorError(s.to_owned());
    let digits = s.strip_prefix('#').unwrap_or(s);
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err());
    }
    let nib = |i: usize| {
        digits
            .get(i..i + 1)
            .and_then(|d| u8::from_str_radix(d, 16).ok())
            .ok_or_else(err)
    };
    let byte = |i: usize| {
        digits
            .get(i..i + 2)
            .and_then(|d| u8::from_str_radix(d, 16).ok())
            .ok_or_else(err)
    };
    match digits.len() {
        3 => Ok(Rgba(nib(0)? * 17, nib(1)? * 17, nib(2)? * 17, 255)),
        6 | 8 => Ok(Rgba(byte(0)?, byte(2)?, byte(4)?, 255)),
        _ => Err(err()),
    }
}

/// `"#rrggbb"` (pixelfx `toHex`; alpha dropped).
pub fn to_hex(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// Linear mix of two colours (t clamped to 0..1); channels truncate, alpha is 255 (pixelfx `mix`).
pub fn mix(a: Rgba, b: Rgba, t: f64) -> Rgba {
    let t = clamp(t, 0.0, 1.0);
    let ch = |x: u8, y: u8| f64::from(x) + (f64::from(y) - f64::from(x)) * t;
    rgb(ch(a.0, b.0), ch(a.1, b.1), ch(a.2, b.2))
}

/// Multiply a colour's brightness by k (capped at 255 per channel), alpha 255.
pub fn scale(c: Rgba, k: f64) -> Rgba {
    let ch = |x: u8| crate::math::js_min(255.0, f64::from(x) * k);
    rgb(ch(c.0), ch(c.1), ch(c.2))
}

/// Perceived brightness 0..1.
pub fn luma(c: Rgba) -> f64 {
    (0.299 * f64::from(c.0) + 0.587 * f64::from(c.1) + 0.114 * f64::from(c.2)) / 255.0
}

/// Step `i` of a list (JS `list[i]` for an integral `i`): `Rgba::NONE` past its end or for a NaN or negative
/// index, where JS gives `undefined`, which the painter also draws as nothing.
pub(crate) fn step_at(list: &[Rgba], i: f64) -> Rgba {
    if i >= 0.0 {
        list.get(i as usize).copied().unwrap_or(Rgba::NONE)
    } else {
        Rgba::NONE
    }
}

/// Resample a ramp to exactly n steps, keeping its ends. An empty ramp gives `Rgba::NONE` steps
/// (pixelfx gives `undefined`, which the painter also draws as nothing).
pub fn fit(ramp: &[Rgba], n: usize) -> Vec<Rgba> {
    if ramp.len() == n {
        return ramp.to_vec();
    }
    let last = ramp.len() as f64 - 1.0;
    let span = (n as f64 - 1.0).max(1.0);
    (0..n)
        .map(|i| step_at(ramp, js_round((i as f64 * last) / span)))
        .collect()
}

/// [`fit`] into an array of exactly `N` steps, so steps index without bounds checks failing.
pub fn ramp<const N: usize>(r: &[Rgba]) -> [Rgba; N] {
    let v = fit(r, N);
    core::array::from_fn(|i| v.get(i).copied().unwrap_or(Rgba::NONE))
}

/// The ramp step for t in 0..1, no blending (pixelfx `at`).
pub fn at(ramp: &[Rgba], t: f64) -> Rgba {
    let i = clamp(
        libm::floor(t * ramp.len() as f64),
        0.0,
        ramp.len() as f64 - 1.0,
    );
    step_at(ramp, i)
}

/// Blended colour along a ramp (for light, where smooth is right): pixelfx `blend`.
pub fn blend(ramp: &[Rgba], t: f64) -> Rgba {
    let x = clamp(t, 0.0, 1.0) * (ramp.len() as f64 - 1.0);
    let i = crate::math::js_min(ramp.len() as f64 - 2.0, libm::floor(x));
    mix(step_at(ramp, i), step_at(ramp, i + 1.0), x - i)
}

/// pixelfx `PAL`: hue-shifted ramps, dark to bright (`src/core/color.js` at 044e12a).
pub mod pal {
    use super::Rgba;

    pub const FIRE: [Rgba; 8] = [
        Rgba::hex24(0x1f0b10),
        Rgba::hex24(0x4a1118),
        Rgba::hex24(0x8c1d1c),
        Rgba::hex24(0xcc3a1a),
        Rgba::hex24(0xf2701f),
        Rgba::hex24(0xffae35),
        Rgba::hex24(0xffe27a),
        Rgba::hex24(0xfff8d6),
    ];

    pub const EMBER: [Rgba; 7] = [
        Rgba::hex24(0x1a0a0e),
        Rgba::hex24(0x3d1012),
        Rgba::hex24(0x701a14),
        Rgba::hex24(0xa8301a),
        Rgba::hex24(0xe0561e),
        Rgba::hex24(0xff8a2a),
        Rgba::hex24(0xffc46a),
    ];

    pub const LAVA: [Rgba; 7] = [
        Rgba::hex24(0x14060a),
        Rgba::hex24(0x380a10),
        Rgba::hex24(0x6e1210),
        Rgba::hex24(0xac2410),
        Rgba::hex24(0xe8501a),
        Rgba::hex24(0xff8c2a),
        Rgba::hex24(0xffd25a),
    ];

    pub const SMOKE: [Rgba; 7] = [
        Rgba::hex24(0x0e0c12),
        Rgba::hex24(0x1d1a22),
        Rgba::hex24(0x2e2a35),
        Rgba::hex24(0x433e4b),
        Rgba::hex24(0x5c5664),
        Rgba::hex24(0x7b7484),
        Rgba::hex24(0xa39cab),
    ];

    pub const DUST: [Rgba; 7] = [
        Rgba::hex24(0x1c1610),
        Rgba::hex24(0x33281c),
        Rgba::hex24(0x4f3f2c),
        Rgba::hex24(0x6e5a40),
        Rgba::hex24(0x927a58),
        Rgba::hex24(0xb89f78),
        Rgba::hex24(0xdcc8a2),
    ];

    pub const ICE: [Rgba; 8] = [
        Rgba::hex24(0x0c1430),
        Rgba::hex24(0x162c5c),
        Rgba::hex24(0x1f4f94),
        Rgba::hex24(0x2f7fcc),
        Rgba::hex24(0x58b4ec),
        Rgba::hex24(0x9bdcff),
        Rgba::hex24(0xd8f4ff),
        Rgba::hex24(0xffffff),
    ];

    pub const WATER: [Rgba; 8] = [
        Rgba::hex24(0x081830),
        Rgba::hex24(0x0e2e5a),
        Rgba::hex24(0x14508c),
        Rgba::hex24(0x1c76bc),
        Rgba::hex24(0x36a2e0),
        Rgba::hex24(0x78ccf4),
        Rgba::hex24(0xc4ecff),
        Rgba::hex24(0xffffff),
    ];

    pub const ELECTRIC: [Rgba; 7] = [
        Rgba::hex24(0x0a0f2e),
        Rgba::hex24(0x172a78),
        Rgba::hex24(0x2550d0),
        Rgba::hex24(0x3f8cff),
        Rgba::hex24(0x7cc8ff),
        Rgba::hex24(0xc8f0ff),
        Rgba::hex24(0xffffff),
    ];

    pub const VOLT: [Rgba; 7] = [
        Rgba::hex24(0x24160a),
        Rgba::hex24(0x5e3a0c),
        Rgba::hex24(0xa8680f),
        Rgba::hex24(0xe8a218),
        Rgba::hex24(0xffd23c),
        Rgba::hex24(0xfff08a),
        Rgba::hex24(0xffffff),
    ];

    pub const POISON: [Rgba; 7] = [
        Rgba::hex24(0x0d1a0c),
        Rgba::hex24(0x173a14),
        Rgba::hex24(0x265e18),
        Rgba::hex24(0x3f8a1e),
        Rgba::hex24(0x6cb82a),
        Rgba::hex24(0xa8e04a),
        Rgba::hex24(0xe0ff8a),
    ];

    pub const HOLY: [Rgba; 8] = [
        Rgba::hex24(0x2a1c08),
        Rgba::hex24(0x5a3e10),
        Rgba::hex24(0x9a6c18),
        Rgba::hex24(0xd8a428),
        Rgba::hex24(0xffd84a),
        Rgba::hex24(0xfff09a),
        Rgba::hex24(0xfffbe6),
        Rgba::hex24(0xffffff),
    ];

    pub const HEAL: [Rgba; 8] = [
        Rgba::hex24(0x08201a),
        Rgba::hex24(0x0f3f2c),
        Rgba::hex24(0x17663e),
        Rgba::hex24(0x24924e),
        Rgba::hex24(0x3ec466),
        Rgba::hex24(0x7ee88a),
        Rgba::hex24(0xc8ffc4),
        Rgba::hex24(0xffffff),
    ];

    pub const ARCANE: [Rgba; 8] = [
        Rgba::hex24(0x140a26),
        Rgba::hex24(0x2b1252),
        Rgba::hex24(0x481c88),
        Rgba::hex24(0x6b2cc0),
        Rgba::hex24(0x9a48ec),
        Rgba::hex24(0xc684ff),
        Rgba::hex24(0xecc8ff),
        Rgba::hex24(0xffffff),
    ];

    pub const SHADOW: [Rgba; 8] = [
        Rgba::hex24(0x050308),
        Rgba::hex24(0x0f0a18),
        Rgba::hex24(0x1c1230),
        Rgba::hex24(0x2c1a4a),
        Rgba::hex24(0x432468),
        Rgba::hex24(0x5e3290),
        Rgba::hex24(0x8048b8),
        Rgba::hex24(0xa970e0),
    ];

    pub const BLOOD: [Rgba; 7] = [
        Rgba::hex24(0x14040a),
        Rgba::hex24(0x3a0a14),
        Rgba::hex24(0x6a101c),
        Rgba::hex24(0xa01824),
        Rgba::hex24(0xd42a30),
        Rgba::hex24(0xff5a50),
        Rgba::hex24(0xff9a8a),
    ];

    pub const EARTH: [Rgba; 7] = [
        Rgba::hex24(0x140e0a),
        Rgba::hex24(0x2a1e14),
        Rgba::hex24(0x47321e),
        Rgba::hex24(0x69492a),
        Rgba::hex24(0x8e6638),
        Rgba::hex24(0xb88a52),
        Rgba::hex24(0xdcb47c),
    ];

    pub const STONE: [Rgba; 8] = [
        Rgba::hex24(0x0e0e12),
        Rgba::hex24(0x1c1c24),
        Rgba::hex24(0x2c2c38),
        Rgba::hex24(0x42424f),
        Rgba::hex24(0x5c5c6b),
        Rgba::hex24(0x7d7d8c),
        Rgba::hex24(0xa4a4b2),
        Rgba::hex24(0xd0d0da),
    ];

    pub const GOLD: [Rgba; 8] = [
        Rgba::hex24(0x1e1206),
        Rgba::hex24(0x4a2c08),
        Rgba::hex24(0x86520c),
        Rgba::hex24(0xc48412),
        Rgba::hex24(0xf0b424),
        Rgba::hex24(0xffe066),
        Rgba::hex24(0xfff6c4),
        Rgba::hex24(0xffffff),
    ];

    pub const LEAF: [Rgba; 8] = [
        Rgba::hex24(0x0c1a0e),
        Rgba::hex24(0x163418),
        Rgba::hex24(0x225224),
        Rgba::hex24(0x357530),
        Rgba::hex24(0x4f9a3a),
        Rgba::hex24(0x7cc048),
        Rgba::hex24(0xb4e070),
        Rgba::hex24(0xe4ffb0),
    ];

    pub const WOOD: [Rgba; 7] = [
        Rgba::hex24(0x160c06),
        Rgba::hex24(0x2e1a0c),
        Rgba::hex24(0x4a2c14),
        Rgba::hex24(0x6a421e),
        Rgba::hex24(0x8e5c2c),
        Rgba::hex24(0xb47c40),
        Rgba::hex24(0xd8a464),
    ];

    pub const METAL: [Rgba; 8] = [
        Rgba::hex24(0x0c0e14),
        Rgba::hex24(0x1c2230),
        Rgba::hex24(0x2e3648),
        Rgba::hex24(0x4a5468),
        Rgba::hex24(0x6e7a90),
        Rgba::hex24(0x9aa6ba),
        Rgba::hex24(0xd0d8e6),
        Rgba::hex24(0xffffff),
    ];

    pub const BONE: [Rgba; 7] = [
        Rgba::hex24(0x1a1814),
        Rgba::hex24(0x3a352c),
        Rgba::hex24(0x5e5648),
        Rgba::hex24(0x8a806c),
        Rgba::hex24(0xb8ad96),
        Rgba::hex24(0xe4dcc8),
        Rgba::hex24(0xfffaf0),
    ];

    pub const NEON: [Rgba; 8] = [
        Rgba::hex24(0x1a0516),
        Rgba::hex24(0x3d0a33),
        Rgba::hex24(0x701560),
        Rgba::hex24(0xb0228f),
        Rgba::hex24(0xf53cd1),
        Rgba::hex24(0xff86e6),
        Rgba::hex24(0xffd2f6),
        Rgba::hex24(0xffffff),
    ];

    pub const CYBER: [Rgba; 8] = [
        Rgba::hex24(0x041418),
        Rgba::hex24(0x08303a),
        Rgba::hex24(0x0c5260),
        Rgba::hex24(0x108088),
        Rgba::hex24(0x18b4b0),
        Rgba::hex24(0x4ce8d8),
        Rgba::hex24(0xb4fff0),
        Rgba::hex24(0xffffff),
    ];

    pub const MATRIX: [Rgba; 8] = [
        Rgba::hex24(0x020a04),
        Rgba::hex24(0x05200c),
        Rgba::hex24(0x0a3c16),
        Rgba::hex24(0x106624),
        Rgba::hex24(0x1a9a34),
        Rgba::hex24(0x3cd650),
        Rgba::hex24(0xa4ffa8),
        Rgba::hex24(0xe8ffe8),
    ];

    pub const CANDY: [Rgba; 7] = [
        Rgba::hex24(0xff4a5a),
        Rgba::hex24(0xff9a2a),
        Rgba::hex24(0xffe03c),
        Rgba::hex24(0x5ad84a),
        Rgba::hex24(0x3cc8ff),
        Rgba::hex24(0x7a6cff),
        Rgba::hex24(0xf53cd1),
    ];

    pub const NIGHT: [Rgba; 6] = [
        Rgba::hex24(0x07060d),
        Rgba::hex24(0x0c0a18),
        Rgba::hex24(0x131026),
        Rgba::hex24(0x1b1734),
        Rgba::hex24(0x241f44),
        Rgba::hex24(0x2f2956),
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    // expected values printed by node from pixelfx 044e12a src/core/color.js
    #[test]
    fn mix_truncates_like_pixelfx() -> Result<(), ColorError> {
        // -0x9fa6ad as an int32 is 0xff60_5953
        assert_eq!(
            mix(hex("#102030")?, hex("#f0e0d0")?, 0.3).pack(),
            0xff60_5953
        );
        assert_eq!(mix(BLACK, WHITE, 0.5).pack(), 0xff7f_7f7f);
        Ok(())
    }

    #[test]
    fn hex24_is_hex() -> Result<(), ColorError> {
        assert_eq!(Rgba::hex24(0x4a1712), hex("#4a1712")?);
        assert_eq!(Rgba::hex24(0xffffff), WHITE);
        Ok(())
    }

    #[test]
    fn fit_resamples_like_pixelfx() -> Result<(), ColorError> {
        let four = [hex("#000")?, hex("#111")?, hex("#222")?, hex("#333")?];
        let reds: Vec<u8> = fit(&four, 7).iter().map(|c| c.0).collect();
        assert_eq!(reds, [0x00, 0x11, 0x11, 0x22, 0x22, 0x33, 0x33]);
        let seven: Vec<Rgba> = (0..7u8).map(|i| Rgba(i * 0x11, 0, 0, 255)).collect();
        let reds: Vec<u8> = fit(&seven, 3).iter().map(|c| c.0).collect();
        assert_eq!(reds, [0x00, 0x33, 0x66]);
        Ok(())
    }
}
