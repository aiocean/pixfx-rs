//! Frames and compositing: port of the frame and composite parts of pixelfx `src/core/fx.js`.

use crate::math::{js_max, js_min};

/// A frame: w x h pixels whose top-left sits at (ox, oy) in world space. `color` holds pixels (ABGR words,
/// see [`crate::color::Rgba::pack`]), `light` additive glow (xBGR words, top byte ignored);
/// [`composite`] puts them together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub ox: i32,
    pub oy: i32,
    pub color: Vec<u32>,
    pub light: Vec<u32>,
}

impl Frame {
    pub fn new(w: u32, h: u32, ox: i32, oy: i32) -> Self {
        let n = w as usize * h as usize;
        Self {
            w,
            h,
            ox,
            oy,
            color: vec![0; n],
            light: vec![0; n],
        }
    }

    /// Empty both buffers (pixelfx `clearFrame`).
    pub fn clear(&mut self) {
        self.color.fill(0);
        self.light.fill(0);
    }
}

/// FNV-1a 64 over the `color` words, then the `light` words, each as 4 little-endian bytes. A stable
/// fingerprint of a frame's pixels for golden checks (native and wasm must agree on it).
pub fn frame_hash(f: &Frame) -> u64 {
    const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    f.color
        .iter()
        .chain(&f.light)
        .flat_map(|w| w.to_le_bytes())
        .fold(BASIS, |h, b| (h ^ u64::from(b)).wrapping_mul(PRIME))
}

/// JS `Uint8ClampedArray` store: NaN gives 0, clamp to 0..255, round half to even.
fn clamp_u8(x: f64) -> u8 {
    if x.is_nan() || x <= 0.0 {
        return 0;
    }
    if x >= 255.0 {
        return 255;
    }
    let f = libm::floor(x);
    let r = if x - f > 0.5 || (x - f == 0.5 && f % 2.0 != 0.0) {
        f + 1.0
    } else {
        f
    };
    r as u8
}

/// Colour over a background, plus light, into RGBA8 bytes (`out`: w * h * 4; extra pixels are left alone).
/// `bg`: `None` for transparent, or RGBA8 bytes of the frame's size (its alpha is ignored; pixels past its
/// end composite as transparent). Over transparency, light becomes colour with alpha its brightest channel,
/// which reads as additive over dark backgrounds (pixelfx `composite` with lightK 1; beanfx `pfx.composite`).
pub fn composite(frame: &Frame, out: &mut [u8], bg: Option<&[u8]>) {
    let n = frame.w as usize * frame.h as usize;
    let pixels = frame
        .color
        .iter()
        .zip(&frame.light)
        .zip(out.as_chunks_mut::<4>().0)
        .take(n);
    for (i, ((&c, &l), o)) in pixels.enumerate() {
        let l = l & 0x00ff_ffff;
        let ch = |v: u32, s: u32| f64::from((v >> s) & 255);
        let ca = c >> 24;
        let ground = bg.and_then(|b| b.get(i * 4..i * 4 + 3));
        let (mut r, mut g, mut b, mut a);
        if ca == 255 {
            (r, g, b, a) = (ch(c, 0), ch(c, 8), ch(c, 16), 255.0);
        } else if let Some(k) = ground {
            let f = f64::from(ca) / 255.0;
            let mixk = |s: u32, kb: u8| ch(c, s) * f + f64::from(kb) * (1.0 - f);
            (r, g, b, a) = (mixk(0, k[0]), mixk(8, k[1]), mixk(16, k[2]), 255.0);
        } else {
            // translucent colour over nothing keeps its alpha
            (r, g, b, a) = (ch(c, 0), ch(c, 8), ch(c, 16), f64::from(ca));
        }
        if l != 0 {
            let (lr, lg, lb) = (ch(l, 0), ch(l, 8), ch(l, 16));
            if a < 255.0 {
                // over transparency: premultiply what's there, add the light, divide by the new alpha
                let pa = a / 255.0;
                let (pr, pg, pb) = (r * pa + lr, g * pa + lg, b * pa + lb);
                let na = js_min(1.0, js_max(pa, js_max(lr, js_max(lg, lb)) / 255.0));
                if na > 0.0 {
                    r = js_min(255.0, pr / na);
                    g = js_min(255.0, pg / na);
                    b = js_min(255.0, pb / na);
                    a = na * 255.0;
                }
            } else {
                r = js_min(255.0, r + lr);
                g = js_min(255.0, g + lg);
                b = js_min(255.0, b + lb);
            }
        }
        *o = [clamp_u8(r), clamp_u8(g), clamp_u8(b), clamp_u8(a)];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_hash_is_fnv1a_64_of_little_endian_words() {
        // 1x1 frame, 8 zero bytes: hand-run FNV-1a 64 (h = (h ^ 0) * prime, eight times)
        assert_eq!(frame_hash(&Frame::new(1, 1, 0, 0)), 0xa8c7_f832_281a_39c5);
        // color 0x12345678 then light 0x89abcdef: bytes 78 56 34 12 ef cd ab 89
        let mut f = Frame::new(1, 1, 0, 0);
        f.color[0] = 0x1234_5678;
        f.light[0] = 0x89ab_cdef;
        assert_eq!(frame_hash(&f), 0x5c5d_6ca7_accc_719d);
    }
}
