//! Shared by the impacts (the leading _ keeps the tools from listing it as an effect).
//! The hit star and a ring that breaks into flying arcs. The star and flying-arc recipes are adapted from
//! Sprite Fusion's open-source impact (https://www.spritefusion.com/pixel-art-vfx/impact), MIT License,
//! Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/impacts/_lib.js`: the functions parry uses (`star`, `brokenRing`).

use pixfx_core::Painter;
use pixfx_core::color::Rgba;
use pixfx_core::math::{TAU, js_max, js_min};

/// One spike of a hit star: angle, length and root half-width.
#[derive(Clone, Copy, Debug)]
pub struct Spike {
    pub a: f64,
    pub len: f64,
    pub w: f64,
}

/// One arc of a broken ring: centre angle, gap (0..1), how fast it flies out, thickness scale (JS default 1).
#[derive(Clone, Copy, Debug)]
pub struct Seg {
    pub a: f64,
    pub gap: f64,
    pub dr: f64,
    pub w: f64,
}

/// A hit star: tapered spikes in three passes (a rim, a body that's paler near the root, a hot core) round a
/// hot heart. `sc` stretches the spikes, `th` thins them, `cols` are [rim, body, inner, core], `a` dithers it out.
#[allow(clippy::too_many_arguments)]
pub fn star(
    p: &mut Painter,
    x: f64,
    y: f64,
    list: &[Spike],
    sc: f64,
    th: f64,
    cols: [Rgba; 4],
    a: f64,
    heart: f64,
) {
    let [rim, body, inner, core] = cols;
    for pass in 0..3 {
        for s in list {
            let (c, sn, l) = (libm::cos(s.a), libm::sin(s.a), s.len * sc);
            let mut d = 0.0;
            while d <= l {
                if pass == 2 && d > l * 0.55 {
                    break;
                }
                let hw0 = s.w * th * (1.0 - d / l) * js_min(1.0, 0.5 + d / 3.0);
                let hw = match pass {
                    0 => hw0 + 0.8,
                    1 => hw0,
                    _ => hw0 * 0.45,
                };
                let col = match pass {
                    0 => rim,
                    1 if d < l * 0.35 => inner,
                    1 => body,
                    _ => core,
                };
                let mut o = -hw;
                while o <= hw {
                    p.dot(x + c * d - sn * o, y + sn * d + c * o, col, a);
                    o += 0.5;
                }
                d += 0.5;
            }
        }
    }
    let r = heart * sc * libm::sqrt(th);
    p.circle(x, y, r + 0.8, inner, a);
    p.circle(x, y, r, core, a);
}

/// A ring that breaks into arcs: `segs` evenly round the ring. `brk` 0 draws a whole ring `th` thick; as `brk`
/// grows each arc shrinks, tapers at both ends into a crescent and drifts out at its own speed. `c` is the
/// outer edge's colour, `c2` the inner's; `sy` squashes it onto a plane; nothing is drawn below `max_y`.
#[allow(clippy::too_many_arguments)]
pub fn broken_ring(
    p: &mut Painter,
    x: f64,
    y: f64,
    r: f64,
    th: f64,
    segs: &[Seg],
    brk: f64,
    c: Rgba,
    c2: Rgba,
    k: f64,
    sy: f64,
    max_y: f64,
) {
    if r <= 0.0 || k <= 0.0 {
        return;
    }
    let full = TAU / segs.len() as f64;
    for s in segs {
        let span = full * (1.0 - s.gap * brk) + 0.02;
        let rr = r * (1.0 + s.dr * brk);
        let m = js_max(4.0, libm::ceil(rr * span * 1.8)) as usize;
        for j in 0..=m {
            let u = j as f64 / m as f64;
            let ang = s.a + (u - 0.5) * span;
            let w = js_max(
                0.5,
                th * s.w * (1.0 - brk * libm::pow(libm::fabs(2.0 * u - 1.0), 1.6)),
            );
            let (ca, sa) = (libm::cos(ang), libm::sin(ang));
            let mut o = -w / 2.0;
            while o <= w / 2.0 + 0.01 {
                if y + sa * (rr + o) * sy < max_y {
                    let col = if o > -0.26 { c } else { c2 };
                    p.dot(x + ca * (rr + o), y + sa * (rr + o) * sy, col, k);
                }
                o += 0.5;
            }
        }
    }
}
