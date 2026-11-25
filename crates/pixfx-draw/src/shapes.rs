//! Point lists for lightning, stars, polygons and trails (pixelfx `shapes.js`: `bolt`, `forks`, `star`, `ngon`,
//! `trail`) and hard-edged craft primitives (`flash_star`, `wedge`, `lobes`, `tempo`).

use crate::color::Rgba;
use crate::math::{Rng, TAU, clamp, js_max, lerp, or_one};
use crate::painter::Painter;

/// Which ramp entry a life fraction `u` (`0` fresh .. `1` dead) shows: a monotonic temperature march.
pub fn tempo(ramp: &[Rgba], u: f64) -> Rgba {
    if ramp.is_empty() {
        return Rgba::NONE;
    }
    let idx = clamp(
        libm::floor(clamp(u, 0.0, 1.0) * ramp.len() as f64),
        0.0,
        (ramp.len() - 1) as f64,
    ) as usize;
    ramp.get(idx).copied().unwrap_or(Rgba::NONE)
}

/// A hard `points`-point star of colour `c` with tips at `r` and inner corners at `r * pinch`, rotated by `rot`.
#[expect(clippy::too_many_arguments, reason = "craft flash parameters")]
pub fn flash_star(
    p: &mut Painter,
    cx: f64,
    cy: f64,
    r: f64,
    c: Rgba,
    points: u32,
    pinch: f64,
    rot: f64,
    a: f64,
) {
    if r < 0.5 {
        p.dot(cx, cy, c, a);
        return;
    }
    let n = points.max(2) * 2;
    let mut pts = Vec::with_capacity(n as usize);
    for i in 0..n {
        let ang = rot + (f64::from(i) * core::f64::consts::PI) / f64::from(points.max(2));
        let rr = if i % 2 == 1 { r * pinch } else { r };
        pts.push((cx + libm::cos(ang) * rr, cy + libm::sin(ang) * rr));
    }
    p.poly(&pts, c, a);
}

/// A tapered solid wedge from `(x0, y0)` with width `w0` to `(x1, y1)` with width `w1`.
#[expect(clippy::too_many_arguments, reason = "craft wedge parameters")]
pub fn wedge(
    p: &mut Painter,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    w0: f64,
    w1: f64,
    c: Rgba,
    a: f64,
) {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let l = or_one(libm::hypot(dx, dy));
    let (nx, ny) = (-dy / l, dx / l);
    p.poly(
        &[
            (x0 + nx * w0 * 0.5, y0 + ny * w0 * 0.5),
            (x1 + nx * w1 * 0.5, y1 + ny * w1 * 0.5),
            (x1 - nx * w1 * 0.5, y1 - ny * w1 * 0.5),
            (x0 - nx * w0 * 0.5, y0 - ny * w0 * 0.5),
        ],
        c,
        a,
    );
}

/// One lobe in a [`lobes`] cluster.
#[derive(Clone, Copy, Debug)]
pub struct Lobe {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

/// Shading and hollowing options for [`lobes`].
#[derive(Clone, Copy, Debug)]
pub struct LobesOpt {
    pub lx: f64,
    pub ly: f64,
    pub hollow: f64,
    pub shadow: f64,
    pub seed: f64,
}

impl Default for LobesOpt {
    fn default() -> Self {
        Self {
            lx: -0.6,
            ly: -0.8,
            hollow: 0.0,
            shadow: 0.35,
            seed: 7.0,
        }
    }
}

/// A cluster of shaded lobes (smoke, fire, dust) with a lit cap toward `(lx, ly)` and a shadow side, eating each
/// lobe from inside as `hollow` (`0..1`) grows so dying puffs break into crescents and dashes.
pub fn lobes(p: &mut Painter, list: &[Lobe], ramp: &[Rgba], opt: LobesOpt) {
    if list.is_empty() || ramp.is_empty() {
        return;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for l in list {
        x0 = x0.min(l.x - l.r);
        y0 = y0.min(l.y - l.r);
        x1 = x1.max(l.x + l.r);
        y1 = y1.max(l.y + l.r);
    }
    let len = or_one(libm::hypot(opt.lx, opt.ly));
    let (ux, uy) = (opt.lx / len, opt.ly / len);
    let n = ramp.len() as f64;
    p.each(x0, y0, x1, y1, |painter, x, y, sx, sy| {
        let mut best: Option<(usize, &Lobe, f64)> = None;
        for (i, l) in list.iter().enumerate() {
            if l.r <= 0.0 {
                continue;
            }
            let d = libm::hypot(x - l.x, y - l.y) / l.r;
            if d <= 1.0 && best.is_none_or(|(_, _, bd)| d < bd) {
                best = Some((i, l, d));
            }
        }
        let Some((idx, l, best_d)) = best else {
            return;
        };
        let (dx, dy) = ((x - l.x) / l.r, (y - l.y) / l.r);
        if opt.hollow > 0.0 {
            let hx = dx - ux * opt.hollow * 0.55;
            let hy = dy - uy * opt.hollow * 0.55;
            let hr = opt.hollow * 1.05;
            if hx * hx + hy * hy < hr * hr {
                return;
            }
            if opt.hollow > 0.85 {
                let ang = libm::atan2(dy, dx) + idx as f64 * 2.39996 + opt.seed;
                let freq = 3.0 + (idx % 3) as f64;
                if libm::sin(ang * freq) > lerp(1.0, -0.2, (opt.hollow - 0.85) / 0.15) {
                    return;
                }
            }
        }
        let facing = dx * ux + dy * uy;
        let v = clamp(
            (1.0 - best_d) * (1.0 - opt.shadow) + facing * opt.shadow * 1.2 + opt.shadow * 0.5,
            0.0,
            1.0,
        );
        let ci = clamp(libm::floor(v * n), 0.0, n - 1.0) as usize;
        if let Some(&c) = ramp.get(ci) {
            painter.put(sx, sy, c, 1.0);
        }
    });
}

/// A jagged lightning path from `(x0, y0)` to `(x1, y1)` by midpoint displacement. `jag` is how far a midpoint may
/// move sideways, as a share of the segment's length.
pub fn bolt(
    r: &mut Rng,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    jag: f64,
    depth: u32,
) -> Vec<(f64, f64)> {
    let mut pts = vec![(x0, y0), (x1, y1)];
    for _ in 0..depth {
        let mut next = Vec::with_capacity(pts.len() * 2 - 1);
        next.push(pts[0]);
        for pair in pts.windows(2) {
            let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
            let (dx, dy) = (bx - ax, by - ay);
            let l = libm::hypot(dx, dy);
            let off = (r.next() * 2.0 - 1.0) * jag * l;
            let l = if l == 0.0 || l.is_nan() { 1.0 } else { l };
            next.push((
                (ax + bx) / 2.0 + (-dy / l) * off,
                (ay + by) / 2.0 + (dx / l) * off,
            ));
            next.push(pair[1]);
        }
        pts = next;
    }
    pts
}

/// Forks off a bolt path: `count` branches, each up to `len` px long. Returns a list of paths.
pub fn forks(
    r: &mut Rng,
    path: &[(f64, f64)],
    count: usize,
    len: f64,
    jag: f64,
    spread: f64,
) -> Vec<Vec<(f64, f64)>> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let i = 1 + libm::floor(r.next() * (path.len() as f64 - 2.0)) as usize;
        let (Some(&(x, y)), Some(&(px, py))) = (path.get(i), path.get(i - 1)) else {
            break;
        };
        let a = libm::atan2(y - py, x - px)
            + (if r.next() < 0.5 { -1.0 } else { 1.0 }) * spread * (0.4 + 0.6 * r.next());
        let l = len * (0.4 + 0.6 * r.next());
        out.push(bolt(
            r,
            x,
            y,
            x + libm::cos(a) * l,
            y + libm::sin(a) * l,
            jag,
            3,
        ));
    }
    out
}

/// A star (pixelfx `star`): `n` points between radii `r1` (tips) and `r0` (inner corners), the first tip at angle
/// `rot` (pixelfx's defaults: `n = 5`, `rot = -PI / 2`, straight up).
pub fn star(cx: f64, cy: f64, r0: f64, r1: f64, n: u32, rot: f64) -> Vec<(f64, f64)> {
    (0..n * 2)
        .map(|i| {
            let a = rot + (f64::from(i) * core::f64::consts::PI) / f64::from(n);
            let r = if i % 2 == 1 { r0 } else { r1 };
            (cx + libm::cos(a) * r, cy + libm::sin(a) * r)
        })
        .collect()
}

/// A regular polygon with `n` sides (pixelfx `ngon`; its defaults: `n = 6`, `rot = 0`).
pub fn ngon(cx: f64, cy: f64, r: f64, n: u32, rot: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let a = rot + (f64::from(i) * TAU) / f64::from(n);
            (cx + libm::cos(a) * r, cy + libm::sin(a) * r)
        })
        .collect()
}

/// Positions of `pos(t - i * dt)` for i = 0..n-1, held at time 0: a moving thing's trail, newest first (pixelfx
/// `trail`).
pub fn trail(pos: impl Fn(f64) -> (f64, f64), t: f64, n: usize, dt: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| pos(js_max(0.0, t - i as f64 * dt)))
        .collect()
}
