//! `feedback/petrified`: the wrapped sprite is turned to stone. Grey creeps up from the feet behind a crackling
//! white edge that spits grit, sets with a pale flash and a dust skirt at the feet, then holds as a statue for as
//! long as the ailment lasts: hairline cracks, a glint running along one, a pebble dropping off, a mote of dust.
//! The opening plays once ([`Effect::loop_from`]) and the held statue loops.
//!
//! Adapted from Pixel VFX Studio's `status/petrified` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026
//! Hugo Duprez); its creeping front, branching cracks and grit are adapted from Sprite Fusion's open-source ice
//! freeze (https://www.spritefusion.com/pixel-art-vfx/ice, MIT License, Copyright (c) 2026 Hugo Duprez). The
//! source's shatter is not here: a held ailment never breaks (`feedback/encase` keeps the crumble).
//! Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it reads the
//! [`Body`] once per pose to bake each cell's stone, walk the cracks and pick the pebble's ledge; with no sprite
//! it uses the stand-in figure.

use std::sync::Arc;

use pixfx_core::particles::fly;
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE};
use pixfx_draw::math::{
    Rng, TAU, clamp, hash, js_min, js_round, rand, smooth, tile_fbm, to_int32, to_uint32,
};
use pixfx_draw::shapes::{Lobe, LobesOpt, lobes};

use super::status::{BOX, Body, ORIGIN, SIZE};

pub struct Petrified;

pub static PETRIFIED: Petrified = Petrified;

const FPS: u32 = 16;
const N: i32 = 28;
/// Seconds per loop, the opening included.
const D: f64 = 1.75;
/// The frames of the opening: the creep over 0..7, the set at 8, the dust skirt over 8..11; the hold loops from 12.
const OPEN: i32 = 12;
/// The frame the stone sets on.
const SET: i32 = 8;
/// Frames the hold loops over.
const HOLD: i32 = N - OPEN;
/// Gravity of a dropping pebble, px/s^2.
const G: f64 = 760.0;
const K_GRIT: f64 = 51.0;
const K_EDGE: f64 = 52.0;
const K_PEBBLE: f64 = 53.0;
const K_MOTE: f64 = 54.0;
const K_SKIRT: f64 = 55.0;

const CRACKS: NumOpt = NumOpt::new("cracks", 7.0).within(0.0, 10.0);
const GRIT: NumOpt = NumOpt::new("grit", 1.0).within(0.0, 2.0);
const MOTTLE: NumOpt = NumOpt::new("mottle", 1.0).within(0.0, 1.5);
const CREEP: NumOpt = NumOpt::new("creep", 1.0).within(0.0, 1.0);

const META: Meta = Meta {
    id: "feedback/petrified",
    name: "Petrified",
    desc: "Grey creeps up the wrapped sprite from the feet behind a crackling white edge that spits grit, sets with a pale flash and a dust skirt, then holds as a statue: hairline cracks, a glint running along one, a pebble dropping off, a mote of dust. The creep plays once; the statue loops.",
    tags: &[
        "feedback",
        "stone",
        "petrify",
        "petrified",
        "status",
        "character",
        "loop",
    ],
    size: SIZE,
    origin: ORIGIN,
    duration: D,
    looping: true,
    fps: FPS,
    seed: 2_771_903_146,
    state: None,
    options: &[CRACKS.spec(), GRIT.spec(), MOTTLE.spec(), CREEP.spec()],
};

/// Eight greys, dark to light.
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x16141c),
    Rgba::hex24(0x2a2733),
    Rgba::hex24(0x433f4d),
    Rgba::hex24(0x5f5a6a),
    Rgba::hex24(0x7d7889),
    Rgba::hex24(0xa29dab),
    Rgba::hex24(0xcdc8d2),
    Rgba::hex24(0xf0edf3),
];

impl Effect for Petrified {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    /// The creep, the set and the dust skirt play once; the held statue loops.
    fn loop_from(&self) -> Option<f64> {
        Some(f64::from(OPEN) / f64::from(FPS))
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(PALETTE[i]);
        let mut state = State {
            c: [c(0), c(1), c(2), c(3), c(4), c(5), c(6), c(7)],
            cracks: js_round(opts.num(&CRACKS)) as usize,
            grit: opts.num(&GRIT),
            mottle: opts.num(&MOTTLE),
            creep: opts.num(&CREEP) >= 0.5,
            seed: f64::from(to_int32(seed as f64)),
            stone: Vec::new(),
            crack: Vec::new(),
            origin: (0.0, 0.0),
            ledge: None,
            bounds: (0.0, 0.0, 0.0, 0.0),
        };
        state.read(None);
        Box::new(state)
    }
}

/// A body cell turned to stone: its cell, its column within the body, how far up the body it sits (0 at the soles,
/// 1 at the crown) and its grey.
#[derive(Clone, Copy)]
struct Stone {
    x: i32,
    y: i32,
    mx: f64,
    up: f64,
    c: Rgba,
}

struct State {
    c: [Rgba; 8],
    cracks: usize,
    grit: f64,
    mottle: f64,
    creep: bool,
    seed: f64,
    stone: Vec<Stone>,
    /// The crack walks, each a list of points.
    crack: Vec<Vec<(f64, f64)>>,
    /// Where the cracks start, on the chest.
    origin: (f64, f64),
    /// The cell the pebble drops from.
    ledge: Option<(i32, i32)>,
    /// The body box: x, y, w, h.
    bounds: (f64, f64, f64, f64),
}

/// The source's crack `walk`: `len` steps from (`x`, `y`) along `ang`, forking now and then; at most 16 walks.
fn walk(
    sil: &Silhouette,
    rng: &mut Rng,
    mut x: f64,
    mut y: f64,
    mut ang: f64,
    len: f64,
    out: &mut Vec<Vec<(f64, f64)>>,
) {
    let mut pts = vec![(x, y)];
    let mut s = 0.0;
    while s < len {
        ang += rng.range(-0.55, 0.55);
        x += libm::cos(ang) * rng.range(1.4, 2.4);
        y += libm::sin(ang) * rng.range(1.4, 2.4);
        if !sil.covers(libm::floor(x) as i32, libm::floor(y) as i32) {
            break;
        }
        pts.push((x, y));
        if pts.len() > 3 && out.len() < 16 && rng.chance(0.14) {
            let fork = ang + rng.sign() * rng.range(0.6, 1.2);
            walk(sil, rng, x, y, fork, len / 2.0, out);
        }
        s += 1.0;
    }
    out.push(pts);
}

impl State {
    /// Each cell's stone, the cracks and the pebble's ledge, from the body of this pose (or the stand-in).
    fn read(&mut self, actor: Option<&Actor>) {
        let body = Body::read(actor, BOX);
        let b = body.bounds;
        let (bw, bh) = (f64::from(b.w), f64::from(b.h));
        let (bx, by) = (f64::from(b.x), f64::from(b.y));
        let sil = &body.sil;
        let c = &self.c;
        // each pixel's stone: its own shading walked down the grey ramp, grained, lit on the top left
        self.stone = body
            .cells
            .iter()
            .map(|cell| {
                let (x, y) = (cell.x, cell.y);
                let (fx, fy) = (f64::from(x), f64::from(y));
                let tone = clamp((cell.l - 0.06) / 0.78, 0.0, 1.0);
                let mottle =
                    tile_fbm(fx * 0.32, fy * 0.32, 0.0, 0.0, self.seed + 5.0, 2) * self.mottle;
                let mut i = 1.0 + js_round(tone * 4.6 + mottle * 1.1);
                if !sil.covers(x - 1, y) || !sil.covers(x, y - 1) {
                    i += 1.0;
                } else if !sil.covers(x + 1, y) || !sil.covers(x, y + 1) {
                    i -= 1.0;
                }
                Stone {
                    x,
                    y,
                    mx: fx - bx,
                    up: 1.0 - (fy + 0.5 - by) / bh.max(1.0),
                    c: c[clamp(i, 1.0, 6.0) as usize],
                }
            })
            .collect();
        // cracks: random walks through the statue from a point on its chest, `size(head, 2.0)` = 14 px of reach
        let (ox, oy) = (bx + js_round(bw * 0.55), by + js_round(bh * 0.42));
        let mut rng = Rng::new(to_uint32(self.seed));
        let mut crack = Vec::new();
        for i in 0..self.cracks {
            let ang = (i as f64 / 7.0) * TAU + rng.range(-0.3, 0.3);
            walk(sil, &mut rng, ox, oy, ang, 14.0, &mut crack);
        }
        self.crack = crack;
        self.origin = (ox, oy);
        // the ledge a pebble drops off: a cell with nothing below it
        let ledges: Vec<(i32, i32)> = sil.open(0, 1).collect();
        let mut pick = Rng::new(hash(&[self.seed, K_PEBBLE]));
        self.ledge = pick.pick(&ledges).copied();
        self.bounds = (bx, by, bw, bh);
    }

    /// How far up the stone has crept on frame `k`: 0 at the feet, past 1 over the head, stuttering as it climbs.
    fn level(&self, k: i32) -> f64 {
        if !self.creep || k >= SET {
            return 1.2;
        }
        let fk = f64::from(k);
        1.2 * smooth(0.0, f64::from(SET), fk) + 0.025 * libm::sin(fk / f64::from(SET) * TAU * 3.0)
    }

    /// The creeping front's height at column `mx` of the body on frame `k`: the level, wobbled by noise.
    fn front(&self, k: i32, mx: f64) -> f64 {
        self.level(k) + 0.06 * tile_fbm(mx * 0.4, 0.5, 0.0, 0.0, self.seed, 2)
    }

    /// The dust skirt of the set at age `age` (frames since the set): encase's lobes, far and near, capped at
    /// `size(width, 1.3)` = 26 px of reach.
    fn skirt(&self, age: i32) -> (Vec<Lobe>, Vec<Lobe>, f64) {
        let f_age = f64::from(age);
        let mut far = Vec::new();
        let mut near = Vec::new();
        for side in [-1.0, 1.0] {
            for j in 0..4 {
                let fj = f64::from(j);
                let mut rg = Rng::new(hash(&[self.seed, K_SKIRT, side, fj]));
                let is_near = (j + if side > 0.0 { 1 } else { 0 }) % 2 == 0;
                let spread = if age == 0 {
                    2.2
                } else {
                    3.6 + libm::sqrt(f_age) * 1.5
                };
                let reach = js_min(26.0, 4.0 + fj * spread + rg.range(-0.8, 0.8));
                let r = (5.4 - fj * 0.8 + rg.range(-0.6, 0.6))
                    * (if age == 0 { 0.75 } else { 1.0 })
                    * (if is_near { 1.0 } else { 0.85 });
                let rise = f_age * rg.range(0.45, 0.95) * (1.0 - fj * 0.15);
                let lobe = Lobe {
                    x: side * reach,
                    y: (if is_near { 0.5 } else { -1.5 }) - r * 0.75 - rise,
                    r,
                };
                if is_near {
                    near.push(lobe);
                } else {
                    far.push(lobe);
                }
            }
        }
        let hollow = [0.0, 0.0, 0.0, 0.74][age.clamp(0, 3) as usize];
        (far, near, hollow)
    }

    /// The body cells on the creeping front of frame `k`, where grit is born.
    fn front_cells(&self, k: i32) -> Vec<&Stone> {
        self.stone
            .iter()
            .filter(|s| libm::fabs(s.up - self.front(k, s.mx)) <= 0.04)
            .collect()
    }

    /// Grit, dust and motes spat off the front: born on frames 0..7, each flying 2..4 frames.
    fn draw_grit(&self, p: &mut Painter, k: i32) {
        let c = &self.c;
        let (bx, _, bw, _) = self.bounds;
        let per_frame = js_round(3.0 * self.grit) as i32;
        for born in (k - 4).max(0)..=k.min(SET - 1) {
            let age = f64::from(k - born) / f64::from(FPS);
            let cells = self.front_cells(born);
            if cells.is_empty() {
                continue;
            }
            for j in 0..per_frame {
                let mut rg = Rng::new(hash(&[self.seed, K_GRIT, f64::from(j), f64::from(born)]));
                let life = rg.int(2.0, 4.0) as i32;
                let Some(&s) = rg.pick(&cells) else {
                    continue;
                };
                let side = if s.mx < bw / 2.0 { -1.0 } else { 1.0 };
                let (x0, y0) = (bx + s.mx + 0.5, f64::from(s.y));
                let (vx, vy) = (side * rg.range(15.0, 60.0), -rg.range(30.0, 90.0));
                let col = rg.pick(&[c[4], c[5], c[6]]).copied().unwrap_or(c[5]);
                let big = rg.chance(0.3);
                let dust = rg.chance(0.3);
                let mote = rg.chance(0.25);
                if k - born < life {
                    let (x, y) = fly(x0, y0, vx, vy, age, 400.0, 0.0);
                    if big {
                        p.rect(libm::floor(x), libm::floor(y), 2.0, 2.0, col, 1.0);
                    } else {
                        p.px(x, y, col);
                    }
                }
                if dust && k - born < life {
                    let (x, y) = fly(
                        x0 + side * 2.0,
                        y0,
                        side * rg.range(10.0, 30.0),
                        -rg.range(4.0, 14.0),
                        age,
                        0.0,
                        0.0,
                    );
                    p.px(x, y, c[4]);
                }
                if mote && k - born < life {
                    let (x, y) = fly(
                        x0,
                        y0,
                        side * rg.range(40.0, 90.0),
                        -rg.range(20.0, 70.0),
                        age,
                        0.0,
                        0.0,
                    );
                    p.px(x, y, c[7]);
                }
            }
        }
    }

    fn draw_back(&self, p: &mut Painter, k: i32) {
        if !self.creep || !(SET..OPEN).contains(&k) {
            return;
        }
        let c = &self.c;
        let age = k - SET;
        let (far, _, hollow) = self.skirt(age);
        let ramp = if age < 2 {
            [c[1], c[2], c[3]]
        } else {
            [c[1], c[2], c[2]]
        };
        lobes(
            p,
            &far,
            &ramp,
            LobesOpt {
                lx: -0.3,
                ly: -1.0,
                hollow,
                shadow: 0.35,
                seed: 3.0,
            },
        );
    }

    fn draw_front(&self, p: &mut Painter, k: i32) {
        let c = &self.c;
        let (bx, by, bw, bh) = self.bounds;
        let creeping = self.creep && k < SET;
        // the stone, behind its crackling edge while it creeps
        for s in &self.stone {
            let d = self.front(k, s.mx) - s.up;
            if d < 0.0 {
                continue;
            }
            let (x, y) = (f64::from(s.x), f64::from(s.y));
            if creeping && d < 0.035 {
                let hot = rand(&[self.seed, K_EDGE, x, y, f64::from(k)]) < 0.5;
                p.paint_cell(x, y, if hot { c[7] } else { c[6] }, 1.0);
            } else if creeping && d < 0.07 {
                p.paint_cell(x, y, c[5], 1.0);
            } else {
                p.paint_cell(x, y, s.c, 1.0);
            }
        }
        if creeping {
            let mut mx = 0.0;
            while mx < bw {
                let my = libm::floor((1.0 - self.front(k, mx)) * bh);
                let (x, y) = (bx + mx, by + my);
                if self
                    .stone
                    .iter()
                    .any(|s| f64::from(s.x) == x && f64::from(s.y) == y)
                {
                    p.glow(x, y, 5.0, c[6], 0.18);
                }
                mx += 2.0;
            }
            self.draw_grit(p, k);
            return;
        }
        if self.creep && k < OPEN {
            // the set: a pale flash, and the near dust lobes rolling out at the feet
            if k <= SET + 1 {
                p.glow(
                    0.0,
                    by + bh * 0.5,
                    34.0,
                    c[6],
                    if k == SET { 0.35 } else { 0.175 },
                );
            }
            let age = k - SET;
            let (_, near, hollow) = self.skirt(age);
            let ramp = if age < 2 {
                [c[2], c[3], c[4]]
            } else {
                [c[2], c[3], c[3]]
            };
            lobes(
                p,
                &near,
                &ramp,
                LobesOpt {
                    lx: -0.3,
                    ly: -1.0,
                    hollow,
                    shadow: 0.35,
                    seed: 9.0,
                },
            );
            return;
        }
        // the held statue: its cracks, a glint, a pebble, a mote; `kh` is the frame within the hold
        let kh = (k - OPEN).rem_euclid(HOLD);
        let glint = (6..8).contains(&kh);
        for (i, pts) in self.crack.iter().enumerate() {
            let n = pts.len();
            for (j, &(x, y)) in pts.iter().enumerate() {
                let (px, py) = (libm::floor(x), libm::floor(y));
                if glint && i == 0 && j + 1 == n {
                    p.px(px, py, WHITE);
                } else {
                    p.px(px, py, c[0]);
                    p.px(px, py - 1.0, c[6]);
                }
            }
        }
        if glint {
            let (ox, oy) = self.origin;
            p.glow(ox, oy, 12.0, c[6], if kh == 6 { 0.25 } else { 0.125 });
        }
        if let Some((lx, ly)) = self.ledge
            && (8..14).contains(&kh)
        {
            let mut rg = Rng::new(hash(&[self.seed, K_PEBBLE, 1.0]));
            let vx = rg.range(-8.0, 8.0);
            let (x0, y0) = (f64::from(lx) + 0.5, f64::from(ly) + 1.0);
            // the pebble rests on the floor for one frame after it lands
            let landed = (8..kh).find(|&f| {
                fly(x0, y0, vx, 0.0, f64::from(f - 8) / f64::from(FPS), G, 0.0).1 >= -1.0
            });
            if landed.is_none_or(|f| kh <= f + 1) {
                let (x, y) = fly(x0, y0, vx, 0.0, f64::from(kh - 8) / f64::from(FPS), G, 0.0);
                p.px(x, js_min(y, -1.0), c[3]);
            }
        }
        if (10..16).contains(&kh) {
            let mut rg = Rng::new(hash(&[self.seed, K_MOTE]));
            let age = f64::from(kh - 10) / f64::from(FPS);
            let (x, y) = fly(
                rg.range(-6.0, 6.0),
                -1.0,
                rg.sign() * rg.range(6.0, 14.0),
                -rg.range(12.0, 24.0),
                age,
                0.0,
                0.0,
            );
            p.dot(x, y, c[4], 1.0 - f64::from(kh - 10) / 6.0);
        }
        // the cold light on the statue, breathing over the hold
        p.glow(
            0.0,
            by + bh * 0.45,
            30.0,
            c[5],
            0.12 + 0.06 * libm::sin(f64::from(kh) / f64::from(HOLD) * TAU),
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
