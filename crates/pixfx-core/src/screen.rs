//! Screen passes: the camera of a hit. Shake moves the view, ripples bend it, a flash lifts it toward white.
//! Port of the camera pass in game-factory `references/pixelfx/demo/bloater.js`.

use crate::frame::Frame;
use crate::math::js_round;

/// A ring of screen distortion alive now: centred on (`x`, `y`) in the destination frame's pixels, growing from
/// `r0` to `r1` over `dur` seconds, `age` seconds in.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Ripple {
    pub x: f64,
    pub y: f64,
    pub r0: f64,
    pub r1: f64,
    pub dur: f64,
    pub amp: f64,
    pub age: f64,
}

/// Pixels a ripple pushes the view at full amplitude, and the half-width of its ring.
const PUSH: f64 = 22.0;
const BAND: f64 = 10.0;

/// Where `dst` pixel (`x`, `y`) reads from `src`, in source pixels before rounding: the shake moves the view by
/// `shake`, each ripple pushes it along the ring's radius.
fn source_of(x: f64, y: f64, shake: (i32, i32), ripples: &[Ripple]) -> (f64, f64) {
    let mut sx = x - f64::from(shake.0);
    let mut sy = y - f64::from(shake.1);
    for r in ripples {
        let u = r.age / r.dur;
        let radius = r.r0 + (r.r1 - r.r0) * u;
        let (ex, ey) = (x - r.x, y - r.y);
        let d = match libm::hypot(ex, ey) {
            d if d == 0.0 || d.is_nan() => 1.0,
            d => d,
        };
        let band = 1.0 - (d - radius).abs() / BAND;
        if band > 0.0 {
            let push = r.amp * PUSH * (1.0 - u) * band;
            sx -= ex / d * push;
            sy -= ey / d * push;
        }
    }
    (sx, sy)
}

/// Clamp a rounded source coordinate into `0..n`; NaN reads the first pixel.
fn clamp_index(v: f64, n: u32) -> usize {
    let top = f64::from(n.saturating_sub(1));
    let v = js_round(v);
    if v.is_nan() || v <= 0.0 {
        0
    } else if v >= top {
        top as usize
    } else {
        v as usize
    }
}

/// Draw `src` into `dst` through the camera: `shake` (whole pixels) moves the view, `ripples` bend it. Colour and
/// light both go through. `src` may be larger than `dst` by an equal margin on each side (the shake reads into
/// it); reads past `src` take its nearest edge pixel.
pub fn ripple(src: &Frame, dst: &mut Frame, ripples: &[Ripple], shake: (i32, i32)) {
    let mx = f64::from(src.w.saturating_sub(dst.w) / 2);
    let my = f64::from(src.h.saturating_sub(dst.h) / 2);
    for y in 0..dst.h {
        for x in 0..dst.w {
            let (sx, sy) = source_of(f64::from(x), f64::from(y), shake, ripples);
            let si = clamp_index(sy + my, src.h) * src.w as usize + clamp_index(sx + mx, src.w);
            let di = y as usize * dst.w as usize + x as usize;
            if let (Some(&c), Some(&l)) = (src.color.get(si), src.light.get(si)) {
                if let Some(d) = dst.color.get_mut(di) {
                    *d = c;
                }
                if let Some(d) = dst.light.get_mut(di) {
                    *d = l;
                }
            }
        }
    }
}

/// Lift each colour byte of `word` toward 255 by `k`; the top byte (alpha or unused) stays.
fn lift(word: u32, k: f64) -> u32 {
    let [r, g, b, top] = word.to_le_bytes();
    let up = |c: u8| {
        let c = f64::from(c);
        js_round(c + (255.0 - c) * k).clamp(0.0, 255.0) as u8
    };
    u32::from_le_bytes([up(r), up(g), up(b), top])
}

/// Lift the frame toward white by `k` (0 leaves it, 1 whites it), colour and light alike.
pub fn flash(frame: &mut Frame, k: f64) {
    if k <= 0.0 || k.is_nan() {
        return;
    }
    for p in frame.color.iter_mut().chain(frame.light.iter_mut()) {
        *p = lift(*p, k);
    }
}
