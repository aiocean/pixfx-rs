//! Numbers, randomness and noise: port of pixelfx `src/core/math.js`. Everything here is pure and
//! deterministic, so an effect drawn at time t always looks the same.
//!
//! JS number semantics are reproduced where they change results: `Math.round` (half up toward +inf),
//! `Math.min`/`Math.max` (NaN wins), `n | 0` (ToInt32) and `n >>> 0` (ToUint32).

pub const TAU: f64 = core::f64::consts::PI * 2.0;

/// JS `Math.round`: nearest integer, halves toward +inf (`-0.5` gives `-0`).
pub fn js_round(x: f64) -> f64 {
    let r = libm::floor(x);
    let up = if x - r >= 0.5 { r + 1.0 } else { r };
    if up == 0.0 && x < 0.0 { -0.0 } else { up }
}

/// JS `Math.sign`: -1, 0 or 1; NaN and a signed zero come back as they are.
pub fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

/// JS `x || 1`: 0 and NaN are falsy, so they give 1.
pub fn or_one(x: f64) -> f64 {
    if x == 0.0 || x.is_nan() { 1.0 } else { x }
}

/// JS `Math.min(a, b)`: NaN if either is NaN, and -0 is less than +0.
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// JS `Math.max(a, b)`: NaN if either is NaN, and +0 is greater than -0.
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b || (a == b && a.is_sign_positive()) {
        a
    } else {
        b
    }
}

/// JS ToUint32 (`n >>> 0`): truncate toward zero, wrap modulo 2^32; NaN and infinities give 0.
pub fn to_uint32(x: f64) -> u32 {
    // Fast path for the common case (every pixel channel goes through here): inside (-2^31, 2^31) ToUint32 is
    // the truncated value wrapped to 32 bits, which `as i32` computes exactly. NaN fails both tests.
    if x > -2_147_483_648.0 && x < 2_147_483_648.0 {
        return x as i32 as u32;
    }
    if !x.is_finite() {
        return 0;
    }
    // fmod of an integer-valued double is exact and lies in (-2^32, 2^32), so the i64 cast is exact
    let m = libm::fmod(libm::trunc(x), 4_294_967_296.0) as i64;
    m as u32
}

/// JS ToInt32 (`n | 0`).
pub fn to_int32(x: f64) -> i32 {
    to_uint32(x) as i32
}

/// `x` clamped to `[a, b]` (pixelfx `clamp`; pass `0.0, 1.0` for its defaults). NaN passes through.
pub fn clamp(x: f64, a: f64, b: f64) -> f64 {
    if x < a {
        a
    } else if x > b {
        b
    } else {
        x
    }
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub fn inv_lerp(a: f64, b: f64, x: f64) -> f64 {
    if b == a { 0.0 } else { (x - a) / (b - a) }
}

pub fn remap(x: f64, a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    lerp(b0, b1, inv_lerp(a0, a1, x))
}

pub fn fract(x: f64) -> f64 {
    x - libm::floor(x)
}

/// `((x % m) + m) % m`: JS `%` is the truncating remainder, as Rust's `%` on f64.
pub fn wrap(x: f64, m: f64) -> f64 {
    ((x % m) + m) % m
}

/// Progress 0..1 of t through the window [t0, t1]: 0 before it, 1 after it.
pub fn seg(t: f64, t0: f64, t1: f64) -> f64 {
    clamp((t - t0) / (t1 - t0), 0.0, 1.0)
}

/// Hermite smoothstep of x between e0 and e1.
pub fn smooth(e0: f64, e1: f64, x: f64) -> f64 {
    let t = seg(x, e0, e1);
    t * t * (3.0 - 2.0 * t)
}

/// 0 → 1 → 0 over the window [t0, t1] (sine bump).
pub fn bump(t: f64, t0: f64, t1: f64) -> f64 {
    if t <= t0 || t >= t1 {
        0.0
    } else {
        libm::sin(core::f64::consts::PI * seg(t, t0, t1))
    }
}

pub fn pingpong(x: f64) -> f64 {
    1.0 - (wrap(x, 2.0) - 1.0).abs()
}

pub fn dist(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    libm::hypot(x1 - x0, y1 - y0)
}

pub fn angle_to(x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    libm::atan2(y1 - y0, x1 - x0)
}

/// 32-bit integer hash of any number of integers (each taken as JS `n | 0`).
pub fn hash(ns: &[f64]) -> u32 {
    let mut h: u32 = 0x9e37_79b9;
    for &n in ns {
        h = (h ^ to_int32(n) as u32).wrapping_mul(0x85eb_ca6b);
        h ^= h >> 13;
        h = h.wrapping_mul(0xc2b2_ae35);
        h ^= h >> 16;
    }
    h
}

/// Deterministic random number in [0, 1) from integers (seed, particle index, which value...):
/// pixelfx `rand(...ns)`.
pub fn rand(ns: &[f64]) -> f64 {
    f64::from(hash(ns)) / 4_294_967_296.0
}

/// Seeded random generator (mulberry32).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    s: u32,
}

impl Rng {
    /// pixelfx `new Rng(seed)`: `seed >>> 0 || 1` (pass the seed through [`to_uint32`] if it is a number).
    pub fn new(seed: u32) -> Self {
        Self {
            s: if seed == 0 { 1 } else { seed },
        }
    }

    #[expect(
        clippy::should_implement_trait,
        reason = "pixelfx's name; an endless stream, not an Iterator"
    )]
    pub fn next(&mut self) -> f64 {
        self.s = self.s.wrapping_add(0x6d2b_79f5);
        let mut t = self.s;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }

    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }

    /// Integer in [a, b], both ends included.
    pub fn int(&mut self, a: f64, b: f64) -> f64 {
        libm::floor(self.range(a, b + 1.0))
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.next() < p
    }

    pub fn sign(&mut self) -> f64 {
        if self.next() < 0.5 { -1.0 } else { 1.0 }
    }

    /// One item of `list`; `None` only when the list is empty (JS gives `undefined`).
    pub fn pick<'l, T>(&mut self, list: &'l [T]) -> Option<&'l T> {
        let i = libm::floor(self.next() * list.len() as f64) as usize;
        list.get(i)
    }

    /// Roughly normal, mean 0, sd 1.
    pub fn gauss(&mut self) -> f64 {
        (self.next() + self.next() + self.next() + self.next() - 2.0) * 1.732
    }
}

/// A generator for one particle: `rng(&[seed, i])` gives the same numbers every frame.
pub fn rng(ns: &[f64]) -> Rng {
    Rng::new(hash(ns))
}

/// Smooth 1D value noise in [-1, 1].
pub fn noise(x: f64, seed: f64) -> f64 {
    let i = libm::floor(x);
    let f = x - i;
    let u = f * f * (3.0 - 2.0 * f);
    lerp(rand(&[i, seed]), rand(&[i + 1.0, seed]), u) * 2.0 - 1.0
}

/// Smooth 2D value noise in [-1, 1].
pub fn noise2(x: f64, y: f64, seed: f64) -> f64 {
    let (ix, iy) = (libm::floor(x), libm::floor(y));
    let (fx, fy) = (x - ix, y - iy);
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let a = rand(&[ix, iy, seed]);
    let b = rand(&[ix + 1.0, iy, seed]);
    let c = rand(&[ix, iy + 1.0, seed]);
    let d = rand(&[ix + 1.0, iy + 1.0, seed]);
    lerp(lerp(a, b, ux), lerp(c, d, ux), uy) * 2.0 - 1.0
}

/// Fractal 2D noise in about [-1, 1] (pixelfx `fbm`; its defaults are `seed = 0`, `octaves = 3`).
pub fn fbm(mut x: f64, mut y: f64, seed: f64, octaves: u32) -> f64 {
    let (mut s, mut a, mut n) = (0.0, 0.5, 0.0);
    for o in 0..octaves {
        s += noise2(x, y, seed + f64::from(o) * 101.0) * a;
        n += a;
        x *= 2.03;
        y *= 2.03;
        a *= 0.5;
    }
    s / n
}

/// 2D value noise whose lattice repeats every `px` cells in x and `py` cells in y (0: no repeat; pixelfx
/// `tileNoise`). Scroll it by exactly `py` cells over a loop and the loop has no seam.
pub fn tile_noise(x: f64, y: f64, px: f64, py: f64, seed: f64) -> f64 {
    let (ix, iy) = (libm::floor(x), libm::floor(y));
    let (fx, fy) = (x - ix, y - iy);
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);
    let cell = |i: f64, period: f64| if period != 0.0 { wrap(i, period) } else { i };
    let (x0, x1) = (cell(ix, px), cell(ix + 1.0, px));
    let (y0, y1) = (cell(iy, py), cell(iy + 1.0, py));
    let a = rand(&[x0, y0, seed]);
    let b = rand(&[x1, y0, seed]);
    let c = rand(&[x0, y1, seed]);
    let d = rand(&[x1, y1, seed]);
    lerp(lerp(a, b, ux), lerp(c, d, ux), uy) * 2.0 - 1.0
}

/// Fractal [`tile_noise`]: every octave keeps the repeat, so scrolled fields still loop (pixelfx `tileFbm`).
pub fn tile_fbm(x: f64, y: f64, px: f64, py: f64, seed: f64, octaves: u32) -> f64 {
    let (mut s, mut a, mut n, mut k) = (0.0, 0.5, 0.0, 1.0);
    for o in 0..octaves {
        s += tile_noise(x * k, y * k, px * k, py * k, seed + f64::from(o) * 101.0) * a;
        n += a;
        a *= 0.5;
        k *= 2.0;
    }
    s / n
}

/// Noise that loops: the same value at phase 0 and phase 1 (pixelfx `loopNoise`).
pub fn loop_noise(phase: f64, radius: f64, seed: f64) -> f64 {
    let a = phase * TAU;
    noise2(
        libm::cos(a) * radius + 50.0,
        libm::sin(a) * radius + 50.0,
        seed,
    )
}

/// Easing curves (pixelfx `ease`): input 0..1, output 0..1 (back and elastic overshoot).
pub mod ease {
    use super::TAU;
    use core::f64::consts::PI;

    /// Default overshoot of [`in_back`] and [`out_back`] (the JS default `s = 1.70158`).
    pub const BACK_S: f64 = 1.70158;

    pub fn linear(t: f64) -> f64 {
        t
    }
    pub fn in_quad(t: f64) -> f64 {
        t * t
    }
    pub fn out_quad(t: f64) -> f64 {
        1.0 - (1.0 - t) * (1.0 - t)
    }
    pub fn in_out_quad(t: f64) -> f64 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - libm::pow(-2.0 * t + 2.0, 2.0) / 2.0
        }
    }
    pub fn in_cubic(t: f64) -> f64 {
        t * t * t
    }
    pub fn out_cubic(t: f64) -> f64 {
        1.0 - libm::pow(1.0 - t, 3.0)
    }
    pub fn in_out_cubic(t: f64) -> f64 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - libm::pow(-2.0 * t + 2.0, 3.0) / 2.0
        }
    }
    pub fn in_quart(t: f64) -> f64 {
        t * t * t * t
    }
    pub fn out_quart(t: f64) -> f64 {
        1.0 - libm::pow(1.0 - t, 4.0)
    }
    pub fn in_expo(t: f64) -> f64 {
        if t <= 0.0 {
            0.0
        } else {
            libm::pow(2.0, 10.0 * t - 10.0)
        }
    }
    pub fn out_expo(t: f64) -> f64 {
        if t >= 1.0 {
            1.0
        } else {
            1.0 - libm::pow(2.0, -10.0 * t)
        }
    }
    pub fn in_sine(t: f64) -> f64 {
        1.0 - libm::cos((t * PI) / 2.0)
    }
    pub fn out_sine(t: f64) -> f64 {
        libm::sin((t * PI) / 2.0)
    }
    pub fn in_out_sine(t: f64) -> f64 {
        -(libm::cos(PI * t) - 1.0) / 2.0
    }
    /// `s` is the overshoot; pass [`BACK_S`] for pixelfx's default.
    pub fn in_back(t: f64, s: f64) -> f64 {
        if t <= 0.0 {
            0.0
        } else if t >= 1.0 {
            1.0
        } else {
            (s + 1.0) * t * t * t - s * t * t
        }
    }
    /// `s` is the overshoot; pass [`BACK_S`] for pixelfx's default.
    pub fn out_back(t: f64, s: f64) -> f64 {
        if t <= 0.0 {
            0.0
        } else if t >= 1.0 {
            1.0
        } else {
            1.0 + (s + 1.0) * libm::pow(t - 1.0, 3.0) + s * libm::pow(t - 1.0, 2.0)
        }
    }
    pub fn out_elastic(t: f64) -> f64 {
        if t <= 0.0 {
            0.0
        } else if t >= 1.0 {
            1.0
        } else {
            libm::pow(2.0, -10.0 * t) * libm::sin((t * 10.0 - 0.75) * (TAU / 3.0)) + 1.0
        }
    }
    pub fn out_bounce(t: f64) -> f64 {
        let (n, d) = (7.5625, 2.75);
        if t < 1.0 / d {
            n * t * t
        } else if t < 2.0 / d {
            let u = t - 1.5 / d;
            n * u * u + 0.75
        } else if t < 2.5 / d {
            let u = t - 2.25 / d;
            n * u * u + 0.9375
        } else {
            let u = t - 2.625 / d;
            n * u * u + 0.984375
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_sign_and_or_one_follow_js() {
        assert_eq!(js_sign(-3.0), -1.0);
        assert_eq!(js_sign(2.0), 1.0);
        assert!(js_sign(-0.0).is_sign_negative() && js_sign(-0.0) == 0.0);
        assert!(js_sign(f64::NAN).is_nan());
        assert_eq!(or_one(0.0), 1.0);
        assert_eq!(or_one(f64::NAN), 1.0);
        assert_eq!(or_one(-2.5), -2.5);
    }
}
