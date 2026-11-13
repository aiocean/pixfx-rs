//! Shared by the screen transitions: a 320 x 180 screen with its origin at the top-left. Each one covers the
//! screen in `color` by about 45% of its duration, holds, then uncovers so the last frame is empty.
//!
//! Port of pixelfx `src/fx/transitions/_lib.js`: the helpers the transitions use (`W`, `H`, `screenMeta`,
//! `clipTo`, `rect`, `field`, `pixels`, `lifeLeft`, `glint`).
//!
//! `glint` is adapted from Sprite Fusion's open-source sparkle (https://www.spritefusion.com/pixel-art-vfx/sparkle),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::math::{TAU, clamp, js_min};
use pixfx_core::{Meta, Painter};

pub const W: usize = 320;
pub const H: usize = 180;
const WF: f64 = W as f64;
const HF: f64 = H as f64;

/// The cover colour every transition defaults to (pixelfx `screenMeta` `options.color`).
pub const COLOR: Rgba = Rgba::hex24(0x0b0a10);

/// The meta every transition shares; pass what differs. `tags` are the whole list, pixelfx's
/// `["transition", "screen", ...tags, "one-shot"]`.
pub const fn screen_meta(
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    tags: &'static [&'static str],
    duration: f64,
    seed: u64,
) -> Meta {
    Meta {
        id,
        name,
        desc,
        tags,
        size: (W as u32, H as u32),
        origin: (0, 0),
        duration,
        looping: false,
        fps: 30,
        seed,
    }
}

/// The painter, clipped to the screen: sparks, smoke and light that fly past an edge are cut there instead of
/// spilling outside the canvas (pixelfx `clipTo`).
pub fn clip_to(p: &mut Painter) {
    p.clip_local(WF, HF);
}

/// An opaque filled rectangle (pixelfx `rect`, a faster `p.rect` with the same pixels).
pub fn rect(p: &mut Painter, x: f64, y: f64, w: f64, h: f64, c: Rgba) {
    p.rect(x, y, w, h, c, 1.0);
}

/// A scalar field over the screen (pixelfx `field`, with its default gradient bounds). Values are stored as
/// `f32`, as pixelfx's `Float32Array`s.
pub struct Field {
    /// The value at each pixel centre, `i = y * W + x`.
    pub f: Vec<f32>,
    /// One over the gradient's length, so `(k - f[i]) * g[i]` is roughly the distance in pixels from pixel `i`
    /// to the level set `f = k`.
    pub g: Vec<f32>,
    /// The gradient's vertical share: -1 rising upward .. 1 downward.
    pub ny: Vec<f32>,
    pub lo: f64,
    pub hi: f64,
    /// The pixels from the lowest value to the highest (ties keep pixel order).
    pub order: Vec<u32>,
    sorted: Vec<f32>,
}

impl Field {
    /// How many pixels have a value below `v`: those pixels are `order[0 .. below(v)]`.
    pub fn below(&self, v: f64) -> usize {
        let (mut a, mut b) = (0, self.sorted.len());
        while a < b {
            let m = (a + b) >> 1;
            if self.sorted.get(m).is_some_and(|&s| f64::from(s) < v) {
                a = m + 1;
            } else {
                b = m;
            }
        }
        a
    }

    /// The value at pixel `i` (0 off the screen).
    pub fn at(&self, i: usize) -> f64 {
        self.f.get(i).copied().map_or(0.0, f64::from)
    }
}

const G_MIN: f64 = 0.45;
const G_MAX: f64 = 3.0;

/// The field `fn_(x, y)` sampled at every pixel centre.
pub fn field(fn_: impl Fn(f64, f64) -> f64) -> Field {
    let mut f = vec![0.0_f32; W * H];
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for (i, slot) in f.iter_mut().enumerate() {
        let v = fn_((i % W) as f64 + 0.5, (i / W) as f64 + 0.5);
        *slot = v as f32;
        if v < lo {
            lo = v;
        }
        if v > hi {
            hi = v;
        }
    }
    let at = |x: usize, y: usize| f.get(y * W + x).copied().map_or(0.0, f64::from);
    let mut g = vec![0.0_f32; W * H];
    let mut ny = vec![0.0_f32; W * H];
    for y in 0..H {
        for x in 0..W {
            let (xa, xb, ya, yb) = (
                x.saturating_sub(1),
                (x + 1).min(W - 1),
                y.saturating_sub(1),
                (y + 1).min(H - 1),
            );
            let gx = (at(xb, y) - at(xa, y)) / (xb - xa) as f64;
            let gy = (at(x, yb) - at(x, ya)) / (yb - ya) as f64;
            let l = libm::hypot(gx, gy);
            let i = y * W + x;
            if let Some(s) = g.get_mut(i) {
                *s = (1.0 / clamp(l, G_MIN, G_MAX)) as f32;
            }
            if let Some(s) = ny.get_mut(i) {
                *s = if l > 1e-6 { (gy / l) as f32 } else { 0.0 };
            }
        }
    }
    let mut order: Vec<u32> = (0..(W * H) as u32).collect();
    let val = |i: u32| f.get(i as usize).copied().unwrap_or(0.0);
    // stable, as V8's typed-array sort: equal values keep pixel order
    order.sort_by(|&a, &b| {
        (f64::from(val(a)) - f64::from(val(b)))
            .partial_cmp(&0.0)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let sorted = order.iter().map(|&i| val(i)).collect();
    Field {
        f,
        g,
        ny,
        lo,
        hi,
        order,
        sorted,
    }
}

/// Writes whole screen pixels by index (pixelfx `pixels`): `at(i)` picks pixel `i = y * W + x`, then `put`,
/// `add` and `blend` write it. pixelfx writes straight to the frame when the painter is unrotated; this goes
/// through the painter's cell mapping, which lands on the same pixels for any whole-pixel placement.
pub struct Pixels<'p, 'a> {
    p: &'p mut Painter<'a>,
    sx: f64,
    sy: f64,
}

pub fn pixels<'p, 'a>(p: &'p mut Painter<'a>) -> Pixels<'p, 'a> {
    Pixels {
        p,
        sx: 0.0,
        sy: 0.0,
    }
}

impl Pixels<'_, '_> {
    pub fn at(&mut self, i: usize) {
        let (x, y) = ((i % W) as f64 + 0.5, (i / W) as f64 + 0.5);
        self.sx = self.p.cell_x(x, y);
        self.sy = self.p.cell_y(x, y);
    }
    pub fn put(&mut self, c: Rgba) {
        self.p.put(self.sx, self.sy, c, 1.0);
    }
    pub fn add(&mut self, c: Rgba, k: f64) {
        self.p.add(self.sx, self.sy, c, k);
    }
    pub fn blend(&mut self, c: Rgba, a: f64) {
        self.p.blend_px(self.sx, self.sy, c, a);
    }
}

/// Particle lives cut so nothing outlives the effect: the smaller of life and what's left before `end`.
pub fn life_left(life: f64, t: f64, end: f64) -> f64 {
    js_min(life, end - t)
}

/// A spinning 8-ray glint at (x, y): long rays white at the heart, `c1` in the middle and `c2` at the tips,
/// short diagonals, the long rays thickened near the heart, and two lights. `ang` spins it; `a` fades it
/// (dithered).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn glint(p: &mut Painter, x: f64, y: f64, len: f64, ang: f64, c1: Rgba, c2: Rgba, a: f64) {
    if len < 0.8 || a <= 0.0 {
        return;
    }
    for i in 0..8 {
        let diag = i % 2 == 1;
        let an = ang + (f64::from(i) * TAU) / 8.0;
        let l = if diag { len * 0.42 } else { len };
        let (ca, sa) = (libm::cos(an), libm::sin(an));
        let mut d = 0.0;
        while d <= l {
            let u = d / l;
            let c = if diag {
                if u < 0.5 { c1 } else { c2 }
            } else if u < 0.3 {
                WHITE
            } else if u < 0.65 {
                c1
            } else {
                c2
            };
            p.dot(
                x + ca * d,
                y + sa * d,
                c,
                a * if u < 0.7 { 1.0 } else { 1.0 - (u - 0.7) / 0.35 },
            );
            if !diag && u < 0.22 && len > 4.0 {
                p.dot(x + ca * d - sa, y + sa * d + ca, c1, a);
                p.dot(x + ca * d + sa, y + sa * d - ca, c1, a);
            }
            d += 0.5;
        }
    }
    p.dot(x, y, WHITE, a);
    p.glow(x, y, 4.0 + len * 1.4, c1, 0.5 * a);
    p.glow(x, y, 2.0 + len * 0.5, WHITE, 0.4 * a);
}
