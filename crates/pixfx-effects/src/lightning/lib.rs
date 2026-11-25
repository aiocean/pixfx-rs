//! Shared by the lightning effects: bolts that fork and thin as they split, inked at the 2x character scale,
//! thin crackles, bowed arcs, heat scorch, and trajectories for pure loops.
//!
//! Port of pixelfx `src/fx/lightning/_lib.js` (the parts its effects use), plus pixelfx's stock 2x actor
//! (`src/core/actor.js` `stockActor(2)`) as the stand-in body the actor effects read when no sprite is handed to
//! them.
//!
//! The bolt shape and the way a bolt is inked (a deep halo, a pale body, a white core, light along it) are
//! adapted from Sprite Fusion's open-source lightning (https://www.spritefusion.com/pixel-art-vfx/lightning),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use std::cell::RefCell;
use std::sync::{Arc, OnceLock};

use pixfx_core::Actor;
use pixfx_core::sim::{FIRE_RAMP, GRAVITY, SMOKE_RAMP};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::{Rng, js_max, js_min, js_round, or_one, rand};

/// Default palette, dark to bright: deep violet shadow, electric blue, icy white.
pub const VOLT: [Rgba; 8] = [
    Rgba::hex24(0x120a33),
    Rgba::hex24(0x2a1b78),
    Rgba::hex24(0x4630c0),
    Rgba::hex24(0x5a62f0),
    Rgba::hex24(0x58a6ff),
    Rgba::hex24(0x9ed8ff),
    Rgba::hex24(0xe4f6ff),
    Rgba::hex24(0xffffff),
];
/// Plasma sparks, head to tail: cyan that reads apart from the violet bolt.
pub const PLASMA: [Rgba; 4] = [
    Rgba::hex24(0xffffff),
    Rgba::hex24(0xc8f8ff),
    Rgba::hex24(0x7ff3ff),
    Rgba::hex24(0x1ab8e6),
];
pub const SOOT: Rgba = Rgba::hex24(0x110c16);
pub const SOOT2: Rgba = Rgba::hex24(0x1c1522);

/// One stroke of a bolt: its points and its weight (1 the trunk, thinner for forks).
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub pts: Vec<(f64, f64)>,
    pub w: f64,
}

/// A bolt: its paths, the trunk first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bolt {
    pub paths: Vec<Path>,
}

/// A bolt from (x0, y0) to (x1, y1): midpoint displacement down to ~3 px segments (`jag`: how far the first
/// midpoint may swing, as a share of the length), with forks that split off along the way, each thinner (`w`)
/// and forking again up to two levels. The trunk first.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn zigzag_w(
    r: &mut Rng,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    jag: f64,
    forks: f64,
    w: f64,
    depth: u32,
) -> Bolt {
    let mut out = Bolt::default();
    let l = libm::hypot(x1 - x0, y1 - y0);
    let mut pts = vec![(x0, y0), (x1, y1)];
    let mut off = l * jag;
    while (pts.len() as f64) < 2.0 + l / 3.0 {
        let mut nxt = Vec::with_capacity(pts.len() * 2);
        nxt.push(pts[0]);
        for i in 0..pts.len() - 1 {
            let ((ax, ay), (bx, by)) = (pts[i], pts[i + 1]);
            let ll = or_one(libm::hypot(bx - ax, by - ay));
            let o = r.range(-off, off);
            nxt.push((
                (ax + bx) / 2.0 + (-(by - ay) / ll) * o,
                (ay + by) / 2.0 + ((bx - ax) / ll) * o,
            ));
            nxt.push((bx, by));
        }
        pts = nxt;
        off *= 0.52;
    }
    let n = pts.len();
    let mut forks_out = Vec::new();
    if depth < 2 {
        let mut i = 2usize;
        while (i as f64) < n as f64 - 3.0 {
            if r.chance(forks / (1.0 + f64::from(depth)) / js_max(4.0, n as f64 / 6.0)) {
                let (ax, ay) = pts[i];
                let (bx, by) = pts[(n - 1).min(i + 3)];
                let sg = r.sign();
                let a = libm::atan2(by - ay, bx - ax) + sg * r.range(0.35, 0.9);
                let ll = (l - (i as f64 / n as f64) * l) * r.range(0.25, 0.55);
                let sub = zigzag_w(
                    r,
                    ax,
                    ay,
                    ax + libm::cos(a) * ll,
                    ay + libm::sin(a) * ll,
                    jag * 1.1,
                    forks,
                    w * 0.55,
                    depth + 1,
                );
                forks_out.extend(sub.paths);
            }
            i += 1;
        }
    }
    out.paths.push(Path { pts, w });
    out.paths.extend(forks_out);
    out
}

/// [`zigzag_w`] for a whole bolt (weight 1, depth 0).
pub fn zigzag(r: &mut Rng, x0: f64, y0: f64, x1: f64, y1: f64, jag: f64, forks: f64) -> Bolt {
    zigzag_w(r, x0, y0, x1, y1, jag, forks, 1.0, 0)
}

/// A bolt bowed off the straight line by `bow` px at its middle (negative: up for a left-to-right arc).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn bowed(
    r: &mut Rng,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    bow: f64,
    jag: f64,
    forks: f64,
) -> Bolt {
    let l = or_one(libm::hypot(x1 - x0, y1 - y0));
    let (nx, ny) = (-(y1 - y0) / l, (x1 - x0) / l);
    let mx = (x0 + x1) / 2.0 + nx * bow + r.range(-2.0, 2.0);
    let my = (y0 + y1) / 2.0 + ny * bow;
    let a = zigzag(r, x0, y0, mx, my, jag, forks);
    let b = zigzag(r, mx, my, x1, y1, jag, forks);
    let mut a_it = a.paths.into_iter();
    let mut b_it = b.paths.into_iter();
    let mut trunk = a_it.next().map(|p| p.pts).unwrap_or_default();
    if let Some(bt) = b_it.next() {
        trunk.extend(bt.pts.into_iter().skip(1));
    }
    let mut paths = vec![Path { pts: trunk, w: 1.0 }];
    paths.extend(a_it);
    paths.extend(b_it);
    Bolt { paths }
}

/// Move every point of a bolt.
pub fn bend(b: &Bolt, f: impl Fn(f64, f64) -> (f64, f64)) -> Bolt {
    Bolt {
        paths: b
            .paths
            .iter()
            .map(|q| Path {
                w: q.w,
                pts: q.pts.iter().map(|&(x, y)| f(x, y)).collect(),
            })
            .collect(),
    }
}

// ---- inking. Pixels are gathered in a grid at a level (higher wins), then drawn once each, so a fork's
// halo never covers the trunk's body and overlapping passes cost nothing extra.
const N: usize = 1024;
const HALF: f64 = 512.0;

/// The gathering grid of pixelfx's module-level `GRID` and `HIT`: one per thread, always left empty.
struct Grid {
    lvl: Vec<u8>,
    hit: Vec<usize>,
}

thread_local! {
    static GRID: RefCell<Grid> = const { RefCell::new(Grid { lvl: Vec::new(), hit: Vec::new() }) };
}

impl Grid {
    fn mark(&mut self, x: f64, y: f64, lvl: u8) {
        let (gx, gy) = (libm::floor(x) + HALF, libm::floor(y) + HALF);
        if !(gx >= 0.0 && gy >= 0.0 && gx < N as f64 && gy < N as f64) {
            return;
        }
        if self.lvl.is_empty() {
            self.lvl = vec![0; N * N];
        }
        let i = gy as usize * N + gx as usize;
        let Some(v) = self.lvl.get_mut(i) else {
            return;
        };
        if *v == 0 {
            self.hit.push(i);
        }
        if lvl > *v {
            *v = lvl;
        }
    }

    fn stamp(&mut self, x: f64, y: f64, r: f64, lvl: u8) {
        if r <= 0.5 {
            return self.mark(js_round(x), js_round(y), lvl);
        }
        let r2 = r * r;
        let mut py = libm::floor(y - r);
        while py <= libm::ceil(y + r) {
            let dy = py + 0.5 - y;
            let mut px = libm::floor(x - r);
            while px <= libm::ceil(x + r) {
                let dx = px + 0.5 - x;
                if dx * dx + dy * dy <= r2 {
                    self.mark(px, py, lvl);
                }
                px += 1.0;
            }
            py += 1.0;
        }
    }

    /// Draw what was gathered: level n in `cols[n - 1]` at alpha `alphas[n - 1]` (dithered below 1).
    fn flush(&mut self, p: &mut Painter, cols: [Rgba; 4], alphas: [f64; 4]) {
        for &i in &self.hit {
            let Some(v) = self.lvl.get_mut(i) else {
                continue;
            };
            let k = usize::from(*v).saturating_sub(1).min(3);
            *v = 0;
            let x = (i % N) as f64 - HALF;
            let y = (i / N) as f64 - HALF;
            let a = alphas[k];
            if a >= 1.0 {
                p.px(x, y, cols[k]);
            } else if a > 0.0 {
                p.dot(x, y, cols[k], a);
            }
        }
        self.hit.clear();
    }
}

fn with_grid(f: impl FnOnce(&mut Grid)) {
    GRID.with(|g| {
        if let Ok(mut g) = g.try_borrow_mut() {
            f(&mut g);
        }
    });
}

/// Every half pixel along a polyline: `f(x, y)`.
fn walk(pts: &[(f64, f64)], mut f: impl FnMut(f64, f64)) {
    for w in pts.windows(2) {
        let ((ax, ay), (bx, by)) = (w[0], w[1]);
        let l = js_max(1.0, libm::hypot(bx - ax, by - ay));
        let mut d = 0.0;
        while d <= l {
            f(ax + ((bx - ax) * d) / l, ay + ((by - ay) * d) / l);
            d += 0.5;
        }
    }
}

/// How [`stroke`] inks a bolt (pixelfx's `o`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeOpts {
    /// A white frame: the body goes white.
    pub hero: bool,
    /// Glow strength.
    pub light: f64,
    /// Alpha.
    pub a: f64,
    /// Draw the trunk only to this share of its length.
    pub up_to: Option<f64>,
    /// Radius of a thicker white core (0: a 1 px core).
    pub core: f64,
    /// No glow above this y.
    pub min_y: Option<f64>,
}

impl Default for StrokeOpts {
    fn default() -> Self {
        Self {
            hero: false,
            light: 0.5,
            a: 1.0,
            up_to: None,
            core: 0.0,
            min_y: None,
        }
    }
}

/// A bolt at full power: a deep halo, a pale body, a white core on the trunk, light every few points. `k` is the
/// stroke's brightness (forks drop out below 0.5, the trunk thins below 0.6), `s` scales the thickness.
pub fn stroke(p: &mut Painter, b: &Bolt, k: f64, c: &[Rgba; 8], s: f64, o: StrokeOpts) {
    if k <= 0.02 {
        return;
    }
    let lk = o.light;
    with_grid(|g| {
        for q in &b.paths {
            let trunk = q.w >= 1.0;
            if !trunk && k < 0.5 {
                continue;
            }
            let rh = (if trunk {
                if k > 0.6 { 2.1 } else { 1.6 }
            } else {
                1.1
            }) * s;
            let rb = (if trunk {
                if k > 0.6 { 1.3 } else { 0.8 }
            } else {
                0.5
            }) * s;
            let pts: &[(f64, f64)] = match o.up_to {
                Some(u) if trunk => {
                    let m = js_max(2.0, libm::ceil(q.pts.len() as f64 * u)) as usize;
                    &q.pts[..m.min(q.pts.len())]
                }
                _ => &q.pts,
            };
            walk(pts, |x, y| {
                g.stamp(x, y, rh, 1);
                g.stamp(x, y, rb, if trunk { 3 } else { 2 });
                if trunk && k > 0.3 {
                    if o.core != 0.0 {
                        g.stamp(x, y, o.core, 4);
                    } else {
                        g.mark(x, y, 4);
                    }
                }
            });
            let mut i = 0;
            while i + 1 < pts.len() {
                let (x, y) = pts[i];
                if o.min_y.is_none_or(|m| y > m) {
                    p.glow(
                        x,
                        y,
                        (if trunk { 14.0 } else { 7.0 }) * s,
                        c[4],
                        lk * k * q.w,
                    );
                }
                i += 3;
            }
        }
        let a = o.a;
        let cols = if o.hero {
            [c[3], c[6], c[7], c[7]]
        } else {
            [c[2], c[4], c[5], c[7]]
        };
        g.flush(p, cols, [a, a, a, a]);
    });
}

const CROSS: [(f64, f64); 4] = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];

/// A thin crackle: a 1 px core (white on the trunk while bright), a 2 px pale body, a dithered halo while `k` is
/// high, light along it. For short arcs that live a frame or two.
pub fn crackle(p: &mut Painter, b: &Bolt, k: f64, c: &[Rgba; 8], glow_k: f64, a: f64) {
    if k <= 0.02 {
        return;
    }
    with_grid(|g| {
        for q in &b.paths {
            let thick = q.w >= 1.0;
            walk(&q.pts, |x, y| {
                if k > 0.45 && thick {
                    for (dx, dy) in CROSS {
                        g.mark(x + dx * 1.5, y + dy * 1.5, 1);
                    }
                }
                if k > 0.25 && (thick || q.w > 0.4) {
                    g.mark(x + 1.0, y, 2);
                    g.mark(x, y + 1.0, 2);
                }
                g.mark(x, y, if thick && k > 0.3 { 4 } else { 3 });
            });
            let mut i = 0;
            while i + 1 < q.pts.len() {
                let (x, y) = q.pts[i];
                p.glow(
                    x,
                    y,
                    (if thick { 9.0 } else { 5.0 }) * (0.5 + k * 0.5),
                    c[4],
                    glow_k * k * q.w,
                );
                i += 3;
            }
        }
        g.flush(
            p,
            [c[3], c[5], c[5], c[7]],
            [js_min(1.0, (k - 0.45) * 1.6) * a, a, a, a],
        );
    });
}

/// A bolt's trunk as a 1 px line in one colour (leaders, afterimages, cooling channels): the segments from
/// `from` to `to` (pixelfx's defaults: 0 and the last point).
pub fn trace(p: &mut Painter, pts: &[(f64, f64)], c: Rgba, a: f64, from: i64, to: i64) {
    let end = to.min(pts.len() as i64 - 1);
    let mut i = from.max(0);
    while i < end {
        let ((x0, y0), (x1, y1)) = (pts[i as usize], pts[i as usize + 1]);
        p.line(x0, y0, x1, y1, c, a);
        i += 1;
    }
}

/// [`trace`] of the whole trunk.
pub fn trace_all(p: &mut Painter, pts: &[(f64, f64)], c: Rgba, a: f64) {
    trace(p, pts, c, a, 0, pts.len() as i64 - 1);
}

/// A heat-ramp colour for `t` (1 white-hot .. 0 cold), or none when it's cold.
pub fn heat(t: f64) -> Option<Rgba> {
    (t > 0.06).then(|| {
        let n = FIRE_RAMP.len();
        let i = libm::floor((1.0 - js_min(1.0, t)) * (n - 1) as f64);
        FIRE_RAMP[js_min((n - 1) as f64, i) as usize]
    })
}

/// Heat left on the floor: a flat ellipse, white-hot in the middle cooling through the fire ramp to soot.
/// `t0` is the heat (1 fresh, 0 cold), `fade` takes the soot away; `ry` pixelfx's default is
/// [`scorch_ry`].
pub fn scorch_with(p: &mut Painter, cx: f64, cy: f64, rx: f64, t0: f64, fade: f64, ry: f64) {
    if fade <= 0.0 {
        return;
    }
    let mut y = libm::floor(cy);
    while y <= cy + ry {
        let mut x = libm::floor(cx - rx);
        while x <= libm::ceil(cx + rx) {
            let (dx, dy) = ((x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry);
            let d = libm::sqrt(dx * dx + dy * dy);
            if d <= 1.0 {
                let n = rand(&[x, y, 41.0]);
                let t = t0 * (1.25 - d) - n * 0.35;
                if let Some(c) = heat(t) {
                    p.dot(x, y, c, fade);
                    if (x as i64) & 3 == 0 {
                        p.glow(x + 0.5, y, 6.0, c, 0.18 * t * fade);
                    }
                } else {
                    p.dot(
                        x,
                        y,
                        if d < 0.5 { SOOT } else { SOOT2 },
                        (if d < 0.95 - n * 0.3 { 1.0 } else { 1.0 - d }) * fade,
                    );
                }
            }
            x += 1.0;
        }
        y += 1.0;
    }
}

/// pixelfx `scorch`'s default `ry`.
pub fn scorch_ry(rx: f64) -> f64 {
    js_max(2.5, rx * 0.12)
}

/// [`scorch_with`] at the default flatness.
pub fn scorch(p: &mut Painter, cx: f64, cy: f64, rx: f64, t0: f64, fade: f64) {
    scorch_with(p, cx, cy, rx, t0, fade, scorch_ry(rx));
}

/// Where a spark launched at (x, y, vx, vy) is after `age` seconds, stepped like the simulation's SPARK (gravity
/// 0.45, drag 2.2, bouncing 0.45 on a floor at `floor`), so pure loops get the same physics.
pub fn flight(
    mut x: f64,
    mut y: f64,
    mut vx: f64,
    mut vy: f64,
    age: f64,
    floor: f64,
) -> (f64, f64, f64, f64) {
    let (g, drag, bounce) = (0.45, 2.2, 0.45);
    let dt = 1.0 / 120.0;
    let mut s = 0.0;
    while s < age {
        let h = js_min(dt, age - s);
        vy += GRAVITY * g * h;
        vx -= vx * drag * h;
        vy -= vy * drag * h;
        let ny = y + vy * h;
        x += vx * h;
        if vy > 0.0 && y <= floor && ny >= floor {
            vy = -vy * bounce;
            vx *= 0.7;
            y = floor - 0.25;
        } else {
            y = ny;
        }
        s += dt;
    }
    (x, y, vx, vy)
}

/// A spark drawn the way the simulation draws one (a streak stretched by speed, a hot head, light), for pure
/// loops. `sp` is the spark ramp, head to tail.
pub fn spark(p: &mut Painter, x: f64, y: f64, vx: f64, vy: f64, k: f64, sp: &[Rgba]) {
    let at = |i: usize| sp.get(i).copied().unwrap_or(Rgba::NONE);
    let v = or_one(libm::hypot(vx, vy));
    let (ux, uy) = (vx / v, vy / v);
    let head = if k > 0.6 {
        at(0)
    } else if k > 0.3 {
        at(1)
    } else {
        at(2)
    };
    let len = js_min(9.0, 1.0 + v * 0.022);
    let tail = if k > 0.5 {
        head
    } else {
        at(sp.len().saturating_sub(1))
    };
    p.line(
        x - ux * len,
        y - uy * len,
        x - ux * len * 0.4,
        y - uy * len * 0.4,
        tail,
        1.0,
    );
    p.line(x - ux * len * 0.45, y - uy * len * 0.45, x, y, head, 1.0);
    p.glow(x, y, 3.0, head, 0.45 * k);
}

/// A soft smoke puff at age `u` (0..1) of its life, drawn like the simulation's SMOKE, for pure loops.
pub fn puff(p: &mut Painter, x: f64, y: f64, size: f64, u: f64, c: Rgba) {
    let k = 1.0 - u;
    let r = size * (0.65 + js_min(1.0, u * 2.5) * 0.45 + u * 0.4);
    let a = js_min(1.0, u * 12.0) * js_min(1.0, k * 2.4) * 0.58;
    if a <= 0.01 {
        return;
    }
    let n = SMOKE_RAMP.len();
    let i = js_min((n - 1) as f64, libm::floor(u * n as f64));
    let col = mix(
        c,
        SMOKE_RAMP.get(i as usize).copied().unwrap_or(Rgba::NONE),
        u,
    );
    p.disc(x, y, r, col, a * 0.45);
    p.disc(x + r * 0.05, y + r * 0.06, r * 0.74, col, a * 0.62);
    if u < 0.5 {
        p.disc(
            x - r * 0.22,
            y - r * 0.3,
            r * 0.5,
            mix(col, WHITE, 0.12),
            a * 0.4 * (1.0 - u * 2.0),
        );
    }
}

// ---- the stand-in body: pixelfx's stock actor at 2x (24 x 38, feet at the origin), read by the effects that
// play on a character when the game hands them none.
const HERO2: [&str; 38] = [
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

/// pixelfx `DUMMY_PAL`: a neutral shell that lets any effect's colours do the talking.
fn dummy(ch: char) -> Rgba {
    match ch {
        'k' => Rgba::hex24(0x0e0c14),
        'A' => Rgba::hex24(0xe4e6ec),
        'B' => Rgba::hex24(0xb6b9c3),
        'C' => Rgba::hex24(0x868995),
        'D' => Rgba::hex24(0x5c5e6a),
        'E' => Rgba::hex24(0x6e707c),
        'F' => Rgba::hex24(0x4b4d58),
        'G' => Rgba::hex24(0x34353f),
        'J' => Rgba::hex24(0x2a2b33),
        'm' => Rgba::hex24(0xf53cd1),
        _ => Rgba::NONE,
    }
}

/// pixelfx `stockActor(2)`: the 2x stand-in character, feet at (0, 0), built once.
pub fn stand_in() -> Option<Arc<Actor>> {
    static ACTOR: OnceLock<Option<Arc<Actor>>> = OnceLock::new();
    ACTOR
        .get_or_init(|| {
            let w = HERO2[0].len() as u32;
            let h = HERO2.len() as u32;
            let pixels: Vec<u32> = HERO2
                .iter()
                .flat_map(|row| row.chars().map(|ch| dummy(ch).pack()))
                .collect();
            Actor::new(-((w / 2) as i32), -(h as i32), w, h, pixels)
                .ok()
                .map(Arc::new)
        })
        .clone()
}

/// The silhouette's edge pixels (solid, with an empty 4-neighbour), row by row, as cells in the effect's space.
pub fn edge_of(a: &Actor) -> Vec<(i32, i32)> {
    let (w, h) = (a.w() as i64, a.h() as i64);
    let on = |x: i64, y: i64| x >= 0 && y >= 0 && x < w && y < h && a.at(x as u32, y as u32) != 0;
    let mut e = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if on(x, y) && (!on(x - 1, y) || !on(x + 1, y) || !on(x, y - 1) || !on(x, y + 1)) {
                e.push((a.x() + x as i32, a.y() + y as i32));
            }
        }
    }
    e
}
