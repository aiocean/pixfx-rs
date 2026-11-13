//! Shared by the shadow effects: the void palette, light that black shapes cut. Not an effect (the leading _
//! keeps the tools away).
//!
//! Port of pixelfx `src/fx/shadow/_lib.js`: the functions the ported shadow effects use (`VOID`, `halo`,
//! `ringLight`, `ringFast`, `unlight`). `ringFast`'s colour and alpha and `ringLight`'s `keep` stay constants
//! here (the functions of the ring's radius and angle pixelfx also accepts have no caller yet).

use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::js_max;

/// Dark magic, dark to bright: void black, deep violet, a white-lilac edge.
pub const VOID: [Rgba; 8] = [
    Rgba::hex24(0x06040b),
    Rgba::hex24(0x150c26),
    Rgba::hex24(0x2a1150),
    Rgba::hex24(0x461a84),
    Rgba::hex24(0x6f2cc0),
    Rgba::hex24(0x9d5cec),
    Rgba::hex24(0xcfa6ff),
    Rgba::hex24(0xf6eaff),
];

/// Take the light back off every pixel in a local box where `dark(x, y)` is true, keeping a share `keep`
/// (0..1): call it last, so black shapes stay black under every glow, spark and ring drawn before.
pub fn unlight(
    p: &mut Painter,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    keep: f64,
    mut dark: impl FnMut(f64, f64) -> bool,
) {
    p.each(x0, y0, x1, y1, |p, x, y, sx, sy| {
        if dark(x, y) {
            p.dim_light(sx, sy, keep);
        }
    });
}

/// The pixel cells of the annulus `r0 <= d < r1` round (x, y), row by row, jumping the hole: `f(cx, cy, d)`.
fn annulus(x: f64, y: f64, r0: f64, r1: f64, mut f: impl FnMut(f64, f64, f64)) {
    let r02 = if r0 > 0.0 { r0 * r0 } else { -1.0 };
    let r12 = r1 * r1;
    let mut py = libm::floor(y - r1);
    while py <= libm::ceil(y + r1) {
        let cy = py + 0.5;
        let dy = cy - y;
        let dy2 = dy * dy;
        if dy2 >= r12 {
            py += 1.0;
            continue;
        }
        let xo = libm::sqrt(r12 - dy2);
        let xi = if dy2 < r02 {
            libm::sqrt(r02 - dy2)
        } else {
            0.0
        };
        let mut px = libm::floor(x - xo);
        let x1 = libm::ceil(x + xo);
        while px <= x1 {
            let cx = px + 0.5;
            let dx = cx - x;
            if xi > 0.0 && dx > -xi && dx < xi {
                px = js_max(px, libm::floor(x + xi) - 1.0);
                px += 1.0;
                continue;
            }
            let d2 = dx * dx + dy2;
            if d2 < r12 && d2 >= r02 {
                f(cx, cy, libm::sqrt(d2));
            }
            px += 1.0;
        }
        py += 1.0;
    }
}

/// A ring from radius `r - w` out to `r` round (x, y) in colour `c` at alpha `a`, visiting only the band's own
/// pixels, so a big thin shock ring stays cheap.
pub fn ring_fast(p: &mut Painter, x: f64, y: f64, r: f64, w: f64, c: Rgba, a: f64) {
    if r <= 0.0 {
        return;
    }
    let r0 = js_max(0.0, r - w);
    annulus(x, y, r0, r, |cx, cy, _| p.dot(cx, cy, c, a));
}

/// Light hugging a circle of radius `big_r` at (x, y), falling off over `w` either side.
pub fn ring_light(p: &mut Painter, x: f64, y: f64, big_r: f64, w: f64, c: Rgba, k: f64) {
    if w < 1.0 || k <= 0.004 {
        return;
    }
    annulus(x, y, js_max(0.0, big_r - w), big_r + w, |cx, cy, d| {
        let f = 1.0 - libm::fabs(d - big_r) / w;
        p.light(cx, cy, c, f * f * k);
    });
}

/// A ring of light hugging a black disc: `k` at radius `r0`, falling to nothing at `r1`, and none inside `r0`.
pub fn halo(p: &mut Painter, x: f64, y: f64, r0: f64, r1: f64, c: Rgba, k: f64) {
    if r1 <= r0 || k <= 0.004 {
        return;
    }
    let w = r1 - r0;
    annulus(x, y, js_max(0.0, r0), r1, |cx, cy, d| {
        let f = 1.0 - (d - r0) / w;
        p.light(cx, cy, c, f * f * k);
    });
}
