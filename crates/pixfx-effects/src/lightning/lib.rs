//! Shared by the lightning effects (the leading _ keeps the tools from listing it as an effect).
//! The bolt shape and the way a bolt is inked (a deep halo, a pale body, a white core, light along it) are
//! adapted from Sprite Fusion's open-source lightning (https://www.spritefusion.com/pixel-art-vfx/lightning),
//! MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/lightning/_lib.js`: what ball-lightning uses (`VOLT`, `PLASMA`, `zigzag`, `stroke`,
//! `crackle`, `flight`, `spark`). The other helpers arrive with the effects that need them. `stroke` and
//! `crackle` take only the options ball-lightning passes (`stroke`: `hero`; `crackle`: the default alpha).

use std::collections::HashMap;

use pixfx_core::Painter;
use pixfx_core::color::Rgba;
use pixfx_core::math::{Rng, js_min, js_round};
use pixfx_core::sim::GRAVITY;

/// Default palette, dark to bright: deep violet shadow, electric blue, icy white.
pub const VOLT: [Rgba; 8] = [
    Rgba(0x12, 0x0a, 0x33, 255),
    Rgba(0x2a, 0x1b, 0x78, 255),
    Rgba(0x46, 0x30, 0xc0, 255),
    Rgba(0x5a, 0x62, 0xf0, 255),
    Rgba(0x58, 0xa6, 0xff, 255),
    Rgba(0x9e, 0xd8, 0xff, 255),
    Rgba(0xe4, 0xf6, 0xff, 255),
    Rgba(0xff, 0xff, 0xff, 255),
];
/// Plasma sparks, head to tail: cyan that reads apart from the violet bolt.
pub const PLASMA: [Rgba; 4] = [
    Rgba(0xff, 0xff, 0xff, 255),
    Rgba(0xc8, 0xf8, 0xff, 255),
    Rgba(0x7f, 0xf3, 0xff, 255),
    Rgba(0x1a, 0xb8, 0xe6, 255),
];

/// One branch of a bolt: its points and its width (1 for the trunk, thinner for forks).
pub struct Path {
    pub pts: Vec<(f64, f64)>,
    pub w: f64,
}

/// A bolt: its paths, the trunk first.
pub struct Bolt {
    pub paths: Vec<Path>,
}

/// A bolt from (x0, y0) to (x1, y1): midpoint displacement down to ~3 px segments (`jag`: how far the first
/// midpoint may swing, as a share of the length), with forks that split off along the way, each thinner and
/// forking again up to two levels.
pub fn zigzag(r: &mut Rng, x0: f64, y0: f64, x1: f64, y1: f64, jag: f64, forks: f64) -> Bolt {
    Bolt {
        paths: zig(r, x0, y0, x1, y1, jag, forks, 1.0, 0),
    }
}

#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
fn zig(
    r: &mut Rng,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    jag: f64,
    forks: f64,
    w: f64,
    depth: u32,
) -> Vec<Path> {
    let l = libm::hypot(x1 - x0, y1 - y0);
    let mut pts = vec![(x0, y0), (x1, y1)];
    let mut off = l * jag;
    while (pts.len() as f64) < 2.0 + l / 3.0 {
        let mut next = vec![pts[0]];
        for pair in pts.windows(2) {
            let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
            let hl = libm::hypot(bx - ax, by - ay);
            let hl = if hl == 0.0 { 1.0 } else { hl };
            let o = r.range(-off, off);
            next.push((
                (ax + bx) / 2.0 + (-(by - ay) / hl) * o,
                (ay + by) / 2.0 + ((bx - ax) / hl) * o,
            ));
            next.push((bx, by));
        }
        pts = next;
        off *= 0.52;
    }
    let n = pts.len();
    let mut out = vec![Path {
        pts: pts.clone(),
        w,
    }];
    if depth < 2 {
        let d = f64::from(depth);
        let mut i = 2;
        while i + 3 < n {
            if r.chance(forks / (1.0 + d) / (n as f64 / 6.0).max(4.0)) {
                let (ax, ay) = pts[i];
                let (bx, by) = pts[(n - 1).min(i + 3)];
                let a = libm::atan2(by - ay, bx - ax) + r.sign() * r.range(0.35, 0.9);
                let len = (l - (i as f64 / n as f64) * l) * r.range(0.25, 0.55);
                out.extend(zig(
                    r,
                    ax,
                    ay,
                    ax + libm::cos(a) * len,
                    ay + libm::sin(a) * len,
                    jag * 1.1,
                    forks,
                    w * 0.55,
                    depth + 1,
                ));
            }
            i += 1;
        }
    }
    out
}

// ---- inking. Pixels are gathered in a grid at a level (higher wins), then drawn once each, so a fork's
// halo never covers the trunk's body and overlapping passes cost nothing extra.
const HALF: i64 = 512;

#[derive(Default)]
struct Ink {
    level: HashMap<(i64, i64), u8>,
    /// Pixels in the order they were first marked, as pixelfx draws them.
    hit: Vec<(i64, i64)>,
}

impl Ink {
    fn mark(&mut self, x: f64, y: f64, lvl: u8) {
        let (ix, iy) = (libm::floor(x) as i64, libm::floor(y) as i64);
        if !(-HALF..HALF).contains(&ix) || !(-HALF..HALF).contains(&iy) {
            return;
        }
        let v = self.level.entry((ix, iy)).or_insert(0);
        if *v == 0 {
            self.hit.push((ix, iy));
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
    fn flush(self, p: &mut Painter, cols: [Rgba; 4], alphas: [f64; 4]) {
        for (x, y) in self.hit {
            let v = usize::from(self.level.get(&(x, y)).copied().unwrap_or(0));
            let (Some(&c), Some(&a)) = (cols.get(v.wrapping_sub(1)), alphas.get(v.wrapping_sub(1)))
            else {
                continue;
            };
            let (x, y) = (x as f64, y as f64);
            if a >= 1.0 {
                p.px(x, y, c);
            } else if a > 0.0 {
                p.dot(x, y, c, a);
            }
        }
    }
}

/// Visit points half a pixel apart along each segment of `pts`.
fn walk(pts: &[(f64, f64)], mut f: impl FnMut(f64, f64)) {
    for pair in pts.windows(2) {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let l = libm::hypot(bx - ax, by - ay).max(1.0);
        let mut d = 0.0;
        while d <= l {
            f(ax + ((bx - ax) * d) / l, ay + ((by - ay) * d) / l);
            d += 0.5;
        }
    }
}

/// A bolt at full power: a deep halo, a pale body, a white core on the trunk, light every few points. `k` is the
/// stroke's brightness (forks drop out below 0.5, the trunk thins below 0.6), `s` scales the thickness, `hero` is
/// a white frame (the body goes white).
pub fn stroke(p: &mut Painter, b: &Bolt, k: f64, c: &[Rgba; 8], s: f64, hero: bool) {
    if k <= 0.02 {
        return;
    }
    let lk = 0.5;
    let mut ink = Ink::default();
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
        walk(&q.pts, |x, y| {
            ink.stamp(x, y, rh, 1);
            ink.stamp(x, y, rb, if trunk { 3 } else { 2 });
            if trunk && k > 0.3 {
                ink.mark(x, y, 4);
            }
        });
        for pt in q.pts.iter().take(q.pts.len().saturating_sub(1)).step_by(3) {
            p.glow(
                pt.0,
                pt.1,
                (if trunk { 14.0 } else { 7.0 }) * s,
                c[4],
                lk * k * q.w,
            );
        }
    }
    let cols = if hero {
        [c[3], c[6], c[7], c[7]]
    } else {
        [c[2], c[4], c[5], c[7]]
    };
    ink.flush(p, cols, [1.0; 4]);
}

const CROSS: [(f64, f64); 4] = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];

/// A thin crackle: a 1 px core (white on the trunk while bright), a 2 px pale body, a dithered halo while `k` is
/// high, light along it. For short arcs that live a frame or two.
pub fn crackle(p: &mut Painter, b: &Bolt, k: f64, c: &[Rgba; 8], glow_k: f64) {
    if k <= 0.02 {
        return;
    }
    let mut ink = Ink::default();
    for q in &b.paths {
        let thick = q.w >= 1.0;
        walk(&q.pts, |x, y| {
            if k > 0.45 && thick {
                for (dx, dy) in CROSS {
                    ink.mark(x + dx * 1.5, y + dy * 1.5, 1);
                }
            }
            if k > 0.25 && (thick || q.w > 0.4) {
                ink.mark(x + 1.0, y, 2);
                ink.mark(x, y + 1.0, 2);
            }
            ink.mark(x, y, if thick && k > 0.3 { 4 } else { 3 });
        });
        for pt in q.pts.iter().take(q.pts.len().saturating_sub(1)).step_by(3) {
            p.glow(
                pt.0,
                pt.1,
                (if thick { 9.0 } else { 5.0 }) * (0.5 + k * 0.5),
                c[4],
                glow_k * k * q.w,
            );
        }
    }
    let halo = js_min(1.0, (k - 0.45) * 1.6);
    ink.flush(p, [c[3], c[5], c[5], c[7]], [halo, 1.0, 1.0, 1.0]);
}

/// Where a spark launched at (x, y, vx, vy) is after `age` seconds, stepped like the simulation's SPARK (gravity,
/// drag, bouncing on a floor at `floor`), so pure loops get the same physics. Returns (x, y, vx, vy).
pub fn flight(x: f64, y: f64, vx: f64, vy: f64, age: f64, floor: f64) -> (f64, f64, f64, f64) {
    const G: f64 = 0.45;
    const DRAG: f64 = 2.2;
    const BOUNCE: f64 = 0.45;
    let dt = 1.0 / 120.0;
    let (mut x, mut y, mut vx, mut vy) = (x, y, vx, vy);
    let mut s = 0.0;
    while s < age {
        let h = js_min(dt, age - s);
        vy += GRAVITY * G * h;
        vx -= vx * DRAG * h;
        vy -= vy * DRAG * h;
        let ny = y + vy * h;
        x += vx * h;
        if vy > 0.0 && y <= floor && ny >= floor {
            vy = -vy * BOUNCE;
            vx *= 0.7;
            y = floor - 0.25;
        } else {
            y = ny;
        }
        s += dt;
    }
    (x, y, vx, vy)
}

/// A spark drawn the way the simulation draws one (a streak stretched by speed, a hot head, light), for pure loops.
pub fn spark(p: &mut Painter, x: f64, y: f64, vx: f64, vy: f64, k: f64, s: &[Rgba; 4]) {
    let h = libm::hypot(vx, vy);
    let sp = if h == 0.0 { 1.0 } else { h };
    let (ux, uy) = (vx / sp, vy / sp);
    let head = if k > 0.6 {
        s[0]
    } else if k > 0.3 {
        s[1]
    } else {
        s[2]
    };
    let len = js_min(9.0, 1.0 + sp * 0.022);
    let tail = if k > 0.5 { head } else { s[3] };
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
