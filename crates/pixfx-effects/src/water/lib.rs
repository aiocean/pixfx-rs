//! The water kit (pixelfx `src/fx/water/_lib.js`; the leading _ keeps the tools from listing it as an effect):
//! the water ramp, droplets that fly over a water surface seen at a low angle and land with rings, ripple trains,
//! lacy foam, foam billows, mist, wet stone.
//!
//! The droplet sprites (a pixel, a shaded 2x2, a round bead with a highlight, each trailing a tail along its
//! motion) are adapted from Sprite Fusion's open-source blood (https://www.spritefusion.com/pixel-art-vfx/blood),
//! MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Beyond `_lib.js`: [`shade_with`], a [`Painter::shade`] whose function may also paint (the JS shaders call
//! `p.blend` from inside `p.shade`, which a Rust closure cannot while the painter is borrowed).

use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE};
use pixfx_draw::math::{TAU, js_max, js_min, noise, noise2};
use pixfx_draw::painter::ShadeOut;

/// Water, deep to white: deep colour only in shadow, tinted middles, white crests.
pub const WATER: [Rgba; 8] = [
    Rgba::hex24(0x0a1630),
    Rgba::hex24(0x123063),
    Rgba::hex24(0x1b5596),
    Rgba::hex24(0x2a7fc4),
    Rgba::hex24(0x45a9e3),
    Rgba::hex24(0x86d3f5),
    Rgba::hex24(0xcdf1ff),
    Rgba::hex24(0xffffff),
];

/// Wet stone for props (lips, basins, vents), outline to highlight (pixelfx `ROCK`, `rock()`).
pub const ROCK: [Rgba; 6] = [
    Rgba::hex24(0x141219),
    Rgba::hex24(0x221f2b),
    Rgba::hex24(0x34303f),
    Rgba::hex24(0x4a4558),
    Rgba::hex24(0x655f74),
    Rgba::hex24(0x8a8498),
];

/// An 8-step water ramp.
pub type Ramp = [Rgba; 8];

/// [`Painter::shade`] whose function gets the painter too, so it can blend under the pixel it then sets (JS
/// `p.shade(.., (x, y, o) => { p.blend(x, y, ..); o.c = .. })`). Same pixels, same order.
pub fn shade_with<F>(p: &mut Painter, x0: f64, y0: f64, x1: f64, y1: f64, mut f: F)
where
    F: FnMut(&mut Painter, f64, f64, &mut ShadeOut),
{
    p.each(x0, y0, x1, y1, |p, lx, ly, sx, sy| {
        let mut o = ShadeOut {
            c: Rgba::NONE,
            a: 1.0,
            l: Rgba::NONE,
            k: 0.0,
        };
        f(p, lx, ly, &mut o);
        if o.c != Rgba::NONE {
            p.put(sx, sy, o.c, o.a);
        }
        if o.k > 0.0 {
            p.add(sx, sy, if o.l != Rgba::NONE { o.l } else { o.c }, o.k);
        }
    });
}

/// JS `Math.hypot(x, y)` as V8 computes it (scaled by the larger, Kahan-summed): `libm::hypot` can differ in the
/// last place, which moves a pixel.
pub fn js_hypot(x: f64, y: f64) -> f64 {
    let (ax, ay) = (libm::fabs(x), libm::fabs(y));
    if ax.is_infinite() || ay.is_infinite() {
        return f64::INFINITY;
    }
    if ax.is_nan() || ay.is_nan() {
        return f64::NAN;
    }
    let max = js_max(ax, ay);
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut comp) = (0.0, 0.0);
    for a in [ax, ay] {
        let n = a / max;
        let summand = n * n - comp;
        let pre = sum + summand;
        comp = (pre - sum) - summand;
        sum = pre;
    }
    libm::sqrt(sum) * max
}

/// JS `a || 1` for a number: 0 and NaN count as 1.
fn or_one(x: f64) -> f64 {
    if x == 0.0 || x.is_nan() { 1.0 } else { x }
}

/// A droplet at (x, y) moving at (vx, vy) px/s: s 1 is a pixel, 2 a shaded 2x2, 3 a round bead with a
/// highlight; a tail trails back along its motion, longer the faster it goes. `c`: an 8-step water ramp. JS
/// default `a = 1`.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn bead(p: &mut Painter, x: f64, y: f64, vx: f64, vy: f64, s: f64, c: &[Rgba; 8], a: f64) {
    let sp = or_one(js_hypot(vx, vy));
    let (ux, uy) = (vx / sp, vy / sp);
    let tail = js_min(5.0, sp * 0.011);
    if tail >= 1.0 {
        let (c3, c4, c5) = (c[3], c[4], c[5]);
        p.line(
            x - ux * (tail + 1.0),
            y - uy * (tail + 1.0),
            x - ux * 1.2,
            y - uy * 1.2,
            |u: f64| {
                if u < 0.45 {
                    c3
                } else if s > 1.0 {
                    c5
                } else {
                    c4
                }
            },
            a,
        );
    }
    if s >= 3.0 {
        p.circle(x, y, 1.75, c[4], a);
        p.circle(x - 0.25, y - 0.25, 1.15, c[6], a);
        p.dot(x - 0.7, y - 0.7, WHITE, a);
    } else if s >= 2.0 {
        let (bx, by) = (libm::floor(x), libm::floor(y));
        p.dot(bx, by, WHITE, a);
        p.dot(bx + 1.0, by, c[6], a);
        p.dot(bx, by + 1.0, c[5], a);
        p.dot(bx + 1.0, by + 1.0, c[4], a);
    } else {
        p.dot(x, y, if sp > 160.0 { c[6] } else { c[5] }, a);
    }
}

/// One droplet of [`Drops`]: x across, z into the screen (negative is far), h height above the surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drop {
    pub x: f64,
    pub z: f64,
    pub h: f64,
    pub vx: f64,
    pub vz: f64,
    pub vh: f64,
    /// 1..3, as [`bead`].
    pub s: f64,
    pub delay: f64,
    pub life: f64,
    pub max: f64,
    /// A drop thrown up by a landing drop (it throws no crown of its own).
    pub small: bool,
}

impl Default for Drop {
    /// pixelfx `add`'s defaults: all 0, `s` 1, `life` 4.
    fn default() -> Self {
        Self {
            x: 0.0,
            z: 0.0,
            h: 0.0,
            vx: 0.0,
            vz: 0.0,
            vh: 0.0,
            s: 1.0,
            delay: 0.0,
            life: 4.0,
            max: 4.0,
            small: false,
        }
    }
}

/// Which drops [`Drops::draw`] draws: behind z = 0, in front of it, or all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Back,
    Front,
    All,
}

/// Droplets in flight over a flat surface seen at a low angle: on screen they sit at (x, z * sy - h). Gravity
/// pulls h down; a drop that reaches the surface is gone, and [`Drops::update`] returns it (pixelfx `onLand`,
/// called by the caller in the order they land).
pub struct Drops {
    sy: f64,
    g: f64,
    drag: f64,
    list: Vec<Drop>,
}

impl Drops {
    /// pixelfx defaults: `sy` 0.22, `g` 820, `drag` 0.35.
    pub fn new(sy: f64, g: f64) -> Self {
        Self {
            sy,
            g,
            drag: 0.35,
            list: Vec::new(),
        }
    }

    /// Add a drop; its `max` is set to its `life`.
    pub fn add(&mut self, d: Drop) {
        self.list.push(Drop { max: d.life, ..d });
    }

    pub fn count(&self) -> usize {
        self.list.len()
    }

    /// Step every drop; the ones that land are removed and returned, in the order pixelfx calls `onLand`.
    pub fn update(&mut self, dt: f64) -> Vec<Drop> {
        let k = self.drag * dt;
        let mut landed = Vec::new();
        for i in (0..self.list.len()).rev() {
            let Some(d) = self.list.get_mut(i) else {
                continue;
            };
            if d.delay > 0.0 {
                d.delay -= dt;
                continue;
            }
            d.life -= dt;
            if d.life <= 0.0 {
                self.list.remove(i);
                continue;
            }
            d.vh -= self.g * dt;
            d.vx -= d.vx * k;
            d.vz -= d.vz * k;
            d.vh -= d.vh * k * 0.5;
            d.x += d.vx * dt;
            d.z += d.vz * dt;
            d.h += d.vh * dt;
            if d.h <= 0.0 && d.vh < 0.0 {
                d.h = 0.0;
                let d = *d;
                self.list.remove(i);
                landed.push(d);
            }
        }
        landed
    }

    /// Draw the drops behind z = 0 ([`Side::Back`]), in front of it, or all.
    pub fn draw(&self, p: &mut Painter, c: &Ramp, side: Side) {
        for d in &self.list {
            if d.delay > 0.0
                || (side == Side::Back && d.z >= 0.0)
                || (side == Side::Front && d.z < 0.0)
            {
                continue;
            }
            let (x, y) = (d.x, d.z * self.sy - d.h);
            bead(
                p,
                x,
                y,
                d.vx,
                d.vz * self.sy - d.vh,
                d.s,
                c,
                js_min(1.0, d.life / 0.2),
            );
            p.glow(x, y, 2.5 + d.s, c[5], 0.1 + 0.05 * d.s);
        }
    }
}

/// The options of [`ring`] (pixelfx defaults: `n` 2, `gap` 3, the rest 0 / false).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingOpts {
    pub n: f64,
    pub gap: f64,
    /// 0 the whole ring, -1 the far half (draw it behind things), 1 the near half.
    pub half: f64,
    pub hot: bool,
    pub glint: f64,
    pub seed: f64,
    pub wob: f64,
}

impl Default for RingOpts {
    fn default() -> Self {
        Self {
            n: 2.0,
            gap: 3.0,
            half: 0.0,
            hot: false,
            glint: 0.0,
            seed: 0.0,
            wob: 0.0,
        }
    }
}

/// A ripple on a surface seen at a low angle: a train of n crests gap px apart around (cx, cy), squashed by sy,
/// the outer crest brightest. k 0..1+ strength: a crisp 1 px crest that steps down the ramp as k falls and
/// only breaks into dashes at the very end. hot: a white crest. glint: a white catch-light on the near side.
/// wob: px the crest wanders in and out round the ring, so it isn't a perfect ellipse.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn ring(p: &mut Painter, cx: f64, cy: f64, r: f64, sy: f64, k: f64, c: &Ramp, o: RingOpts) {
    if k <= 0.02 || r < 0.5 {
        return;
    }
    let RingOpts {
        n,
        gap,
        half,
        hot,
        glint,
        seed,
        wob,
    } = o;
    let mut j = 0.0;
    while j < n {
        let rr = r - j * gap;
        if rr < 0.6 {
            break;
        }
        let kj = k * (1.0 - j * 0.42);
        let m = js_max(6.0, libm::ceil(rr * TAU * 1.3));
        let need = 1.15 - 2.3 * js_min(1.0, kj * 1.25);
        let mut i = 0.0;
        while i < m {
            let a = (i / m) * TAU;
            i += 1.0;
            let s = libm::sin(a);
            let near = s > 0.0;
            if (half > 0.0 && !near) || (half < 0.0 && near) {
                continue;
            }
            // as it weakens the crest breaks into glinting dashes (whole pixels, no dither), fixed to the ring
            if noise(a * 4.7 + seed * 13.1 + j * 5.3, seed) * 0.65
                + noise(a * 11.3 + seed * 7.7, seed + 3.0) * 0.35
                < need
            {
                continue;
            }
            // the near side and the side toward the light (upper left) read brighter
            let lit = kj * (if near { 1.0 } else { 0.7 }) * (0.88 - 0.12 * libm::cos(a));
            let col = if hot && near && j == 0.0 {
                WHITE
            } else if lit > 0.7 {
                c[6]
            } else if lit > 0.38 {
                c[5]
            } else {
                c[4]
            };
            let ra = if wob != 0.0 && !wob.is_nan() {
                rr + wob * noise(a * 2.6 + seed * 1.9 + j, seed + 11.0)
            } else {
                rr
            };
            let (x, y) = (cx + libm::cos(a) * ra, cy + s * ra * sy);
            p.dot(x, y, col, 1.0);
            if j == 0.0 {
                p.light(x, y, c[4], 0.3 * lit);
            }
        }
        j += 1.0;
    }
    if glint > 0.0 && half >= 0.0 {
        let (x, y) = (cx + libm::cos(2.05) * r, cy + libm::sin(2.05) * r * sy);
        p.px(x, y, WHITE);
        p.glow(x, y, 3.0 + glint * 2.0, c[5], 0.35 * glint);
    }
}

/// The options of [`Ripples::add`] (pixelfx defaults: `n` 2, `gap` 3, `k` 1, `hot` 0.1, the rest 0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RippleOpts {
    pub n: f64,
    pub gap: f64,
    pub k: f64,
    /// Seconds of white crest.
    pub hot: f64,
    pub glint: f64,
    pub delay: f64,
}

impl Default for RippleOpts {
    fn default() -> Self {
        Self {
            n: 2.0,
            gap: 3.0,
            k: 1.0,
            hot: 0.1,
            glint: 0.0,
            delay: 0.0,
        }
    }
}

struct Ripple {
    x: f64,
    z: f64,
    r0: f64,
    r1: f64,
    life: f64,
    age: f64,
    k: f64,
    n: f64,
    gap: f64,
    hot: f64,
    glint: f64,
    seed: f64,
}

/// Ripples that spread and fade.
pub struct Ripples {
    sy: f64,
    seq: f64,
    list: Vec<Ripple>,
}

impl Ripples {
    pub fn new(sy: f64) -> Self {
        Self {
            sy,
            seq: 0.0,
            list: Vec::new(),
        }
    }

    pub fn add(&mut self, x: f64, z: f64, r0: f64, r1: f64, life: f64, o: RippleOpts) {
        self.seq += 1.0;
        self.list.push(Ripple {
            x,
            z,
            r0,
            r1,
            life,
            age: -o.delay,
            k: o.k,
            n: o.n,
            gap: o.gap,
            hot: o.hot,
            glint: o.glint,
            seed: self.seq,
        });
    }

    pub fn count(&self) -> usize {
        self.list.len()
    }

    pub fn update(&mut self, dt: f64) {
        for i in (0..self.list.len()).rev() {
            let Some(q) = self.list.get_mut(i) else {
                continue;
            };
            q.age += dt;
            if q.age >= q.life {
                self.list.remove(i);
            }
        }
    }

    /// `half` as in [`ring`].
    pub fn draw(&self, p: &mut Painter, c: &Ramp, half: f64) {
        for q in &self.list {
            if q.age < 0.0 {
                continue;
            }
            let u = q.age / q.life;
            let r = q.r0 + (q.r1 - q.r0) * (1.0 - libm::pow(1.0 - u, 2.6));
            let k = q.k * libm::pow(1.0 - u, 1.7);
            ring(
                p,
                q.x,
                q.z * self.sy,
                r,
                self.sy,
                k,
                c,
                RingOpts {
                    n: q.n,
                    gap: q.gap * (0.55 + 0.6 * u),
                    half,
                    hot: q.age < q.hot,
                    glint: q.glint * k,
                    seed: q.seed,
                    wob: 0.0,
                },
            );
        }
    }
}

/// Lacy foam floating on the surface around (cx, cy), over an rx x ry patch: the crests of a noise field make
/// a white net with holes, thinning out as k falls (1 thick, 0 gone). t drifts it; seed picks the pattern.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn lace(
    p: &mut Painter,
    cx: f64,
    cy: f64,
    rx: f64,
    ry: f64,
    k: f64,
    c: &Ramp,
    t: f64,
    seed: f64,
) {
    if k <= 0.01 || rx < 1.0 {
        return;
    }
    let sy = ry / rx;
    let c6 = c[6];
    p.shade(cx - rx, cy - ry, cx + rx, cy + ry, |x, y, o| {
        let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
        let d = dx * dx + dy * dy;
        if d > 1.0 {
            return None;
        }
        let (u, v) = ((x - cx) * 0.36, ((y - cy) / sy) * 0.36);
        let n = noise2(u + seed * 7.13, v - t * 0.35, seed);
        let m = noise2(u * 2.2 - t * 0.3, v * 2.2 + seed * 3.1, seed + 9.0);
        let net = 1.0 - libm::fabs(n) * 4.6 + m * 0.3;
        let need = 1.02 - k * (1.15 - d * 0.9);
        if net < need {
            return None;
        }
        o.c = if net > need + 0.4 { WHITE } else { c6 };
        None
    });
}

struct Puff {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    r0: f64,
    r1: f64,
    life: f64,
    max: f64,
    c: Rgba,
    a: f64,
    delay: f64,
}

/// Mist: soft puffs with no hard edge (alpha falls off to the rim), lit on top, that swell, drift on a slowing
/// push and thin away.
#[derive(Default)]
pub struct Mist {
    list: Vec<Puff>,
}

impl Mist {
    pub fn new() -> Self {
        Self::default()
    }

    /// JS defaults: `a = 0.25`, `delay = 0`.
    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    pub fn add(
        &mut self,
        x: f64,
        y: f64,
        vx: f64,
        vy: f64,
        r0: f64,
        r1: f64,
        life: f64,
        c: Rgba,
        a: f64,
        delay: f64,
    ) {
        self.list.push(Puff {
            x,
            y,
            vx,
            vy,
            r0,
            r1,
            life,
            max: life,
            c,
            a,
            delay,
        });
    }

    pub fn count(&self) -> usize {
        self.list.len()
    }

    pub fn update(&mut self, dt: f64) {
        for i in (0..self.list.len()).rev() {
            let Some(m) = self.list.get_mut(i) else {
                continue;
            };
            if m.delay > 0.0 {
                m.delay -= dt;
                continue;
            }
            m.life -= dt;
            if m.life <= 0.0 {
                self.list.remove(i);
                continue;
            }
            m.vx -= m.vx * 2.4 * dt;
            m.vy -= m.vy * 2.4 * dt + 7.0 * dt;
            m.x += m.vx * dt;
            m.y += m.vy * dt;
        }
    }

    pub fn draw(&self, p: &mut Painter, haze: Option<Rgba>) {
        for m in &self.list {
            if m.delay > 0.0 {
                continue;
            }
            let age = 1.0 - m.life / m.max;
            let r = m.r0 + (m.r1 - m.r0) * (1.0 - libm::pow(1.0 - age, 2.2));
            let a = m.a * js_min(1.0, age / 0.12) * libm::pow(1.0 - age, 1.4);
            if a < 0.01 {
                continue;
            }
            p.disc(m.x, m.y, r, m.c, |d: f64, _| {
                a * (1.0 - d * d) * (1.0 - d * 0.5)
            });
            p.disc(
                m.x - r * 0.2,
                m.y - r * 0.32,
                r * 0.55,
                WHITE,
                |d: f64, _| a * 0.25 * (1.0 - d) * (1.0 - d),
            );
            if let Some(h) = haze {
                p.glow(
                    m.x,
                    m.y,
                    r * 1.5,
                    h,
                    (0.16 * a) / m.a * js_min(1.0, m.a * 3.0),
                );
            }
        }
    }
}

/// A billow of churning foam: a shadowed underside, a white body, a lit cap. a < 1 dithers it away (JS
/// default `a = 1`).
pub fn billow(p: &mut Painter, x: f64, y: f64, r: f64, c: &Ramp, a: f64) {
    if r < 0.9 {
        return p.dot(x, y, c[6], a);
    }
    p.circle(x + r * 0.12, y + r * 0.2, r, c[4], a);
    p.circle(x, y, r * 0.84, c[6], a);
    if r > 1.6 {
        p.circle(x - r * 0.3, y - r * 0.32, r * 0.46, WHITE, a);
    }
}

/// A pebble or rock: lit upper left, shaded lower right, a dark rim. `st`: a 6-step stone ramp ([`ROCK`]).
pub fn pebble(p: &mut Painter, x: f64, y: f64, r: f64, st: &[Rgba; 6]) {
    p.shade(
        x - r - 1.0,
        y - r - 1.0,
        x + r + 1.0,
        y + r + 1.0,
        |px, py, o| {
            let (dx, dy) = (px - x, py - y);
            let d = js_hypot(dx * 1.1, dy);
            if d > r + 0.8 {
                return None;
            }
            let lit = (dx + dy) / js_max(0.5, r);
            o.c = if d > r {
                st[0]
            } else if lit < -0.7 {
                st[5]
            } else if lit < 0.2 {
                st[4]
            } else if lit < 0.8 {
                st[3]
            } else {
                st[2]
            };
            None
        },
    );
}

/// The four neighbours a winking glint lights round itself.
pub const CROSS: [(f64, f64); 4] = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];
