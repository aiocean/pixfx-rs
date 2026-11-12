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

/// Resample a ramp to exactly n steps, keeping its ends. An empty ramp gives `Rgba::NONE` steps
/// (pixelfx gives `undefined`, which the painter also draws as nothing).
pub fn fit(ramp: &[Rgba], n: usize) -> Vec<Rgba> {
    if ramp.len() == n {
        return ramp.to_vec();
    }
    let last = ramp.len() as f64 - 1.0;
    let span = (n as f64 - 1.0).max(1.0);
    (0..n)
        .map(|i| {
            let j = js_round((i as f64 * last) / span);
            if j >= 0.0 {
                ramp.get(j as usize).copied().unwrap_or(Rgba::NONE)
            } else {
                Rgba::NONE
            }
        })
        .collect()
}

/// The ramp step for t in 0..1, no blending (pixelfx `at`).
pub fn at(ramp: &[Rgba], t: f64) -> Rgba {
    let i = clamp(
        libm::floor(t * ramp.len() as f64),
        0.0,
        ramp.len() as f64 - 1.0,
    );
    if i >= 0.0 {
        ramp.get(i as usize).copied().unwrap_or(Rgba::NONE)
    } else {
        Rgba::NONE
    }
}

/// Blended colour along a ramp (for light, where smooth is right): pixelfx `blend`.
pub fn blend(ramp: &[Rgba], t: f64) -> Rgba {
    let x = clamp(t, 0.0, 1.0) * (ramp.len() as f64 - 1.0);
    let i = crate::math::js_min(ramp.len() as f64 - 2.0, libm::floor(x));
    let step = |j: f64| {
        if j >= 0.0 {
            ramp.get(j as usize).copied().unwrap_or(Rgba::NONE)
        } else {
            Rgba::NONE
        }
    };
    mix(step(i), step(i + 1.0), x - i)
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
