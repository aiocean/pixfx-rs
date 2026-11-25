//! Geyser: port of pixelfx `src/fx/water/geyser.js`.
//!
//! The column (a banded white core, strands running through it, light down its length) and the slam with its
//! floor ring follow Sprite Fusion's open-source holy-light (https://www.spritefusion.com/pixel-art-vfx/holy-light),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::sim::{Kind, Lights, Particles, ParticlesOpts, Spawn};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{WHITE, ramp};
use pixfx_draw::math::{
    Rng, TAU, clamp, ease, js_max, js_min, js_round, noise, noise2, rand, smooth, to_uint32,
};

use super::lib::{
    Drop, Drops, Mist, Ramp, RingOpts, RippleOpts, Ripples, Side, WATER, billow, js_hypot, lace,
    ring,
};

pub struct Geyser;

pub static GEYSER: Geyser = Geyser;

const META: Meta = Meta {
    id: "water/geyser",
    name: "Geyser",
    desc: "A pool heaves and burbles faster and faster, then blasts a white column of water skyward with a churning foam head; it rains back down in drops and steam, slumps and sputters out.",
    tags: &[
        "water", "geyser", "eruption", "steam", "column", "nature", "one-shot",
    ],
    size: (184, 168),
    origin: (92, 146),
    duration: 5.0,
    looping: false,
    fps: 30,
    seed: 4,
    state: None,
    options: &[HEIGHT.spec()],
};

const HEIGHT: NumOpt = NumOpt::new("height", 100.0).within(20.0, 140.0);

const SY: f64 = 0.22;
const G: f64 = 820.0;
const ERUPT: f64 = 1.15;
const FALL: f64 = 2.6;
const GONE: f64 = 3.3;
const PI: f64 = core::f64::consts::PI;
// small spurts while it builds, and the sputters after: [time, height]
const SPURTS: [(f64, f64); 6] = [
    (0.46, 7.0),
    (0.72, 12.0),
    (0.92, 9.0),
    (1.04, 15.0),
    (GONE + 0.15, 15.0),
    (GONE + 0.55, 8.0),
];

fn pool_r(tt: f64) -> f64 {
    (12.0
        + 7.0 * smooth(0.0, 1.0, tt)
        + 15.0 * ease::out_cubic(clamp((tt - ERUPT) / 1.2, 0.0, 1.0)))
        * (1.0 - 0.25 * smooth(GONE, 4.6, tt))
}
fn pool_k(tt: f64) -> f64 {
    smooth(0.0, 0.2, tt) * (1.0 - smooth(4.0, 4.85, tt))
}
fn head(tt: f64) -> f64 {
    smooth(ERUPT + 0.08, ERUPT + 0.3, tt) * (1.0 - smooth(FALL + 0.1, FALL + 0.45, tt))
}
fn spurt(tt: f64) -> f64 {
    let mut h: f64 = 0.0;
    for (s0, hs) in SPURTS {
        h = js_max(h, hs * libm::sin(PI * clamp((tt - s0) / 0.2, 0.0, 1.0)));
    }
    h
}

impl Effect for Geyser {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let hmax = opts.num(&HEIGHT);
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            r: Rng::new(to_uint32(seed as f64)),
            hmax,
            s: seed as f64,
            v0: libm::sqrt(2.0 * G * hmax),
            ps: Particles::new(ParticlesOpts::default()),
            mist: Mist::new(),
            lights: Lights::new(),
            rings: Ripples::new(SY),
            drops: Drops::new(SY, G),
            plinks: Vec::new(),
            pops: Vec::new(),
            t: 0.0,
            spurted: [false; 6],
            erupted: false,
            collapsed: false,
        })
    }
}

struct Plink {
    x: f64,
    y: f64,
    s: f64,
    age: f64,
}

struct Pop {
    x: f64,
    y: f64,
    age: f64,
    r: f64,
}

struct State {
    c: Ramp,
    r: Rng,
    hmax: f64,
    s: f64,
    v0: f64,
    ps: Particles,
    mist: Mist,
    lights: Lights,
    rings: Ripples,
    drops: Drops,
    plinks: Vec<Plink>,
    pops: Vec<Pop>,
    t: f64,
    spurted: [bool; 6],
    erupted: bool,
    collapsed: bool,
}

impl State {
    // how tall the column stands: a fast blast, surging while it holds, slumping when the pressure goes
    fn top(&self, tt: f64) -> f64 {
        if !(ERUPT..=GONE).contains(&tt) {
            return 0.0;
        }
        self.hmax
            * ease::out_cubic(clamp((tt - ERUPT) / 0.3, 0.0, 1.0))
            * (0.93 + 0.07 * noise(tt * 2.2, self.s))
            * (1.0 - ease::in_quad(clamp((tt - FALL) / (GONE - FALL), 0.0, 1.0)))
    }

    fn emit(&mut self, rate: f64, dt: f64) -> bool {
        self.r.chance(js_min(1.0, rate * dt))
    }

    // the column: a white core banded out to blue edges, shadowed on the right; 1 px strands whose streaks run up
    // in jet time (fast and long low down, bunching where the water slows near the top); it spreads as it rises,
    // its sides ragged and running, a sway through it, light all the way up
    fn column(&self, p: &mut Painter, hh: f64, thin: f64) {
        if hh < 1.0 {
            return;
        }
        let (c, t, s, v0, hmax) = (&self.c, self.t, self.s, self.v0, self.hmax);
        let tau =
            |h: f64| (v0 - libm::sqrt(js_max(0.0, v0 * v0 - 2.0 * G * js_min(h, hmax * 0.96)))) / G;
        let cols = [WHITE, c[6], c[5], c[4], c[3]];
        p.shade(-22.0, -hh - 2.0, 22.0, 0.0, |x, y, out| {
            let h = -y;
            let v = h / hh;
            let sway = noise(t * 1.6 + h * 0.025, s + 5.0) * 2.5 * v;
            let rag = noise2(
                h * 0.2 - t * 11.0,
                if x < sway { 3.0 } else { 7.0 },
                s + 6.0,
            ) * (0.5 + 1.8 * v);
            let w = (3.6 + 7.5 * libm::pow(v, 1.8)) * thin + rag;
            let q = (x - sway) / js_max(1.0, w);
            let aq = libm::fabs(q);
            if aq > 1.0 {
                return None;
            }
            let bx = libm::floor(x);
            let streak = noise2(bx * 1.618 + 0.3, (tau(h) - t) * 14.0, s);
            let body = noise2(
                libm::floor((x + 40.0) / 2.0) * 2.3,
                (tau(h) - t) * 5.0,
                s + 1.0,
            );
            let mut lvl: f64 = if aq < 0.42 {
                0.0
            } else if aq < 0.7 {
                1.0
            } else if aq < 0.9 {
                2.0
            } else {
                3.0
            };
            if q > 0.38 {
                lvl += 1.0;
            }
            if streak > 0.28 {
                lvl -= 1.0;
            }
            if body < -0.4 {
                lvl += 1.0;
            }
            out.c = cols
                .get(clamp(lvl, 0.0, 4.0) as usize)
                .copied()
                .unwrap_or(WHITE);
            None
        });
        p.glow_line(0.0, 0.0, 0.0, -hh, 16.0 + 8.0 * thin, c[5], 0.12 * thin);
    }

    // the head: water boiling over at the top and falling back round the column in an umbrella of streaks, foam
    // billows churning in it, shadowed underneath, lit on top
    fn foam_head(&self, p: &mut Painter, hh: f64, k: f64, back: bool) {
        if k <= 0.01 || hh < 4.0 {
            return;
        }
        let (c, t, s) = (&self.c, self.t, self.s);
        if back {
            // the umbrella: streaks pouring off the rim of the head, running down and thinning out
            for i in 0..16 {
                let fi = f64::from(i);
                let side = if i % 2 == 1 { 1.0 } else { -1.0 };
                let x0 = side * (5.0 + 9.0 * rand(&[fi, s + 20.0])) * k;
                let len = (14.0 + 26.0 * rand(&[fi, s + 21.0])) * k;
                let speed = 70.0 + 50.0 * rand(&[fi, s + 22.0]);
                for j in 0..3 {
                    let f = (t * speed / len + f64::from(j) / 3.0 + rand(&[fi, s + 23.0])) % 1.0;
                    let y0 = -hh + 2.0 + f * len;
                    let x = x0 + side * f * f * 6.0;
                    let l = 1.0 + 4.0 * f * (1.0 - f) * 2.0;
                    if f < 0.85 {
                        let col = if f < 0.4 {
                            c[6]
                        } else if f < 0.7 {
                            c[5]
                        } else {
                            c[4]
                        };
                        p.line(x, y0, x + side * 0.6, y0 + l, col, 1.0);
                    }
                }
            }
        }
        let dim: Ramp = [c[0], c[1], c[2], c[3], c[3], c[4], c[5], c[5]];
        for i in 0..12 {
            let fi = f64::from(i);
            let a = (fi / 12.0) * TAU + t * (if i % 2 == 1 { 1.3 } else { -1.1 });
            let z = libm::sin(a);
            if (z < 0.0) != back {
                continue;
            }
            let rho = (3.0 + 7.0 * rand(&[fi, s])) * k;
            let x = libm::cos(a) * rho * 1.25 + noise(t * 3.0 + fi, s + 8.0) * 2.5;
            let y = -hh + z * rho * 0.45 + (rho / 10.0) * 4.0 - 4.0 * (1.0 - rho / (10.0 * k))
                + noise(t * 3.0 + fi + 50.0, s + 9.0) * 2.5;
            let r = (2.0 + 2.6 * rand(&[fi, s + 2.0]) + 0.7 * libm::sin(t * 7.0 + fi)) * k;
            if back {
                billow(p, x, y, r, &dim, 1.0);
            } else {
                p.circle(x + r * 0.15, y + r * 0.25, r, c[4], 1.0);
                p.circle(x, y, r * 0.82, if rho > 6.0 * k { c[5] } else { c[6] }, 1.0);
                p.circle(x - r * 0.25, y - r * 0.3, r * 0.5, WHITE, 1.0);
            }
        }
        if !back {
            p.glow(0.0, -hh - 2.0, 38.0, c[5], 0.34 * k);
        }
    }

    fn draw_back(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        // the pool: dark see-through water, a lit far rim
        let (pr, pk) = (pool_r(t), pool_k(t));
        if pk > 0.0 {
            let (c1, c2, c4) = (c[1], c[2], c[4]);
            p.each(-pr, -pr * SY, pr, pr * SY, |p, x, y, sx, sy| {
                let d = libm::pow(x / pr, 2.0) + libm::pow(y / (pr * SY), 2.0);
                if d > 1.0 {
                    return;
                }
                p.blend_px(
                    sx,
                    sy,
                    if d < 0.15 { c2 } else { c1 },
                    0.6 * pk * js_min(1.0, (1.0 - d) * 4.0),
                );
                if y < 0.0 && d > 0.78 && d < 0.95 {
                    p.blend_px(sx, sy, c4, 0.7 * pk);
                }
            });
        }
        self.rings.draw(p, c, -1.0);
        let u = (t - ERUPT) / 0.4;
        if u > 0.0 && u < 1.0 {
            ring(
                p,
                0.0,
                0.5,
                6.0 + 42.0 * ease::out_cubic(u),
                SY,
                1.5 * (1.0 - u),
                c,
                RingOpts {
                    n: 3.0,
                    gap: 1.6 + 2.0 * u,
                    half: -1.0,
                    hot: u < 0.3,
                    seed: 9.0,
                    ..RingOpts::default()
                },
            );
        }
        self.foam_head(p, self.top(t), head(t), true);
        self.drops.draw(p, c, Side::Back);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (c, t, s) = (&self.c, self.t, self.s);
        let hh = self.top(t);
        let hk = head(t);
        let k = clamp(t / ERUPT, 0.0, 1.0);
        let thin = smooth(ERUPT, ERUPT + 0.12, t) * (1.0 - 0.55 * smooth(FALL, GONE, t));
        self.rings.draw(p, c, 1.0);
        let u = (t - ERUPT) / 0.4;
        if u > 0.0 && u < 1.0 {
            ring(
                p,
                0.0,
                0.5,
                6.0 + 42.0 * ease::out_cubic(u),
                SY,
                1.5 * (1.0 - u),
                c,
                RingOpts {
                    n: 3.0,
                    gap: 1.6 + 2.0 * u,
                    half: 1.0,
                    hot: u < 0.3,
                    seed: 9.0,
                    ..RingOpts::default()
                },
            );
        }
        // foam rides the pool round the foot of the column
        let fk = smooth(ERUPT, ERUPT + 0.3, t) * (1.0 - smooth(GONE - 0.3, 4.4, t));
        if fk > 0.0 {
            lace(
                p,
                0.0,
                0.0,
                8.0 + 10.0 * fk,
                (8.0 + 10.0 * fk) * SY + 0.5,
                fk,
                c,
                t,
                s,
            );
        }
        // the heaving mound while it builds, beating faster
        if t < ERUPT + 0.05 {
            let beat = libm::pow(js_max(0.0, libm::sin(t * (8.0 + 18.0 * k * k))), 3.0);
            let rd = 4.0 + 3.5 * k + 1.5 * beat;
            let hd = 1.5 + 3.0 * k + 3.0 * beat;
            let (c3, c4, c5, c6) = (c[3], c[4], c[5], c[6]);
            p.shade(-rd - 1.0, -hd - 1.0, rd + 1.0, 0.5, |x, y, out| {
                let (dx, dy) = (x / rd, y / hd);
                let d = dx * dx + dy * dy;
                if d > 1.0 || y > 0.5 {
                    return None;
                }
                // water heaving up: a white rim where it faces the light, a tinted body, shadow low on the right
                let lit = -dx * 0.7 - dy * 0.7;
                out.c = if d > 0.68 {
                    if lit > 0.3 {
                        WHITE
                    } else if lit > -0.15 {
                        c6
                    } else {
                        c4
                    }
                } else if lit > 0.55 {
                    c6
                } else if lit > 0.12 {
                    c5
                } else if lit > -0.3 {
                    c4
                } else {
                    c3
                };
                None
            });
            p.px(-rd * 0.35, -hd * 0.6, WHITE);
            p.glow(
                0.0,
                -5.0,
                16.0 + 16.0 * k + 8.0 * beat,
                c[5],
                0.06 + 0.12 * k + 0.12 * beat,
            );
        }
        // spurts while it builds and sputters as it dies: a short white jet with a bead on top
        let sp = spurt(t);
        if sp > 1.0 && hh < 1.0 {
            let (c5, c6) = (c[5], c[6]);
            p.shade(-2.0, -sp - 1.0, 2.0, 0.0, |x, y, out| {
                let q = x / (1.0 + 0.8 * (1.0 + y / sp));
                if libm::fabs(q) > 1.0 {
                    return None;
                }
                out.c = if q < -0.3 {
                    c6
                } else if q < 0.35 {
                    WHITE
                } else {
                    c5
                };
                None
            });
            p.circle(0.0, -sp, 1.3, c[6], 1.0);
            p.px(0.0, -sp - 1.0, WHITE);
            p.glow_line(0.0, 0.0, 0.0, -sp, 8.0, c[5], 0.3);
        }
        self.column(p, hh, thin);
        // the foot: billows boil out round it
        if hk > 0.0 || (hh > 1.0 && t < GONE) {
            for i in 0..7 {
                let fi = f64::from(i);
                let a = (fi / 7.0) * TAU + t * 2.1;
                let r = 4.0 + 2.0 * libm::sin(t * 5.0 + fi * 1.7);
                let (x, y) = (libm::cos(a) * r * 1.3, libm::sin(a) * r * SY - 2.0);
                billow(
                    p,
                    x,
                    y,
                    (1.6 + 1.2 * rand(&[fi, s + 4.0])) * js_max(hk, thin * 0.6),
                    c,
                    1.0,
                );
            }
        }
        self.foam_head(p, hh, hk, false);
        for q in &self.pops {
            let g = q.age / 0.12;
            for (dx, dy) in [
                (1.0, 0.0),
                (-1.0, 0.0),
                (0.7, -0.7),
                (-0.7, -0.7),
                (0.0, -1.0),
            ] {
                p.dot(
                    q.x + dx * (q.r + 3.0 * g),
                    q.y + dy * (q.r + 3.0 * g),
                    c[6],
                    1.0 - g,
                );
            }
        }
        for q in &self.plinks {
            let g = q.age / 0.14;
            if g < 0.5 {
                p.px(q.x, q.y, WHITE);
            }
            if q.s > 1.0 {
                p.line(
                    q.x,
                    q.y - 1.0,
                    q.x,
                    q.y - 1.0 - 3.0 * (1.0 - g),
                    if g < 0.4 { WHITE } else { c[6] },
                    1.0,
                );
            }
            p.glow(q.x, q.y, 4.0, c[5], 0.25 * (1.0 - g));
        }
        self.drops.draw(p, c, Side::Front);
        // the blast's first two frames: a white burst at the vent
        let f = (t - ERUPT) / 0.07;
        if (0.0..1.0).contains(&f) {
            p.circle(0.0, -4.0, 7.0 * (1.0 - f * 0.4), WHITE, 1.0);
            let c6 = c[6];
            p.ellipse(
                0.0,
                0.0,
                12.0,
                2.6,
                |d: f64, _: f64| if d < 0.6 { WHITE } else { c6 },
                1.0,
            );
        }
        self.ps.draw(p, Layer::Both);
        self.mist.draw(p, Some(c[5]));
        self.lights.draw(p);
        // light: the column and its head, the pool round its foot
        if hh > 1.0 {
            p.glow_line(0.0, 0.0, 0.0, -hh, 26.0, c[4], 0.2 * thin);
        }
        p.glow(
            0.0,
            -2.0,
            46.0,
            c[4],
            0.24 * js_max(hk, 0.3 * k * (if t < ERUPT { 1.0 } else { 0.0 })),
        );
        p.glow(0.0, 2.0, 60.0, c[3], 0.12 * pool_k(t));
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let c = self.c;
        let hh = self.top(t);
        let hk = head(t);
        let k = t / ERUPT;
        // building: bubbles burst on the heaving mound, faster and faster, steam starts to lift
        if t < ERUPT && self.emit(1.5 + 9.0 * k * k, dt) {
            let r = &mut self.r;
            let a = r.range(0.0, TAU);
            let rr = r.range(0.0, 7.0);
            let (x, z) = (libm::cos(a) * rr, libm::sin(a) * rr);
            let y = z * SY - r.range(1.0, 3.0);
            let pr = r.range(1.0, 2.0);
            self.pops.push(Pop {
                x,
                y,
                age: 0.0,
                r: pr,
            });
            let r1 = r.range(5.0, 9.0);
            self.rings.add(
                x,
                z,
                1.0,
                r1,
                0.55,
                RippleOpts {
                    n: 1.0,
                    k: 0.85,
                    ..RippleOpts::default()
                },
            );
            let mut i = 0.0;
            while i < r.int(1.0, 3.0) {
                i += 1.0;
                let vx = r.range(-25.0, 25.0);
                let vz = r.range(-15.0, 15.0);
                let vh = r.range(50.0, 120.0);
                self.drops.add(Drop {
                    x,
                    z,
                    h: 2.0,
                    vx,
                    vz,
                    vh,
                    s: 1.0,
                    small: true,
                    life: 1.0,
                    ..Drop::default()
                });
            }
        }
        if t < ERUPT && self.emit(3.0 + 7.0 * k, dt) {
            let r = &mut self.r;
            let x = r.range(-8.0, 8.0);
            let y = -r.range(1.0, 5.0);
            let vx = r.range(-10.0, 10.0);
            let vy = -r.range(6.0, 18.0);
            let r1 = r.range(5.0, 9.0);
            let life = r.range(0.9, 1.5);
            self.mist.add(x, y, vx, vy, 2.0, r1, life, c[6], 0.22, 0.0);
        }
        for (si, (s0, hs)) in SPURTS.into_iter().enumerate() {
            let Some(done) = self.spurted.get_mut(si) else {
                continue;
            };
            if *done || t < s0 + 0.08 {
                continue;
            }
            *done = true;
            let r = &mut self.r;
            let n = 3.0 + js_round(hs / 4.0);
            let mut i = 0.0;
            while i < n {
                i += 1.0;
                let a = r.range(0.0, TAU);
                let v = r.range(10.0, 40.0);
                let vh = r.range(20.0, 90.0);
                let s = if r.chance(0.3) { 2.0 } else { 1.0 };
                self.drops.add(Drop {
                    x: 0.0,
                    z: 0.0,
                    h: hs * 0.8,
                    vx: libm::cos(a) * v,
                    vz: libm::sin(a) * v,
                    vh,
                    s,
                    ..Drop::default()
                });
            }
        }
        if !self.erupted && t >= ERUPT {
            self.erupted = true;
            // the blast: light, a ring thrown across the ground, water flung high, steam rolling out
            self.lights.add(0.0, -26.0, 64.0, c[5], 0.85, 0.6);
            self.lights.add(0.0, -4.0, 28.0, WHITE, 0.75, 0.1);
            self.rings.add(
                0.0,
                0.0,
                6.0,
                40.0,
                1.2,
                RippleOpts {
                    n: 2.0,
                    gap: 3.0,
                    k: 1.2,
                    hot: 0.12,
                    glint: 1.0,
                    ..RippleOpts::default()
                },
            );
            let r = &mut self.r;
            for _ in 0..44 {
                let a = r.range(0.0, TAU);
                let v = r.range(30.0, 120.0);
                let h = r.range(2.0, 10.0);
                let vh = r.range(180.0, 400.0);
                let s = if r.chance(0.15) {
                    3.0
                } else if r.chance(0.35) {
                    2.0
                } else {
                    1.0
                };
                let delay = r.range(0.0, 0.12);
                self.drops.add(Drop {
                    x: libm::cos(a) * 3.0,
                    z: libm::sin(a) * 3.0,
                    h,
                    vx: libm::cos(a) * v,
                    vz: libm::sin(a) * v,
                    vh,
                    s,
                    delay,
                    ..Drop::default()
                });
            }
            for i in 0..12 {
                let s = if i % 2 == 1 { 1.0 } else { -1.0 };
                let x = s * r.range(3.0, 9.0);
                let y = -r.range(1.0, 6.0);
                let vx = s * r.range(30.0, 80.0);
                let vy = -r.range(4.0, 18.0);
                let r1 = r.range(9.0, 14.0);
                let life = r.range(1.2, 1.9);
                let col = if r.chance(0.5) { c[6] } else { WHITE };
                let delay = r.range(0.0, 0.15);
                self.mist.add(x, y, vx, vy, 3.0, r1, life, col, 0.3, delay);
            }
            for _ in 0..14 {
                let x = r.range(-16.0, 16.0);
                let y = -r.range(8.0, 80.0);
                let vx = r.range(-10.0, 10.0);
                let life = r.range(0.3, 0.7);
                let col = if r.chance(0.5) { WHITE } else { c[6] };
                let size = r.range(1.0, 2.6);
                let delay = r.range(0.05, 0.5);
                self.ps.spawn(
                    r,
                    Spawn {
                        col: Some(col),
                        size,
                        delay,
                        ..Spawn::new(Kind::Star, x, y, vx, -6.0, life)
                    },
                );
            }
        }
        // holding: the head sheds drops that rain back down, spray hangs off the column, steam pours off the top
        if hk > 0.0 {
            if self.emit(60.0 * hk, dt) {
                let r = &mut self.r;
                let a = r.range(0.0, TAU);
                let v = r.range(10.0, 60.0);
                let x = r.range(-9.0, 9.0);
                let z = r.range(-4.0, 4.0);
                let h = hh + r.range(-8.0, 3.0);
                let vh = r.range(-40.0, 60.0);
                let s = if r.chance(0.08) {
                    3.0
                } else if r.chance(0.3) {
                    2.0
                } else {
                    1.0
                };
                self.drops.add(Drop {
                    x,
                    z,
                    h,
                    vx: libm::cos(a) * v,
                    vz: libm::sin(a) * v,
                    vh,
                    s,
                    ..Drop::default()
                });
            }
            if self.emit(9.0 * hk, dt) {
                let r = &mut self.r;
                let x = r.range(-8.0, 8.0);
                let y = -hh + r.range(-6.0, 2.0);
                let vx = r.range(-34.0, 34.0);
                let vy = -r.range(14.0, 34.0);
                let r1 = r.range(8.0, 13.0);
                let life = r.range(0.9, 1.4);
                let col = if r.chance(0.6) { c[6] } else { WHITE };
                self.mist.add(x, y, vx, vy, 3.0, r1, life, col, 0.2, 0.0);
            }
            if self.emit(5.0 * hk, dt) {
                let r = &mut self.r;
                let x = r.sign() * r.range(3.0, 9.0);
                let y = -r.range(1.0, 6.0);
                let vx = r.sign() * r.range(20.0, 50.0);
                let vy = -r.range(2.0, 10.0);
                let r1 = r.range(8.0, 12.0);
                let life = r.range(1.0, 1.6);
                self.mist.add(x, y, vx, vy, 3.0, r1, life, c[6], 0.22, 0.0);
            }
            if self.emit(40.0 * hk, dt) {
                let r = &mut self.r;
                let s = r.sign();
                let h2 = r.range(8.0, hh * 0.95);
                let x = s * (3.0 + 6.0 * libm::pow(h2 / hh, 1.8));
                let vx = s * r.range(10.0, 45.0);
                let vy = r.range(-20.0, 10.0);
                let life = r.range(0.6, 1.2);
                let col = if r.chance(0.5) { WHITE } else { c[6] };
                let size = r.range(0.6, 1.3);
                self.ps.spawn(
                    r,
                    Spawn {
                        col: Some(col),
                        size,
                        ..Spawn::new(Kind::Flake, x, -h2, vx, vy, life)
                    },
                );
            }
            if self.emit(4.0 * hk, dt) {
                let r = &mut self.r;
                let x = r.range(-2.0, 2.0);
                let z = r.range(-2.0, 2.0);
                let r1 = pool_r(t) * r.range(0.7, 1.0);
                self.rings.add(
                    x,
                    z,
                    4.0,
                    r1,
                    0.9,
                    RippleOpts {
                        n: 1.0,
                        k: 0.8,
                        ..RippleOpts::default()
                    },
                );
            }
            if self.emit(7.0 * hk, dt) {
                let r = &mut self.r;
                let x = r.range(-14.0, 14.0);
                let y = -r.range(10.0, hh + 8.0);
                let vx = r.range(-6.0, 6.0);
                let life = r.range(0.25, 0.55);
                let col = if r.chance(0.4) { WHITE } else { c[6] };
                let size = r.range(0.8, 2.2);
                self.ps.spawn(
                    r,
                    Spawn {
                        col: Some(col),
                        size,
                        ..Spawn::new(Kind::Star, x, y, vx, -4.0, life)
                    },
                );
            }
        }
        if !self.collapsed && t >= FALL + 0.12 {
            self.collapsed = true;
            // the head falls apart: a heavy rain of drops all at once
            let top = self.top(t);
            let r = &mut self.r;
            for _ in 0..40 {
                let a = r.range(0.0, TAU);
                let v = r.range(15.0, 80.0);
                let x = r.range(-6.0, 6.0);
                let z = r.range(-3.0, 3.0);
                let h = top + r.range(-8.0, 4.0);
                let vh = r.range(-40.0, 60.0);
                let s = if r.chance(0.12) {
                    3.0
                } else if r.chance(0.35) {
                    2.0
                } else {
                    1.0
                };
                let delay = r.range(0.0, 0.25);
                self.drops.add(Drop {
                    x,
                    z,
                    h,
                    vx: libm::cos(a) * v,
                    vz: libm::sin(a) * v,
                    vh,
                    s,
                    delay,
                    ..Drop::default()
                });
            }
            for _ in 0..6 {
                let x = r.range(-8.0, 8.0);
                let y = -top + r.range(-6.0, 6.0);
                let vx = r.range(-34.0, 34.0);
                let vy = -r.range(10.0, 26.0);
                let r1 = r.range(9.0, 13.0);
                let life = r.range(0.8, 1.2);
                self.mist.add(x, y, vx, vy, 4.0, r1, life, c[6], 0.18, 0.0);
            }
        }
        for i in (0..self.pops.len()).rev() {
            if let Some(q) = self.pops.get_mut(i) {
                q.age += dt;
                if q.age > 0.12 {
                    self.pops.remove(i);
                }
            }
        }
        for i in (0..self.plinks.len()).rev() {
            if let Some(q) = self.plinks.get_mut(i) {
                q.age += dt;
                if q.age > 0.14 {
                    self.plinks.remove(i);
                }
            }
        }
        for d in self.drops.update(dt) {
            let in_pool = js_hypot(d.x, d.z) < pool_r(self.t);
            self.rings.add(
                d.x,
                d.z,
                0.6,
                (if in_pool { 2.5 } else { 1.5 }) + d.s * 1.6,
                0.32 + d.s * 0.1,
                RippleOpts {
                    n: 1.0,
                    k: if in_pool { 0.95 } else { 0.7 },
                    hot: 0.05,
                    ..RippleOpts::default()
                },
            );
            self.plinks.push(Plink {
                x: d.x,
                y: d.z * SY,
                s: d.s,
                age: 0.0,
            });
            if d.s > 1.0 && !d.small {
                for _ in 0..2 {
                    let a = self.r.range(0.0, TAU);
                    let v = self.r.range(8.0, 28.0);
                    let vh = self.r.range(45.0, 95.0);
                    self.drops.add(Drop {
                        x: d.x,
                        z: d.z,
                        h: 0.5,
                        vx: libm::cos(a) * v,
                        vz: libm::sin(a) * v,
                        vh,
                        s: 1.0,
                        small: true,
                        life: 1.0,
                        ..Drop::default()
                    });
                }
            }
        }
        self.rings.update(dt);
        self.mist.update(dt);
        self.ps.update(&mut self.r, dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer != Layer::Front {
            self.draw_back(p);
        }
        if layer != Layer::Back {
            self.draw_front(p);
        }
    }
}
