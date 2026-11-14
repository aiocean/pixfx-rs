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
