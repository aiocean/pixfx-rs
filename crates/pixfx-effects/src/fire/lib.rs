//! Shared by the fire effects (the leading _ keeps the tools from listing it as an effect).
//! Flame tongues, licks, flame puffs that cool into smoke, embers and sparks, with their motion in closed form so
//! looping effects can draw them from t alone and stay seamless. The tongue, lick, puff, smoke, ember and spark
//! recipes are adapted from Sprite Fusion's open-source fire (https://www.spritefusion.com/pixel-art-vfx/fire),
//! MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/fire/_lib.js`: the functions candle, campfire and brazier use (`flameBlade`,
//! `flicker` and `simDraw` arrive with the effects that need them).

use pixfx_core::Painter;
use pixfx_core::color::{Rgba, WHITE, mix};
use pixfx_core::math::{Rng, clamp, js_max, js_min, js_round, noise, rand, rng, wrap};
use pixfx_core::sim::SMOKE_RAMP;

/// A fire ramp hot-first: `H[0]` white .. `H[8]` smouldering.
pub type Hot = [Rgba; 9];

/// The noise of one frame: `N(rate, seed, off)`.
pub type Noise<'a> = &'a dyn Fn(f64, f64, f64) -> f64;

/// Tongue layers: `(width share, height share, step of the hot-first ramp)`.
pub type Layers = [(f64, f64, usize)];

/// Smooth value noise in [-1, 1] that repeats every `period` cells (a whole number).
pub fn pnoise(x: f64, period: f64, seed: f64) -> f64 {
    let i = libm::floor(x);
    let f = x - i;
    let u = f * f * (3.0 - 2.0 * f);
    let (a, b) = (
        rand(&[wrap(i, period), seed]),
        rand(&[wrap(i + 1.0, period), seed]),
    );
    (a + (b - a) * u) * 2.0 - 1.0
}

/// Noise for a loop D seconds long (pixelfx `noiseFor(D)`): [`LoopNoise::at`]`(tt, rate, seed, off)` wobbles
/// about `rate` times a second and is the same at tt and tt + D. D = 0 for one-shots (plain noise, no repeat).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoopNoise(pub f64);

impl LoopNoise {
    pub fn at(self, tt: f64, rate: f64, seed: f64, off: f64) -> f64 {
        let d = self.0;
        if d == 0.0 || d.is_nan() {
            return noise(tt * rate + off, seed);
        }
        let period = js_max(1.0, js_round(rate * d));
        pnoise((tt / d) * period + off, period, seed)
    }
}

/// A particle drawn by [`looped`]: how long each copy lives (may be longer than the loop).
pub trait Life {
    fn life(&self) -> f64;
}

/// Seamless particles for a loop D seconds long: count particles, each born at its own phase and reborn every D.
/// make(r, i) builds particle i from its own rng (the same every frame) and returns it with a life (which may
/// be longer than D); draw(q, age, i) is called for every live copy.
pub fn looped<Q: Life>(
    t: f64,
    d: f64,
    count: usize,
    seed: f64,
    mut make: impl FnMut(&mut Rng, usize) -> Q,
    mut draw: impl FnMut(&Q, f64, usize),
) {
    for i in 0..count {
        let mut r = rng(&[seed, i as f64]);
        let born = r.next() * d;
        let q = make(&mut r, i);
        let mut age = wrap(t - born, d);
        while age < q.life() {
            draw(&q, age, i);
            age += d;
        }
    }
}

/// A fire ramp hot-first (H[0] white .. H[8] smouldering) from a palette dark to bright.
pub fn hot_first(f: &Hot) -> Hot {
    let mut h = *f;
    h.reverse();
    h
}

/// Tongue layers: red rim, orange, yellow, white heart.
pub const LAYERS: [(f64, f64, usize); 4] = [
    (1.0, 1.0, 6),
    (0.76, 0.86, 4),
    (0.54, 0.68, 2),
    (0.3, 0.42, 0),
];

/// Layers for big tongues: more orange and a smaller white heart, so the light doesn't wash them out.
pub const BIG_LAYERS: [(f64, f64, usize); 6] = [
    (1.0, 1.0, 6),
    (0.8, 0.9, 5),
    (0.62, 0.78, 4),
    (0.44, 0.6, 3),
    (0.26, 0.4, 1),
    (0.12, 0.22, 0),
];

/// The same layers a step cooler, for flames further back: dark red rim, deep orange, gold, yellow.
pub const BACK_LAYERS: [(f64, f64, usize); 4] = [
    (1.0, 1.0, 7),
    (0.76, 0.86, 5),
    (0.54, 0.68, 3),
    (0.3, 0.42, 2),
];

/// A flame tongue standing at (x, y), h tall, w its half-width at the base, its tip swept sx px sideways: a red
/// rim round orange round yellow round a white heart, every row's width and centre jittered by noise so the
/// edges crackle. N(rate, seed, off) is the effect's noise at this frame; n seeds this tongue. H: hot-first ramp.
/// jit: how far the edges crackle (px); wob: how far the tip wanders (px).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn flame_tongue(
    p: &mut Painter,
    x: f64,
    y: f64,
    h: f64,
    w: f64,
    sx: f64,
    nf: Noise,
    n: f64,
    hh: &Hot,
    layers: &Layers,
    jit: f64,
    wob: f64,
) {
    if h < 1.0 {
        return;
    }
    // Rows climb away from the origin; once a row's centre line is past the frame's reach, so is every pixel of
    // it and of every row above (a pixel cell (px, yy) lies at least |floor(yy) + 0.5| from the origin), so the
    // rest are skipped. Same pixels as drawing them all, but a huge `h` costs only the rows that can show. (A NaN
    // placement makes `top` NaN and skips every row; it would have drawn nothing anyway.)
    let top = -(p.reach() + 2.0);
    for &(lw, lh, ri) in layers {
        let l = h * lh;
        let c = hh.get(ri).copied().unwrap_or(Rgba::NONE);
        let mut r = 0.0;
        while r <= l && y - r >= top {
            let u = r / js_max(1.0, l);
            let half = w * lw * libm::pow(1.0 - u, 0.85) * js_min(1.0, (r + 1.5) / 3.5)
                + nf(9.0, n, r * 0.45) * jit;
            if half >= 0.3 {
                let cx = x + sx * u * u * lh + nf(4.0, n + 7.0, r * 0.2) * u * wob;
                let yy = js_round(y - r);
                let (mut px, x1) = (js_round(cx - half), js_round(cx + half));
                while px <= x1 {
                    p.px(px, yy, c);
                    px += 1.0;
                }
            }
            r += 1.0;
        }
    }
}

/// A solid disc drawn straight in screen space (fast; a disc looks the same mirrored or turned).
pub fn solid(p: &mut Painter, x: f64, y: f64, r: f64, c: Rgba) {
    if r < 0.6 {
        return p.px(x, y, c);
    }
    let (cx, cy, r2) = (p.to_x(x, y), p.to_y(x, y), r * r);
    let mut sy = libm::floor(cy - r);
    while sy <= libm::ceil(cy + r) {
        let dy = sy + 0.5 - cy;
        let s2 = r2 - dy * dy;
        if s2 >= 0.0 {
            let s = libm::sqrt(s2);
            let mut sx = libm::ceil(cx - s - 0.5);
            while sx <= libm::floor(cx + s - 0.5) {
                p.put(sx, sy, c, 1.0);
                sx += 1.0;
            }
        }
        sy += 1.0;
    }
}

/// A translucent disc in screen space: smoke and haze.
pub fn soft(p: &mut Painter, x: f64, y: f64, r: f64, c: Rgba, a: f64) {
    if a <= 0.004 {
        return;
    }
    if r < 0.5 {
        return p.blend(x, y, c, a);
    }
    let (cx, cy, r2) = (p.to_x(x, y), p.to_y(x, y), r * r);
    let mut sy = libm::floor(cy - r);
    while sy <= libm::ceil(cy + r) {
        let dy = sy + 0.5 - cy;
        let s2 = r2 - dy * dy;
        if s2 >= 0.0 {
            let s = libm::sqrt(s2);
            let mut sx = libm::ceil(cx - s - 0.5);
            while sx <= libm::floor(cx + s - 0.5) {
                p.blend_px(sx, sy, c, a);
                sx += 1.0;
            }
        }
        sy += 1.0;
    }
}

/// A ball of flame with life left k (1 new, 0 gone) cooling down the hot-first ramp H as it ages: a darker rim,
/// a lit core up and to the left, a little tongue on top, a hot heart while it's young, and its light.
/// frac (0..1) nudges how hot it starts. (tx, ty): which way its tongue points (up, unless it's streaming).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn fire_puff(
    p: &mut Painter,
    x: f64,
    y: f64,
    k: f64,
    size: f64,
    frac: f64,
    hh: &Hot,
    glow_k: f64,
    tx: f64,
    ty: f64,
) {
    let age = 1.0 - k;
    let n = hh.len() as f64;
    let heat = js_min(0.999, libm::pow(age, 0.62) + frac * 0.1);
    let idx = js_min(n - 2.0, libm::floor(heat * (n - 1.0)));
    let step = |i: f64| {
        if i >= 0.0 {
            hh.get(i as usize).copied().unwrap_or(Rgba::NONE)
        } else {
            Rgba::NONE
        }
    };
    let r = size * (0.55 + 0.45 * js_min(1.0, age * 7.0)) * (1.0 - age * 0.55);
    let c = step(idx);
    if idx >= n - 2.0 {
        return soft(p, x, y, r, c, js_min(1.0, k * 3.0) * 0.85);
    }
    solid(p, x, y, r, step(js_min(n - 1.0, idx + 2.0)));
    if r > 1.6 && ty > -0.99 {
        solid(
            p,
            x + tx * r * 1.5,
            y + ty * r * 1.5,
            r * 0.3,
            step(js_min(n - 1.0, idx + 1.0)),
        );
    }
    solid(p, x - r * 0.16, y - r * 0.22, r * 0.78, c);
    if r > 1.6 {
        solid(p, x + tx * r * 0.9, y + ty * r * 0.9, r * 0.42, c);
    }
    if idx <= 3.0 && r > 1.8 {
        solid(
            p,
            x - r * 0.28,
            y - r * 0.34,
            r * 0.36,
            step(js_max(0.0, idx - 1.0)),
        );
    }
    if idx < 6.0 {
        p.glow(x, y, r * 2.4 + 2.0, c, glow_k * k);
    }
}

/// [`fire_puff`] with pixelfx's defaults: light 0.18, its tongue pointing up.
pub fn fire_puff_up(p: &mut Painter, x: f64, y: f64, k: f64, size: f64, frac: f64, hh: &Hot) {
    fire_puff(p, x, y, k, size, frac, hh, 0.18, 0.0, -1.0);
}

/// A lick: a blob of flame torn off a tongue's tip, a red rim round a yellow heart, shrinking as it rises.
pub fn lick(p: &mut Painter, x: f64, y: f64, k: f64, s: f64, hh: &Hot) {
    let r = s * (0.4 + 0.6 * k);
    solid(p, x, y, r + 0.6, if k > 0.5 { hh[5] } else { hh[6] });
    if k > 0.35 {
        solid(p, x, y - 0.3, r * 0.6, if k > 0.7 { hh[2] } else { hh[4] });
    }
}

fn ramp_at(r: &[Rgba], t: f64) -> Rgba {
    let len = r.len() as f64;
    let i = js_min(len - 1.0, js_max(0.0, libm::floor(t * len)));
    if i >= 0.0 {
        r.get(i as usize).copied().unwrap_or(Rgba::NONE)
    } else {
        Rgba::NONE
    }
}

/// A puff of smoke with life left k: three soft discs (a body, a denser core sitting low, a lit cap while it's
/// young), swelling and thinning. col tints it (`None` for plain smoke); big for slower, fuller smoke.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn smoke_puff(
    p: &mut Painter,
    x: f64,
    y: f64,
    k: f64,
    size: f64,
    col: Option<Rgba>,
    big: bool,
    a_mul: f64,
) {
    let age = 1.0 - k;
    let r = size * (0.65 + js_min(1.0, age * 2.5) * 0.45 + age * if big { 0.5 } else { 0.4 });
    let a = js_min(1.0, age * 12.0) * js_min(1.0, k * 2.4) * if big { 0.7 } else { 0.58 } * a_mul;
    let c = match col {
        Some(col) if col != Rgba::NONE => mix(col, ramp_at(&SMOKE_RAMP, age), age),
        _ => ramp_at(&SMOKE_RAMP, age * 1.05),
    };
    soft(p, x, y, r, c, a * 0.45);
    soft(p, x + r * 0.05, y + r * 0.06, r * 0.74, c, a * 0.62);
    if age < 0.5 {
        soft(
            p,
            x - r * 0.22,
            y - r * 0.3,
            r * 0.5,
            mix(c, WHITE, 0.12),
            a * 0.4 * (1.0 - age * 2.0),
        );
    }
}

/// An ember with life left k: a hot pixel cooling through cols [hot, warm, cool, rim], flickering out near the
/// end (ph: its flicker phase), with its own light. Big ones (size 2) are a little cross with a darker rim.
pub fn ember(p: &mut Painter, x: f64, y: f64, k: f64, size: f64, cols: &[Rgba; 4], ph: f64) {
    if k < 0.25 && libm::sin(ph) < 0.0 {
        return;
    }
    let c = if k > 0.6 {
        cols[0]
    } else if k > 0.3 {
        cols[1]
    } else {
        cols[2]
    };
    if size >= 2.0 {
        let rim = if k > 0.45 { cols[2] } else { cols[3] };
        p.px(x - 1.0, y, rim);
        p.px(x + 1.0, y, rim);
        p.px(x, y - 1.0, rim);
        p.px(x, y + 1.0, rim);
        p.px(x, y, c);
        p.glow(x, y, 8.0, c, 0.4 * k);
        return;
    }
    p.px(x, y, c);
    if size > 1.4 {
        p.px(x, y + 1.0, cols[3]);
    }
    p.glow(x, y, 3.0 + size, c, 0.35 * k);
}

/// A spark: a streak stretched along its velocity, a hot head and a cooler tail, and a point of light.
pub fn spark(p: &mut Painter, x: f64, y: f64, vx: f64, vy: f64, k: f64, cols: &[Rgba; 4]) {
    let h = libm::hypot(vx, vy);
    // `|| 1`: 0 and NaN are falsy
    let sp = if h == 0.0 || h.is_nan() { 1.0 } else { h };
    let (ux, uy, len) = (vx / sp, vy / sp, js_min(9.0, 1.0 + sp * 0.022));
    let head = if k > 0.6 {
        cols[0]
    } else if k > 0.3 {
        cols[1]
    } else {
        cols[2]
    };
    p.line(
        x - ux * len,
        y - uy * len,
        x - ux * len * 0.4,
        y - uy * len * 0.4,
        if k > 0.5 { head } else { cols[3] },
        1.0,
    );
    p.line(x - ux * len * 0.45, y - uy * len * 0.45, x, y, head, 1.0);
    p.glow(x, y, 3.0, head, 0.45 * k);
}

fn along(x: f64, v: f64, a: f64, k: f64, t: f64) -> (f64, f64) {
    if k <= 0.0 {
        return (x + v * t + 0.5 * a * t * t, v + a * t);
    }
    let (e, vt) = (libm::exp(-k * t), a / k);
    (x + vt * t + ((v - vt) * (1.0 - e)) / k, vt + (v - vt) * e)
}

/// Where a body is t seconds after launch from (x, y) at (vx, vy), under acceleration (ax, ay) px/s² and linear
/// drag kx, ky (1/s): [x, y, vx, vy]. The particle kinds' own numbers: FIRE (0, -95, 5, 3.2), SMOKE
/// (0, -16, 1.6, 1.2), EMBER (0, -30, 1.6, 1.6), SPARK (0, 427, 2.2, 2.2).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn drift(
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    t: f64,
    ax: f64,
    ay: f64,
    kx: f64,
    ky: f64,
) -> [f64; 4] {
    let (xx, xv) = along(x, vx, ax, kx, t);
    let (yy, yv) = along(y, vy, ay, ky, t);
    [xx, yy, xv, yv]
}

/// A spark's flight: falls, drags, and bounces off the floor at y = floor, losing speed each time
/// (pixelfx `sparkAt` with its defaults g = 427, k = 2.2).
pub fn spark_at(x: f64, y: f64, vx: f64, vy: f64, t: f64, floor: f64) -> [f64; 4] {
    let (g, k) = (427.0, 2.2);
    let mut s = [x, y, vx, vy];
    let mut left = t;
    for _ in 0..3 {
        let e = drift(s[0], s[1], s[2], s[3], left, 0.0, g, k, k);
        if e[1] < floor || s[1] >= floor {
            return e;
        }
        let (mut lo, mut hi) = (0.0, left);
        for _ in 0..16 {
            let m = (lo + hi) / 2.0;
            if drift(s[0], s[1], s[2], s[3], m, 0.0, g, k, k)[1] < floor {
                lo = m;
            } else {
                hi = m;
            }
        }
        let c = drift(s[0], s[1], s[2], s[3], hi, 0.0, g, k, k);
        s = [c[0], floor - 0.25, c[2] * 0.7, -c[3].abs() * 0.45];
        left -= hi;
    }
    let e = drift(s[0], s[1], s[2], s[3], left, 0.0, g, k, k);
    if e[1] < floor {
        e
    } else {
        [e[0], floor - 0.25, e[2], 0.0]
    }
}

/// A pool of light on the floor: an ellipse of glow, k at its middle falling to 0 at its rim.
pub fn floor_light(p: &mut Painter, x: f64, y: f64, rx: f64, ry: f64, c: Rgba, k: f64) {
    if k <= 0.004 || rx < 1.0 || ry < 0.5 {
        return;
    }
    p.shade(x - rx, y - ry, x + rx, y + ry, |px, py, out| {
        let (dx, dy) = ((px - x) / rx, (py - y) / ry);
        let d = dx * dx + dy * dy;
        if d >= 1.0 {
            return None;
        }
        let f = 1.0 - libm::sqrt(d);
        out.l = c;
        out.k = f * f * k;
        None
    });
}

const SMOKE1: Rgba = Rgba::hex24(0x5b5566);

/// One tongue of a [`burn`] fire: `(dx, dy, height share, half-width, back row?)`.
pub type Tongue = (f64, f64, f64, f64, bool);

/// What [`burn`] draws and how: pixelfx's `o` (its defaults in brackets, for ports that leave one unset).
pub struct Burn<'a> {
    pub h: &'a Hot,
    pub d: f64,
    pub nl: LoopNoise,
    pub s: f64,
    pub height: f64,
    /// The fire's base [0, 0].
    pub x: f64,
    pub y: f64,
    /// [0.1]
    pub lean: f64,
    /// How wide embers and smoke leave [6].
    pub spread: f64,
    /// Light strength [1].
    pub glow: f64,
    /// [46]
    pub glow_r: f64,
    /// Rates per second: flame puffs [8] and licks [4] per tongue, smoke [12], embers [8].
    pub puffs: f64,
    pub licks: f64,
    pub smoke: f64,
    pub embers: f64,
    /// [3]
    pub smoke_size: f64,
    /// [(1.6, 2.6)]
    pub smoke_life: (f64, f64),
    /// [3]
    pub wind: f64,
}

struct BurnTongue {
    dx: f64,
    y: f64,
    h: f64,
    w: f64,
    back: bool,
    env: f64,
    n: f64,
}

/// The tongues of one [`burn`] and how they move.
struct Fire<'a> {
    o: &'a Burn<'a>,
    tt: Vec<BurnTongue>,
}

impl Fire<'_> {
    fn fla(&self, tt: f64) -> f64 {
        let (nl, s) = (self.o.nl, self.o.s);
        0.74 + 0.16 * nl.at(tt, 7.3, s + 1.0, 0.0) + 0.1 * nl.at(tt, 19.0, s + 2.0, 0.0)
    }
    fn lean(&self, tt: f64) -> f64 {
        self.o.nl.at(tt, 0.67, self.o.s + 90.0, 0.0) * self.o.lean
    }
    fn tongue(&self, i: usize) -> Option<&BurnTongue> {
        self.tt.get(i)
    }
    fn height(&self, i: usize, tt: f64) -> f64 {
        let Some(&BurnTongue { h, n, .. }) = self.tongue(i) else {
            return f64::NAN;
        };
        let nl = self.o.nl;
        let leap = js_max(0.0, nl.at(tt, 2.6, n + 70.0, 0.0) - 0.55) * 1.4;
        (h + 0.25 * h * nl.at(tt, 3.3, n, 0.0) + h * leap) * (0.9 + 0.1 * self.fla(tt))
    }
    fn sweep(&self, i: usize, tt: f64, h: f64) -> f64 {
        let (n, dx) = self.tongue(i).map_or((f64::NAN, f64::NAN), |q| (q.n, q.dx));
        self.o.nl.at(tt, 2.2, n + 40.0, 0.0) * (1.0 + h * 0.08) + self.lean(tt) * h * 0.9
            - clamp(dx * 0.3, -2.5, 2.5)
    }
    fn tip(&self, i: usize, tt: f64) -> (f64, f64) {
        let h = self.height(i, tt);
        let (dx, y) = self.tongue(i).map_or((f64::NAN, f64::NAN), |q| (q.dx, q.y));
        (
            self.o.x + dx + self.sweep(i, tt, h) * 0.85,
            self.o.y + y - h * 0.88 + 3.0,
        )
    }
    fn tongue_at(&self, p: &mut Painter, t: f64, i: usize) {
        let Some(q) = self.tongue(i) else {
            return;
        };
        let h = self.height(i, t);
        let (nl, hh) = (self.o.nl, self.o.h);
        let nf = |rate: f64, seed: f64, off: f64| nl.at(t, rate, seed, off);
        let layers: &Layers = if q.back {
            &BACK_LAYERS
        } else if q.w > 2.2 {
            &BIG_LAYERS
        } else {
            &LAYERS
        };
        flame_tongue(
            p,
            self.o.x + q.dx,
            self.o.y + q.y,
            h,
            q.w,
            self.sweep(i, t, h),
            &nf,
            q.n,
            hh,
            layers,
            0.5,
            1.1 + h * 0.04,
        );
    }
}

struct PlumePuff {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    s: f64,
}

impl Life for PlumePuff {
    fn life(&self) -> f64 {
        self.life
    }
}

struct ShedPuff {
    i: usize,
    fire: f64,
    sl: f64,
    ox: f64,
    oy: f64,
    vx: f64,
    vy: f64,
    s: f64,
    f: f64,
    life: f64,
}

impl Life for ShedPuff {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Lick {
    i: usize,
    vx: f64,
    vy: f64,
    s: f64,
    life: f64,
}

impl Life for Lick {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Ember {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    s: f64,
    a: f64,
    b: f64,
}

impl Life for Ember {
    fn life(&self) -> f64 {
        self.life
    }
}

/// A fire for loops, drawn from t alone. Tongues T: [(dx, dy, height share, half-width, back row?), ...] from the
/// base (o.x, o.y), back row first then outer to inner; licks torn off their tips, flame puffs that burn out into
/// smoke, a smoke plume curling off with the draught, embers, and the fire's light.
/// `mid(p, fl)` is called after the tongues, before what flies off them: draw what the fire burns out of there.
/// Wide tongues get BIG_LAYERS. (pixelfx also returns `{ tip, fl }`; no ported caller uses them.)
pub fn burn(
    p: &mut Painter,
    t: f64,
    tongues: &[Tongue],
    o: &Burn,
    mid: impl FnOnce(&mut Painter, f64),
) {
    let (hh, d, nl, s) = (o.h, o.d, o.nl, o.s);
    let (hc, x0, y0, w, g) = (o.height, o.x, o.y, o.spread, o.glow);
    let smoke = mix(SMOKE1, hh[6], 0.25);
    let emb = [hh[1], hh[2], hh[4], hh[6]];
    let glow = mix(mix(hh[3], hh[4], 0.5), WHITE, 0.25);
    let mut rr = rng(&[s, 77.0]);
    let count = |rate: f64| js_round(rate * d) as usize;
    let tt: Vec<BurnTongue> = tongues
        .iter()
        .enumerate()
        .map(|(i, &(dx, dy, h, ww, back))| BurnTongue {
            dx: dx + rr.range(-0.3, 0.3),
            y: dy,
            h: 2.0 + hc * h * rr.range(0.92, 1.08),
            w: ww * rr.range(0.92, 1.08),
            back,
            env: h,
            n: (s * 7.0 + i as f64) * 13.0,
        })
        .collect();
    let fire = Fire { o, tt };
    let fl = fire.fla(t);
    let n_t = fire.tt.len();
    let env = |i: usize| fire.tongue(i).map_or(f64::NAN, |q| q.env);

    // the plume: each puff keeps the sway the plume had when it left, so the wiggle climbs it
    let (life0, life1) = o.smoke_life;
    looped(
        t,
        d,
        count(o.smoke),
        s + 300.0,
        |r, _| PlumePuff {
            x: r.range(-w, w) * 0.7,
            y: -hc * 0.8 - r.range(0.0, 6.0),
            vx: r.range(-2.0, 2.0),
            vy: -r.range(10.0, 17.0),
            life: r.range(life0, life1),
            s: r.range(0.7, 1.1) * o.smoke_size,
        },
        |q, age, _| {
            let [x, y, ..] = drift(q.x, q.y, q.vx, q.vy, age, o.wind, -20.0, 1.6, 1.0);
            smoke_puff(
                p,
                x0 + x + nl.at(t - age, 0.8, s + 60.0, 0.0) * (1.5 + 2.5 * age),
                y0 + y,
                1.0 - age / q.life,
                q.s,
                Some(smoke),
                false,
                1.0,
            );
        },
    );
    // flame puffs shed from the tips
    let mut puffs: Vec<[f64; 5]> = Vec::new();
    looped(
        t,
        d,
        n_t * count(o.puffs),
        s + 200.0,
        |r, j| {
            let i = j % n_t;
            let (fire_t, smokes) = (r.range(0.14, 0.3), r.chance(0.45));
            ShedPuff {
                i,
                fire: fire_t,
                sl: r.range(0.8, 1.4),
                ox: r.range(-1.0, 1.0),
                oy: r.range(0.0, 4.0),
                vx: r.range(-4.0, 4.0),
                vy: -r.range(15.0, 30.0),
                s: r.range(0.8, 1.6) * (0.8 + 0.6 * env(i)),
                f: r.next(),
                life: fire_t + if smokes { 1.4 } else { 0.0 },
            }
        },
        |q, age, _| {
            let (tx, ty) = fire.tip(q.i, t - age);
            let [x, y, vx, vy] = drift(
                tx + q.ox,
                ty + q.oy,
                q.vx,
                q.vy,
                js_min(age, q.fire),
                0.0,
                -95.0,
                5.0,
                3.2,
            );
            if age < q.fire {
                puffs.push([x, y, 1.0 - age / q.fire, q.s, q.f]);
                return;
            }
            if age - q.fire > q.sl {
                return;
            }
            let [sx, sy, ..] = drift(
                x,
                y,
                vx * 0.6,
                vy * 0.6 - 6.0,
                age - q.fire,
                o.wind,
                -16.0,
                1.6,
                1.2,
            );
            smoke_puff(
                p,
                sx,
                sy,
                1.0 - (age - q.fire) / q.sl,
                q.s * 0.7,
                None,
                false,
                1.0,
            );
        },
    );
    for i in 0..n_t {
        if fire.tt.get(i).is_some_and(|q| q.back) {
            fire.tongue_at(p, t, i);
        }
    }
    for i in 0..n_t {
        if fire.tt.get(i).is_some_and(|q| !q.back) {
            fire.tongue_at(p, t, i);
        }
    }
    mid(p, fl);
    // licks torn off the tips, rising and shrinking
    looped(
        t,
        d,
        n_t * count(o.licks),
        s + 220.0,
        |r, j| Lick {
            i: j % n_t,
            vx: r.range(-3.0, 3.0),
            vy: -r.range(12.0, 22.0),
            s: r.range(0.9, 1.6) * (0.8 + 0.5 * env(j % n_t)),
            life: 0.26,
        },
        |q, age, _| {
            let (tx, ty) = fire.tip(q.i, t - age);
            lick(
                p,
                tx + q.vx * age,
                ty - 2.0 + q.vy * age,
                1.0 - age / q.life,
                q.s,
                hh,
            );
        },
    );
    for &[x, y, k, sz, f] in &puffs {
        fire_puff_up(p, x, y, k, sz, f, hh);
    }
    // embers carried up by the heat, wandering and cooling
    looped(
        t,
        d,
        count(o.embers),
        s + 500.0,
        |r, _| Ember {
            x: r.range(-w, w),
            y: -r.range(hc * 0.3, hc * 0.8),
            vx: r.range(-15.0, 15.0),
            vy: -r.range(25.0, 60.0),
            life: r.range(1.1, 2.4),
            s: if r.chance(0.15) {
                2.0
            } else {
                r.range(1.0, 1.6)
            },
            a: r.range(0.0, 6.3),
            b: r.range(0.0, 6.3),
        },
        |q, age, _| {
            let [x, y, ..] = drift(q.x, q.y, q.vx, q.vy, age, 0.0, -30.0, 1.6, 1.6);
            ember(
                p,
                x0 + x + 7.0 * (libm::sin(age * 2.3 + q.a) - libm::sin(q.a)),
                y0 + y + 2.0 * (libm::sin(age * 3.0 + q.b) - libm::sin(q.b)),
                1.0 - age / q.life,
                q.s,
                &emb,
                age * 40.0 + q.a,
            );
        },
    );
    // light: a broad warm wash, lights on the flames, a hot heart at the base
    if g > 0.0 {
        let (r1, lx) = (o.glow_r, x0 + fire.lean(t) * 12.0);
        p.glow(lx, y0 - hc * 0.35, r1 * 1.5, glow, 0.22 * fl * g);
        p.glow(lx, y0 - hc * 0.75, r1, glow, 0.34 * fl * g);
        p.glow(x0, y0 - hc * 0.2, r1 * 0.32, hh[2], 0.2 * fl * g);
        p.glow(x0, y0 - 2.0, 8.0, hh[0], 0.14 * fl * g);
    }
}
