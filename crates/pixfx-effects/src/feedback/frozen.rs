//! `feedback/frozen`: the wrapped sprite is chilled to the bone. Its body goes blue-grey with a lit rim, frost
//! ferns creep in from its flanks, icicles hang from the chin, hair and hands and drip, snow sits on the head and
//! shoulders, breath steams between shivers, frost spreads underfoot; every second a shiver cracks the ferns off,
//! jolts the overlay and flings flakes.
//!
//! Adapted from Pixel VFX Studio's `status/frozen` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026 Hugo
//! Duprez). Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it reads
//! the [`Body`] once per pose to chill its colours and plan the ferns, icicles, snow and mouth; with no sprite it
//! uses the stand-in figure. The sprite itself never moves: the shiver jolts only what hangs on it.

use std::sync::Arc;

use pixfx_core::particles::{cycle, fly};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options, sparkle};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::{
    Rng, TAU, clamp, fract, js_min, js_round, rand, smooth, to_int32, to_uint32,
};

use super::status::{BOX, Body, ORIGIN, SIZE, floor_light};

pub struct Frozen;

pub static FROZEN: Frozen = Frozen;

const FPS: u32 = 16;
const N: i32 = 32;
/// Seconds per loop.
const D: f64 = 2.0;
/// Frames between shivers: two a loop.
const SHIVER: i32 = 16;
const K_FROST: f64 = 41.0;
const K_FERN: f64 = 42.0;
const K_SNOW: f64 = 43.0;
const K_FLING: f64 = 44.0;

const FERNS: NumOpt = NumOpt::new("ferns", 6.0).within(0.0, 8.0);
const ICICLES: NumOpt = NumOpt::new("icicles", 6.0).within(0.0, 6.0);
const SNOW: NumOpt = NumOpt::new("snow", 1.0).within(0.0, 1.5);
const SHIVER_OPT: NumOpt = NumOpt::new("shiver", 1.0).within(0.0, 1.0);

const META: Meta = Meta {
    id: "feedback/frozen",
    name: "Frozen",
    desc: "The wrapped sprite goes blue-grey with a lit rim; frost ferns creep in from its flanks, icicles hang from its chin, hair and hands and drip, snow settles on its head and shoulders, breath steams, frost spreads underfoot, and every second a shiver cracks the ferns off and flings flakes.",
    tags: &[
        "feedback",
        "ice",
        "freeze",
        "frozen",
        "cold",
        "status",
        "character",
        "loop",
    ],
    size: SIZE,
    origin: ORIGIN,
    duration: D,
    looping: true,
    fps: FPS,
    seed: 3_507_412_219,
    state: None,
    options: &[FERNS.spec(), ICICLES.spec(), SNOW.spec(), SHIVER_OPT.spec()],
};

/// Dark to light: `c[7]` is white.
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x151a3d),
    Rgba::hex24(0x212f6b),
    Rgba::hex24(0x2c58aa),
    Rgba::hex24(0x3a8bd8),
    Rgba::hex24(0x62c8f4),
    Rgba::hex24(0xa6ecff),
    Rgba::hex24(0xdcfbff),
    Rgba::hex24(0xffffff),
];

impl Effect for Frozen {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(PALETTE[i]);
        let mut state = State {
            c: [c(0), c(1), c(2), c(3), c(4), c(5), c(6), c(7)],
            ferns: js_round(opts.num(&FERNS)) as usize,
            icicles: js_round(opts.num(&ICICLES)) as usize,
            snow: opts.num(&SNOW),
            shiver: opts.num(&SHIVER_OPT) >= 0.5,
            seed: f64::from(to_int32(seed as f64)),
            body: Vec::new(),
            fern: Vec::new(),
            icicle: Vec::new(),
            caps: Vec::new(),
            mouth: (0.0, 0.0),
            top: 0.0,
            bounds: (0.0, 0.0, 0.0, 0.0),
        };
        state.read(None);
        Box::new(state)
    }
}

/// A fern pixel: its cell and when it grows (0 at the root, 1 at the tip).
#[derive(Clone, Copy)]
struct FernPx {
    x: i32,
    y: i32,
    u: f64,
}

/// An icicle: its top cell, its full length, its own phase and its period in seconds.
#[derive(Clone, Copy)]
struct Icicle {
    x: i32,
    y: i32,
    len: f64,
    phase: f64,
    period: f64,
}

struct State {
    c: [Rgba; 8],
    ferns: usize,
    icicles: usize,
    snow: f64,
    shiver: bool,
    seed: f64,
    /// Every body cell chilled: the per-pose table.
    body: Vec<(i32, i32, Rgba)>,
    fern: Vec<FernPx>,
    icicle: Vec<Icicle>,
    /// Snow caps: an up cell and its depth (1 or 2 px).
    caps: Vec<(i32, i32, i32)>,
    /// Where the breath comes from.
    mouth: (f64, f64),
    /// The row the falling flakes stop at (2 under the crown's row).
    top: f64,
    /// The body box: x, y, w, h.
    bounds: (f64, f64, f64, f64),
}

/// pixelfx `travel`: how far something has moved after `age` seconds, launched at `v` under acceleration `g` with
/// linear drag `d`.
fn travel(v: f64, g: f64, d: f64, age: f64) -> f64 {
    fly(0.0, 0.0, 0.0, v, age, g, d).1
}

/// Frost on the floor round the feet: a white crust on row 0 out to `reach`, a pale bloom under it.
fn frost(p: &mut Painter, c: &[Rgba; 8], t: f64, reach: f64, seed: f64) {
    let mut x = -libm::ceil(reach) - 1.0;
    while x <= reach + 1.0 {
        let q = libm::fabs(x + 0.5) / reach;
        let h = rand(&[seed, K_FROST, x, 17.0]);
        let k = 1.0 - q * q;
        if q <= 1.0 + h * 0.15 {
            let tw = libm::sin((t / D) * TAU * 2.0 + rand(&[seed, K_FROST, x, 3.0]) * TAU) > 0.94;
            let col = if tw || h < 0.3 {
                c[7]
            } else if q > 0.85 {
                c[5]
            } else {
                c[6]
            };
            p.px(x, 0.0, col);
            if h < 0.3 * k {
                p.px(x, -1.0, c[6]);
            }
            let rows = 1.0 + js_round(2.0 * k);
            let mut j = 1.0;
            while j <= rows {
                p.dot(
                    x,
                    j,
                    if j == 1.0 { c[5] } else { c[4] },
                    (1.05 - j / 4.0) * 0.85,
                );
                j += 1.0;
            }
        }
        x += 1.0;
    }
}

impl State {
    /// The source's `plan(a, seed)` and its chilled body, from the body of this pose (or the stand-in).
    fn read(&mut self, actor: Option<&Actor>) {
        let body = Body::read(actor, BOX);
        let b = body.bounds;
        let (bw, bh) = (f64::from(b.w), f64::from(b.h));
        let (bx, by) = (b.x, b.y);
        let sil = &body.sil;
        let c = &self.c;
        let mut rng = Rng::new(to_uint32(self.seed * 11.0 + 1.0));
        // the body, chilled: its colours pulled toward ice; the rim frosted and lit
        self.body = body
            .cells
            .iter()
            .map(|cell| {
                let ice = c[clamp(1.0 + js_round(cell.l * 5.6), 1.0, 7.0) as usize];
                let (x, y) = (cell.x, cell.y);
                let open_left = !sil.covers(x - 1, y);
                let rim = open_left || !sil.covers(x, y - 1) || !sil.covers(x + 1, y);
                let col = if rim && cell.l < 0.3 {
                    if f64::from(y - by) < bh * 0.5 || open_left {
                        c[5]
                    } else {
                        c[3]
                    }
                } else {
                    mix(cell.c, ice, 0.38)
                };
                (x, y, col)
            })
            .collect();
        // ferns: a stem walking in from a side cell between 20 % and 85 % height with fronds branching off it at
        // alternate sides, each pixel ordered by when it grows; reach about `size(head, 1.4)` = 10
        let mut roots: Vec<(i32, i32, f64)> = Vec::new();
        for (x, y) in sil.cells() {
            let my = f64::from(y - by);
            if my <= bh * 0.2 || my >= bh * 0.85 {
                continue;
            }
            if !sil.covers(x - 1, y) {
                roots.push((x, y, -1.0));
            }
            if !sil.covers(x + 1, y) {
                roots.push((x, y, 1.0));
            }
        }
        for i in (1..roots.len()).rev() {
            let j = libm::floor(rng.next() * (i as f64 + 1.0)) as usize;
            roots.swap(i, j);
        }
        roots.truncate(self.ferns);
        let mut fern = Vec::new();
        for &(x0, y0, dir) in &roots {
            let (mut x, mut y) = (f64::from(x0), f64::from(y0));
            let mut ang = if dir < 0.0 {
                -0.5
            } else {
                core::f64::consts::PI + 0.5
            };
            ang += rng.range(-0.4, 0.4);
            for s in 0..9 {
                x += libm::cos(ang) * 1.1;
                y += libm::sin(ang) * 1.1;
                let (px, py) = (js_round(x) as i32, js_round(y) as i32);
                if !sil.covers(px, py) {
                    break;
                }
                let su = f64::from(s);
                fern.push(FernPx {
                    x: px,
                    y: py,
                    u: su / 9.0,
                });
                if s % 2 == 1 {
                    for side in [-1.0, 1.0] {
                        let ba = ang + side * 1.05;
                        let l = 1.0 + js_round(rng.range(0.0, 2.0) * (1.0 - su / 9.0));
                        let mut k = 1.0;
                        while k <= l {
                            let (fx, fy) = (
                                js_round(x + libm::cos(ba) * k) as i32,
                                js_round(y + libm::sin(ba) * k) as i32,
                            );
                            if sil.covers(fx, fy) {
                                fern.push(FernPx {
                                    x: fx,
                                    y: fy,
                                    u: (su + k * 0.6) / 9.0,
                                });
                            }
                            k += 1.0;
                        }
                    }
                }
                ang += rng.range(-0.25, 0.25);
            }
        }
        self.fern = fern;
        // icicles under the hair tips, the jaw and the hands: bottom edges with clear space under them, one per
        // region; `size(head, 0.5..0.8)` = 4..6 long
        let bottom = by + b.h as i32;
        let clear = |x: i32, y: i32| -> i32 {
            let mut n = 0;
            while n < 7 && y + 1 + n < bottom && !sil.covers(x, y + 1 + n) {
                n += 1;
            }
            n
        };
        let hangs: Vec<(i32, i32, i32)> = sil
            .open(0, 1)
            .filter(|&(x, y)| {
                let my = y - by;
                sil.covers(x, y - 1) && f64::from(my) < bh * 0.72 && my > 4
            })
            .map(|(x, y)| (x, y, clear(x, y)))
            .filter(|&(_, _, n)| n >= 4)
            .collect();
        let (hw, h36, h50) = (bw / 2.0, bh * 0.36, bh * 0.5);
        let my = |h: &(i32, i32, i32)| f64::from(h.1 - by);
        let mx = |h: &(i32, i32, i32)| f64::from(h.0 - bx);
        let leftmost = |f: &dyn Fn(&(i32, i32, i32)) -> bool| {
            hangs.iter().filter(|h| f(h)).min_by_key(|h| h.0).copied()
        };
        let rightmost = |f: &dyn Fn(&(i32, i32, i32)) -> bool| {
            hangs.iter().filter(|h| f(h)).min_by_key(|h| -h.0).copied()
        };
        let picks = [
            leftmost(&|h| my(h) < h36),
            rightmost(&|h| my(h) < h36),
            rightmost(&|h| my(h) >= h36 && my(h) < h50 && mx(h) < hw),
            leftmost(&|h| my(h) >= h36 && my(h) < h50 && mx(h) >= hw),
            leftmost(&|h| my(h) >= h50 && mx(h) < hw),
            rightmost(&|h| my(h) >= h50 && mx(h) >= hw),
        ];
        self.icicle = picks
            .iter()
            .flatten()
            .enumerate()
            .map(|(i, &(x, y, n))| Icicle {
                x,
                y: y + 1,
                len: js_min(f64::from(n - 1), rng.range(3.0, 5.5)),
                phase: rng.next(),
                period: if i % 2 == 1 { 1.0 } else { 2.0 },
            })
            .take(self.icicles)
            .collect();
        // snow ledges: the upward edges of the head and the shoulders, 1 px or 2
        self.caps = sil
            .open(0, -1)
            .filter(|&(_, y)| f64::from(y - by) < bh * 0.6)
            .map(|(x, y)| {
                let deep = rand(&[self.seed, K_SNOW, f64::from(x), 3.0]) < 0.45;
                (x, y, if deep { 2 } else { 1 })
            })
            .collect();
        // the mouth: 30 % down the body on its front edge (the scene mirrors a sprite facing the other way)
        let row = (js_round(bh * 0.3) as i32).clamp(0, (b.h as i32 - 1).max(0));
        let right = body.rows.get(row as usize).map_or(0, |r| r.1);
        self.mouth = (f64::from(right - 1), f64::from(by + row));
        self.top = f64::from(by);
        self.bounds = (f64::from(bx), f64::from(by), bw, bh);
    }

    /// Seconds since the last shiver and the overlay's jolt on the shiver's four frames.
    fn beat_at(&self, k: i32) -> (f64, f64) {
        let ks = k.rem_euclid(SHIVER);
        let ph = f64::from(ks) / f64::from(FPS);
        let dx = if self.shiver && ks < 4 {
            if ks % 2 == 1 { 1.0 } else { -1.0 }
        } else {
            0.0
        };
        (ph, dx)
    }

    fn draw_back(&self, p: &mut Painter, k: i32) {
        let t = f64::from(k) / f64::from(FPS);
        // cold light under the feet, `size(width, 1.2)` = 24 half-wide
        floor_light(p, 24.0, 4.0, self.c[4], 0.24);
        // the frost crust creeping out `size(width, 0.65)` = 13 and back by `size(hand, 1.5)` = 3
        let reach = 13.0 + 3.0 * libm::sin((t / D) * TAU);
        frost(p, &self.c, t, reach, self.seed);
    }

    fn draw_front(&self, p: &mut Painter, k: i32) {
        let t = f64::from(k) / f64::from(FPS);
        let c = &self.c;
        let (ph, dx) = self.beat_at(k);
        let (bx, by, bw, bh) = self.bounds;
        for &(x, y, col) in &self.body {
            p.paint_cell(f64::from(x), f64::from(y), col, 1.0);
        }
        // frost ferns creeping in from the outline; each shiver cracks most of it off and it creeps back
        let grow = if self.shiver {
            0.2 + 0.65 * smooth(0.05, 0.95, ph)
        } else {
            0.85
        };
        for f in &self.fern {
            if f.u < grow {
                let (x, y) = (f64::from(f.x), f64::from(f.y));
                let col = if f.u > grow - 0.1 {
                    WHITE
                } else if rand(&[self.seed, K_FERN, x, y, 5.0]) < 0.5 {
                    c[6]
                } else {
                    c[5]
                };
                p.px(x + dx, y, col);
            }
        }
        // snow settled on the head and shoulders
        for &(x, y, d) in &self.caps {
            let (x, y) = (f64::from(x), f64::from(y));
            p.px(x + dx, y - 1.0, c[7]);
            if d > 1 {
                let col = if rand(&[self.seed, K_SNOW, x, y, 9.0]) < 0.5 {
                    c[7]
                } else {
                    c[6]
                };
                p.px(x + dx, y - 2.0, col);
            }
        }
        // icicles: they grow, a drop gathers and falls, they shrink back; 2 wide with the shadow (`size(hand, 1.0)`)
        for ic in &self.icicle {
            let u = fract(t / ic.period + ic.phase);
            let l = ic.len * (0.55 + 0.45 * smooth(0.0, 0.7, u));
            let (x, y0) = (f64::from(ic.x) + dx, f64::from(ic.y));
            let mut j = 0.0;
            while j < l {
                let v = j / l;
                let col = if v > 0.75 {
                    WHITE
                } else if v < 0.3 {
                    c[7]
                } else {
                    c[6]
                };
                p.px(x, y0 + j, col);
                if v < 0.5 {
                    p.px(x + 1.0, y0 + j, c[4]);
                }
                j += 1.0;
            }
            if u > 0.7 && u < 0.86 {
                p.px(x, y0 + l, c[6]);
            } else if u >= 0.86 {
                let f = (u - 0.86) * ic.period;
                let y = y0 + l + 0.5 * 760.0 * f * f;
                if y < -1.0 {
                    p.px(x, y, c[6]);
                    p.px(x, y - 1.0, c[5]);
                }
            }
            p.glow(x, y0 + l * 0.6, 4.0, c[5], 0.18);
        }
        // breath steaming from the mouth between shivers, `size(eye.1, 1.6)` = 3 across at most
        let (mx0, my0) = self.mouth;
        for i in 0..4 {
            let fi = f64::from(i);
            let age = ph - 0.35 - fi * 0.05;
            if !(0.0..=0.65).contains(&age) {
                continue;
            }
            let u = age / 0.65;
            let x = mx0 + dx + 1.0 + travel(16.0, 0.0, 2.2, age) + fi;
            let y = my0 - travel(6.0, 3.0, 1.0, age) - fi * 0.5;
            let rad = 0.8 + 2.4 * libm::sqrt(u);
            p.disc(x, y, rad, c[7], 0.5 * (1.0 - u) * js_min(1.0, u * 8.0));
            p.disc(
                x - rad * 0.2,
                y - rad * 0.3,
                rad * 0.5,
                WHITE,
                0.4 * (1.0 - u),
            );
        }
        // snow falling from 13 rows above the crown and settling; flakes flung off by the shiver
        let top = self.top;
        cycle(
            t,
            D,
            js_round(9.0 * self.snow) as usize,
            self.seed + 2.0,
            |r| r.range(1.2, 1.8),
            |age, _, r, _| {
                let x = r.range(-14.0, 14.0) + libm::sin(age * 3.0 + r.next() * TAU) * 1.5;
                let y = top - 13.0 + age * 14.0;
                if y > top + 2.0 {
                    return;
                }
                let col = if r.next() < 0.5 { WHITE } else { c[6] };
                p.dot(x, y, col, js_min(1.0, age * 4.0));
            },
        );
        if self.shiver && ph < 0.5 {
            for i in 0..8 {
                let fi = f64::from(i);
                let side = if i % 2 == 1 { 1.0 } else { -1.0 };
                let vx = side * (20.0 + 30.0 * rand(&[self.seed, K_FLING, fi, 4.0]));
                let vy = -(20.0 + 30.0 * rand(&[self.seed, K_FLING, fi, 5.0]));
                let x = side * (6.0 + 6.0 * rand(&[self.seed, K_FLING, fi, 6.0])) + vx * ph;
                let y = by
                    + 3.0 * rand(&[self.seed, K_FLING, fi, 7.0])
                    + vy * ph
                    + 0.5 * 300.0 * ph * ph;
                p.dot(x, y, if i % 3 != 0 { c[6] } else { WHITE }, 1.2 - ph * 2.0);
            }
        }
        if self.shiver && ph < 0.25 {
            let arm = js_round(3.0 * (1.0 - ph / 0.25));
            sparkle(
                p,
                bx + js_round(bw * 0.3) + dx,
                by + 4.0,
                arm,
                c[6],
                c[5],
                0.4,
            );
        }
        // cold light: off the body, lifting with each shiver
        let surge = if self.shiver {
            0.12 * libm::exp(-ph / 0.2)
        } else {
            0.0
        };
        p.glow(0.0, by + bh * 0.45, 30.0, c[4], 0.2 + surge);
        p.glow(0.0, by + 2.0, 16.0, c[5], 0.1);
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.read(actor.as_deref());
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let k = (js_round(t * f64::from(FPS)) as i32).rem_euclid(N);
        match layer {
            Layer::Back => self.draw_back(p, k),
            Layer::Front => self.draw_front(p, k),
            Layer::Both => {
                self.draw_back(p, k);
                self.draw_front(p, k);
            }
        }
    }
}
