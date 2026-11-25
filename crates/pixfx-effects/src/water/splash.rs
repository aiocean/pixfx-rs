//! Splash: port of pixelfx `src/fx/water/splash.js`.
//!
//! The droplet recipe (shaded beads with tails that fly, land and throw up a little crown of their own) is
//! adapted from Sprite Fusion's open-source blood (https://www.spritefusion.com/pixel-art-vfx/blood),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use std::collections::HashMap;

use pixfx_core::sim::{Kind, Lights, Particles, ParticlesOpts, Spawn, sparkle};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_max, noise, rand, smooth, to_uint32};

use super::lib::{
    Drop, Drops, Mist, ROCK, Ramp, RippleOpts, Ripples, Side, WATER, bead, lace, pebble,
};

pub struct Splash;

pub static SPLASH: Splash = Splash;

const META: Meta = Meta {
    id: "water/splash",
    name: "Splash",
    desc: "A stone drops into still water: a white hit, a crown that tears into droplets arcing out to plink down with rings, a jet that bounces back, foam and drifting mist.",
    tags: &["water", "splash", "impact", "drop", "liquid", "one-shot"],
    size: (176, 100),
    origin: (88, 78),
    duration: 2.5,
    looping: false,
    fps: 30,
    seed: 3,
    state: None,
    options: &[STONE.spec()],
};

/// pixelfx `stone: true`: 0 for no falling stone, anything else for one.
const STONE: NumOpt = NumOpt::new("stone", 1.0);

const SY: f64 = 0.22;
const HIT: f64 = 0.16;
const G: f64 = 820.0;
const FINGERS: usize = 12;
const JET: f64 = 0.3;
const TEAR: f64 = 0.11;
const PI: f64 = core::f64::consts::PI;

// the crown over time u after the hit: it leaps up in a few frames, flares out, then falls back
fn height(u: f64) -> f64 {
    19.0 * ease::out_cubic(clamp(u / 0.09, 0.0, 1.0))
        * (1.0 - ease::in_quad(clamp((u - 0.12) / 0.3, 0.0, 1.0)))
}
fn foot(u: f64) -> f64 {
    3.0 + 7.5 * ease::out_cubic(clamp(u / 0.45, 0.0, 1.0))
}
fn lip(u: f64) -> f64 {
    foot(u) + height(u) * 0.42 + 3.0 * ease::out_cubic(clamp(u / 0.25, 0.0, 1.0))
}
// the jet that bounces back up the middle once the crown has fallen
fn jet_h(u: f64) -> f64 {
    26.0 * (ease::out_cubic(clamp(u / 0.12, 0.0, 1.0))
        - ease::in_quad(clamp((u - 0.15) / 0.28, 0.0, 1.0)))
}

impl Effect for Splash {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: Ramp = ramp(&opts.palette_or(&WATER));
        let mut r = Rng::new(to_uint32(seed as f64));
        let phi = r.range(0.0, TAU);
        let spikes = (0..FINGERS)
            .map(|k| {
                let a = ((k as f64 * TAU) / FINGERS as f64 + phi) % TAU;
                (a, r.range(0.2, 0.36))
            })
            .collect();
        let stone = opts.num(&STONE);
        Box::new(State {
            c,
            r,
            phi,
            seed: seed as f64,
            stone: stone != 0.0 && !stone.is_nan(),
            ps: Particles::new(ParticlesOpts::default()),
            mist: Mist::new(),
            lights: Lights::new(),
            rings: Ripples::new(SY),
            drops: Drops::new(SY, G),
            plinks: Vec::new(),
            bubbles: Vec::new(),
            spikes,
            t: 0.0,
            done: [false; 9],
        })
    }
}

struct Plink {
    x: f64,
    y: f64,
    s: f64,
    age: f64,
}

struct Bubble {
    x: f64,
    z: f64,
    t0: f64,
    r: f64,
    popped: bool,
}

struct State {
    c: Ramp,
    r: Rng,
    phi: f64,
    seed: f64,
    stone: bool,
    ps: Particles,
    mist: Mist,
    lights: Lights,
    rings: Ripples,
    drops: Drops,
    plinks: Vec<Plink>,
    bubbles: Vec<Bubble>,
    /// The fingers: angle and length share.
    spikes: Vec<(f64, f64)>,
    t: f64,
    /// pixelfx `once` keys: hit, spray, tear, fall, bead, slump, glug0..2.
    done: [bool; 9],
}

const HIT_K: usize = 0;
const SPRAY: usize = 1;
const TEAR_K: usize = 2;
const FALL: usize = 3;
const BEAD: usize = 4;
const SLUMP: usize = 5;
const GLUG: usize = 6;

struct Cell {
    x: f64,
    y: f64,
    c: Rgba,
    a: f64,
    r: u8,
}

impl State {
    fn once(&mut self, key: usize, at: f64) -> bool {
        let Some(d) = self.done.get_mut(key) else {
            return false;
        };
        if !*d && self.t >= at {
            *d = true;
            true
        } else {
            false
        }
    }

    // the lip isn't level: it rises and dips round the crown
    fn scallop(&self, a: f64, h: f64) -> f64 {
        h * (0.86
            + 0.14
                * noise(
                    libm::cos(a) * 2.2 + libm::sin(a) * 1.3 + self.phi * 3.0,
                    self.seed,
                ))
    }

    // the crown: a thin sheet from its foot to a rolled lip. The far wall's shadowed inside shows over the near
    // lip; the near wall is see-through between bright streaks, white down its lit edge; fingers rise off the lip
    fn crown(&self, p: &mut Painter, back: bool) {
        let c = &self.c;
        let u = self.t - HIT;
        let h = height(u);
        if u < 0.0 || h < 0.7 {
            return;
        }
        let (r0, rt, white) = (foot(u), lip(u), u < 0.045);
        let mut cells: Vec<Cell> = Vec::new();
        let mut index: HashMap<i64, usize> = HashMap::new();
        let mut put = |x: f64, y: f64, col: Rgba, a: f64, rank: u8| {
            let (bx, by) = (libm::floor(x), libm::floor(y));
            let key = ((by + 4096.0) * 8192.0 + bx + 4096.0) as i64;
            match index.get(&key) {
                Some(&i) => {
                    if let Some(cur) = cells.get_mut(i)
                        && rank > cur.r
                    {
                        *cur = Cell {
                            x: bx,
                            y: by,
                            c: col,
                            a,
                            r: rank,
                        };
                    }
                }
                None => {
                    index.insert(key, cells.len());
                    cells.push(Cell {
                        x: bx,
                        y: by,
                        c: col,
                        a,
                        r: rank,
                    });
                }
            }
        };
        let n = libm::ceil(rt * TAU * 1.6);
        let mut i = 0.0;
        while i < n {
            let a = (i / n) * TAU;
            i += 1.0;
            let (s, co) = (libm::sin(a), libm::cos(a));
            if (s < 0.0) != back {
                continue;
            }
            let lane = libm::floor(((a + self.phi) * rt) / 2.2);
            let streak = rand(&[lane, 31.0]) < 0.3;
            let hh = self.scallop(a, h);
            // as it falls, each lane of the sheet gives way from the top at its own moment
            let keep = 1.0 - clamp((u - 0.13 - 0.14 * rand(&[lane, 7.0])) / 0.1, 0.0, 1.0);
            let steps = libm::ceil(hh * 1.4) + 2.0;
            let mut k = 0.0;
            while k <= steps * keep {
                let v = k / steps;
                k += 1.0;
                let r = r0 + (rt - r0) * libm::pow(v, 2.2);
                let (x, y) = (co * r, s * r * SY - hh * v);
                if white {
                    put(x, y, WHITE, 1.0, 3);
                } else if back {
                    let col = if v > 0.72 {
                        if streak { c[5] } else { c[4] }
                    } else if streak {
                        c[3]
                    } else {
                        c[2]
                    };
                    put(x, y, col, 1.0, 1);
                } else if libm::fabs(co) > 0.9 {
                    let col = if co < 0.0 {
                        if libm::fabs(co) > 0.97 { WHITE } else { c[6] }
                    } else {
                        c[4]
                    };
                    put(x, y, col, 1.0, 2);
                } else if v < 0.1 {
                    put(x, y, if v < 0.05 { WHITE } else { c[6] }, 1.0, 2);
                } else if streak {
                    put(x, y, if co < -0.2 { c[6] } else { c[5] }, 1.0, 1);
                } else {
                    put(x, y, if co < 0.0 { c[5] } else { c[4] }, 0.22, 0);
                }
            }
            // the rolled lip: a hard white line on the near side, softer on the far, gone where the lane broke
            if !white && keep >= 1.0 {
                let (x, y) = (co * rt, s * rt * SY - hh);
                let col = if back {
                    c[5]
                } else if co < 0.45 {
                    WHITE
                } else {
                    c[6]
                };
                put(x, y, col, 1.0, 3);
                if !back && co < 0.2 {
                    put(x, y + 1.0, c[6], 1.0, 2);
                }
            }
        }
        for q in &cells {
            if q.r == 0 {
                p.blend(q.x, q.y, q.c, q.a);
            } else {
                p.px(q.x, q.y, q.c);
            }
        }
        // fingers: spikes running on up and out from the lip, a bead on each until it tears off
        let reach = 1.0 - smooth(TEAR, TEAR + 0.1, u);
        for &(fa, fl) in &self.spikes {
            if (libm::sin(fa) < 0.0) != back || reach <= 0.0 {
                continue;
            }
            let hh = self.scallop(fa, h);
            let l = h * fl * reach;
            let (x0, y0) = (libm::cos(fa) * rt, libm::sin(fa) * rt * SY - hh);
            let (x1, y1) = (x0 + libm::cos(fa) * l * 0.45, y0 - l);
            if l >= 1.0 {
                let col = if white {
                    WHITE
                } else if back {
                    c[4]
                } else {
                    c[6]
                };
                p.line(x0, y0, x1, y1, col, 1.0);
            }
            if u > 0.04 && u < TEAR {
                bead(
                    p,
                    x1 - if back { 0.0 } else { 0.5 },
                    y1 - 1.0,
                    0.0,
                    -1.0,
                    if back { 1.0 } else { 2.0 },
                    c,
                    1.0,
                );
            }
            if !back && libm::sin(fa) > 0.3 {
                p.glow(x0, y0, 6.0, c[5], 0.22 * reach);
            }
        }
    }

    fn jet(&self, p: &mut Painter) {
        let c = &self.c;
        let u = self.t - HIT - JET;
        let h = jet_h(u);
        if u < 0.0 || h < 0.8 {
            return;
        }
        let top = -h;
        // a mound of water heaves up under it
        let (c5, c6) = (c[5], c[6]);
        p.ellipse(
            0.0,
            0.0,
            5.0,
            2.4,
            |d: f64, a: f64| {
                if libm::sin(a) > 0.0 {
                    Rgba::NONE
                } else if d < 0.5 {
                    WHITE
                } else if libm::cos(a) < 0.0 {
                    c6
                } else {
                    c5
                }
            },
            1.0,
        );
        let c4 = c[4];
        p.shade(-3.0, top - 2.0, 3.0, 0.0, |x, y, out| {
            let v = clamp((y - top) / h, 0.0, 1.0);
            let w = 1.15 + 1.5 * v * v * v;
            let q = x / w;
            if libm::fabs(q) > 1.0 {
                return None;
            }
            out.c = if q < -0.5 {
                c6
            } else if q < 0.3 {
                WHITE
            } else if q < 0.72 {
                c5
            } else {
                c4
            };
            None
        });
        if u < 0.12 {
            // the bulb at its tip, about to pinch off
            p.circle(0.0, top - 1.0, 1.7, c[5], 1.0);
            p.circle(-0.3, top - 1.3, 1.1, c[6], 1.0);
            p.px(-1.0, top - 2.0, WHITE);
        }
        p.glow_line(0.0, -1.0, 0.0, top, 7.0, c[5], 0.3);
    }

    fn draw_back(&self, p: &mut Painter) {
        let c = &self.c;
        let u = self.t - HIT;
        // the cavity the stone punches, dark inside the foot of the crown
        if u > 0.0 && u < 0.26 {
            let r = foot(u) * 0.85 * (1.0 - smooth(0.12, 0.26, u));
            let (c1, c2) = (c[1], c[2]);
            p.ellipse(
                0.0,
                0.0,
                r,
                r * SY + 0.6,
                |_: f64, a: f64| if libm::sin(a) < 0.0 { c1 } else { c2 },
                1.0,
            );
        }
        self.rings.draw(p, c, -1.0);
        self.crown(p, true);
        self.drops.draw(p, c, Side::Back);
    }

    fn draw_front(&self, p: &mut Painter) {
        let c = &self.c;
        let t = self.t;
        let u = t - HIT;
        self.rings.draw(p, c, 1.0);
        if u > 0.2 {
            let rx = 4.0 + 11.0 * ease::out_cubic(clamp((u - 0.2) / 0.7, 0.0, 1.0));
            let k = smooth(0.2, 0.32, u) * (1.0 - smooth(0.5, 2.0, u)) * 0.9;
            lace(p, 0.0, 0.0, rx, rx * SY + 0.5, k, c, t, self.seed);
        }
        self.crown(p, false);
        self.jet(p);
        for b in &self.bubbles {
            let g = (t - b.t0) / 0.2;
            if g > 1.0 {
                continue;
            }
            let r = b.r * ease::out_back(g, ease::BACK_S);
            let (x, y) = (b.x, b.z * SY);
            let mut a = PI;
            while a <= TAU + 0.01 {
                p.dot(x + libm::cos(a) * r, y + libm::sin(a) * r, c[6], 1.0);
                a += 0.5 / js_max(1.0, r);
            }
            p.px(x - r * 0.4, y - r * 0.6, WHITE);
            p.glow(x, y - 1.0, 4.0, c[5], 0.25);
        }
        for q in &self.plinks {
            let k = q.age / 0.14;
            if k < 0.5 {
                p.px(q.x, q.y, WHITE);
            }
            if q.s > 1.0 {
                p.line(
                    q.x,
                    q.y - 1.0,
                    q.x,
                    q.y - 1.0 - 3.0 * (1.0 - k),
                    if k < 0.4 { WHITE } else { c[6] },
                    1.0,
                );
            }
            p.glow(q.x, q.y, 4.0, c[5], 0.3 * (1.0 - k));
        }
        self.drops.draw(p, c, Side::Front);
        // the hit: a flat white flash on the surface for two frames
        if (0.0..0.07).contains(&u) {
            let c6 = c[6];
            p.ellipse(
                0.0,
                0.0,
                10.0,
                2.4,
                |d: f64, _: f64| if d < 0.6 { WHITE } else { c6 },
                1.0,
            );
            sparkle(p, 0.0, -3.0, 5.0 * (1.0 - u / 0.07), c[6], c[5], 0.5);
        }
        // the stone, falling fast with a streak of air off it
        if self.stone && t < HIT {
            let v = t / HIT;
            let y = -70.0 + 70.0 * (0.4 * v + 0.6 * v * v);
            p.line(0.0, y - 12.0, 0.0, y - 3.0, ROCK[4], |w: f64| w * 0.8);
            pebble(p, 0.0, y, 2.6, &ROCK);
        }
        self.mist.draw(p, Some(c[5]));
        self.ps.draw(p, Layer::Both);
        self.lights.draw(p);
        // the disturbed water keeps a cool light on it while it settles
        if u > 0.0 {
            p.glow(0.0, -2.0, 34.0, c[4], 0.2 * (1.0 - smooth(0.0, 1.6, u)));
            p.glow(0.0, 2.0, 64.0, c[3], 0.16 * (1.0 - smooth(0.1, 2.1, u)));
        }
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let c = self.c;
        if self.once(HIT_K, HIT) {
            self.lights.add(0.0, -6.0, 52.0, c[5], 0.6, 0.45);
            self.lights.add(0.0, -4.0, 16.0, WHITE, 0.5, 0.08);
            self.rings.add(
                0.0,
                0.0,
                8.0,
                56.0,
                2.0,
                RippleOpts {
                    n: 2.0,
                    gap: 3.4,
                    k: 1.35,
                    hot: 0.16,
                    glint: 1.0,
                    ..RippleOpts::default()
                },
            );
            self.rings.add(
                0.0,
                0.0,
                5.0,
                38.0,
                1.7,
                RippleOpts {
                    n: 1.0,
                    k: 0.9,
                    delay: 0.14,
                    ..RippleOpts::default()
                },
            );
            // the first sheet of spray, thrown up fast and fine
            let r = &mut self.r;
            for _ in 0..20 {
                let a = r.range(0.0, TAU);
                let v = r.range(50.0, 125.0);
                let h = r.range(1.0, 6.0);
                let vh = r.range(160.0, 270.0);
                let s = if r.chance(0.25) { 2.0 } else { 1.0 };
                let delay = r.range(0.0, 0.04);
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
            // mist rolls out low along the water
            for i in 0..9 {
                let s = if i % 2 == 1 { 1.0 } else { -1.0 };
                let x = s * r.range(4.0, 12.0);
                let y = -r.range(1.0, 7.0);
                let vx = s * r.range(25.0, 70.0);
                let vy = -r.range(4.0, 16.0);
                let r0 = r.range(2.5, 4.0);
                let r1 = r.range(7.0, 11.0);
                let life = r.range(0.9, 1.5);
                let col = if r.chance(0.5) { c[6] } else { c[5] };
                let al = r.range(0.3, 0.42);
                let delay = r.range(0.0, 0.1);
                self.mist.add(x, y, vx, vy, r0, r1, life, col, al, delay);
            }
        }
        if self.once(SPRAY, HIT + 0.04) {
            let r = &mut self.r;
            for _ in 0..20 {
                let a = r.range(0.0, TAU);
                let x = libm::cos(a) * 9.0;
                let y = libm::sin(a) * 9.0 * SY - r.range(6.0, 15.0);
                let vx = libm::cos(a) * r.range(25.0, 90.0);
                let vy = -r.range(30.0, 100.0);
                let life = r.range(0.7, 1.3);
                let col = if r.chance(0.5) { WHITE } else { c[6] };
                let size = r.range(0.6, 1.5);
                self.ps.spawn(
                    r,
                    Spawn {
                        col: Some(col),
                        size,
                        ..Spawn::new(Kind::Flake, x, y, vx, vy, life)
                    },
                );
            }
            for _ in 0..5 {
                let x = r.range(-13.0, 13.0);
                let y = -r.range(6.0, 20.0);
                let life = r.range(0.25, 0.55);
                let size = r.range(1.0, 2.4);
                let delay = r.range(0.0, 0.15);
                self.ps.spawn(
                    r,
                    Spawn {
                        col: Some(c[6]),
                        size,
                        delay,
                        ..Spawn::new(Kind::Star, x, y, 0.0, -4.0, life)
                    },
                );
            }
        }
        if self.once(TEAR_K, HIT + TEAR) {
            // the beads on the finger tips tear off and arc away
            let (h, rt) = (height(TEAR), lip(TEAR));
            let r = &mut self.r;
            for &(fa, fl) in &self.spikes {
                let a = fa + r.range(-0.08, 0.08);
                let v = r.range(32.0, 88.0);
                let l = h * fl;
                let vh = r.range(90.0, 190.0);
                let s = if r.chance(0.3) { 3.0 } else { 2.0 };
                self.drops.add(Drop {
                    x: libm::cos(fa) * (rt + l * 0.45),
                    z: libm::sin(fa) * rt,
                    h: h + l + 1.0,
                    vx: libm::cos(a) * v,
                    vz: libm::sin(a) * v,
                    vh,
                    s,
                    ..Drop::default()
                });
            }
        }
        // as the crown gives way its lip comes apart into drops
        let u = self.t - HIT;
        if u > 0.13 && u < 0.3 && self.r.chance(0.3) {
            let a = self.r.range(0.0, TAU);
            let rt = lip(u);
            let v = self.r.range(15.0, 45.0);
            let sc = self.scallop(a, height(u));
            let h = sc * self.r.range(0.6, 1.0);
            let vh = self.r.range(-10.0, 45.0);
            let s = if self.r.chance(0.3) { 2.0 } else { 1.0 };
            self.drops.add(Drop {
                x: libm::cos(a) * rt,
                z: libm::sin(a) * rt,
                h,
                vx: libm::cos(a) * v,
                vz: libm::sin(a) * v,
                vh,
                s,
                ..Drop::default()
            });
        }
        if self.once(FALL, HIT + 0.3) {
            // the crown falls back, shedding a few heavy drops close in
            let r = &mut self.r;
            for _ in 0..8 {
                let a = r.range(0.0, TAU);
                let rl = lip(0.3);
                let v = r.range(15.0, 40.0);
                let h = r.range(3.0, 7.0);
                let vh = r.range(30.0, 80.0);
                let s = if r.chance(0.4) { 2.0 } else { 1.0 };
                self.drops.add(Drop {
                    x: libm::cos(a) * rl,
                    z: libm::sin(a) * rl,
                    h,
                    vx: libm::cos(a) * v,
                    vz: libm::sin(a) * v,
                    vh,
                    s,
                    ..Drop::default()
                });
            }
        }
        if self.once(BEAD, HIT + JET + 0.12) {
            // the jet's tip pinches off into a bead that hangs, then drops back
            let vx = self.r.range(-6.0, 6.0);
            self.drops.add(Drop {
                x: 0.0,
                z: 0.2,
                h: jet_h(0.12) + 1.0,
                vx,
                vz: 0.0,
                vh: 55.0,
                s: 3.0,
                ..Drop::default()
            });
            let r = &mut self.r;
            for _ in 0..3 {
                let x = r.range(-3.0, 3.0);
                let y = -jet_h(0.12) + r.range(2.0, 8.0);
                let vx = r.range(-14.0, 14.0);
                let vy = -r.range(4.0, 12.0);
                let r1 = r.range(4.0, 6.0);
                let life = r.range(0.6, 1.0);
                self.mist.add(x, y, vx, vy, 2.0, r1, life, c[6], 0.3, 0.0);
            }
        }
        if self.once(SLUMP, HIT + JET + 0.38) {
            self.rings.add(
                0.0,
                0.0,
                3.0,
                30.0,
                1.5,
                RippleOpts {
                    n: 2.0,
                    gap: 2.6,
                    k: 0.9,
                    ..RippleOpts::default()
                },
            );
            let r = &mut self.r;
            for _ in 0..4 {
                let x = r.range(-6.0, 6.0);
                let y = -r.range(1.0, 4.0);
                let vx = r.range(-30.0, 30.0);
                let vy = -r.range(3.0, 10.0);
                let r1 = r.range(6.0, 9.0);
                let life = r.range(0.8, 1.2);
                self.mist.add(x, y, vx, vy, 2.5, r1, life, c[5], 0.28, 0.0);
            }
        }
        // the stone sinks and lets go of its air: a few bubbles come up and pop
        for (i, at) in [(0, 0.95), (1, 1.2), (2, 1.5)] {
            if self.once(GLUG + i, HIT + at) {
                let x = self.r.range(-5.0, 5.0);
                let z = self.r.range(-6.0, 6.0);
                let r = self.r.range(1.3, 2.3);
                self.bubbles.push(Bubble {
                    x,
                    z,
                    t0: self.t,
                    r,
                    popped: false,
                });
            }
        }
        for b in &mut self.bubbles {
            if !b.popped && self.t - b.t0 > 0.2 {
                b.popped = true;
                self.rings.add(
                    b.x,
                    b.z,
                    1.0,
                    7.0,
                    0.6,
                    RippleOpts {
                        n: 1.0,
                        k: 0.9,
                        ..RippleOpts::default()
                    },
                );
                self.plinks.push(Plink {
                    x: b.x,
                    y: b.z * SY,
                    s: 1.0,
                    age: 0.0,
                });
                for _ in 0..2 {
                    let vx = self.r.range(-20.0, 20.0);
                    let vz = self.r.range(-10.0, 10.0);
                    let vh = self.r.range(50.0, 90.0);
                    self.drops.add(Drop {
                        x: b.x,
                        z: b.z,
                        h: 1.0,
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
            // every drop that comes down plinks: a white fleck, a ring, and the bigger ones throw up a tiny crown
            self.rings.add(
                d.x,
                d.z,
                0.6,
                2.0 + d.s * 1.8,
                0.34 + d.s * 0.1,
                RippleOpts {
                    n: 1.0,
                    k: 0.95,
                    hot: 0.06,
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
                let mut i = 0.0;
                while i < d.s {
                    i += 1.0;
                    let a = self.r.range(0.0, TAU);
                    let v = self.r.range(10.0, 34.0);
                    let vh = self.r.range(55.0, 110.0);
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
