//! Shared by the explosions. Not an effect (the leading _ keeps the tools away).
//! The fire balls, rocks and scorch are adapted from Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/explosions/_lib.js`: the functions the ported explosions use (`drawDirt`, `crater`,
//! `rock`, `dome`, `pool`, `cull`, `fadeEdges`).

use core::f64::consts::PI;

use pixfx_core::color::{Rgba, WHITE, scale};
use pixfx_core::math::{js_max, js_min, js_round, rand};
use pixfx_core::sim::{Bit, Kind, Particles};
use pixfx_core::{Layer, Painter};

pub const OUTLINE: Rgba = Rgba::hex24(0x1a1422);

/// Stone colours: a lit face, the body, a shaded face, and its outline (none when the caller turned outlines off:
/// [`pixfx_core::Options::outline_color`] over [`OUTLINE`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stone {
    pub light: Rgba,
    pub body: Rgba,
    pub shade: Rgba,
    pub edge: Option<Rgba>,
}

/// Drop particles that leave the box (local px), so nothing is drawn past the canvas.
pub fn cull(ps: &mut Particles, x0: f64, y0: f64, x1: f64, y1: f64) {
    ps.list_mut()
        .retain(|q| !(q.x < x0 || q.x > x1 || q.y < y0 || q.y > y1));
}

/// Soft particles (smoke, dust, vapour) fade out over w px as they near the sides x0, x1 and go before they reach
/// them; y1, a bottom edge, gets a thin w of 6 (keep things off it by design). pixelfx defaults: w 30, y1
/// `f64::INFINITY`.
pub fn fade_edges(ps: &mut Particles, x0: f64, x1: f64, w: f64, y1: f64) {
    ps.list_mut().retain_mut(|q| {
        if !matches!(q.kind, Kind::Smoke | Kind::Dust | Kind::Vapour) {
            return true;
        }
        let r = q.size * 1.65;
        let e = js_min((js_min(q.x - x0, x1 - q.x) - r) / w, (y1 - q.y - r) / 6.0);
        if e <= 0.0 {
            return false;
        }
        if e < 1.0 {
            q.life = js_min(q.life, q.max * 0.42 * e);
        }
        true
    });
}

/// A soft pool of light, an ellipse rx by ry at (x, y): light falling on the floor (keep ry small, it's perspective).
pub fn pool(p: &mut Painter, x: f64, y: f64, rx: f64, ry: f64, c: Rgba, k: f64) {
    if k <= 0.004 || rx < 1.0 || ry < 0.5 {
        return;
    }
    p.shade(x - rx, y - ry, x + rx, y + ry, |px, py, out| {
        let (dx, dy) = ((px - x) / rx, (py - y) / ry);
        let d = dx * dx + dy * dy;
        if d >= 1.0 {
            return None;
        }
        let f = 1.0 - libm::sqrt(d);
        out.l = c;
        out.k = f * f * k;
        None
    });
}

/// A rock (a Bits CHUNK): a lit side, a shaded side and an outline, turning as it spins. `heat` (0..1, pixelfx
/// `b.heat ?? 0`) makes it molten: its heart and shaded side glow through the fire ramp `fr` and it gives off
/// light, cooling to stone.
pub fn rock(p: &mut Painter, b: &Bit, st: &Stone, fr: &[Rgba], heat: f64) {
    let r = 0.6 + b.s * 0.75;
    let fade = js_min(1.0, b.life / 0.5);
    let lx = libm::cos(b.a) * r * 0.4;
    let ly = libm::sin(b.a) * r * 0.4;
    let f = |i: usize| fr.get(i).copied().unwrap_or(Rgba::NONE);
    p.shade(
        b.x - r - 1.0,
        b.y - r - 1.0,
        b.x + r + 1.0,
        b.y + r + 1.0,
        |x, y, out| {
            let (dx, dy) = (x - b.x, y - b.y);
            let d = libm::hypot(dx * 1.15, dy);
            if d > r + 0.9 {
                return None;
            }
            out.a = fade;
            if d > r {
                return st.edge;
            }
            let lit = dx * lx + dy * ly;
            if heat > 0.08 {
                let g = heat * (1.1 - d / (r + 0.5)) + if lit > 0.3 { heat * 0.45 } else { 0.0 };
                if g > 0.5 {
                    return Some(f(if g > 1.0 {
                        1
                    } else if g > 0.8 {
                        2
                    } else if g > 0.62 {
                        3
                    } else {
                        4
                    }));
                }
                if g > 0.32 {
                    return Some(f(5));
                }
            }
            Some(if lit < -0.4 {
                st.light
            } else if lit > 0.6 {
                st.shade
            } else {
                st.body
            })
        },
    );
    if heat > 0.05 {
        p.glow(b.x, b.y, r * 2.0 + 3.0, f(3), 0.32 * heat * fade);
    }
}

/// Draw a particle system's GRIT as dirt in flight: fast bits stretch into short streaks (a darker tail, the bit at
/// the head), slow ones are the usual 1 or 2 px chips. The rest of the system draws as usual. layer as in draw.
pub fn draw_dirt(p: &mut Painter, ps: &Particles, layer: Layer) {
    ps.draw_where(p, layer, |q| q.kind != Kind::Grit);
    if layer == Layer::Back {
        return;
    }
    for q in ps.list() {
        if q.kind != Kind::Grit || q.delay > 0.0 {
            continue;
        }
        let k = q.life / q.max;
        let sp = libm::hypot(q.vx, q.vy);
        if k < 0.25 && k * 4.0 < 0.5 {
            continue;
        }
        if sp > 140.0 {
            let l = js_min(5.0, sp * 0.012);
            let (ux, uy) = (q.vx / sp, q.vy / sp);
            let tail = scale(q.col, 0.7);
            p.line(
                q.x - ux * l,
                q.y - uy * l,
                q.x,
                q.y,
                |u: f64| if u < 0.5 { tail } else { q.col },
                1.0,
            );
        } else {
            p.px(q.x, q.y, q.col);
            if q.size >= 2.0 {
                p.px(q.x + 1.0, q.y, q.col);
                p.px(q.x, q.y + 1.0, scale(q.col, 0.8));
                p.px(q.x + 1.0, q.y + 1.0, scale(q.col, 0.7));
            }
        }
    }
}

/// A crater in the floor at x: a dark pit (an ellipse in perspective) with its far wall lit, dirt heaped in a lip on
/// either side, and loose earth scattered round it. `e`: an earth ramp (dark to bright, 5+ steps); r: half-width;
/// k: 0..1 how far it has opened; a: alpha (fade it out; pixelfx default 1).
pub fn crater(p: &mut Painter, e: &[Rgba], x: f64, r: f64, k: f64, a: f64) {
    if k <= 0.0 || a <= 0.0 {
        return;
    }
    let n = e.len();
    let c = |i: usize| e.get(i).copied().unwrap_or(Rgba::NONE);
    let rx = r * k;
    let ry = js_max(1.5, r * 0.17 * k);
    let mut py = -1.0;
    while py <= libm::ceil(ry) + 1.0 {
        let mut px = libm::floor(-rx - 1.0);
        while px <= libm::ceil(rx + 1.0) {
            let dx = (px + 0.5) / rx;
            let dy = (py + 0.5 - ry * 0.35) / ry;
            let d = libm::sqrt(dx * dx + dy * dy);
            if d <= 1.08 {
                // the pit: darkest at its far side under the lit rim, the near wall a step lighter
                let col = if d > 0.92 {
                    if dy < 0.0 { c(n.wrapping_sub(2)) } else { c(2) }
                } else if dy < -0.2 {
                    c(0)
                } else if dy < 0.35 {
                    c(1)
                } else {
                    c(2)
                };
                p.dot(x + px, py, col, a);
            }
            px += 1.0;
        }
        py += 1.0;
    }
    // the lip: dirt heaped up either side, lit on top, and a scatter of loose earth
    for s in [-1.0, 1.0] {
        for i in 0..9 {
            let fi = f64::from(i);
            let u = fi / 8.0;
            let bump = if rand(&[fi, if s > 0.0 { 3.0 } else { 4.0 }]) < 0.4 {
                1.0
            } else {
                0.0
            };
            let h = js_round(libm::sin(PI * u) * 2.2 * k + bump * k);
            let hx = x + s * (rx * 0.82 + u * r * 0.55);
            let mut j = 0.0;
            while j <= h {
                let col = if j == h {
                    c(n.wrapping_sub(2))
                } else if j == h - 1.0 {
                    c(3)
                } else {
                    c(2)
                };
                p.dot(hx, -j, col, a);
                j += 1.0;
            }
        }
    }
    for i in 0..26 {
        let fi = f64::from(i);
        let s = if rand(&[fi, 7.0]) < 0.5 { -1.0 } else { 1.0 };
        let d = rx * (1.05 + rand(&[fi, 8.0]) * 1.4);
        let y = js_round(rand(&[fi, 9.0]) * 3.0) - 1.0;
        if rand(&[fi, 10.0]) < k {
            let col = if rand(&[fi, 11.0]) < 0.5 { c(3) } else { c(2) };
            p.dot(x + s * d, y, col, a);
        }
    }
}

/// A hemisphere of white-hot air on the floor at x, radius r: an orange rim, a hot body, a white core; k 1 → 0 as
/// it dies. `f` is an 8-step fire palette (dark to bright).
pub fn dome(p: &mut Painter, f: &[Rgba], x: f64, r: f64, k: f64) {
    let c = |i: usize| f.get(i).copied().unwrap_or(Rgba::NONE);
    let e = 1.4 / r;
    p.shade(x - r - 1.5, -r - 1.5, x + r + 1.5, 0.5, |px, py, _| {
        let d = libm::hypot(px - x, py) / r;
        if d > 1.0 + e {
            return None;
        }
        Some(if d > 1.0 {
            c(2)
        } else if d > 0.8 {
            if k > 0.45 { c(6) } else { c(5) }
        } else if d > 0.5 {
            if k > 0.45 { WHITE } else { c(6) }
        } else if k > 0.2 {
            WHITE
        } else {
            c(6)
        })
    });
}
