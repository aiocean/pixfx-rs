//! Shared by the arcane effects (the leading _ keeps the tools from listing it as an effect).
//! A dithered light ramp, closed-form motes and streaks for loops, crackling arcs, floor rings and an original
//! rune alphabet. The dithered ramp (tone) is adapted from Sprite Fusion's open-source black hole
//! (https://www.spritefusion.com/pixel-art-vfx/black-hole), and the arc drawing from its energy
//! (https://www.spritefusion.com/pixel-art-vfx/energy), MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/arcane/_lib.js`: the functions rune-circle uses (`floorRing`, `glyph`, `mote`).

use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, scale};
use pixfx_draw::math::{js_max, js_min, js_round, or_one};

/// A drifting mote at (x, y) moving (vx, vy) px/s, drawn like the sim's MOTE: bright head, darker tail, light.
/// JS defaults: `size = 1`, `k = 1`, `glow = 0.4`.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn mote(
    p: &mut Painter,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    c: Rgba,
    size: f64,
    k: f64,
    glow: f64,
) {
    let h = libm::hypot(vx, vy);
    let sp = or_one(h);
    let (ux, uy) = (vx / sp, vy / sp);
    let len = js_min(16.0, 0.5 + sp * 0.026) * size;
    let a = if k < 0.35 { k / 0.35 } else { 1.0 };
    if len > 1.2 {
        p.line(
            x - ux * len,
            y - uy * len,
            x - ux * len * 0.35,
            y - uy * len * 0.35,
            scale(c, 0.55),
            a,
        );
    }
    p.line(x - ux * len * 0.4, y - uy * len * 0.4, x, y, c, a);
    if glow > 0.0 {
        p.glow(x, y, 2.5 + size, c, glow * k);
    }
}

/// A ring lying on the floor (or any tilted plane): centre (cx, cy), radius r, squashed by sy, 1 px thick at
/// any size. `c` is a function of the angle g around it (sin(g) > 0 is the near side, towards the viewer); the
/// alpha is the constant `a`. `keep(g)` picks which part to draw: `sin(g) < 0` is the far half, for drawBack.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn floor_ring(
    p: &mut Painter,
    cx: f64,
    cy: f64,
    r: f64,
    sy: f64,
    c: impl Fn(f64) -> Rgba,
    a: f64,
    keep: impl Fn(f64) -> bool,
) {
    if r < 0.5 {
        return;
    }
    let ry = r * sy;
    p.shade(
        cx - r - 1.0,
        cy - ry - 1.0,
        cx + r + 1.0,
        cy + ry + 1.0,
        |x, y, o| {
            let nx = (x - cx) / r;
            let ny = (y - cy) / js_max(0.3, ry);
            let d = libm::hypot(nx, ny);
            if d < 1e-6 {
                return None;
            }
            let g = libm::hypot(nx / r, ny / js_max(0.3, ry)) / d;
            if (d - 1.0).abs() / g > 0.5 {
                return None;
            }
            let ang = libm::atan2(ny, nx);
            if !keep(ang) {
                return None;
            }
            o.c = c(ang);
            o.a = a;
            None
        },
    );
}

/// An original rune alphabet: angular 3 x 5 glyphs that stay readable standing upright at 1x.
pub const GLYPHS: [[&str; 5]; 8] = [
    [".#.", "#.#", ".#.", ".#.", "#.#"],
    ["#.#", "#.#", ".#.", "#.#", "#.#"],
    ["##.", "#.#", "##.", "#..", "#.."],
    ["#..", "#.#", "###", "#.#", "..#"],
    [".##", "#..", ".#.", "..#", "##."],
    ["###", ".#.", "###", ".#.", ".#."],
    ["#.#", "###", "#.#", "..#", "..#"],
    [".#.", ".##", ".#.", "##.", ".#."],
];

fn sprite(p: &mut Painter, rows: &[&str; 5], x: f64, y: f64, tint: Rgba, a: f64) {
    for (r, row) in rows.iter().enumerate() {
        for (q, ch) in row.chars().enumerate() {
            if ch != '.' && ch != ' ' {
                p.dot(x + q as f64, y + r as f64, tint, a);
            }
        }
    }
}

/// Rune i standing on (x, y): its bottom row sits on y, centred on x. c body colour, a alpha (dithered), under an
/// optional darker colour (`Rgba::NONE` for none) drawn a pixel lower so the glyph has a little thickness.
pub fn glyph(p: &mut Painter, i: i64, x: f64, y: f64, c: Rgba, a: f64, under: Rgba) {
    let n = GLYPHS.len() as i64;
    let Some(g) = GLYPHS.get((((i % n) + n) % n) as usize) else {
        return;
    };
    let (gx, gy) = (js_round(x) - 1.0, js_round(y) - g.len() as f64 + 1.0);
    if under != Rgba::NONE {
        sprite(p, g, gx, gy + 1.0, under, a);
    }
    sprite(p, g, gx, gy, c, a);
}
