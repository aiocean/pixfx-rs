//! OKLab (Björn Ottosson's perceptual colour space): distances in it follow how different two colours look, so
//! matching, merging and edge finding judge colours the way an eye does. `libm::cbrt` keeps the bits the same on
//! every platform.

use crate::Rgba;

/// A colour in OKLab: `l` lightness 0..1, `a` green to red, `b` blue to yellow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

/// An sRGB channel (0..255) as linear light (0..1).
fn linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        libm::pow((c + 0.055) / 1.055, 2.4)
    }
}

/// `c` in OKLab; its alpha is ignored.
pub fn lab(c: Rgba) -> Lab {
    let (r, g, b) = (linear(c.0), linear(c.1), linear(c.2));
    let l = libm::cbrt(0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b);
    let m = libm::cbrt(0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b);
    let s = libm::cbrt(0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b);
    Lab {
        l: 0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
        a: 1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
        b: 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
    }
}

/// How different two colours look: the straight-line distance between them in OKLab.
pub fn delta_e(x: Lab, y: Lab) -> f64 {
    let (dl, da, db) = (x.l - y.l, x.a - y.a, x.b - y.b);
    libm::sqrt(dl * dl + da * da + db * db)
}

/// A linear-light channel (0..1) back to sRGB (0..255), rounded and clamped.
fn encoded(c: f64) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let v = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * libm::pow(c, 1.0 / 2.4) - 0.055
    };
    libm::round(v.clamp(0.0, 1.0) * 255.0) as u8
}

/// The sRGB colour of `p` with alpha `alpha`, its channels clamped to what sRGB holds: the inverse of [`lab`].
pub fn rgb(p: Lab, alpha: u8) -> Rgba {
    let l = p.l + 0.396_337_777_4 * p.a + 0.215_803_757_3 * p.b;
    let m = p.l - 0.105_561_345_8 * p.a - 0.063_854_172_8 * p.b;
    let s = p.l - 0.089_484_177_5 * p.a - 1.291_485_548_0 * p.b;
    let (l, m, s) = (l * l * l, m * m * m, s * s * s);
    Rgba(
        encoded(4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s),
        encoded(-1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s),
        encoded(-0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s),
        alpha,
    )
}
