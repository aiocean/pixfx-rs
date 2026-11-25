//! Wave crash: port of pixelfx `src/fx/water/wave-crash.js`.
//!
//! The lip's crescent (a thick arc, white on its outer rim, cooling to deep blue underneath, with light along it)
//! follows Sprite Fusion's open-source slash (https://www.spritefusion.com/pixel-art-vfx/slash), MIT License,
//! Copyright (c) 2026 Hugo Duprez.

use std::collections::BTreeMap;

use core::f64::consts::PI;

use pixfx_core::sim::{Kind, Lights, Particles, ParticlesOpts, Spawn, sparkle};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{
    Rng, clamp, ease, js_max, js_min, js_round, noise, noise2, smooth, to_uint32,
};

use super::lib::{Drop, Drops, Mist, Ramp, RippleOpts, Ripples, Side, WATER, billow, js_hypot};

pub struct WaveCrash;

pub static WAVE_CRASH: WaveCrash = WaveCrash;

const META: Meta = Meta {
    id: "water/wave-crash",
    name: "Wave crash",
    desc: "A swell rolls in and steepens, its lip throws forward and curls over a dark barrel, then crashes into a burst of white foam and spray that surges on and fades to lace.",
    tags: &[
        "water", "wave", "crash", "surf", "foam", "spray", "one-shot",
    ],
    size: (224, 104),
    origin: (112, 86),
    duration: DUR,
    looping: false,
    fps: 30,
    seed: 6,
    state: None,
    options: &[],
};

const DUR: f64 = 3.2;
const SY: f64 = 0.22;
const G: f64 = 820.0;
const CRASH: f64 = 1.2;
const LIP0: f64 = 0.62;
const EDGE: f64 = 110.0;

fn clamp1(x: f64) -> f64 {
    clamp(x, 0.0, 1.0)
}

// the swell: where its crest is, how tall, how steep its face, as it rolls in, crashes and runs on as whitewater
fn crest_x(t: f64) -> f64 {
    if t < CRASH {
        -80.0 + 72.0 * ease::in_out_sine(t / CRASH)
    } else {
        -8.0 + 30.0 * ease::out_cubic(clamp1((t - CRASH) / 1.4))
    }
}
fn crest_h(t: f64) -> f64 {
    if t < CRASH {
        5.0 + 33.0 * ease::in_out_sine(clamp1(t / (CRASH * 0.92)))
    } else {
        38.0 * (1.0 - ease::out_cubic(clamp1((t - CRASH) / 0.45)))
            + 5.0 * (1.0 - smooth(CRASH, CRASH + 1.6, t))
    }
}
fn face_w(t: f64) -> f64 {
    if t < CRASH {
        24.0 - 16.0 * smooth(0.3, CRASH, t)
    } else {
        8.0 + 14.0 * smooth(CRASH, CRASH + 0.6, t)
    }
}
// the lip: thrown forward off the crest and falling, until its tip hits the water at CRASH
fn lip_k(t: f64) -> f64 {
    clamp1((t - LIP0) / (CRASH - LIP0))
}

fn height(x: f64, tt: f64) -> f64 {
    let (xc, a) = (crest_x(tt), crest_h(tt));
    let dx = x - xc;
    a * libm::exp(-libm::pow(
        dx / if dx < 0.0 { 30.0 } else { face_w(tt) },
        2.0,
    ))
}

// a point on the lip at s (0 at the crest, 1 at its tip), and its thickness there
fn lip(s: f64, tt: f64) -> (f64, f64, f64) {
    let (k, xc, a) = (lip_k(tt), crest_x(tt), crest_h(tt));
    (
        xc + 1.0 + s * 30.0 * libm::pow(k, 0.8),
        -a - 4.0 * s * (1.0 - s) * (1.0 + 4.0 * k) + s * s * a * ease::in_quad(k) * 0.98,
        (11.0 - 8.5 * s) * (0.5 + 0.5 * k),
    )
}

impl Effect for WaveCrash {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            r: Rng::new(to_uint32(seed as f64)),
            s: seed as f64,
            ps: Particles::new(ParticlesOpts::default()),
            mist: Mist::new(),
            lights: Lights::new(),
            rings: Ripples::new(SY),
            puffs: Vec::new(),
            plinks: Vec::new(),
            drops: Drops::new(SY, G),
            t: 0.0,
            crashed: false,
        })
    }
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
    delay: f64,
}

struct Plink {
    x: f64,
    y: f64,
    age: f64,
}

struct State {
    c: Ramp,
    r: Rng,
    s: f64,
    ps: Particles,
    mist: Mist,
    lights: Lights,
    rings: Ripples,
    puffs: Vec<Puff>,
    plinks: Vec<Plink>,
    drops: Drops,
    t: f64,
    crashed: bool,
}

/// `R.chance(a) ? 3 : R.chance(b) ? 2 : 1`, drawn lazily as JS does.
fn size3(r: &mut Rng, a: f64, b: f64) -> f64 {
    if r.chance(a) {
        3.0
    } else if r.chance(b) {
        2.0
    } else {
        1.0
    }
}

impl State {
    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    fn puff(&mut self, x: f64, y: f64, vx: f64, vy: f64, r0: f64, r1: f64, life: f64, delay: f64) {
        self.puffs.push(Puff {
            x,
            y,
            vx,
            vy,
            r0,
            r1,
            life,
            max: life,
            delay,
        });
    }

    fn crash(&mut self) {
        let (c, r) = (self.c, &mut self.r);
        let (x0, _, _) = lip(1.0, CRASH - 1e-3);
        self.lights.add(x0, -20.0, 56.0, c[5], 0.8, 0.55);
        self.lights.add(x0, -4.0, 22.0, WHITE, 0.6, 0.1);
        for _ in 0..46 {
            let a = r.range(-0.9, 0.6) * PI;
            let v = r.range(30.0, 120.0);
            let x = x0 + r.range(-6.0, 6.0);
            let z = r.range(-4.0, 4.0);
            let h = r.range(2.0, 8.0);
            let vx = libm::cos(a) * v * 0.85 + 18.0;
            let vz = libm::sin(a) * v * 0.5;
            let vh = r.range(120.0, 320.0);
            let s = size3(r, 0.15, 0.35);
            let delay = r.range(0.0, 0.1);
            self.drops.add(Drop {
                x,
                z,
                h,
                vx,
                vz,
                vh,
                s,
                delay,
                ..Drop::default()
            });
        }
        // the spit: a plume shot straight up where the lip lands
        for _ in 0..14 {
            let x = x0 + r.range(-3.0, 3.0);
            let z = r.range(-2.0, 2.0);
            let h = r.range(2.0, 6.0);
            let vx = r.range(-15.0, 25.0);
            let vz = r.range(-8.0, 8.0);
            let vh = r.range(220.0, 300.0);
            let s = if r.chance(0.3) { 2.0 } else { 1.0 };
            let delay = r.range(0.04, 0.16);
            self.drops.add(Drop {
                x,
                z,
                h,
                vx,
                vz,
                vh,
                s,
                delay,
                ..Drop::default()
            });
        }
        for _ in 0..34 {
            let x = x0 + r.range(-12.0, 10.0);
            let y = -r.range(2.0, 12.0);
            let vx = r.range(-40.0, 130.0);
            let vy = -r.range(30.0, 150.0);
            let r0 = r.range(1.0, 2.0);
            let r1 = r.range(1.8, 4.2);
            let life = r.range(0.4, 0.95);
            let delay = r.range(0.0, 0.14);
            self.puffs.push(Puff {
                x,
                y,
                vx,
                vy,
                r0,
                r1,
                life,
                max: life,
                delay,
            });
        }
        // fine spray hangs in the air over the burst
        for _ in 0..34 {
            let a = r.range(-PI * 0.95, -PI * 0.05);
            let v = r.range(40.0, 160.0);
            let x = x0 + r.range(-8.0, 8.0);
            let y = -r.range(4.0, 14.0);
            let life = r.range(0.6, 1.3);
            let col = if r.chance(0.5) { WHITE } else { c[6] };
            let size = r.range(0.6, 1.4);
            let delay = r.range(0.0, 0.1);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(col),
                    size,
                    delay,
                    ..Spawn::new(
                        Kind::Flake,
                        x,
                        y,
                        libm::cos(a) * v + 25.0,
                        libm::sin(a) * v,
                        life,
                    )
                },
            );
        }
        for _ in 0..12 {
            let x = x0 + r.range(-10.0, 14.0);
            let y = -r.range(3.0, 16.0);
            let vx = r.range(-20.0, 70.0);
            let vy = -r.range(6.0, 24.0);
            let r1 = r.range(9.0, 15.0);
            let life = r.range(1.1, 1.7);
            let col = if r.chance(0.5) { c[6] } else { WHITE };
            let delay = r.range(0.0, 0.2);
            self.mist.add(x, y, vx, vy, 3.0, r1, life, col, 0.26, delay);
        }
        for _ in 0..10 {
            let x = x0 + r.range(-20.0, 24.0);
            let y = -r.range(6.0, 40.0);
            let life = r.range(0.3, 0.6);
            let size = r.range(1.0, 2.4);
            let delay = r.range(0.05, 0.4);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(c[6]),
                    size,
                    delay,
                    ..Spawn::new(Kind::Star, x, y, 0.0, -5.0, life)
                },
            );
        }
        self.rings.add(
            x0,
            0.0,
            6.0,
            40.0,
            1.2,
            RippleOpts {
                n: 2.0,
                gap: 3.0,
                k: 1.1,
                hot: 0.12,
                ..RippleOpts::default()
            },
        );
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let c = self.c;
        let k = lip_k(t);
        // wind tears spray off the crest; the lip's tip sheds drops as it throws
        if t < CRASH && crest_h(t) > 14.0 && self.r.chance(0.5) {
            let r = &mut self.r;
            let (xc, a) = (crest_x(t), crest_h(t));
            let x = xc + r.range(-4.0, 3.0);
            let y = -a - r.range(0.0, 2.0);
            let vx = -r.range(20.0, 60.0);
            let vy = -r.range(10.0, 40.0);
            let life = r.range(0.5, 0.9);
            let col = if r.chance(0.5) { WHITE } else { c[6] };
            let size = r.range(0.6, 1.3);
            self.ps.spawn(
                r,
                Spawn {
                    col: Some(col),
                    size,
                    ..Spawn::new(Kind::Flake, x, y, vx, vy, life)
                },
            );
        }
        if k > 0.3 && t < CRASH && self.r.chance(0.35) {
            let r = &mut self.r;
            let (x, y, _) = lip(1.0, t);
            let z = r.range(-3.0, 3.0);
            let vx = r.range(30.0, 90.0);
            let vz = r.range(-10.0, 10.0);
            let vh = r.range(-20.0, 40.0);
            let s = if r.chance(0.3) { 2.0 } else { 1.0 };
            self.drops.add(Drop {
                x,
                z,
                h: -y,
                vx,
                vz,
                vh,
                s,
                ..Drop::default()
            });
        }
        if !self.crashed && t >= CRASH {
            self.crashed = true;
            self.crash();
        }
        // the whitewater surges on, boiling up foam at its front, then calms
        if self.crashed && t < CRASH + 1.1 && self.r.chance(0.45 * (1.0 - (t - CRASH) / 1.1)) {
            let r = &mut self.r;
            let x = crest_x(t) + r.range(-6.0, 14.0);
            let y = -r.range(1.0, 4.0);
            let vx = r.range(10.0, 40.0);
            let vy = -r.range(10.0, 40.0);
            let r0 = r.range(0.9, 1.8);
            let r1 = r.range(1.8, 3.6);
            let life = r.range(0.4, 0.8);
            self.puff(x, y, vx, vy, r0, r1, life, 0.0);
        }
        if self.crashed && t < CRASH + 0.8 && self.r.chance(0.12) {
            let r = &mut self.r;
            let x = crest_x(t) + r.range(-8.0, 8.0);
            let y = -r.range(2.0, 6.0);
            let vx = r.range(10.0, 40.0);
            let vy = -r.range(3.0, 10.0);
            let r1 = r.range(7.0, 11.0);
            let life = r.range(0.8, 1.1);
            self.mist.add(x, y, vx, vy, 3.0, r1, life, c[6], 0.2, 0.0);
        }
        for i in (0..self.puffs.len()).rev() {
            let Some(f) = self.puffs.get_mut(i) else {
                continue;
            };
            if f.delay > 0.0 {
                f.delay -= dt;
                continue;
            }
            f.life -= dt;
            if f.life <= 0.0 {
                self.puffs.remove(i);
                continue;
            }
            f.vx -= f.vx * 2.5 * dt;
            f.vy += (160.0 - f.vy * 2.5) * dt;
            f.x += f.vx * dt;
            f.y = js_min(f.y + f.vy * dt, -1.0);
        }
        for i in (0..self.plinks.len()).rev() {
            let Some(q) = self.plinks.get_mut(i) else {
                continue;
            };
            q.age += dt;
            if q.age > 0.12 {
                self.plinks.remove(i);
            }
        }
        for d in self.drops.update(dt) {
            self.rings.add(
                d.x,
                d.z,
                0.6,
                2.0 + d.s * 1.8,
                0.3 + d.s * 0.1,
                RippleOpts {
                    n: 1.0,
                    k: 0.9,
                    hot: 0.05,
                    ..RippleOpts::default()
                },
            );
            self.plinks.push(Plink {
                x: d.x,
                y: d.z * SY,
                age: 0.0,
            });
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

impl State {
    fn fade(&self) -> f64 {
        smooth(0.0, 0.25, self.t) * (1.0 - smooth(DUR - 0.7, DUR - 0.1, self.t))
    }

    // the swell's body, column by column: deep blue down its back and base with wind lines, a lit rim along its
    // surface, white water at the crest, a lit window high on its face where the light comes through, and streaks
    // of foam drawn up the face
    fn body(&self, p: &mut Painter, fade: f64) {
        let (c, t, s) = (&self.c, self.t, self.s);
        let (xc, a_) = (crest_x(t), crest_h(t));
        let mut xx = -EDGE;
        while xx < EDGE {
            let px = xx;
            xx += 1.0;
            let x = px + 0.5;
            let h = height(x, t);
            let edge = 1.0 - smooth(EDGE - 24.0, EDGE, libm::fabs(x));
            if h < 0.6 || edge <= 0.0 {
                continue;
            }
            let face = x > xc;
            let near = libm::exp(-libm::pow((x - xc) / 9.0, 2.0));
            let a = fade * edge * 1.4;
            let mut yy = libm::floor(-h);
            while yy < 0.0 {
                let y = yy + 0.5;
                let u = (y + h) / h;
                let depth = y + h;
                let col = if depth < 1.0 {
                    if near > 0.5 && a_ > 14.0 && noise(x * 0.6 + t * 9.0, s) > -0.35 {
                        WHITE
                    } else if face {
                        c[5]
                    } else {
                        c[6]
                    }
                } else if depth < 2.4 && near > 0.35 && a_ > 14.0 {
                    if noise2(x * 0.5, y * 0.5 + t * 6.0, s + 1.0) > -0.1 {
                        WHITE
                    } else {
                        c[6]
                    }
                } else {
                    // face and back shade differently; blend the two across the crest so there's no seam
                    let win = near * (1.0 - u) * 1.6;
                    let up = noise2((x * 0.9 - y * 0.35) * 0.8, y * 0.12 + t * 5.0, s + 2.0);
                    let wind = noise2(x * 0.18 - t * 2.0, y * 0.9, s + 3.0);
                    let fi = if up > 0.55 && u < 0.85 {
                        6.0
                    } else if win > 0.8 {
                        5.0
                    } else if win > 0.45 || up > 0.3 {
                        4.0
                    } else if win > 0.2 || up > 0.05 {
                        3.0
                    } else if u < 0.6 {
                        2.0
                    } else {
                        1.0
                    };
                    let bi = if u < 0.14 || wind > 0.55 {
                        4.0
                    } else if wind > 0.25 || u < 0.32 {
                        3.0
                    } else if u < 0.7 {
                        2.0
                    } else {
                        1.0
                    };
                    let i = js_round(bi + (fi - bi) * smooth(xc - 5.0, xc + 5.0, x));
                    c.get(i as usize).copied().unwrap_or(Rgba::NONE)
                };
                p.dot(px, yy, col, a);
                yy += 1.0;
            }
        }
    }

    // the lip: a thick arc thrown off the crest, white along its outer edge, tinted through, dark underneath
    fn curl(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        let k = lip_k(t);
        if k <= 0.0 || t >= CRASH {
            return;
        }
        // JS Map: insertion order, a replaced cell keeps its place
        let mut index: BTreeMap<i64, usize> = BTreeMap::new();
        let mut cells: Vec<(f64, f64, Rgba, f64)> = Vec::new();
        let mut s = 0.0;
        while s <= 1.0 {
            let (x, y, w) = lip(s, t);
            let (x2, y2, _) = lip(js_min(1.0, s + 0.012), t);
            let (tx, ty) = (x2 - x, y2 - y);
            let l = js_hypot(tx, ty);
            let l = if l == 0.0 || l.is_nan() { 1.0 } else { l };
            let (nx, ny) = (ty / l, -tx / l);
            let mut q = 0.0;
            while q <= 1.0 {
                let px = x + nx * (q - 0.5) * w;
                let py = y + ny * (q - 0.5) * w;
                let key = (libm::floor(py) * 1024.0 + libm::floor(px)) as i64;
                // q 0 is the outer (upper) rim, 1 the underside
                let col = if q < 0.18 {
                    WHITE
                } else if q < 0.36 {
                    if s > 0.8 { WHITE } else { c[6] }
                } else if q < 0.56 {
                    c[5]
                } else if q < 0.76 {
                    c[4]
                } else {
                    c[3]
                };
                match index.get(&key) {
                    Some(&i) => {
                        if let Some(cur) = cells.get_mut(i)
                            && q < cur.3
                        {
                            *cur = (px, py, col, q);
                        }
                    }
                    None => {
                        index.insert(key, cells.len());
                        cells.push((px, py, col, q));
                    }
                }
                q += 0.5 / js_max(1.0, w);
            }
            s += 0.012;
        }
        for &(x, y, col, _) in &cells {
            p.px(x, y, col);
        }
        let mut s = 0.0;
        while s <= 1.0 {
            let (x, y, w) = lip(s, t);
            p.glow(x, y, 6.0 + w, c[5], 0.24 * k);
            s += 0.125;
        }
        // the tip glints as it throws
        let (tx, ty, _) = lip(1.0, t);
        let g = libm::sin(PI * clamp1((k - 0.55) / 0.45));
        if g > 0.05 {
            sparkle(p, tx, ty, 6.0 * g, c[6], c[5], 0.5 * g);
        }
        // light caught under the barrel
        let (bx, by, _) = lip(0.6, t);
        p.glow(bx - 3.0, by + 8.0, 16.0, c[4], 0.25 * k);
    }

    // the sea it rolls on: a lit line along the surface, dark water under it, glints sliding along
    fn sea(&self, p: &mut Painter, fade: f64) {
        let (c, t, s) = (&self.c, self.t, self.s);
        let mut xx = -EDGE;
        while xx < EDGE {
            let px = xx;
            xx += 1.0;
            let x = px + 0.5;
            let edge = 1.0 - smooth(EDGE - 30.0, EDGE, libm::fabs(x));
            let a = fade * edge;
            if a <= 0.02 {
                continue;
            }
            let g = noise(x * 0.3 - t * 3.0, s + 4.0);
            p.dot(px, 0.0, if g > 0.55 { c[6] } else { c[4] }, a * 1.2);
            p.blend(x, 1.5, c[2], 0.6 * a);
            p.blend(x, 2.5, c[1], 0.45 * a);
        }
    }

    fn draw_back(&self, p: &mut Painter) {
        let fade = self.fade();
        self.sea(p, fade);
        self.rings.draw(p, &self.c, -1.0);
        self.drops.draw(p, &self.c, Side::Back);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (c, t, s) = (&self.c, self.t, self.s);
        let fade = self.fade();
        self.body(p, fade);
        self.curl(p);
        self.rings.draw(p, c, 1.0);
        // lace left on the water behind the surge, thinning out
        if self.crashed {
            let k = 1.0 - smooth(CRASH + 0.4, DUR - 0.15, t);
            let (x0, x1) = (-20.0, crest_x(t) + 10.0);
            let mut xx = libm::floor(x0);
            while xx < x1 {
                let x = xx + 0.5;
                let n = 1.0 - libm::fabs(noise2(x * 0.35, t * 0.8, s + 7.0)) * 4.0;
                if n > 1.15 - k * 1.1 {
                    p.px(xx, 0.0, if n > 1.4 - k { WHITE } else { c[6] });
                }
                if n > 1.4 - k * 0.9 {
                    p.px(xx, 1.0, c[5]);
                }
                xx += 1.0;
            }
        }
        for f in &self.puffs {
            if f.delay > 0.0 {
                continue;
            }
            let g = 1.0 - f.life / f.max;
            let r = (f.r0 + (f.r1 - f.r0) * ease::out_cubic(g)) * (1.0 - smooth(0.7, 1.0, g));
            if r > 0.6 {
                billow(p, f.x, f.y, r, c, 1.0);
            }
        }
        for q in &self.plinks {
            if q.age < 0.06 {
                p.px(q.x, q.y, WHITE);
            }
            p.glow(q.x, q.y, 4.0, c[5], 0.25 * (1.0 - q.age / 0.12));
        }
        self.drops.draw(p, c, Side::Front);
        // the crash: a white burst where the lip lands
        let f = (t - CRASH) / 0.08;
        if (0.0..1.0).contains(&f) {
            let (x0, _, _) = lip(1.0, CRASH - 1e-3);
            p.circle(x0, -3.0, 8.0 * (1.0 - f * 0.4), WHITE, 1.0);
            let c6 = c[6];
            p.ellipse(
                x0,
                0.0,
                14.0,
                2.6,
                |d: f64, _| if d < 0.6 { WHITE } else { c6 },
                1.0,
            );
        }
        self.ps.draw(p, Layer::Both);
        self.mist.draw(p, Some(c[5]));
        self.lights.draw(p);
        // light: the crest catches it while it stands, the whitewater after
        if t < CRASH {
            p.glow(
                crest_x(t) + 4.0,
                -crest_h(t) + 2.0,
                26.0,
                c[4],
                0.22 * smooth(0.2, CRASH, t),
            );
            p.glow_line(
                crest_x(t) - 26.0,
                -crest_h(t) * 0.45,
                crest_x(t),
                -crest_h(t),
                10.0,
                c[5],
                0.14 * smooth(0.2, CRASH, t),
            );
        } else {
            p.glow(
                crest_x(t),
                -10.0,
                34.0,
                c[4],
                0.26 * (1.0 - smooth(CRASH, DUR - 0.3, t)),
            );
        }
    }
}
