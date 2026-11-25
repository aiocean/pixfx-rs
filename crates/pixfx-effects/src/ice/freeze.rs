//! Freeze: port of pixelfx `src/fx/ice/freeze.js`.
//!
//! It wraps the sprite it plays on ([`Effect::wraps_actor`]): frost climbs that sprite's pixels and turns them to
//! ice. With no sprite handed it freezes pixelfx's stand-in, the 24 x 38 dummy (`stockActor(2)`), so its frames
//! match pixelfx's. A sprite handed before the first step rebuilds everything from the seed, as pixelfx's
//! `create` with that actor would; one handed later (a new pose) rebuilds only the frost's climb over it, from a
//! generator of its own, so the rest of the effect plays on unchanged.
//!
//! pixelfx acts it out on the character (`act`: poses, the hits); that staging is dropped here. Beyond pixelfx:
//! the frost sealing round it and the burst fire impact cues ([`pixfx_core::cue::impact`]) at pixelfx's hit
//! times; the `feel` option scales them, 0 for none.
//!
//! The crack-and-burst recipe (cracks walking out from a point, a white frame, the ice flung apart as shards with
//! ice dust and glints) is adapted from Sprite Fusion's open-source ice
//! (https://www.spritefusion.com/pixel-art-vfx/ice), MIT License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::PI;
use std::sync::Arc;

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{BitKind, Bits, Kind, Lights, Particles, ParticlesOpts, flare, sparkle};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{
    Rng, TAU, clamp, ease, js_max, js_min, js_round, js_sign, rand, rng, seg, smooth, to_uint32,
};

use super::lib::{
    Crack, FractureOpts, PALETTE, Pane, PaneEvent, Panes, Pt, bit, centroid, draw_fractures, emit,
    end_by, fractures, ice_of, in_poly, split_poly, step,
};

pub struct Freeze;

pub static FREEZE: Freeze = Freeze;

const META: Meta = Meta {
    id: "ice/freeze",
    name: "Freeze",
    desc: "Frost races in and climbs the character, turning it to ice; a faceted block crystallizes round it pane by pane, glints, cracks and bursts, and the character thaws free.",
    tags: &[
        "ice",
        "freeze",
        "frozen",
        "status",
        "crowd-control",
        "one-shot",
    ],
    size: (176, 128),
    origin: (88, 108),
    duration: D,
    looping: false,
    fps: 30,
    seed: 5,
    state: None,
    options: &[FEEL.spec()],
};

const D: f64 = 4.0;
/// Frost lines race in along the floor to the feet.
const SNAP: f64 = 0.16;
/// And climb the character.
const CLIMB: (f64, f64) = (0.14, 0.62);
/// The block crystallizes, pane by pane.
const SEAL: f64 = 0.6;
/// Light slides across the faces while it holds.
const SWEEPS: [f64; 2] = [1.12, 1.82];
const CRACK: f64 = 2.2;
const CRACK2: f64 = 2.34;
const FLASH: f64 = 2.46;
const BURST: f64 = 2.52;
const THAW: f64 = 0.36;
/// The floor frost goes.
const MELT: (f64, f64) = (2.95, 3.7);
const REACH: f64 = 38.0;
const CORE: Pt = (0.0, -21.0);

// the block: a crown of three crystal points over straight sides, eleven corners round the character and eight
// inside, cut into thirteen panes
const L0: usize = 0;
const L1: usize = 1;
const L2: usize = 2;
const P1: usize = 3;
const N1: usize = 4;
const P2: usize = 5;
const N2: usize = 6;
const P3: usize = 7;
const R2: usize = 8;
const R1: usize = 9;
const R0: usize = 10;
const I0: usize = 11;
const I1: usize = 12;
const I2: usize = 13;
const I3: usize = 14;
const I4: usize = 15;
const I5: usize = 16;
const I6: usize = 17;
const I7: usize = 18;
/// The block's corners, in pixelfx's order (`Object.values(V)`).
const V: [Pt; 19] = [
    (-15.0, 0.0),
    (-17.0, -10.0),
    (-17.0, -31.0),
    (-12.0, -45.0),
    (-6.0, -42.0),
    (0.0, -52.0),
    (6.0, -43.0),
    (12.0, -47.0),
    (17.0, -30.0),
    (16.0, -9.0),
    (14.0, 0.0),
    (-6.0, -36.0),
    (6.0, -35.0),
    (-9.0, -21.0),
    (3.0, -25.0),
    (9.0, -13.0),
    (-5.0, -8.0),
    (2.0, 0.0),
    (0.0, -40.0),
];
/// Each pane: its corners, how much light it catches (0 shadow .. 1 lit) and when it crystallizes after SEAL
/// (from the floor up).
const FACETS: [(&[usize], f64, f64); 13] = [
    (&[L0, L1, I5, I6], 0.62, 0.0),
    (&[I5, I4, I6], 0.42, 0.03),
    (&[I4, R2, R1, R0, I6], 0.2, 0.05),
    (&[L1, L2, I2, I5], 0.74, 0.08),
    (&[I2, I3, I4, I5], 0.5, 0.1),
    (&[I1, R2, I4, I3], 0.28, 0.13),
    (&[L2, I0, I2], 0.7, 0.15),
    (&[I0, I1, I3, I2], 0.58, 0.18),
    (&[N1, I7, N2, I1, I0], 0.52, 0.2),
    (&[I1, N2, P3, R2], 0.4, 0.23),
    (&[L2, P1, N1, I0], 0.88, 0.25),
    (&[P2, N2, I7], 0.66, 0.28),
    (&[N1, P2, I7], 1.0, 0.31),
];
const OUTLINE: [usize; 11] = [L0, L1, L2, P1, N1, P2, N2, P3, R2, R1, R0];
const X0: i32 = -18;
const X1: i32 = 18;
const Y0: i32 = -53;
const Y1: i32 = 0;
const BW: i32 = X1 - X0;

// pixelfx's stand-in, the dummy (`stockActor(2)`): a neutral shell that lets any effect's colours do the talking
const DUMMY: [&str; 38] = [
    "...........kkk..........",
    "..........kAABk.........",
    ".........kAAABJk........",
    ".........kABBJJk........",
    ".........kBBBJCDk.......",
    ".........kBCJDDDk.......",
    ".........kkJJJDk........",
    ".......kkkkABBJCkk......",
    "......kAABCJBCCCkDk.....",
    "......kABBCJCCCmDkDk....",
    "......kBBCDDJCDDDkDk....",
    "......kABCDJDDDDDkk.....",
    ".....kAABDkDDDDJkk......",
    ".....kABCDkDDJJGkk......",
    ".....kABCkJJJGGGkk......",
    "....kJJCDkFFFGGGk.......",
    "....kAJJDkJJJGGGk.......",
    "....kJCJJkABBJGGk.......",
    ".....kJCCkBBCDJJJk......",
    ".....kCCCJBBCDJJDk......",
    ".....kCCCDJCCDkJDDk.....",
    "......kCDDJBCDkDDDk.....",
    ".......kDJJJCDkDDDkk....",
    ".......kJBCJCDkDDDkk....",
    ".......kBDJBCDkCDDkk....",
    "........kJJBCDkCDDk.....",
    "........kBBBCDkCCDDk....",
    "........kBBBCDkCCDDk....",
    ".........kBBCDkJDDDk....",
    "........kBJJJJkCJJJk....",
    "........kBBCCDkCCDDk....",
    "........kBBCDDkCDDk.....",
    "........kBBCDkCCDDk.....",
    "........kBCCDkCCDDk.....",
    "........kBCDDkCDDDk.....",
    ".......kJBCJkkkDJkkkk...",
    ".......kAJJAAAAkCCCCCk..",
    "........kkkkkkkkkkkkk...",
];
const DUMMY_PAL: [(char, Rgba); 10] = [
    ('k', Rgba::hex24(0x0e0c14)),
    ('A', Rgba::hex24(0xe4e6ec)),
    ('B', Rgba::hex24(0xb6b9c3)),
    ('C', Rgba::hex24(0x868995)),
    ('D', Rgba::hex24(0x5c5e6a)),
    ('E', Rgba::hex24(0x6e707c)),
    ('F', Rgba::hex24(0x4b4d58)),
    ('G', Rgba::hex24(0x34353f)),
    ('J', Rgba::hex24(0x2a2b33)),
    ('m', Rgba::hex24(0xf53cd1)),
];

/// The character it freezes: its top-left in the effect's space, its size, which pixels it covers and their
/// colours.
struct Body {
    x: f64,
    y: f64,
    w: usize,
    h: usize,
    on: Vec<bool>,
    px: Vec<Rgba>,
}

impl Body {
    /// pixelfx's stand-in dummy, its feet at the origin.
    fn stand_in() -> Self {
        let (w, h) = (24, DUMMY.len());
        let mut on = Vec::with_capacity(w * h);
        let mut px = Vec::with_capacity(w * h);
        for row in DUMMY {
            for ch in row.chars() {
                let c = DUMMY_PAL
                    .iter()
                    .find(|(k, _)| *k == ch)
                    .map_or(Rgba::NONE, |(_, c)| *c);
                on.push(c != Rgba::NONE);
                px.push(c);
            }
        }
        Self {
            x: -12.0,
            y: -(h as f64),
            w,
            h,
            on,
            px,
        }
    }

    fn of(a: &Actor) -> Self {
        let (w, h) = (a.w() as usize, a.h() as usize);
        let px: Vec<Rgba> = a.pixels().iter().map(|&p| Rgba::unpack(p)).collect();
        Self {
            x: f64::from(a.x()),
            y: f64::from(a.y()),
            w,
            h,
            on: a.pixels().iter().map(|&p| p >> 24 != 0).collect(),
            px,
        }
    }

    fn on(&self, x: i64, y: i64) -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < self.w
            && (y as usize) < self.h
            && self
                .on
                .get(y as usize * self.w + x as usize)
                .copied()
                .unwrap_or(false)
    }
}

/// A tick of rime standing out from the outline once the frost has passed.
struct Feather {
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
    l: f64,
    at: f64,
}

/// The character frozen: each pixel's ice colour and when the frost reaches it, in the order it does, and the
/// rime feathers.
struct Frost {
    body: Body,
    ice: Vec<Rgba>,
    reach: Vec<f32>,
    order: Vec<usize>,
    feathers: Vec<Feather>,
}

impl Frost {
    /// A randomized flood up from the feet, quicker round the outline, so the front creeps up the edges first in
    /// ragged fingers.
    fn new(body: Body, c: &[Rgba; 8], r: &mut Rng, seed: f64) -> Self {
        let (w, h) = (body.w, body.h);
        let n = w * h;
        let on = |x: i64, y: i64| body.on(x, y);
        let mut dist = vec![f32::INFINITY; n];
        let mut ice = vec![Rgba::NONE; n];
        let mut rim = vec![false; n];
        let mut open: Vec<usize> = Vec::new();
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                if !on(x, y) {
                    continue;
                }
                let i = y as usize * w + x as usize;
                rim[i] = !on(x - 1, y) || !on(x + 1, y) || !on(x, y - 1);
                let up = !on(x - 1, y) || !on(x, y - 1);
                let k = ice_of(body.px.get(i).copied().unwrap_or(Rgba::NONE));
                let first = c.iter().position(|&q| q == c[k]).unwrap_or(k);
                let base = if up { c[(first + 1).min(7)] } else { c[k] };
                ice[i] = mix(base, c[6], 0.2);
                if (y == h as i64 - 1 || !on(x, y + 1)) && y >= h as i64 - 4 {
                    dist[i] = r.range(0.0, 1.5) as f32;
                    open.push(i);
                }
            }
        }
        let mut done = vec![false; n];
        while !open.is_empty() {
            let mut bi = 0;
            for j in 1..open.len() {
                if dist[open[j]] < dist[open[bi]] {
                    bi = j;
                }
            }
            let i = open.swap_remove(bi);
            if done[i] {
                continue;
            }
            done[i] = true;
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            for (dx, dy) in [(1, 0), (-1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (x + dx, y + dy);
                if !on(nx, ny) {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                let wt = (if rim[j] { 0.55 } else { 1.0 }) * (if dy < 0 { 1.0 } else { 1.3 })
                    + r.range(0.0, 1.5);
                let nd = f64::from(dist[i]) + wt;
                if nd < f64::from(dist[j]) {
                    dist[j] = nd as f32;
                    open.push(j);
                }
            }
        }
        let mut max_d = 0.0_f64;
        for &d in &dist {
            if d < f32::INFINITY {
                max_d = js_max(max_d, f64::from(d));
            }
        }
        let mut reach = vec![0.0_f32; n];
        let mut order = Vec::new();
        for i in 0..n {
            if dist[i] < f32::INFINITY {
                reach[i] = (CLIMB.0
                    + (CLIMB.1 - CLIMB.0) * libm::pow(f64::from(dist[i]) / max_d, 0.85))
                    as f32;
                order.push(i);
            }
        }
        order.sort_by(|&a, &b| {
            (f64::from(reach[a]) - f64::from(reach[b]))
                .partial_cmp(&0.0)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        // rime feathers: little ticks of frost standing out from the outline once the frost has passed
        let mut feathers = Vec::new();
        for &i in &order {
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            let fi = i as f64;
            if !rim[i] || rand(&[fi, seed, 3.0]) > 0.42 || y > h as i64 - 3 {
                continue;
            }
            let dx = if !on(x - 1, y) || (on(x + 1, y) && rand(&[fi, 9.0]) < 0.5) {
                -1.0
            } else {
                1.0
            };
            let dy = if !on(x, y - 1) || rand(&[fi, 4.0]) < 0.5 {
                -1.0
            } else {
                0.0
            };
            feathers.push(Feather {
                x: body.x + x as f64 + dx,
                y: body.y + y as f64 + dy,
                dx,
                dy,
                l: 1.0 + libm::floor(rand(&[fi, 7.0]) * 2.6),
                at: f64::from(reach[i]) + 0.04,
            });
        }
        Self {
            body,
            ice,
            reach,
            order,
            feathers,
        }
    }

    fn reach(&self, k: usize) -> f64 {
        self.order
            .get(k)
            .and_then(|&i| self.reach.get(i))
            .map_or(f64::INFINITY, |&r| f64::from(r))
    }
}

/// A pane of the block, worked out once: its corners, light, when it sets, centre, tint, alpha, where the light
/// sweep reaches it, its perimeter.
struct Facet {
    pts: Vec<Pt>,
    lit: f64,
    at: f64,
    c: Pt,
    tint: Rgba,
    alpha: f64,
    d: f64,
    len: f64,
}

/// An edge of the block, once: its ends, the panes it borders, whether it's on the outline, how it faces the
/// light, whether it lies on the floor, when it first shows.
struct Edge {
    a: Pt,
    b: Pt,
    fs: Vec<usize>,
    outer: bool,
    lit: f64,
    floor: bool,
    at: f64,
}

/// The block: its panes, edges, and which pane every pixel belongs to.
struct Block {
    facets: Vec<Facet>,
    edges: Vec<Edge>,
    fid: Vec<i8>,
}

fn in_block(x: f64, y: f64) -> bool {
    let pts: Vec<Pt> = OUTLINE.iter().map(|&k| V[k]).collect();
    in_poly(&pts, x, y)
}

impl Block {
    fn new(c: &[Rgba; 8]) -> Self {
        let outline: Vec<Pt> = OUTLINE.iter().map(|&k| V[k]).collect();
        let facets: Vec<Facet> = FACETS
            .iter()
            .map(|&(keys, lit, at)| {
                let pts: Vec<Pt> = keys.iter().map(|&k| V[k]).collect();
                let m = centroid(&pts);
                let mut len = 0.0;
                for (k, q) in pts.iter().enumerate() {
                    if let Some(n) = pts.get((k + 1) % pts.len()) {
                        len += libm::hypot(n.0 - q.0, n.1 - q.1);
                    }
                }
                Facet {
                    lit,
                    at: SEAL + at,
                    c: (m.x, m.y),
                    tint: step(c, 3.0 + js_round(lit * 3.0)),
                    alpha: 0.2 + 0.24 * lit,
                    d: clamp((m.x + 0.55 * m.y + 36.0) / 46.0, 0.0, 1.0),
                    len,
                    pts,
                }
            })
            .collect();
        // which pane every pixel belongs to
        let mut fid = vec![-1_i8; (BW * (Y1 - Y0)) as usize];
        for y in Y0..Y1 {
            for x in X0..X1 {
                let (fx, fy) = (f64::from(x), f64::from(y));
                if !in_poly(&outline, fx + 0.5, fy + 0.5) {
                    continue;
                }
                let f = match facets
                    .iter()
                    .position(|f| in_poly(&f.pts, fx + 0.5, fy + 0.5))
                {
                    Some(f) => f,
                    None => {
                        let dist = |f: &Facet| libm::hypot(f.c.0 - fx, f.c.1 - fy);
                        let mut b = 0;
                        for (k, f) in facets.iter().enumerate() {
                            if let Some(fb) = facets.get(b)
                                && dist(f) < dist(fb)
                            {
                                b = k;
                            }
                        }
                        b
                    }
                };
                if let Some(slot) = fid.get_mut(((y - Y0) * BW + (x - X0)) as usize) {
                    *slot = f as i8;
                }
            }
        }
        // every edge once: its ends, the panes it borders, whether it's on the outline and how it faces the light
        let mut keyed: Vec<(usize, usize, Vec<usize>)> = Vec::new();
        for (fi, &(keys, _, _)) in FACETS.iter().enumerate() {
            for (k, &a) in keys.iter().enumerate() {
                let b = keys[(k + 1) % keys.len()];
                match keyed
                    .iter_mut()
                    .find(|e| (e.0 == a && e.1 == b) || (e.0 == b && e.1 == a))
                {
                    Some(e) => e.2.push(fi),
                    None => keyed.push((a, b, vec![fi])),
                }
            }
        }
        let edges = keyed
            .into_iter()
            .map(|(a, b, fs)| {
                let ((x0, y0), (x1, y1)) = (V[a], V[b]);
                let cc = fs
                    .first()
                    .and_then(|&f| facets.get(f))
                    .map_or((0.0, 0.0), |f| f.c);
                let (mut nx, mut ny) = (y1 - y0, x0 - x1);
                if nx * ((x0 + x1) / 2.0 - cc.0) + ny * ((y0 + y1) / 2.0 - cc.1) < 0.0 {
                    nx = -nx;
                    ny = -ny;
                }
                let mut at = f64::INFINITY;
                for &f in &fs {
                    if let Some(f) = facets.get(f) {
                        at = js_min(at, f.at);
                    }
                }
                Edge {
                    a: V[a],
                    b: V[b],
                    outer: fs.len() == 1,
                    lit: (-nx * 0.55 - ny * 0.84) / libm::hypot(nx, ny),
                    floor: y0 == 0.0 && y1 == 0.0,
                    at,
                    fs,
                }
            })
            .collect();
        Self { facets, edges, fid }
    }

    fn at(&self, x: i32, y: i32) -> i8 {
        if !(X0..X1).contains(&x) || !(Y0..Y1).contains(&y) {
            return -1;
        }
        self.fid
            .get(((y - Y0) * BW + (x - X0)) as usize)
            .copied()
            .unwrap_or(-1)
    }
}

impl Effect for Freeze {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    /// Beyond pixelfx: the block sealing round it and the burst ask the game for an impact's answer (pixelfx's
    /// act hits these times).
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let mut cues = impact(0.62, 0.2, CORE.0, CORE.1, opts);
        cues.extend(impact(BURST, 0.45, CORE.0, CORE.1, opts));
        cues
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        Box::new(State::new(c, seed as f64, Body::stand_in()))
    }
}

struct State {
    c: [Rgba; 8],
    seed: f64,
    r: Rng,
    ps: Particles,
    bits: Bits,
    lights: Lights,
    panes: Panes,
    frost: Frost,
    block: Block,
    cracks: Vec<Crack>,
    cracks2: Vec<Crack>,
    t: f64,
    formed: usize,
    sealed: bool,
    struck: u8,
    burst: bool,
    white: bool,
}

impl State {
    fn new(c: [Rgba; 8], seed: f64, body: Body) -> Self {
        let mut r = Rng::new(to_uint32(seed));
        let frost = Frost::new(body, &c, &mut r, seed);
        let block = Block::new(&c);
        let inb = |x: f64, y: f64| in_block(x, y);
        let cracks = fractures(
            &mut r,
            &inb,
            CORE.0,
            CORE.1,
            3,
            FractureOpts {
                len: 9.0,
                a0: Some(-2.2),
                spread: 3.6,
                ..FractureOpts::default()
            },
        );
        let cracks2 = fractures(
            &mut r,
            &inb,
            CORE.0 + 1.0,
            CORE.1 - 2.0,
            6,
            FractureOpts {
                len: 16.0,
                fork: 0.22,
                ..FractureOpts::default()
            },
        );
        Self {
            c,
            seed,
            r,
            ps: Particles::new(ParticlesOpts {
                ground: 0.0,
                sparks: vec![c[7], c[6], c[5], c[4]],
                haze: Some(c[5]),
            }),
            bits: Bits::new(0.0),
            lights: Lights::new(),
            panes: Panes::new(0.0),
            frost,
            block,
            cracks,
            cracks2,
            t: 0.0,
            formed: 0,
            sealed: false,
            struck: 0,
            burst: false,
            white: false,
        }
    }

    fn jolt(&self) -> f64 {
        let t = self.t;
        if (CRACK..CRACK + 0.06).contains(&t) {
            1.0
        } else if (CRACK2..FLASH).contains(&t) {
            if libm::floor(t * 40.0) % 2.0 != 0.0 {
                1.0
            } else {
                -1.0
            }
        } else {
            0.0
        }
    }

    /// The frozen character: each pixel turns as the frost reaches it, white at the front; after the burst the
    /// ice runs off it from the top down.
    fn draw_character(&self, p: &mut Painter) {
        let (c, t, f) = (&self.c, self.t, &self.frost);
        let (w, h) = (f.body.w, f.body.h as f64);
        let thaw = if t >= BURST {
            ease::in_quad(seg(t, BURST, BURST + THAW)) * (h + 2.0) - 2.0
        } else {
            -1.0
        };
        for &i in &f.order {
            let age = t - f.reach.get(i).map_or(f64::INFINITY, |&r| f64::from(r));
            if age < 0.0 {
                break;
            }
            let (x, y) = ((i % w) as f64, (i / w) as f64);
            if y < thaw {
                continue;
            }
            let col = if y < thaw + 1.2 && t >= BURST {
                c[7]
            } else if age < 0.016 {
                WHITE
            } else if age < 0.045 {
                c[6]
            } else {
                f.ice.get(i).copied().unwrap_or(c[6])
            };
            p.px(f.body.x + x, f.body.y + y, col);
            if age < 0.03 {
                p.light(f.body.x + x, f.body.y + y, c[6], 0.3);
            }
        }
        if t >= BURST {
            return;
        }
        for ft in &f.feathers {
            let g = clamp((t - ft.at) / 0.08, 0.0, 1.0);
            if g <= 0.0 {
                continue;
            }
            let l = js_round(ft.l * g);
            let mut j = 0.0;
            while j < l {
                p.px(
                    ft.x + ft.dx * j,
                    ft.y + ft.dy * j,
                    if j == l - 1.0 { WHITE } else { c[6] },
                );
                j += 1.0;
            }
        }
    }

    /// The block: panes fill in as they crystallize (edges traced in white, a white flash, then glassy), a light
    /// sweep that breaks at every facet, cracks, the white frame before it bursts.
    fn draw_block(&self, p: &mut Painter) {
        let (c, t, b) = (&self.c, self.t, &self.block);
        if t >= BURST || t < SEAL - 0.08 {
            return;
        }
        let jx = self.jolt();
        let wh = t >= FLASH;
        // light slides across the block: each facet flashes whole as the light passes it, the lit ones brightest
        let gs: Vec<f64> = b
            .facets
            .iter()
            .map(|f| {
                let mut g = 0.0;
                for s0 in SWEEPS {
                    g = js_max(
                        g,
                        libm::sin(PI * clamp((t - s0 - f.d * 0.32) / 0.13, 0.0, 1.0)),
                    );
                }
                g * (0.35 + 0.65 * f.lit)
            })
            .collect();
        for y in Y0..Y1 {
            for x in X0..X1 {
                let fi = b.at(x, y);
                let (Ok(fi), (fx, fy)) = (usize::try_from(fi), (f64::from(x), f64::from(y))) else {
                    continue;
                };
                let (Some(f), Some(&g)) = (b.facets.get(fi), gs.get(fi)) else {
                    continue;
                };
                let age = t - f.at;
                if age < 0.0 {
                    continue;
                }
                if wh {
                    p.px(fx + jx, fy, WHITE);
                    continue;
                }
                if age < 0.03 {
                    p.blend(fx + jx, fy, c[7], 0.55);
                    continue;
                }
                let fresh = 1.0 - clamp((age - 0.03) / 0.1, 0.0, 1.0);
                p.blend(
                    fx + jx,
                    fy,
                    if fresh > 0.5 || g > 0.5 { c[6] } else { f.tint },
                    f.alpha + 0.25 * fresh + 0.15 * g,
                );
                if g > 0.05 {
                    p.light(fx + jx, fy, c[5], 0.42 * g);
                }
            }
        }
        // edges: the outline lit where it faces up and left, cool on the shadow side with a deep rim outside it;
        // inner edges pale; any pane that has just set rings white
        for e in &b.edges {
            if e.floor || t < e.at {
                continue;
            }
            let ((x0, y0), (x1, y1)) = (e.a, e.b);
            let fresh = e.fs.iter().any(|&fi| {
                b.facets.get(fi).is_some_and(|f| t - f.at < 0.1)
                    || gs.get(fi).is_some_and(|&g| g > 0.6)
            });
            if e.outer && e.lit < 0.0 && !wh {
                p.line(x0 + jx + 1.0, y0, x1 + jx + 1.0, y1, c[1], 1.0);
            }
            if e.outer {
                p.glow_line(x0 + jx, y0, x1 + jx, y1, 5.0, c[4], 0.1);
            }
            let col = if wh {
                c[6]
            } else if fresh {
                WHITE
            } else if e.outer {
                if e.lit > 0.3 {
                    WHITE
                } else if e.lit > -0.2 {
                    c[6]
                } else {
                    c[4]
                }
            } else if (y0 + y1) / 2.0 < -30.0 {
                c[6]
            } else {
                c[5]
            };
            p.line(x0 + jx, y0, x1 + jx, y1, col, 1.0);
        }
        // a pane about to set: its edges traced in white from its first corner round, a bead of light at the head
        for f in &b.facets {
            let age = t - f.at;
            if !(-0.07..0.0).contains(&age) {
                continue;
            }
            let mut left = ((age + 0.07) / 0.07) * f.len;
            for (k, &(x0, y0)) in f.pts.iter().enumerate() {
                if left <= 0.0 {
                    break;
                }
                let Some(&(x1, y1)) = f.pts.get((k + 1) % f.pts.len()) else {
                    continue;
                };
                let l = libm::hypot(x1 - x0, y1 - y0);
                let v = js_min(1.0, left / l);
                left -= l;
                p.line(
                    x0 + jx,
                    y0,
                    x0 + jx + (x1 - x0) * v,
                    y0 + (y1 - y0) * v,
                    WHITE,
                    1.0,
                );
                if v < 1.0 {
                    p.glow(x0 + jx + (x1 - x0) * v, y0 + (y1 - y0) * v, 6.0, c[6], 0.6);
                }
            }
        }
        if t >= CRACK && !wh {
            draw_fractures(p, &self.cracks, (t - CRACK) * 150.0, c, jx);
        }
        if t >= CRACK2 && !wh {
            draw_fractures(p, &self.cracks2, (t - CRACK2) * 190.0, c, jx);
        }
    }
}

/// Frost on the floor: a crust on the floor line, rime ferns standing up from it, a cold bloom under the surface.
fn floor_frost(p: &mut Painter, c: &[Rgba; 8], t: f64, out: f64) {
    let mut x = -REACH;
    while x <= REACH {
        let xx = x;
        x += 1.0;
        let d = libm::fabs(xx + 0.5) / REACH;
        let arrive = SNAP * (1.0 - d);
        let age = t - arrive;
        if age < 0.0 {
            continue;
        }
        let a = out * clamp((1.12 - d) / 0.2, 0.0, 1.0);
        let g = smooth(0.0, 0.4, age);
        let h = rand(&[xx, 31.0]);
        let s = if xx < 0.0 { -1.0 } else { 1.0 };
        if a <= 0.0 {
            continue;
        }
        p.dot(xx, 0.0, if age < 0.05 || h < 0.3 { WHITE } else { c[6] }, a);
        if h < 0.55 {
            let l = js_round(g * (1.0 + rand(&[xx, 7.0]) * 3.4) * (1.0 - d * 0.55));
            let mut j = 1.0;
            while j <= l {
                p.dot(xx, -j, if j == l { WHITE } else { c[5] }, a);
                j += 1.0;
            }
            if l >= 2.0 {
                p.dot(xx + s, -l + 1.0, c[6], a);
            }
            if l >= 4.0 {
                p.dot(xx - s, -l + 2.0, c[5], a);
            }
        }
        let dd = g * (2.0 + rand(&[xx, 11.0]) * 4.0) * (1.0 - d * 0.5);
        let mut j = 1.0;
        while j <= dd {
            let col = if rand(&[xx * 13.0 + j, 5.0]) < 0.07 {
                WHITE
            } else if j < 2.0 {
                c[5]
            } else if j < 4.0 {
                c[4]
            } else {
                c[3]
            };
            p.dot(xx, j, col, a * (1.05 - j / (dd + 1.0)));
            j += 1.0;
        }
    }
}

/// The block's panes, cut until each is small enough.
fn chop(r: &mut Rng, pts: Vec<Pt>, depth: u32) -> Vec<Vec<Pt>> {
    let c = centroid(&pts);
    if c.area < 20.0 || depth > 2 {
        return vec![pts];
    }
    let a = r.range(0.0, PI);
    let mut out = Vec::new();
    for q in split_poly(&pts, c.x, c.y, a) {
        out.extend(chop(r, q, depth + 1));
    }
    out
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        let body = actor.map_or_else(Body::stand_in, |a| Body::of(&a));
        if self.t == 0.0 {
            // before the first step: everything from the seed, as pixelfx's `create` with this actor
            *self = State::new(self.c, self.seed, body);
        } else {
            // a new pose mid-play: only the frost's climb over it, from a generator of its own
            let mut r = rng(&[self.seed, 911.0]);
            self.frost = Frost::new(body, &self.c, &mut r, self.seed);
        }
    }

    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let c = self.c;
        let State {
            r,
            ps,
            bits,
            lights,
            panes,
            frost,
            block,
            formed,
            sealed,
            struck,
            burst,
            white,
            ..
        } = self;
        let (bx, by, bw, bh) = (
            frost.body.x,
            frost.body.y,
            frost.body.w,
            frost.body.h as f64,
        );
        // frost lines race in along the floor from both sides, vapour riding them
        if t < SNAP {
            for s in [-1.0, 1.0] {
                let x = s * REACH * (1.0 - t / SNAP);
                if r.chance(0.12) {
                    emit!(
                        ps,
                        r,
                        Kind::Vapour,
                        x,
                        -r.range(0.0, 2.0),
                        -s * r.range(40.0, 90.0),
                        -r.range(0.0, 5.0),
                        r.range(0.3, 0.5),
                        c[6],
                        r.range(1.4, 2.4)
                    );
                }
                if r.chance(0.45) {
                    emit!(
                        ps,
                        r,
                        Kind::Streak,
                        x + s * r.range(0.0, 6.0),
                        -r.range(1.0, 4.0),
                        -s * r.range(50.0, 90.0),
                        0.0,
                        r.range(0.1, 0.16),
                        if r.chance(0.5) { c[6] } else { c[5] }
                    );
                }
            }
        }
        // stars and motes kicked off the climbing front
        if t > CLIMB.0 && t < CLIMB.1 + 0.03 {
            let n = frost.order.len();
            let mut lo = 0;
            while lo < n && frost.reach(lo) < t - 0.06 {
                lo += 1;
            }
            let mut hi = lo;
            while hi < n && frost.reach(hi) <= t {
                hi += 1;
            }
            if hi > lo && r.chance(0.45) {
                let k = r.int(lo as f64, hi as f64 - 1.0) as usize;
                if let Some(&i) = frost.order.get(k) {
                    emit!(
                        ps,
                        r,
                        Kind::Star,
                        bx + (i % bw) as f64 + 0.5,
                        by + (i / bw) as f64,
                        r.range(-8.0, 8.0),
                        -r.range(0.0, 18.0),
                        r.range(0.2, 0.42),
                        if r.chance(0.5) { c[6] } else { WHITE },
                        r.range(0.6, 1.9)
                    );
                }
            }
            if hi > lo && r.chance(0.12) {
                let k = r.int(lo as f64, hi as f64 - 1.0) as usize;
                if let Some(&i) = frost.order.get(k) {
                    emit!(
                        ps,
                        r,
                        Kind::Flake,
                        bx + (i % bw) as f64,
                        by + (i / bw) as f64,
                        r.range(-20.0, 20.0),
                        -r.range(10.0, 30.0),
                        r.range(0.5, 0.9),
                        c[6],
                        r.range(0.5, 1.2)
                    );
                }
            }
        }
        // each pane snaps into place with a chip of shards, a puff and a glint
        while let Some(f) = block.facets.get(*formed)
            && t >= f.at
        {
            *formed += 1;
            let m = centroid(&f.pts);
            for _ in 0..3 {
                let a = libm::atan2(m.y - CORE.1, m.x - CORE.0) + r.range(-0.7, 0.7);
                let sp = r.range(50.0, 120.0);
                bit!(bits, BitKind::Shard;
                    x = m.x,
                    y = m.y,
                    vx = libm::cos(a) * sp,
                    vy = libm::sin(a) * sp - 40.0,
                    g = 0.8,
                    drag = 0.8,
                    bounce = 0.3,
                    life = r.range(0.35, 0.7),
                    a = r.range(0.0, TAU),
                    va = r.range(-18.0, 18.0),
                    s = r.range(1.0, 2.2),
                    c = step(&c, r.int(5.0, 7.0)),
                );
            }
            if m.y > -14.0 {
                // JS `Math.sign(c.x || 1)`
                let sx = if m.x == 0.0 || m.x.is_nan() {
                    1.0
                } else {
                    js_sign(m.x)
                };
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    m.x,
                    -1.0,
                    sx * r.range(10.0, 30.0),
                    -r.range(2.0, 8.0),
                    r.range(0.5, 0.8),
                    c[6],
                    r.range(2.2, 3.4)
                );
            }
            emit!(
                ps,
                r,
                Kind::Star,
                m.x + r.range(-2.0, 2.0),
                m.y + r.range(-2.0, 2.0),
                0.0,
                -4.0,
                r.range(0.25, 0.45),
                WHITE,
                r.range(1.2, 2.2)
            );
            lights.add(m.x, m.y, 16.0, c[6], 0.4, 0.12);
        }
        if !*sealed && *formed == block.facets.len() {
            *sealed = true;
            lights.add(0.0, -26.0, 70.0, c[5], 0.6, 0.4);
            for _ in 0..5 {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    r.range(-15.0, 15.0),
                    -r.range(6.0, 44.0),
                    r.range(-6.0, 6.0),
                    -r.range(0.0, 8.0),
                    r.range(0.3, 0.6),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(1.0, 2.2)
                );
            }
        }
        // while it holds: glints wink on the corners, cold vapour rolls off the foot, a little snow
        if *sealed && t < CRACK2 {
            if r.chance(0.035)
                && let Some(&(x, y)) = r.pick(&V)
            {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    x + r.range(-1.0, 1.0),
                    y + r.range(-1.0, 1.0),
                    0.0,
                    -2.0,
                    r.range(0.3, 0.6),
                    if r.chance(0.6) { c[6] } else { WHITE },
                    r.range(0.8, 2.2)
                );
            }
            if r.chance(0.025) {
                let s = r.sign();
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    s * r.range(12.0, 18.0),
                    -r.range(1.0, 3.0),
                    s * r.range(8.0, 24.0),
                    -r.range(1.0, 5.0),
                    r.range(0.7, 1.1),
                    c[6],
                    r.range(1.8, 3.0)
                );
            }
            if r.chance(0.012) {
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    r.range(-30.0, 30.0),
                    -r.range(56.0, 76.0),
                    r.range(-4.0, 4.0),
                    r.range(4.0, 10.0),
                    r.range(1.2, 1.8),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(0.5, 1.4)
                );
            }
        }
        // it cracks: a jolt, chips spall off, light leaks from the cracks
        if *struck == 0 && t >= CRACK {
            *struck = 1;
            lights.add(CORE.0, CORE.1, 26.0, WHITE, 0.4, 0.1);
            for _ in 0..4 {
                bit!(bits, BitKind::Shard;
                    x = r.range(-14.0, 14.0),
                    y = -r.range(10.0, 40.0),
                    vx = r.range(-40.0, 40.0),
                    vy = -r.range(10.0, 60.0),
                    g = 0.9,
                    drag = 0.6,
                    bounce = 0.3,
                    life = r.range(0.5, 0.9),
                    a = r.range(0.0, TAU),
                    va = r.range(-20.0, 20.0),
                    s = r.range(1.0, 2.0),
                    c = step(&c, r.int(5.0, 7.0)),
                );
            }
        }
        if *struck == 1 && t >= CRACK2 {
            *struck = 2;
            lights.add(CORE.0, CORE.1, 40.0, c[6], 0.35, 0.16);
            for _ in 0..3 {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    r.range(-12.0, 12.0),
                    -r.range(8.0, 40.0),
                    0.0,
                    -3.0,
                    r.range(0.15, 0.3),
                    WHITE,
                    r.range(1.0, 2.0)
                );
            }
        }
        if !*white && t >= FLASH {
            *white = true;
            lights.add(CORE.0, CORE.1, 60.0, WHITE, 0.9, 0.1);
        }
        // it bursts: the panes fly off tumbling, shards and ice dust, glints, a flurry, a flash of light
        if !*burst && t >= BURST {
            *burst = true;
            lights.add(CORE.0, CORE.1, 64.0, WHITE, 0.85, 0.14);
            lights.add(CORE.0, CORE.1 - 4.0, 72.0, c[4], 0.55, 0.6);
            for f in &block.facets {
                for part in chop(r, f.pts.clone(), 0) {
                    let m = centroid(&part);
                    let (dx, dy) = (m.x - CORE.0, m.y - CORE.1);
                    let d = libm::hypot(dx, dy);
                    let d = if d == 0.0 { 1.0 } else { d };
                    let sp = r.range(40.0, 85.0) * (1.2 - js_min(0.5, m.area / 120.0));
                    let pts = part.iter().map(|&(x, y)| (x - m.x, y - m.y)).collect();
                    let mut q = Pane::new(pts, m.x, m.y);
                    q.vx = (dx / d) * sp + r.range(-15.0, 15.0);
                    q.vy = (dy / d) * sp * 0.6 - r.range(90.0, 190.0);
                    q.a = 0.0;
                    q.va = r.range(-9.0, 9.0);
                    q.f = r.range(-0.8, 0.8);
                    q.vf = r.sign() * r.range(5.0, 13.0);
                    q.lit = f.lit;
                    q.g = 0.85;
                    q.drag = 0.7;
                    q.brittle = 1;
                    q.life = r.range(1.0, 1.45);
                    panes.add(q);
                }
            }
            for _ in 0..50 {
                let (mut x, mut y) = (0.0, 0.0);
                let mut k = 0;
                while k < 12 && block.at(x as i32, y as i32) < 0 {
                    x = r.int(f64::from(X0), f64::from(X1) - 1.0);
                    y = r.int(f64::from(Y0), f64::from(Y1) - 1.0);
                    k += 1;
                }
                let (dx, dy) = (x - CORE.0, y - CORE.1);
                let d = libm::hypot(dx, dy);
                let d = if d == 0.0 { 1.0 } else { d };
                let sp = r.range(50.0, 130.0);
                bit!(bits, BitKind::Shard;
                    x = x,
                    y = y,
                    vx = (dx / d) * sp,
                    vy = (dy / d) * sp * 0.7 - r.range(60.0, 170.0),
                    g = 0.85,
                    drag = 0.9,
                    bounce = 0.3,
                    life = r.range(0.8, 1.4),
                    a = r.range(0.0, TAU),
                    va = r.range(-22.0, 22.0),
                    s = r.range(1.5, 3.2),
                    c = step(&c, r.int(4.0, 7.0)),
                );
            }
            for f in &frost.feathers {
                if r.chance(0.5) {
                    bit!(bits, BitKind::Shard;
                        x = f.x,
                        y = f.y,
                        vx = f.dx * r.range(20.0, 70.0),
                        vy = -r.range(20.0, 90.0),
                        g = 0.8,
                        drag = 0.8,
                        bounce = 0.3,
                        life = r.range(0.4, 0.8),
                        a = r.range(0.0, TAU),
                        va = r.range(-20.0, 20.0),
                        s = r.range(1.0, 1.8),
                        c = WHITE,
                    );
                }
            }
            for _ in 0..12 {
                let s = r.sign();
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    s * r.range(2.0, 16.0),
                    -r.range(1.0, 6.0),
                    s * r.range(30.0, 90.0),
                    -r.range(4.0, 16.0),
                    r.range(0.6, 1.1),
                    c[6],
                    r.range(2.2, 4.0)
                );
            }
            for _ in 0..8 {
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    r.range(-14.0, 14.0),
                    -r.range(8.0, 40.0),
                    r.range(-30.0, 30.0),
                    -r.range(10.0, 30.0),
                    r.range(0.6, 1.0),
                    c[6],
                    r.range(3.0, 5.0)
                );
            }
            for _ in 0..18 {
                let a = r.range(-PI, 0.0);
                let sp = r.range(40.0, 130.0);
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    r.range(-10.0, 10.0),
                    CORE.1 + r.range(-12.0, 12.0),
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    r.range(1.0, 1.5),
                    if r.chance(0.6) { WHITE } else { c[6] },
                    r.range(0.5, 1.6)
                );
            }
            for _ in 0..7 {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    r.range(-26.0, 26.0),
                    -r.range(6.0, 52.0),
                    r.range(-12.0, 12.0),
                    -r.range(4.0, 16.0),
                    r.range(0.4, 0.8),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(1.0, 2.4)
                );
            }
        }
        // glints wink on the debris as it settles
        if *burst && t < BURST + 0.9 && r.chance(0.07) {
            let n = bits.count().max(1);
            let i = libm::floor(r.next() * n as f64) as usize;
            let b = bits
                .list()
                .get(i)
                .map_or((0.0, -10.0), |q| (q.bit.x, q.bit.y));
            emit!(
                ps,
                r,
                Kind::Star,
                b.0,
                b.1 - 1.0,
                0.0,
                -3.0,
                r.range(0.2, 0.4),
                if r.chance(0.5) { WHITE } else { c[6] },
                r.range(0.6, 1.6)
            );
        }
        // the last of the ice runs off the character
        if *burst && t < BURST + THAW && r.chance(0.12) {
            let y = -bh + ease::in_quad(seg(t, BURST, BURST + THAW)) * bh;
            bit!(bits, BitKind::Drip;
                x = r.range(-9.0, 9.0),
                y = y,
                vy = 10.0,
                g = 0.5,
                life = 0.8,
                c = c[5],
            );
        }
        ps.update(r, dt);
        bits.update(dt);
        panes.update(dt, |e| {
            if let PaneEvent::Land { vx, x, y, .. } = e {
                // a pane that lands breaks into splinters that skip along the floor, with a puff of ice dust and
                // a glint
                for _ in 0..3 {
                    bit!(bits, BitKind::Shard;
                        x = x + r.range(-2.0, 2.0),
                        y = y - 1.0,
                        vx = vx * 0.6 + r.range(-50.0, 50.0),
                        vy = -r.range(30.0, 100.0),
                        g = 0.8,
                        drag = 0.6,
                        bounce = 0.35,
                        life = r.range(0.4, 0.8),
                        a = r.range(0.0, TAU),
                        va = r.range(-18.0, 18.0),
                        s = r.range(1.2, 2.4),
                        c = step(&c, r.int(5.0, 7.0)),
                    );
                }
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    x,
                    y - 1.0,
                    r.range(-25.0, 25.0),
                    -r.range(2.0, 10.0),
                    r.range(0.4, 0.7),
                    c[6],
                    r.range(1.4, 2.4)
                );
                if r.chance(0.4) {
                    emit!(
                        ps,
                        r,
                        Kind::Star,
                        x,
                        y - 2.0,
                        0.0,
                        -6.0,
                        r.range(0.2, 0.4),
                        WHITE,
                        r.range(0.8, 1.6)
                    );
                }
            }
        });
        lights.update(dt);
        end_by(t, D, ps, Some(bits), Some(panes));
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let (c, t) = (&self.c, self.t);
        if layer != Layer::Front {
            let out = 1.0 - smooth(MELT.0, MELT.1, t);
            if out > 0.0 {
                floor_frost(p, c, t, out);
            }
            self.ps.draw(p, Layer::Back);
        }
        if layer == Layer::Back {
            return;
        }
        self.draw_character(p);
        self.draw_block(p);
        // the cold light: a pool under the ice, a glow in it that brightens as it cracks, the climbing front's light
        let hold = smooth(CLIMB.0, SEAL, t) * (1.0 - smooth(BURST, BURST + 0.6, t));
        let crk = smooth(CRACK, FLASH, t);
        if hold > 0.0 {
            p.glow(
                0.0,
                -2.0,
                34.0,
                c[4],
                0.22 * hold * (0.85 + 0.15 * libm::sin(t * 3.0)),
            );
            p.glow(CORE.0, CORE.1 - 4.0, 40.0, c[3], (0.22 + 0.1 * crk) * hold);
        }
        if t > CLIMB.0 && t < CLIMB.1 {
            p.glow(
                0.0,
                -38.0 * ease::out_quad(seg(t, CLIMB.0, CLIMB.1)),
                18.0,
                c[5],
                0.28,
            );
        }
        // the seal: a star at the crown as the last pane sets
        if let Some(last) = self.block.facets.last() {
            let g = t - last.at;
            if g > 0.0 && g < 0.3 {
                sparkle(
                    p,
                    V[P2].0,
                    V[P2].1,
                    8.0 * ease::out_back(js_min(1.0, g / 0.06), ease::BACK_S) * (1.0 - g / 0.3),
                    c[6],
                    c[4],
                    0.6,
                );
            }
        }
        if (FLASH..BURST + 0.16).contains(&t) {
            let a = clamp((t - FLASH) / 0.22, 0.0, 1.0);
            flare(
                p,
                CORE.0,
                CORE.1,
                34.0 * libm::pow(1.0 - a, 1.4),
                1.0 - a,
                c,
            );
        }
        self.panes.draw(p, c);
        self.bits.draw(p);
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
