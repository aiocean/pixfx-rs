//! Canvas-exact blending: colour laid over a frame the way a browser's Canvas 2D does it (Chrome's Skia raster
//! blitter, 8 bits a channel, premultiplied). For code that reproduces what a canvas drew (`fillRect` under
//! `globalAlpha`, `drawImage` of a sprite) pixel for pixel. Ported pixelfx effects blend with
//! [`crate::Painter::blend_px`] instead, which reproduces pixelfx's own arithmetic.
//!
//! A [`Frame`]'s colour layer is straight alpha; each blend premultiplies the pixel it lands on, blends in 8 bits,
//! and straightens the result again as `getImageData` would read it.

use crate::color::Rgba;
use crate::frame::Frame;
use crate::math::{clamp, js_round};

/// Fill the whole-pixel box (`x`, `y`, `w`, `h`), in frame space plus the frame's offset, with `c` at alpha `a`
/// (0 to 1): a canvas `fillRect` under `globalAlpha = a`.
pub fn fill(frame: &mut Frame, x: i32, y: i32, w: i32, h: i32, c: Rgba, a: f64) {
    let a = clamp(a, 0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    for yy in y..y + h {
        for xx in x..x + w {
            if let Some(px) = pixel(frame, xx, yy) {
                *px = over(*px, c, a);
            }
        }
    }
}

/// Lay the premultiplied pixel `src` over the frame's pixel at world (`wx`, `wy`): one pixel of a canvas
/// `drawImage` of a premultiplied sprite.
pub fn over_premul(frame: &mut Frame, wx: i32, wy: i32, src: [u8; 4]) {
    if src[3] == 0 {
        return;
    }
    let Some(px) = pixel(frame, wx, wy) else {
        return;
    };
    if src[3] == 255 {
        *px = Rgba(src[0], src[1], src[2], 255).pack();
        return;
    }
    let d = Rgba::unpack(*px);
    let da = u32::from(d.3);
    let keep = 256 - u32::from(src[3]);
    let ch = |s: u8, dp: u32| (u32::from(s) + ((dp * keep) >> 8)).min(255);
    let pm = |v: u8| mul_div255(u32::from(v), da);
    let oa = ch(src[3], da);
    let out = |s: u8, v: u8| unpremultiply(ch(s, pm(v)), oa) as u8;
    *px = Rgba(out(src[0], d.0), out(src[1], d.1), out(src[2], d.2), oa as u8).pack();
}

/// An opaque colour `c` at coverage `cover`/256 over a premultiplied pixel (Skia `SkBlendARGB32`): how a canvas
/// paints a rectangle's partly covered edge pixels into a premultiplied buffer.
pub fn cover_over(d: [u8; 4], c: Rgba, cover: i32) -> [u8; 4] {
    if cover >= 256 {
        return [c.0, c.1, c.2, 255];
    }
    let s = cover.max(0) as u32;
    let ds = 256 - ((255 * s) >> 8);
    let ch = |sv: u8, dv: u8| (((u32::from(sv) * s) >> 8) + ((u32::from(dv) * ds) >> 8)).min(255) as u8;
    [ch(c.0, d[0]), ch(c.1, d[1]), ch(c.2, d[2]), ch(255, d[3])]
}

/// A premultiplied pixel as straight colour, as `getImageData` reads it.
pub fn straighten(p: [u8; 4]) -> Rgba {
    let a = u32::from(p[3]);
    let ch = |v: u8| unpremultiply(u32::from(v), a) as u8;
    Rgba(ch(p[0]), ch(p[1]), ch(p[2]), p[3])
}

/// `a * b / 255`, rounded, in integers (Skia `SkMulDiv255Round`).
pub fn mul_div255(a: u32, b: u32) -> u32 {
    let p = a * b + 128;
    (p + (p >> 8)) >> 8
}

/// Colour `c` at alpha `a` over the straight ABGR word `d`: the alpha taken to 8 bits, both colours premultiplied,
/// the destination scaled by `256 - alpha` in 8.8 fixed point, the sum read back straight. At low alpha the
/// rounding through premultiplied bytes is coarse; that is the canvas's look, kept on purpose.
fn over(d: u32, c: Rgba, a: f64) -> u32 {
    let sa = js_round(a * 255.0) as u32;
    if sa == 0 {
        return d;
    }
    let da = d >> 24;
    let keep = 256 - sa;
    let oa = sa + ((da * keep) >> 8);
    let mix = |cv: u8, s: u32| {
        let dp = mul_div255((d >> s) & 255, da);
        unpremultiply(mul_div255(u32::from(cv), sa) + ((dp * keep) >> 8), oa)
    };
    (oa << 24) | (mix(c.2, 16) << 16) | (mix(c.1, 8) << 8) | mix(c.0, 0)
}

/// A premultiplied channel `p` back to straight at alpha `a` (Skia `SkUnPremultiply`).
fn unpremultiply(p: u32, a: u32) -> u32 {
    if a == 0 {
        return 0;
    }
    let scale = ((255_u64 << 24) + u64::from(a / 2)) / u64::from(a);
    ((u64::from(p) * scale + (1 << 23)) >> 24).min(255) as u32
}

/// The colour word at world (`x`, `y`), if it is on the frame.
fn pixel(frame: &mut Frame, x: i32, y: i32) -> Option<&mut u32> {
    let (lx, ly) = (i64::from(x) - i64::from(frame.ox), i64::from(y) - i64::from(frame.oy));
    if lx < 0 || ly < 0 || lx >= i64::from(frame.w) || ly >= i64::from(frame.h) {
        return None;
    }
    frame.color.get_mut((ly * i64::from(frame.w) + lx) as usize)
}
