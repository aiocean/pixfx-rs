//! Hex shield: port of Sprite Fusion's shield (https://www.spritefusion.com/pixel-art-vfx/shield), MIT License,
//! Copyright (c) 2026 Hugo Duprez. The source is vendored verbatim at `tools/spritefusion/arcane/hex-shield.ts`
//! and is this port's parity reference (`tools/spritefusion-ref.mjs`).
//!
//! The source works in world units of 2 art pixels (its `PX`); this port works in art pixels, and halving is exact
//! in floating point, so the numbers are the same. Its particles move as pixfx-core's [`Particles`] (pixelfx's
//! `sim.js` adapts the same Sprite Fusion code) but are drawn here, the source's way, in list order.
//!
//! Beyond the source: each blow fires an impact cue ([`pixfx_core::cue::impact`], weight 0: a light shake) where
//! the source calls `shake(0.18)`; the `feel` option scales them, 0 for none. Not ported: the `actor`, `ground`,
//! `hits` and `shake` options, and the `hit()` and `drop()` calls a game makes while it plays.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Kind, Lights, Particle, Particles, ParticlesOpts, Spawn};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, scale};
use pixfx_draw::math::{
    Rng, TAU, clamp, fract, js_max, js_min, js_round, smooth, to_int32, to_uint32,
};

use core::f64::consts::{FRAC_PI_2, PI};

pub struct HexShield;

pub static HEX_SHIELD: HexShield = HexShield;

const META: Meta = Meta {
    id: "arcane/hex-shield",
    name: "Hex shield",
    desc: "A hemisphere of hex cells forms cell by cell over the character, bottom to top; four bolts strike it, spark off and send ripples over its surface, then it breaks up into shards.",
    tags: &[
        "arcane",
        "shield",
        "barrier",
        "dome",
        "hex",
        "ripple",
        "block",
        "character",
        "one-shot",
    ],
    size: (256, 128),
    // multiples of 4, so the source's dither (by local pixel) lands on the same screen cells
    origin: (48, 84),
    duration: 3.9,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[RADIUS.spec(), HOLD.spec(), FEEL.spec()],
};

/// Dome radius in px; the canvas fits 40.
const RADIUS: NumOpt = NumOpt::new("radius", 40.0).within(16.0, 40.0);
/// Seconds the dome holds before it breaks up; the one-shot's length fits 2.75.
const HOLD: NumOpt = NumOpt::new("hold", 2.75).within(0.8, 2.75);

const ICE: [Rgba; 8] = [
    Rgba::hex24(0xffffff),
    Rgba::hex24(0xdcfbff),
    Rgba::hex24(0xa6ecff),
    Rgba::hex24(0x62c8f4),
    Rgba::hex24(0x3a8bd8),
    Rgba::hex24(0x2c58aa),
    Rgba::hex24(0x212f6b),
    Rgba::hex24(0x151a3d),
];
const TRACER: Rgba = Rgba::hex24(0xffd65a);
const TRACER_TAIL: Rgba = Rgba::hex24(0xff8a2a);
const TRACER_CORE: Rgba = Rgba::hex24(0xfffbe8);
const HOT: Rgba = Rgba::hex24(0xfff3a6);
const YELLOW: Rgba = Rgba::hex24(0xffe26b);
const ORANGE: Rgba = Rgba::hex24(0xff8a2a);
const DEEP_ORANGE: Rgba = Rgba::hex24(0xf2561d);
/// The blows: when, and where on the dome (longitude, latitude).
const HITS: [(f64, f64, f64); 4] = [
    (1.05, 0.75, 0.55),
    (1.38, 1.05, 0.95),
    (1.62, 0.5, 0.25),
    (2.05, 0.9, 1.2),
];
const GRID: f64 = 6.2;
const BAYER: [f64; 16] = [
    0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
];

/// The source's `bayer(x, y)`, by local pixel.
fn bayer(x: i64, y: i64) -> f64 {
    let i = (((y & 3) << 2) | (x & 3)) as usize;
    (BAYER.get(i).copied().unwrap_or(0.0) + 0.5) / 16.0
}

/// The source's `hash(n, k)`: `((Math.imul(n + 1 + k * 7919, 2654435761) >>> 0) % 100000) / 100000`.
fn hash(n: f64, k: f64) -> f64 {
    let m = to_int32(n + 1.0 + k * 7919.0).wrapping_mul(to_int32(2_654_435_761.0)) as u32;
    f64::from(m % 100_000) / 100_000.0
}

/// The hex cell holding grid point (gx, gy): its id and how far out from its centre the point is (1 on the edge).
fn cell(gx: f64, gy: f64) -> (i32, f64) {
    let s3 = libm::sqrt(3.0);
    let q = (s3 / 3.0) * gx - gy / 3.0;
    let r = (2.0 / 3.0) * gy;
    let s = -q - r;
    let (mut rq, mut rr) = (js_round(q), js_round(r));
    let rs = js_round(s);
    let (dq, dr, ds) = (libm::fabs(rq - q), libm::fabs(rr - r), libm::fabs(rs - s));
    if dq > dr && dq > ds {
        rq = -rr - rs;
    } else if dr > ds {
        rr = -rq - rs;
    }
    let hx = s3 * (rq + rr / 2.0);
    let hy = 1.5 * rr;
    let dx = libm::fabs(gx - hx);
    let dy = gy - hy;
    let id = to_int32(rq * 73_856_093.0) ^ to_int32(rr * 19_349_663.0);
    let e = js_max(
        js_max(dx, libm::fabs(dx / 2.0 + (dy * s3) / 2.0)),
        libm::fabs(dx / 2.0 - (dy * s3) / 2.0),
    ) / (s3 / 2.0);
    (id, e)
}

impl Effect for HexShield {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond the source: each blow asks the game for a light shake where it lands.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let rad = opts.num(&RADIUS);
        HITS.iter()
            .flat_map(|&(t, lon, lat)| {
                let (x, y) = surf(rad, lon, lat);
                impact(t, 0.0, x, y, opts)
            })
            .collect()
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&ICE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(WHITE);
        Box::new(State {
            c: [c(0), c(1), c(2), c(3), c(4)],
            rad: opts.num(&RADIUS),
            gone: opts.num(&HOLD),
            r: Rng::new(to_uint32(seed as f64)),
            ps: Particles::new(ParticlesOpts::default()),
            glints: Vec::new(),
            lights: Lights::new(),
            hits: HITS.map(|(t, lon, lat)| Hit {
                t,
                lon,
                lat,
                done: false,
            }),
            t: 0.0,
        })
    }
}

/// The point on a dome of radius `rad` at (lon, lat), the dome's foot at (0, 0).
fn surf(rad: f64, lon: f64, lat: f64) -> (f64, f64) {
    (rad * libm::sin(lon) * libm::cos(lat), -rad * libm::sin(lat))
}

struct Hit {
    t: f64,
    lon: f64,
    lat: f64,
    done: bool,
}

/// A blow's white-hot point (the source's GLINT particle, which pixfx-core has no kind for).
struct Glint {
    x: f64,
    y: f64,
    life: f64,
    max: f64,
}

struct State {
    c: [Rgba; 5],
    rad: f64,
    gone: f64,
    r: Rng,
    ps: Particles,
    glints: Vec<Glint>,
    lights: Lights,
    hits: [Hit; 4],
    t: f64,
}

impl State {
    fn sparks(&self) -> [Rgba; 3] {
        [WHITE, self.c[1], self.c[2]]
    }

    /// How hard the ripples of blows already landed light the dome at (lon, lat).
    fn ripple(&self, lon: f64, lat: f64) -> f64 {
        let mut hit: f64 = 0.0;
        for hh in &self.hits {
            let k = self.t - hh.t;
            if k < 0.0 || k > 0.7 || !hh.done {
                continue;
            }
            let d = libm::acos(clamp(
                libm::cos(lat) * libm::cos(hh.lat) * libm::cos(lon - hh.lon)
                    + libm::sin(lat) * libm::sin(hh.lat),
                -1.0,
                1.0,
            ));
            let front = k * 3.2;
            if libm::fabs(d - front) < 0.22 {
                hit = js_max(hit, 1.0 - k / 0.7);
            } else if d < front && d < 0.5 {
                hit = js_max(hit, 0.4 * (1.0 - k / 0.7));
            }
        }
        hit
    }

    /// The dome's far half (`far`, behind the character: only its cell edges, dim) or near half.
    fn dome(&self, p: &mut Painter, far: bool) {
        let (rad, t, c) = (self.rad, self.t, &self.c);
        let mut y = libm::floor(-rad - 1.0);
        while y <= 0.0 {
            let mut x = libm::floor(-rad - 1.0);
            while x <= libm::ceil(rad + 1.0) {
                self.dome_px(p, far, x, y);
                x += 1.0;
            }
            y += 1.0;
        }
        let a = smooth(0.0, 0.15, t) * (1.0 - smooth(self.gone - 0.05, self.gone + 0.45, t));
        if !far && a > 0.0 {
            ring_top(p, rad, 3.0, 1.0, c[2], a);
        }
        if !far {
            p.glow(0.0, -rad * 0.5, rad * 1.3, c[3], 0.22 * a);
        }
    }

    fn dome_px(&self, p: &mut Painter, far: bool, x: f64, y: f64) {
        let (rad, t, c) = (self.rad, self.t, &self.c);
        let u = (x + 0.5) / rad;
        let v = (-y - 0.5) / rad;
        let q = u * u + v * v;
        if q > 1.0 || v < 0.0 {
            return;
        }
        let z = libm::sqrt(1.0 - q) * if far { -1.0 } else { 1.0 };
        let lon = libm::atan2(u, z);
        let lat = libm::asin(clamp(v, 0.0, 1.0));
        let (id, e) = cell(lon * GRID, lat * GRID * 1.15);
        let h = hash(f64::from(id & 0xffff), f64::from((id as u32) >> 16));
        let appear = 0.05 + (lat / FRAC_PI_2) * 0.6 + h * 0.08;
        let gone = self.gone + h * 0.55;
        if t < appear || t > gone {
            return;
        }
        let (ix, iy) = (x as i64, y as i64);
        if far {
            if e > 0.82 && bayer(ix, iy) < 0.5 {
                p.px(x, y, c[4]);
            }
            return;
        }
        let (age, end, rim, hit) = (
            t - appear,
            gone - t,
            1.0 - libm::fabs(z),
            self.ripple(lon, lat),
        );
        let flash = age < 0.1 || end < 0.08;
        let sweep = libm::fabs(fract(lon * 0.35 + lat * 0.45 - t * 0.55) - 0.5) < 0.04;
        if e > 0.8 {
            let col = if flash || hit > 0.6 {
                WHITE
            } else if hit > 0.2 || sweep || rim > 0.75 {
                c[1]
            } else if rim > 0.4 {
                c[2]
            } else {
                c[3]
            };
            if flash || hit > 0.2 || sweep || 0.5 + rim > bayer(ix, iy) {
                p.px(x, y, col);
            }
        } else {
            let a = (if flash { 0.7 } else { 0.08 + 0.3 * rim * rim }) + hit * 0.6;
            if a > bayer(ix, iy) {
                let col = if flash || hit > 0.5 {
                    c[1]
                } else if rim > 0.7 {
                    c[3]
                } else {
                    c[4]
                };
                p.px(x, y, col);
            }
        }
        if (ix * 7 + iy * 3) & 31 == 0 {
            p.glow(x, y, 7.0, c[3], 0.15 + 0.4 * hit);
        }
    }

    fn draw_glint(p: &mut Painter, g: &Glint) {
        let (k, size) = (g.life / g.max, 1.8);
        let r = size * (0.5 + k * 0.5);
        disc(p, g.x, g.y, r + 0.8, DEEP_ORANGE);
        disc(p, g.x, g.y, r, HOT);
        p.glow(g.x, g.y, size * 4.0, WHITE, 0.6 * k);
    }

    /// One particle the source's way (its `drawOne`); speeds are taken in its world units, twice art pixels.
    fn draw_particle(&self, p: &mut Painter, q: &Particle) {
        let (k, ax, ay) = (q.life / q.max, q.x, q.y);
        let (px, py) = (libm::floor(ax), libm::floor(ay));
        let (vx, vy) = (q.vx * 2.0, q.vy * 2.0);
        let h = libm::hypot(vx, vy);
        let sp = if h == 0.0 || h.is_nan() { 1.0 } else { h };
        let (ux, uy) = (vx / sp, vy / sp);
        match q.kind {
            Kind::Star => {
                let tw = 0.6 + 0.4 * libm::sin(self.t * (8.0 + (q.seed % 5.0)) + q.seed);
                if q.size < 1.0 {
                    if tw * k > 0.25 {
                        p.px(px, py, if tw > 0.8 { WHITE } else { q.col });
                    }
                    p.glow(ax, ay, 3.0, q.col, 0.4 * k * tw);
                    return;
                }
                let arm = js_max(1.0, js_round(q.size * tw * js_min(1.0, k * 3.0)));
                let tip = scale(q.col, 0.75);
                let mut d = 1.0;
                while d <= arm {
                    let cc = if d == arm && arm > 1.0 { tip } else { q.col };
                    p.px(px + d, py, cc);
                    p.px(px - d, py, cc);
                    p.px(px, py + d, cc);
                    p.px(px, py - d, cc);
                    d += 1.0;
                }
                if arm >= 3.0 {
                    for j in 0..4 {
                        let dx = if j & 1 == 1 { -1.0 } else { 1.0 };
                        let dy = if j & 2 == 2 { -1.0 } else { 1.0 };
                        p.px(px + dx, py + dy, tip);
                    }
                }
                p.px(px, py, WHITE);
                p.glow(ax, ay, 3.0 + arm * 2.0, q.col, 0.5 * k * tw);
            }
            Kind::Mote => {
                let len = js_min(16.0, 0.5 + sp * 0.013) * q.size;
                if k < 0.35 && bayer(px as i64, py as i64) > k / 0.35 {
                    return;
                }
                line(
                    p,
                    ax - ux * len,
                    ay - uy * len,
                    ax - ux * len * 0.35,
                    ay - uy * len * 0.35,
                    scale(q.col, 0.55),
                );
                line(p, ax - ux * len * 0.4, ay - uy * len * 0.4, ax, ay, q.col);
                p.glow(ax, ay, 2.5 + q.size, q.col, 0.4 * k);
            }
            Kind::Spark => {
                let hot = if k > 0.6 {
                    HOT
                } else if k > 0.3 {
                    YELLOW
                } else {
                    ORANGE
                };
                let len = js_min(9.0, 1.0 + sp * 0.011);
                line(
                    p,
                    ax - ux * len,
                    ay - uy * len,
                    ax - ux * len * 0.4,
                    ay - uy * len * 0.4,
                    if k > 0.5 { hot } else { DEEP_ORANGE },
                );
                line(p, ax - ux * len * 0.45, ay - uy * len * 0.45, ax, ay, hot);
                p.glow(ax, ay, 3.0, hot, 0.45 * k);
            }
            _ => {}
        }
    }
}

/// The source's `Px.line`: its ends rounded to whole pixels, then Bresenham.
fn line(p: &mut Painter, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgba) {
    p.line(
        js_round(x0),
        js_round(y0),
        js_round(x1),
        js_round(y1),
        c,
        1.0,
    );
}

/// The source's `Px.disc`: a solid disc of pixels whose centres fall within `r`.
fn disc(p: &mut Painter, cx: f64, cy: f64, r: f64, c: Rgba) {
    if r <= 0.5 {
        return p.px(js_round(cx), js_round(cy), c);
    }
    let r2 = r * r;
    let mut y = libm::floor(cy - r);
    while y <= libm::ceil(cy + r) {
        let dy = y + 0.5 - cy;
        let mut x = libm::floor(cx - r);
        while x <= libm::ceil(cx + r) {
            let dx = x + 0.5 - cx;
            if dx * dx + dy * dy <= r2 {
                p.px(x, y, c);
            }
            x += 1.0;
        }
        y += 1.0;
    }
}

/// The source's `seg`: dots every half pixel from (x0, y0) to (x1, y1).
fn seg(p: &mut Painter, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgba) {
    let n = js_max(1.0, libm::ceil(libm::hypot(x1 - x0, y1 - y0) * 2.0));
    let mut i = 0.0;
    while i <= n {
        p.dot(x0 + ((x1 - x0) * i) / n, y0 + ((y1 - y0) * i) / n, c, 1.0);
        i += 1.0;
    }
}

/// The source's `ringE(buf, 0, 0, rx, ry, thick, c, a, ang => sin(ang) > 0)`: the flat ring round the dome's foot,
/// its near half (in front of the character).
fn ring_top(p: &mut Painter, rx: f64, ry: f64, thick: f64, c: Rgba, a: f64) {
    let n = libm::ceil(js_max(rx, ry) * TAU * 1.3);
    let mut j = 0.0;
    while j < n {
        let ang = (j / n) * TAU;
        j += 1.0;
        if libm::sin(ang) <= 0.0 {
            continue;
        }
        let (ca, sa) = (libm::cos(ang), libm::sin(ang));
        let mut o = -thick / 2.0;
        while o <= thick / 2.0 {
            p.dot(ca * (rx + o), sa * (ry + o * (ry / js_max(1.0, rx))), c, a);
            o += 0.5;
        }
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let (t, rad) = (self.t, self.rad);
        let sparks = self.sparks();
        let r = &mut self.r;
        if t < 0.75 {
            for _ in 0..3 {
                let lon = r.range(-PI / 2.0, PI / 2.0);
                let (x, y) = surf(rad, lon, js_min((t / 0.65) * (PI / 2.0), 1.5));
                let vx = r.range(-20.0, 20.0) / 2.0;
                let vy = r.range(-30.0, 10.0) / 2.0;
                let life = r.range(0.2, 0.4);
                let col = r.pick(&sparks).copied();
                let size = r.range(0.6, 1.2);
                self.ps.spawn(
                    r,
                    Spawn {
                        col,
                        size,
                        ..Spawn::new(Kind::Star, x, y, vx, vy, life)
                    },
                );
            }
        }
        for h in &mut self.hits {
            if t >= h.t && !h.done {
                h.done = true;
                let (x, y) = surf(rad, h.lon, h.lat);
                self.lights.add(x, y, 40.0, self.c[2], 0.9, 0.18);
                // the glint's seed, drawn as the source's particle draws it
                r.next();
                self.glints.push(Glint {
                    x,
                    y,
                    life: 0.05,
                    max: 0.05,
                });
                for _ in 0..14 {
                    let a = r.range(-1.0, 1.0) + if h.lat > 0.9 { -0.6 } else { 0.0 };
                    let sp = r.range(200.0, 600.0);
                    let vx = (libm::cos(a) * sp) / 2.0;
                    let vy = (libm::sin(a) * sp - 80.0) / 2.0;
                    let life = r.range(0.1, 0.35);
                    self.ps.spawn(
                        r,
                        Spawn {
                            col: None,
                            ..Spawn::new(Kind::Spark, x, y, vx, vy, life)
                        },
                    );
                }
            }
        }
        if t > self.gone - 0.05 && t < self.gone + 0.6 {
            for _ in 0..4 {
                let lon = r.range(-1.4, 1.4);
                let lat = r.range(0.0, 1.4);
                let (x, y) = surf(rad, lon, lat);
                let kind = if r.chance(0.5) {
                    Kind::Star
                } else {
                    Kind::Mote
                };
                let vx = r.range(-30.0, 30.0) / 2.0;
                let vy = -r.range(20.0, 80.0) / 2.0;
                let life = r.range(0.3, 0.7);
                let col = r.pick(&sparks).copied();
                let size = r.range(0.6, 1.4);
                self.ps.spawn(
                    r,
                    Spawn {
                        col,
                        size,
                        ..Spawn::new(kind, x, y, vx, vy, life)
                    },
                );
            }
        }
        self.ps.update(r, dt);
        self.glints.retain_mut(|g| {
            g.life -= dt;
            g.life > 0.0
        });
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return self.dome(p, true);
        }
        self.dome(p, false);
        for h in &self.hits {
            let k = (h.t - self.t) / 0.12;
            if !(0.0..=1.0).contains(&k) {
                continue;
            }
            let (x1, y1) = surf(self.rad, h.lon, h.lat);
            let (x, y) = (x1 + 170.0 * k, y1 - 30.0 * k);
            seg(p, x + 14.0, y - 2.5, x + 6.0, y - 1.0, TRACER_TAIL);
            seg(p, x + 6.0, y - 1.0, x, y, TRACER);
            p.px(x, y, TRACER_CORE);
            p.glow(x, y, 8.0, TRACER, 0.5);
        }
        for g in &self.glints {
            Self::draw_glint(p, g);
        }
        for q in self.ps.list() {
            self.draw_particle(p, q);
        }
        self.lights.draw(p);
    }
}
