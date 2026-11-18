//! `feedback/poisoned`: the wrapped sprite is sick to the bone. Green rolls up its body in waves, dark veins creep
//! out of a wound on the forearm and draw back, blisters swell on the skin and pop, toxic sweat beads at the brow
//! and hands, falls and hisses on the floor, and a ring pulses out across the floor on every tick.
//!
//! Adapted from Pixel VFX Studio's `status/poisoned` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026
//! Hugo Duprez). Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it
//! reads the [`Body`] once per pose to place the wound, walk the veins, pick the blisters and the sweat sources
//! and bake the sick colour per cell; with no sprite it uses the stand-in figure. The sprite never sways: its
//! motion is the green wave.

use std::sync::Arc;

use pixfx_core::{
    Actor, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette, band,
};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::{Rng, TAU, clamp, js_max, js_min, js_round, smooth, to_int32, to_uint32};

use super::status::{BOX, Body, floor_light};

pub struct Poisoned;

pub static POISONED: Poisoned = Poisoned;

const FPS: u32 = 16;
const N: i32 = 24;
/// Seconds per loop.
const D: f64 = 1.5;
/// Frames between ticks: two a loop.
const TICK: i32 = 12;
/// Gravity of a falling drop, px/s^2.
const G: f64 = 760.0;

const VEINS: NumOpt = NumOpt::new("veins", 4.0).within(0.0, 6.0);
const BLISTERS: NumOpt = NumOpt::new("blisters", 3.0).within(0.0, 3.0);
const SWEAT: NumOpt = NumOpt::new("sweat", 3.0).within(0.0, 4.0);
const RING: NumOpt = NumOpt::new("ring", 1.0).within(0.0, 1.0);
const VEINS_COLOR: ColorOpt = ColorOpt::new("veins_color", Rgba::hex24(0x2c1a3a));

const META: Meta = Meta {
    id: "feedback/poisoned",
    name: "Poisoned",
    desc: "Green rolls up the wrapped sprite's body in waves, dark veins creep out of a wound on its forearm and draw back, blisters swell and pop on every tick, toxic sweat beads at the brow and hands, falls and hisses on the floor, and a ring pulses out across the floor.",
    tags: &[
        "feedback",
        "poison",
        "toxic",
        "sick",
        "dot",
        "status",
        "character",
        "loop",
    ],
    size: (64, 64),
    origin: (32, 55),
    duration: D,
    looping: true,
    fps: FPS,
    seed: 1_932_880_477,
    state: None,
    options: &[
        VEINS.spec(),
        BLISTERS.spec(),
        SWEAT.spec(),
        RING.spec(),
        VEINS_COLOR.spec(),
    ],
};

/// Dark to light: `c[7]` the palest.
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x0e1d14),
    Rgba::hex24(0x173320),
    Rgba::hex24(0x23552e),
    Rgba::hex24(0x318232),
    Rgba::hex24(0x55b92f),
    Rgba::hex24(0x8eea3f),
    Rgba::hex24(0xcfff63),
    Rgba::hex24(0xf6ffc2),
];

impl Effect for Poisoned {
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
            veins: js_round(opts.num(&VEINS)) as usize,
            blisters: js_round(opts.num(&BLISTERS)) as usize,
            sweat: js_round(opts.num(&SWEAT)) as usize,
            ring: opts.num(&RING) >= 0.5,
            vein: opts.color(&VEINS_COLOR),
            seed: f64::from(to_int32(seed as f64)),
            body: Vec::new(),
            vein_px: Vec::new(),
            wound: (0, 0),
            blister: [Vec::new(), Vec::new()],
            sweat_at: Vec::new(),
            bounds: (0.0, 0.0),
        };
        state.read(None);
        Box::new(state)
    }
}

/// A body cell as the wave tints it: its cell, its own colour, its row within the body (0..1) and its sick step.
#[derive(Clone, Copy)]
struct Skin {
    x: i32,
    y: i32,
    c: Rgba,
    l: f64,
    row: f64,
    sick: Rgba,
}

/// A vein pixel: its cell and when it grows (0 at the wound, 1 at the tip).
#[derive(Clone, Copy)]
struct VeinPx {
    x: i32,
    y: i32,
    u: f64,
}

/// A blister: its cell and its full radius.
#[derive(Clone, Copy)]
struct Blister {
    x: f64,
    y: f64,
    r: f64,
}

struct State {
    c: [Rgba; 8],
    veins: usize,
    blisters: usize,
    sweat: usize,
    ring: bool,
    vein: Rgba,
    seed: f64,
    body: Vec<Skin>,
    vein_px: Vec<VeinPx>,
    wound: (i32, i32),
    /// Three blisters per tick, two alternating sets.
    blister: [Vec<Blister>; 2],
    /// Where sweat beads: the brow and the hands.
    sweat_at: Vec<(f64, f64)>,
    /// The body box's top and height.
    bounds: (f64, f64),
}

/// The source's `walk`: a vein wandering `len` steps from (`x`, `y`) along `ang`, forking at most twice; its pixels
/// go to `out` keyed by when they grow, a fork's before its parent's.
#[expect(clippy::too_many_arguments, reason = "the source's recursion")]
fn walk(
    sil: &Silhouette,
    rng: &mut Rng,
    mut x: f64,
    mut y: f64,
    mut ang: f64,
    len: f64,
    depth: u32,
    out: &mut Vec<VeinPx>,
) {
    let mut pts = Vec::new();
    let mut s = 0.0;
    while s < len {
        ang += rng.range(-0.5, 0.5);
        x += libm::cos(ang);
        y += libm::sin(ang);
        let (px, py) = (js_round(x) as i32, js_round(y) as i32);
        if !sil.covers(px, py) {
            break;
        }
        pts.push(VeinPx {
            x: px,
            y: py,
            u: s / len,
        });
        if depth < 2 && s > 2.0 && rng.chance(0.16) {
            let fork = ang + rng.sign() * rng.range(0.6, 1.1);
            walk(sil, rng, x, y, fork, len * 0.55, depth + 1, out);
        }
        s += 1.0;
    }
    out.extend(pts);
}

impl State {
    /// The source's `body(a, seed)` and the sick colour per cell, from the body of this pose (or the stand-in).
    fn read(&mut self, actor: Option<&Actor>) {
        let body = Body::read(actor, BOX);
        let b = body.bounds;
        let bh = f64::from(b.h);
        let by = b.y;
        let sil = &body.sil;
        let c = &self.c;
        let mut rng = Rng::new(to_uint32(self.seed * 7.0 + 3.0));
        let last = (b.h as i32 - 1).max(0);
        let row_at = |q: f64| (js_round(bh * q) as i32).clamp(0, last);
        // the wound: one in from the right edge of the row at 60 % height
        let wrow = row_at(0.6);
        let (l, r) = body.rows.get(wrow as usize).copied().unwrap_or((0, -1));
        let (wx, wy) = (r - 1, by + wrow);
        self.wound = (wx, wy);
        // veins: random walks out of the wound, `size(head, 1.6)` = 11 px of reach
        let mut vein_px = Vec::new();
        for i in 0..self.veins {
            let ang = core::f64::consts::PI + (i as f64 / 4.0 - 0.45) * 2.2;
            walk(
                sil,
                &mut rng,
                f64::from(wx),
                f64::from(wy),
                ang,
                11.0,
                0,
                &mut vein_px,
            );
        }
        self.vein_px = vein_px;
        // blisters: on interior skin cells, `size(eye.0, 1..2)` across
        let interior: Vec<(i32, i32)> = sil
            .cells()
            .filter(|&(x, y)| {
                y - by > 4 && sil.covers(x - 1, y) && sil.covers(x + 1, y) && sil.covers(x, y - 1)
            })
            .collect();
        for set in &mut self.blister {
            *set = (0..3)
                .map(|_| {
                    let pick = libm::floor(rng.next() * interior.len() as f64) as usize;
                    let (x, y) = interior.get(pick).copied().unwrap_or((wx, wy));
                    Blister {
                        x: f64::from(x),
                        y: f64::from(y),
                        r: rng.range(1.1, 1.8),
                    }
                })
                .take(self.blisters)
                .collect();
        }
        // sweat: the brow (25 % down the body on its front edge) and the two hands
        let mut sweat = Vec::new();
        let brow = row_at(0.25);
        let brow_r = body.rows.get(brow as usize).map_or(0, |r| r.1);
        sweat.push((brow_r - 1, by + brow));
        for side in [-1, 1] {
            let hand = sil.cells().find(|&(x, y)| {
                let my = f64::from(y - by);
                my > bh * 0.5
                    && my < bh * 0.7
                    && (if side < 0 { x <= l + 1 } else { x >= r - 1 })
                    && !sil.covers(x, y + 1)
            });
            if let Some(h) = hand {
                sweat.push(h);
            }
        }
        sweat.truncate(self.sweat);
        self.sweat_at = sweat
            .iter()
            .map(|&(x, y)| (f64::from(x) + 0.5, f64::from(y)))
            .collect();
        // the body's own colours sickened: the green step each cell's brightness maps to
        self.body = body
            .cells
            .iter()
            .map(|cell| Skin {
                x: cell.x,
                y: cell.y,
                c: cell.c,
                l: cell.l,
                row: f64::from(cell.y - by) / js_max(1.0, bh),
                sick: c[clamp(1.0 + js_round(cell.l * 6.2), 1.0, 7.0) as usize],
            })
            .collect();
        self.bounds = (f64::from(by), bh);
    }

    /// Seconds since the last tick.
    fn phase(k: i32) -> f64 {
        f64::from(k.rem_euclid(TICK)) / f64::from(FPS)
    }

    /// The floor ring pulsing out on the tick: `size(width, 0.25)` + `size(width, 0.9)` = 23 px wide at most.
    fn ring(&self, p: &mut Painter, ph: f64, keep: impl Fn(f64) -> bool) {
        let kr = ph / 0.55;
        if !self.ring || kr >= 1.0 {
            return;
        }
        let c = &self.c;
        band(
            p,
            0.0,
            0.0,
            5.0 + 18.0 * libm::sqrt(kr),
            0.18,
            js_max(1.0, 2.5 * (1.0 - kr)),
            1.0 - kr,
            &[c[3], c[4], c[5], c[6]],
            keep,
            false,
        );
    }

    fn draw_back(&self, p: &mut Painter, k: i32) {
        let ph = Self::phase(k);
        self.ring(p, ph, |g| libm::sin(g) < 0.0);
        // the floor glows sick, surging on the tick; `size(width, 1.2)` = 24 half-wide
        floor_light(p, 24.0, 4.0, self.c[4], 0.14 + 0.2 * libm::exp(-ph / 0.2));
    }

    fn draw_front(&self, p: &mut Painter, k: i32) {
        let t = f64::from(k) / f64::from(FPS);
        let c = &self.c;
        let ph = Self::phase(k);
        let tick = (k / TICK).rem_euclid(2) as usize;
        let (by, bh) = self.bounds;
        // the body, its own colours sickened by green that rolls up it in two waves a loop
        for s in &self.body {
            let wave = 0.5 + 0.5 * libm::sin(TAU * (s.row * 1.6 + (t / D) * 2.0));
            let kk = 0.22 + 0.4 * wave * wave;
            let green = if wave > 0.85 && s.l > 0.4 {
                mix(s.sick, c[6], 0.3)
            } else {
                s.sick
            };
            p.px(f64::from(s.x), f64::from(s.y), mix(s.c, green, kk));
        }
        // veins creeping out from the wound and drawing back, the newest in green
        let grow = smooth(0.0, D * 0.55, t) * (1.0 - smooth(D * 0.62, D, t)) * 1.15;
        for v in &self.vein_px {
            if v.u < grow {
                let col = if v.u > grow - 0.12 { c[2] } else { self.vein };
                p.px(f64::from(v.x), f64::from(v.y), col);
            }
        }
        let (wx, wy) = (f64::from(self.wound.0), f64::from(self.wound.1));
        p.px(wx, wy, c[6]);
        p.px(wx - 1.0, wy, c[3]);
        p.glow(wx, wy, 6.0, c[5], 0.3);
        // blisters: on the tick they swell up out of the skin, shine, and pop
        for bl in &self.blister[tick] {
            let (x, y) = (bl.x, bl.y);
            let grow2 = clamp(ph / 0.16, 0.0, 1.0);
            if ph <= 0.38 {
                let r = bl.r * grow2;
                p.circle(x, y, r + 0.6, c[3], 1.0);
                p.circle(x - 0.2, y - 0.2, r, c[5], 1.0);
                if r > 0.8 {
                    p.px(x - 0.5, y - 0.8, WHITE);
                }
                p.glow(x, y, 4.0, c[5], 0.3 * grow2);
            } else if ph < 0.56 {
                let kk = (ph - 0.38) / 0.18;
                for j in 0..6 {
                    let an = (f64::from(j) / 6.0) * TAU + bl.x;
                    let d = bl.r + 0.5 + kk * 3.5;
                    let col = if j % 2 == 1 { c[6] } else { c[5] };
                    p.dot(
                        x + libm::cos(an) * d,
                        y + libm::sin(an) * d - kk * 2.0,
                        col,
                        1.2 - kk,
                    );
                }
                p.glow(x, y, 6.0, c[6], 0.4 * (1.0 - kk));
            }
        }
        // toxic sweat: beads gather, run, fall and hiss on the floor
        for (i, &(x, y0)) in self.sweat_at.iter().enumerate() {
            let per = D / 2.0;
            let age = (((t - i as f64 * 0.37) % per) + per) % per;
            let gather = 0.35;
            if age < gather {
                let kk = age / gather;
                let run = js_round(kk * 2.0);
                p.px(x, y0 + run, if kk > 0.5 { c[6] } else { c[5] });
                if kk > 0.5 {
                    p.px(x, y0 + run - 1.0, c[4]);
                }
                continue;
            }
            let f = age - gather;
            let fall_h = -1.0 - (y0 + 2.0);
            let tf = libm::sqrt((2.0 * js_max(1.0, fall_h)) / G);
            if f < tf {
                let y = y0 + 2.0 + 0.5 * G * f * f;
                p.line(x, y - js_min(3.0, G * f * 0.012), x, y, c[4], 1.0);
                p.px(x, y, c[6]);
                continue;
            }
            let h = f - tf;
            if h > 0.4 {
                continue;
            }
            // the hiss: a green fleck spreads, a wisp of vapour curls up, sizzle sparks
            let kk = h / 0.4;
            let col = if kk < 0.4 { c[6] } else { c[4] };
            p.line(
                x - 1.0 - kk * 2.0,
                -1.0,
                x + 1.0 + kk * 2.0,
                -1.0,
                col,
                1.2 - kk,
            );
            for j in 0..3 {
                let fj = f64::from(j);
                p.dot(
                    x + (fj - 1.0) * (2.0 + 3.0 * kk),
                    -2.0 - f64::from(j % 2) - 5.0 * kk * (1.0 - kk),
                    c[6],
                    1.0 - kk,
                );
            }
            p.disc(
                x + libm::sin(kk * 6.0) * 1.5,
                -3.0 - kk * 6.0,
                1.0 + 1.5 * kk,
                c[5],
                0.35 * (1.0 - kk),
            );
            p.glow(x, -1.5, 5.0, c[5], 0.35 * (1.0 - kk));
        }
        // the near half of the floor ring
        self.ring(p, ph, |g| libm::sin(g) >= 0.0);
        // a sick light on the body, surging with the tick
        p.glow(
            0.0,
            by + bh * 0.45,
            26.0,
            c[4],
            0.14 + 0.2 * libm::exp(-ph / 0.18),
        );
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
