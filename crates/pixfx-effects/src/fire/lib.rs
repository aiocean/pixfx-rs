//! Shared by the fire effects (the leading _ keeps the tools from listing it as an effect).
//! Flame tongues, licks, flame puffs that cool into smoke, embers and sparks, with their motion in closed form so
//! looping effects can draw them from t alone and stay seamless. The tongue, lick, puff, smoke, ember and spark
//! recipes are adapted from Sprite Fusion's open-source fire (https://www.spritefusion.com/pixel-art-vfx/fire),
//! MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/fire/_lib.js` (all of it but `flicker`, which no effect calls), plus `Stone` and
//! `rock`, which flame-pillar and meteor each define the same in pixelfx. Beyond pixelfx: fills laid down a game
//! pixel at a time (tongues, blades, the fire effects' columns) paint the whole game cell
//! ([`Painter::paint_cell`]), so a denser raster shows no gaps between them; on a 1x raster that is `px`.

use pixfx_core::Layer;
use pixfx_core::sim::{Bit, Flags, Kind, Particles, SMOKE_RAMP};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp, rgb, scale, step_at};
use pixfx_draw::math::{Rng, clamp, js_max, js_min, js_round, or_one, rand, rng, wrap};

/// A hot-first fire ramp: `[0]` white .. `[8]` smouldering.
pub type Hot = [Rgba; 9];

/// Tongue layers: (width share, height share, step of the hot-first ramp).
pub type Layers = [(f64, f64, usize)];

/// Smooth value noise in [-1, 1] that repeats every `period` cells (a whole number).
pub fn pnoise(x: f64, period: f64, seed: f64) -> f64 {
    let i = libm::floor(x);
    let f = x - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = rand(&[wrap(i, period), seed]);
    let b = rand(&[wrap(i + 1.0, period), seed]);
    (a + (b - a) * u) * 2.0 - 1.0
}

/// Noise for a loop `d` seconds long (pixelfx `noiseFor(D)`, D > 0): wobbles about `rate` times a second and is
/// the same at `tt` and `tt + d`.
pub fn nl(d: f64, tt: f64, rate: f64, seed: f64, off: f64) -> f64 {
    let per = js_max(1.0, js_round(rate * d));
    pnoise((tt / d) * per + off, per, seed)
}

/// Seamless particles for a loop `d` seconds long: `count` particles, each born at its own phase and reborn every
/// `d`. `make(r, i)` builds particle i from its own rng (the same every frame) and returns its life (which may be
/// longer than `d`) and itself; `draw(q, age, i)` is called for every live copy.
pub fn looped<Q>(
    t: f64,
    d: f64,
    count: f64,
    seed: f64,
    mut make: impl FnMut(&mut Rng, usize) -> (f64, Q),
    mut draw: impl FnMut(&Q, f64, usize),
) {
    let mut i = 0;
    while (i as f64) < count {
        let mut r = rng(&[seed, i as f64]);
        let born = r.next() * d;
        let (life, q) = make(&mut r, i);
        let mut age = wrap(t - born, d);
        while age < life {
            draw(&q, age, i);
            age += d;
        }
        i += 1;
    }
}

/// A fire ramp hot-first (`[0]` white .. `[8]` smouldering) from a palette dark to bright, any length.
pub fn hot_first(palette: &[Rgba]) -> Hot {
    let mut h: Hot = ramp(palette);
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

fn hot(h: &Hot, i: usize) -> Rgba {
    h.get(i).copied().unwrap_or(Rgba::NONE)
}

/// A flame tongue standing at (x, y), h tall, w its half-width at the base, its tip swept sx px sideways: a red
/// rim round orange round yellow round a white heart, every row's width and centre jittered by noise so the
/// edges crackle. `n(rate, seed, off)` is the effect's noise at this frame; `ns` seeds this tongue. `jit`: how far
/// the edges crackle (px, JS default 0.5); `wob`: how far the tip wanders (px, JS default 1.1).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn flame_tongue(
    p: &mut Painter,
    x: f64,
    y: f64,
    h: f64,
    w: f64,
    sx: f64,
    n: &impl Fn(f64, f64, f64) -> f64,
    ns: f64,
    heat: &Hot,
    layers: &Layers,
    jit: f64,
    wob: f64,
) {
    if h < 1.0 {
        return;
    }
    for &(lw, lh, ri) in layers {
        let l = h * lh;
        let c = hot(heat, ri);
        let mut r = 0.0;
        while r <= l {
            let u = r / js_max(1.0, l);
            let half = w * lw * libm::pow(1.0 - u, 0.85) * js_min(1.0, (r + 1.5) / 3.5)
                + n(9.0, ns, r * 0.45) * jit;
            if half >= 0.3 {
                let cx = x + sx * u * u * lh + n(4.0, ns + 7.0, r * 0.2) * u * wob;
                let yy = js_round(y - r);
                let x1 = js_round(cx + half);
                let mut px = js_round(cx - half);
                while px <= x1 {
                    p.paint_cell(px, yy, c, 1.0);
                    px += 1.0;
                }
            }
            r += 1.0;
        }
    }
}

/// The same tongue lying along any direction (ux, uy) from its root (x, y): for jets, breath and comet tails.
/// `len` long, `w` half-width at the root, `bend`: px the tip is swept sideways (to the left of the direction).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn flame_blade(
    p: &mut Painter,
    x: f64,
    y: f64,
    ux: f64,
    uy: f64,
    len: f64,
    w: f64,
    bend: f64,
    n: &impl Fn(f64, f64, f64) -> f64,
    ns: f64,
    heat: &Hot,
    layers: &Layers,
    jit: f64,
    wob: f64,
) {
    if len < 1.0 {
        return;
    }
    let (nx, ny) = (-uy, ux);
    for &(lw, lh, ri) in layers {
        let l = len * lh;
        let c = hot(heat, ri);
        let mut r = 0.0;
        while r <= l {
            let u = r / js_max(1.0, l);
            let half = w * lw * libm::pow(1.0 - u, 0.85) * js_min(1.0, (r + 1.5) / 3.5)
                + n(9.0, ns, r * 0.45) * jit;
            if half >= 0.3 {
                let off = bend * u * u * lh + n(4.0, ns + 7.0, r * 0.2) * u * wob;
                let (cx, cy) = (x + ux * r + nx * off, y + uy * r + ny * off);
                let mut q = -half;
                while q <= half {
                    p.paint_cell(cx + nx * q, cy + ny * q, c, 1.0);
                    q += 0.5;
                }
            }
            r += 0.5;
        }
    }
}

/// The screen rows and columns of a disc of radius `r` round local (x, y), drawn straight in screen space;
/// `put(sx, sy)` for each. `r` is in game pixels; on a denser raster the disc covers as many texels as it needs.
fn screen_disc(
    p: &mut Painter,
    x: f64,
    y: f64,
    r: f64,
    mut put: impl FnMut(&mut Painter, f64, f64),
) {
    let (cx, cy) = (p.to_x(x, y), p.to_y(x, y));
    let r = r / p.texel();
    let r2 = r * r;
    let mut sy = libm::floor(cy - r);
    let sy1 = libm::ceil(cy + r);
    while sy <= sy1 {
        let dy = sy + 0.5 - cy;
        let s2 = r2 - dy * dy;
        if s2 >= 0.0 {
            let s = libm::sqrt(s2);
            let mut sx = libm::ceil(cx - s - 0.5);
            let sx1 = libm::floor(cx + s - 0.5);
            while sx <= sx1 {
                put(p, sx, sy);
                sx += 1.0;
            }
        }
        sy += 1.0;
    }
}

/// A solid disc drawn straight in screen space (fast; a disc looks the same mirrored or turned).
pub fn solid(p: &mut Painter, x: f64, y: f64, r: f64, c: Rgba) {
    if r < 0.6 {
        return p.px(x, y, c);
    }
    screen_disc(p, x, y, r, |p, sx, sy| p.put(sx, sy, c, 1.0));
}

/// A translucent disc in screen space: smoke and haze.
pub fn soft(p: &mut Painter, x: f64, y: f64, r: f64, c: Rgba, a: f64) {
    if a <= 0.004 {
        return;
    }
    if r < 0.5 {
        return p.blend(x, y, c, a);
    }
    screen_disc(p, x, y, r, |p, sx, sy| p.blend_px(sx, sy, c, a));
}

/// A ball of flame with life left k (1 new, 0 gone) cooling down the hot-first ramp as it ages: a darker rim,
/// a lit core up and to the left, a little tongue on top, a hot heart while it's young, and its light.
/// `frac` (0..1) nudges how hot it starts. (tx, ty): which way its tongue points (JS defaults: `glow_k` 0.18,
/// (0, -1), up).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn fire_puff(
    p: &mut Painter,
    x: f64,
    y: f64,
    k: f64,
    size: f64,
    frac: f64,
    h: &Hot,
    glow_k: f64,
    tx: f64,
    ty: f64,
) {
    let age = 1.0 - k;
    let n = h.len() as f64;
    let heat = js_min(0.999, libm::pow(age, 0.62) + frac * 0.1);
    let idx = js_min(n - 2.0, libm::floor(heat * (n - 1.0)));
    let r = size * (0.55 + 0.45 * js_min(1.0, age * 7.0)) * (1.0 - age * 0.55);
    let c = step_at(h, idx);
    if idx >= n - 2.0 {
        return soft(p, x, y, r, c, js_min(1.0, k * 3.0) * 0.85);
    }
    solid(p, x, y, r, step_at(h, js_min(n - 1.0, idx + 2.0)));
    if r > 1.6 && ty > -0.99 {
        solid(
            p,
            x + tx * r * 1.5,
            y + ty * r * 1.5,
            r * 0.3,
            step_at(h, js_min(n - 1.0, idx + 1.0)),
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
            step_at(h, js_max(0.0, idx - 1.0)),
        );
    }
    if idx < 6.0 {
        p.glow(x, y, r * 2.4 + 2.0, c, glow_k * k);
    }
}

/// A lick: a blob of flame torn off a tongue's tip, a red rim round a yellow heart, shrinking as it rises.
pub fn lick(p: &mut Painter, x: f64, y: f64, k: f64, s: f64, h: &Hot) {
    let r = s * (0.4 + 0.6 * k);
    solid(p, x, y, r + 0.6, if k > 0.5 { h[5] } else { h[6] });
    if k > 0.35 {
        solid(p, x, y - 0.3, r * 0.6, if k > 0.7 { h[2] } else { h[4] });
    }
}

fn ramp_at(r: &[Rgba], t: f64) -> Rgba {
    step_at(
        r,
        js_min(
            r.len() as f64 - 1.0,
            js_max(0.0, libm::floor(t * r.len() as f64)),
        ),
    )
}

/// A puff of smoke with life left k: three soft discs (a body, a denser core sitting low, a lit cap while it's
/// young), swelling and thinning. `col` tints it (`None` for plain smoke, JS 0); `big` for slower, fuller smoke;
/// `a_mul` scales its alpha (JS defaults: `None`, false, 1).
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
        Some(col) => mix(col, ramp_at(&SMOKE_RAMP, age), age),
        None => ramp_at(&SMOKE_RAMP, age * 1.05),
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
/// end (`ph`: its flicker phase), with its own light. Big ones (size 2) are a little cross with a darker rim.
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
    let sp = or_one(libm::hypot(vx, vy));
    let (ux, uy) = (vx / sp, vy / sp);
    let len = js_min(9.0, 1.0 + sp * 0.022);
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
    let e = libm::exp(-k * t);
    let vt = a / k;
    (x + vt * t + ((v - vt) * (1.0 - e)) / k, vt + (v - vt) * e)
}

/// Where a body is t seconds after launch from (x, y) at (vx, vy), under acceleration (ax, ay) px/s² and linear
/// drag kx, ky (1/s): (x, y, vx, vy).
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
) -> (f64, f64, f64, f64) {
    let (x, vx) = along(x, vx, ax, kx, t);
    let (y, vy) = along(y, vy, ay, ky, t);
    (x, y, vx, vy)
}

/// A spark's flight: falls, drags, and bounces off the floor at y = floor, losing speed each time (JS defaults
/// `g` 427, `k` 2.2).
pub fn spark_at(x: f64, y: f64, vx: f64, vy: f64, t: f64, floor: f64) -> (f64, f64, f64, f64) {
    const G: f64 = 427.0;
    const K: f64 = 2.2;
    let at = |s: (f64, f64, f64, f64), t: f64| drift(s.0, s.1, s.2, s.3, t, 0.0, G, K, K);
    let mut s = (x, y, vx, vy);
    let mut left = t;
    for _ in 0..3 {
        let e = at(s, left);
        if e.1 < floor || s.1 >= floor {
            return e;
        }
        let (mut lo, mut hi) = (0.0, left);
        for _ in 0..16 {
            let m = (lo + hi) / 2.0;
            if at(s, m).1 < floor {
                lo = m;
            } else {
                hi = m;
            }
        }
        let c = at(s, hi);
        s = (c.0, floor - 0.25, c.2 * 0.7, -libm::fabs(c.3) * 0.45);
        left -= hi;
    }
    let e = at(s, left);
    if e.1 < floor {
        e
    } else {
        (e.0, floor - 0.25, e.2, 0.0)
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

/// A core [`Particles`] system drawn with its flame puffs recoloured down the hot-first ramp (the core draws them
/// in its own fire colours): smoke and back particles, then the flames, then sparks and the rest on top.
pub fn sim_draw(p: &mut Painter, ps: &Particles, h: &Hot, layer: Layer) {
    let puffs = |p: &mut Painter, back: bool| {
        for q in ps.list() {
            if q.kind == Kind::Fire && q.delay <= 0.0 && q.flags.contains(Flags::BACK) == back {
                fire_puff(
                    p,
                    q.x,
                    q.y,
                    q.life / q.max,
                    q.size,
                    q.seed % 1.0,
                    h,
                    0.18,
                    0.0,
                    -1.0,
                );
            }
        }
    };
    let rest = |q: &pixfx_core::sim::Particle| q.kind != Kind::Fire;
    if layer != Layer::Front {
        ps.draw_where(p, Layer::Back, rest);
        puffs(p, true);
    }
    if layer != Layer::Back {
        puffs(p, false);
        ps.draw_where(p, Layer::Front, rest);
    }
}

/// The fire's look and motion for [`burn`] (pixelfx `burn`'s `o`).
pub struct Burn<'a> {
    /// The hot-first ramp.
    pub h: &'a Hot,
    /// The loop's length, seconds.
    pub d: f64,
    /// The seed.
    pub s: f64,
    /// The base.
    pub x: f64,
    pub y: f64,
    pub height: f64,
    pub lean: f64,
    /// How wide embers and smoke leave.
    pub spread: f64,
    /// Rates per second: flame puffs and licks (per tongue), smoke, embers.
    pub puffs: f64,
    pub licks: f64,
    pub smoke: f64,
    pub smoke_size: f64,
    pub smoke_life: (f64, f64),
    pub embers: f64,
    /// Light strength and reach.
    pub glow: f64,
    pub glow_r: f64,
    pub wind: f64,
}

struct Tongue {
    dx: f64,
    y: f64,
    h: f64,
    w: f64,
    back: bool,
    env: f64,
    n: f64,
}

/// A fire for loops, drawn from t alone. Tongues `tongues`: (dx, dy, height share, half-width, back row?) from
/// the base, back row first then outer to inner; licks torn off their tips, flame puffs that burn out into smoke,
/// a smoke plume curling off with the draught, embers, and the fire's light. `mid` is called after the tongues,
/// before what flies off them: draw what the fire burns out of there. Wide tongues get [`BIG_LAYERS`].
pub fn burn(
    p: &mut Painter,
    t: f64,
    tongues: &[(f64, f64, f64, f64, bool)],
    o: &Burn,
    mid: impl FnOnce(&mut Painter),
) {
    let (h, d, s, hc, x0, y0, w_, g) = (o.h, o.d, o.s, o.height, o.x, o.y, o.spread, o.glow);
    let nlt = |tt: f64, rate: f64, seed: f64, off: f64| nl(d, tt, rate, seed, off);
    let n = |rate: f64, seed: f64, off: f64| nl(d, t, rate, seed, off);
    let fla =
        |tt: f64| 0.74 + 0.16 * nlt(tt, 7.3, s + 1.0, 0.0) + 0.1 * nlt(tt, 19.0, s + 2.0, 0.0);
    let fl = fla(t);
    let smoke = mix(SMOKE1, h[6], 0.25);
    let emb = [h[1], h[2], h[4], h[6]];
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.25);
    let mut r = rng(&[s, 77.0]);
    let count = |rate: f64| js_round(rate * d);
    let tt_: Vec<Tongue> = tongues
        .iter()
        .enumerate()
        .map(|(i, &(dx, dy, hh, w, back))| {
            let dx = dx + r.range(-0.3, 0.3);
            let hv = 2.0 + hc * hh * r.range(0.92, 1.08);
            let w = w * r.range(0.92, 1.08);
            Tongue {
                dx,
                y: dy,
                h: hv,
                w,
                back,
                env: hh,
                n: (s * 7.0 + i as f64) * 13.0,
            }
        })
        .collect();
    let nt = tt_.len();
    let lean = |tt: f64| nlt(tt, 0.67, s + 90.0, 0.0) * o.lean;
    let height = |q: &Tongue, tt: f64| {
        let leap = js_max(0.0, nlt(tt, 2.6, q.n + 70.0, 0.0) - 0.55) * 1.4;
        (q.h + 0.25 * q.h * nlt(tt, 3.3, q.n, 0.0) + q.h * leap) * (0.9 + 0.1 * fla(tt))
    };
    let sweep = |q: &Tongue, tt: f64, hh: f64| {
        nlt(tt, 2.2, q.n + 40.0, 0.0) * (1.0 + hh * 0.08) + lean(tt) * hh * 0.9
            - clamp(q.dx * 0.3, -2.5, 2.5)
    };
    let tip = |i: usize, tt: f64| -> (f64, f64) {
        let Some(q) = tt_.get(i) else {
            return (f64::NAN, f64::NAN);
        };
        let hh = height(q, tt);
        (
            x0 + q.dx + sweep(q, tt, hh) * 0.85,
            y0 + q.y - hh * 0.88 + 3.0,
        )
    };
    let tongue_at = |p: &mut Painter, q: &Tongue| {
        let hh = height(q, t);
        let layers: &Layers = if q.back {
            &BACK_LAYERS
        } else if q.w > 2.2 {
            &BIG_LAYERS
        } else {
            &LAYERS
        };
        flame_tongue(
            p,
            x0 + q.dx,
            y0 + q.y,
            hh,
            q.w,
            sweep(q, t, hh),
            &n,
            q.n,
            h,
            layers,
            0.5,
            1.1 + hh * 0.04,
        );
    };

    // the plume: each puff keeps the sway the plume had when it left, so the wiggle climbs it
    looped(
        t,
        d,
        count(o.smoke),
        s + 300.0,
        |r, _| {
            let x = r.range(-w_, w_) * 0.7;
            let y = -hc * 0.8 - r.range(0.0, 6.0);
            let vx = r.range(-2.0, 2.0);
            let vy = -r.range(10.0, 17.0);
            let life = r.range(o.smoke_life.0, o.smoke_life.1);
            let sz = r.range(0.7, 1.1) * o.smoke_size;
            (life, (x, y, vx, vy, life, sz))
        },
        |&(qx, qy, vx, vy, life, sz), age, _| {
            let (x, y, _, _) = drift(qx, qy, vx, vy, age, o.wind, -20.0, 1.6, 1.0);
            smoke_puff(
                p,
                x0 + x + nlt(t - age, 0.8, s + 60.0, 0.0) * (1.5 + 2.5 * age),
                y0 + y,
                1.0 - age / life,
                sz,
                Some(smoke),
                false,
                1.0,
            );
        },
    );
    // flame puffs shed from the tips
    let mut puffs: Vec<(f64, f64, f64, f64, f64)> = Vec::new();
    looped(
        t,
        d,
        nt as f64 * count(o.puffs),
        s + 200.0,
        |r, j| {
            let i = j % nt.max(1);
            let env = tt_.get(i).map_or(0.0, |q| q.env);
            let fire = r.range(0.14, 0.3);
            let smokes = r.chance(0.45);
            let q = Shed {
                i,
                fire,
                sl: r.range(0.8, 1.4),
                ox: r.range(-1.0, 1.0),
                oy: r.range(0.0, 4.0),
                vx: r.range(-4.0, 4.0),
                vy: -r.range(15.0, 30.0),
                s: r.range(0.8, 1.6) * (0.8 + 0.6 * env),
                f: r.next(),
            };
            (fire + if smokes { 1.4 } else { 0.0 }, q)
        },
        |q, age, _| {
            let (tx, ty) = tip(q.i, t - age);
            let (x, y, vx, vy) = drift(
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
                puffs.push((x, y, 1.0 - age / q.fire, q.s, q.f));
                return;
            }
            if age - q.fire > q.sl {
                return;
            }
            let (sx, sy, _, _) = drift(
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
    for q in tt_.iter().filter(|q| q.back) {
        tongue_at(p, q);
    }
    for q in tt_.iter().filter(|q| !q.back) {
        tongue_at(p, q);
    }
    mid(p);
    // licks torn off the tips, rising and shrinking
    looped(
        t,
        d,
        nt as f64 * count(o.licks),
        s + 220.0,
        |r, j| {
            let i = j % nt.max(1);
            let env = tt_.get(i).map_or(0.0, |q| q.env);
            let vx = r.range(-3.0, 3.0);
            let vy = -r.range(12.0, 22.0);
            let sz = r.range(0.9, 1.6) * (0.8 + 0.5 * env);
            (0.26, (i, vx, vy, sz))
        },
        |&(i, vx, vy, sz), age, _| {
            let (tx, ty) = tip(i, t - age);
            lick(
                p,
                tx + vx * age,
                ty - 2.0 + vy * age,
                1.0 - age / 0.26,
                sz,
                h,
            );
        },
    );
    for &(x, y, k, sz, f) in &puffs {
        fire_puff(p, x, y, k, sz, f, h, 0.18, 0.0, -1.0);
    }
    // embers carried up by the heat, wandering and cooling
    looped(
        t,
        d,
        count(o.embers),
        s + 500.0,
        |r, _| {
            let x = r.range(-w_, w_);
            let y = -r.range(hc * 0.3, hc * 0.8);
            let vx = r.range(-15.0, 15.0);
            let vy = -r.range(25.0, 60.0);
            let life = r.range(1.1, 2.4);
            let sz = if r.chance(0.15) {
                2.0
            } else {
                r.range(1.0, 1.6)
            };
            let a = r.range(0.0, 6.3);
            let b = r.range(0.0, 6.3);
            (
                life,
                Ember {
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    s: sz,
                    a,
                    b,
                },
            )
        },
        |q, age, _| {
            draw_ember(p, x0, y0, q, age, &emb);
        },
    );
    // light: a broad warm wash, lights on the flames, a hot heart at the base
    if g > 0.0 {
        let r1 = o.glow_r;
        let lx = x0 + lean(t) * 12.0;
        p.glow(lx, y0 - hc * 0.35, r1 * 1.5, glow, 0.22 * fl * g);
        p.glow(lx, y0 - hc * 0.75, r1, glow, 0.34 * fl * g);
        p.glow(x0, y0 - hc * 0.2, r1 * 0.32, h[2], 0.2 * fl * g);
        p.glow(x0, y0 - 2.0, 8.0, h[0], 0.14 * fl * g);
    }
}

/// A flame puff shed off a tongue's tip ([`burn`] and campfire).
pub struct Shed {
    pub i: usize,
    pub fire: f64,
    pub sl: f64,
    pub ox: f64,
    pub oy: f64,
    pub vx: f64,
    pub vy: f64,
    pub s: f64,
    pub f: f64,
}

/// An ember carried up by a fire's heat ([`burn`] and campfire).
pub struct Ember {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub life: f64,
    pub s: f64,
    pub a: f64,
    pub b: f64,
}

/// An [`Ember`] `age` seconds old, rising off a fire based at (x0, y0), wandering and cooling.
pub fn draw_ember(p: &mut Painter, x0: f64, y0: f64, q: &Ember, age: f64, emb: &[Rgba; 4]) {
    let (x, y, _, _) = drift(q.x, q.y, q.vx, q.vy, age, 0.0, -30.0, 1.6, 1.6);
    ember(
        p,
        x0 + x + 7.0 * (libm::sin(age * 2.3 + q.a) - libm::sin(q.a)),
        y0 + y + 2.0 * (libm::sin(age * 3.0 + q.b) - libm::sin(q.b)),
        1.0 - age / q.life,
        q.s,
        emb,
        age * 40.0 + q.a,
    );
}

/// The smoke colour fires mix their own red into (pixelfx `SMOKE1`, `#5b5566`).
pub const SMOKE1: Rgba = Rgba::hex24(0x5b5566);

/// A stone colour's shades for thrown rock, grit and dust (pixelfx `stone(c)`, which flame-pillar and meteor each
/// define the same).
pub struct Stone {
    pub light: Rgba,
    pub body: Rgba,
    pub shade: Rgba,
    pub dust: Rgba,
}

impl Stone {
    pub fn of(c: Rgba) -> Self {
        Self {
            light: mix(scale(c, 1.35), WHITE, 0.08),
            body: c,
            shade: scale(c, 0.64),
            dust: mix(scale(c, 1.2), rgb(150.0, 140.0, 150.0), 0.3),
        }
    }
}

/// The dark rim thrown rock is outlined in.
pub const OUTLINE: Rgba = Rgba::hex24(0x1a1422);

/// A thrown rock: a lumpy ball lit from the side it turns to, outlined, fading in its last half second
/// (pixelfx `rock(p, b, st)`, which flame-pillar and meteor each define the same).
pub fn rock(p: &mut Painter, b: &Bit, st: &Stone) {
    let r = 0.6 + b.s * 0.75;
    let fade = js_min(1.0, b.life / 0.5);
    let (lx, ly) = (libm::cos(b.a) * r * 0.4, libm::sin(b.a) * r * 0.4);
    p.shade(
        b.x - r - 1.0,
        b.y - r - 1.0,
        b.x + r + 1.0,
        b.y + r + 1.0,
        |x, y, out| {
            let (dx, dy) = (x - b.x, y - b.y);
            let d = libm::hypot(dx * 1.15, dy);
            if d > r + 0.9 {
                return None;
            }
            let lit = dx * lx + dy * ly;
            out.c = if d > r {
                OUTLINE
            } else if lit < -0.4 {
                st.light
            } else if lit > 0.6 {
                st.shade
            } else {
                st.body
            };
            out.a = fade;
            None
        },
    );
}
