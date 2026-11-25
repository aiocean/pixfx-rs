//! Waterfall: port of pixelfx `src/fx/water/waterfall.js`.
//!
//! The pool's caustic light (two warped sine nets, light where they cross zero) is adapted from Sprite Fusion's
//! open-source bubbles (https://www.spritefusion.com/pixel-art-vfx/bubbles), MIT License, Copyright (c) 2026 Hugo
//! Duprez.

use core::f64::consts::PI;

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{WHITE, ramp};
use pixfx_draw::math::{TAU, clamp, ease, js_max, js_min, js_round, rand, smooth, tile_noise};

use super::lib::{CROSS, ROCK, Ramp, RingOpts, WATER, bead, billow, js_hypot, ring};

pub struct Waterfall;

pub static WATERFALL: Waterfall = Waterfall;

const HEIGHT: NumOpt = NumOpt::new("height", 92.0).within(20.0, 400.0);
const WIDTH: NumOpt = NumOpt::new("width", 13.0).within(1.0, 100.0);
/// 1 draws the rock outcrop the river pours from, 0 leaves it out (pixelfx `ledge: true`).
const LEDGE: NumOpt = NumOpt::new("ledge", 1.0);

const META: Meta = Meta {
    id: "water/waterfall",
    name: "Waterfall",
    desc: "A river pours from a notch in a rock outcrop: a white curl, a deep sheet that splits into strands, churning foam, spray, drifting mist and rings on a dark pool.",
    tags: &[
        "water",
        "waterfall",
        "cascade",
        "foam",
        "mist",
        "ambient",
        "loop",
    ],
    size: (144, 136),
    origin: (72, 118),
    duration: 2.0,
    looping: true,
    fps: 30,
    seed: 5,
    state: None,
    options: &[HEIGHT.spec(), WIDTH.spec(), LEDGE.spec()],
};

const D: f64 = 2.0;
const SY: f64 = 0.2;
const G: f64 = 760.0;
const POOL: (f64, f64) = (56.0, 11.0);

impl Effect for Waterfall {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let (h, w, s) = (opts.num(&HEIGHT), opts.num(&WIDTH), seed as f64);
        let ledge = opts.num(&LEDGE);
        let ledge = ledge != 0.0 && !ledge.is_nan();
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            h,
            w,
            s,
            rocks: if ledge {
                Some(RockMap::new(h, w, s))
            } else {
                None
            },
        })
    }
}

struct State {
    c: Ramp,
    h: f64,
    w: f64,
    s: f64,
    /// The outcrop's boulder map, built once (pixelfx caches it per width, height and seed); none without a ledge.
    rocks: Option<RockMap>,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            draw_back(p, t, self);
        }
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

// half the sheet's width at depth d below the lip: it spreads a little as it falls
fn half_w(w: f64, d: f64, h: f64) -> f64 {
    w + 3.2 * libm::pow(clamp(d / h, 0.0, 1.0), 1.6)
}

fn wrap1(x: f64) -> f64 {
    ((x % 1.0) + 1.0) % 1.0
}

// the falling sheet, in 1 px lanes. Lanes on the far side of the fall are dark and see-through, so the night shows
// between strands; the front lanes carry white dashes sliding down them, stretching as the water speeds up (the
// texture runs in fall time, √depth). The top is glassy and dark, and the lower third splits into strands that
// fan apart with gaps between, each lit down its left edge.
fn sheet(p: &mut Painter, c: &Ramp, ph: f64, h: f64, w: f64, s0: f64) {
    let mut y = -h + 2.0;
    while y <= -1.0 {
        let d = y + 0.5 + h;
        let v = d / h;
        let s = libm::sqrt(d + 3.0);
        let hw0 = half_w(w, d, h);
        let split = smooth(0.56, 0.95, v);
        let glass = 1.0 - smooth(4.0, 24.0, d);
        let wl = tile_noise(2.0, s * 1.4 - ph * 12.0, 0.0, 12.0, s0 + 2.0) * (0.4 + 1.8 * v);
        let wr = tile_noise(11.0, s * 1.4 - ph * 12.0, 0.0, 12.0, s0 + 2.0) * (0.4 + 1.8 * v);
        let fan = if split > 0.0 {
            tile_noise(3.0, s * 0.8 - ph * 4.0, 0.0, 4.0, s0 + 61.0) * 1.6
        } else {
            0.0
        };
        let sd = s * 1.5 - ph * 20.0;
        let sb = s * 0.45 - ph * 6.0;
        let mut xx = -w - 8.0;
        while xx <= w + 8.0 {
            let px = xx;
            xx += 1.0;
            let x = px + 0.5;
            let hw = hw0 + if x < 0.0 { wl } else { wr };
            let ax = libm::fabs(x);
            if ax > hw {
                continue;
            }
            let e = ax / hw;
            let mut edge = false;
            if split > 0.0 {
                let f = wrap1((x / (1.0 + 0.22 * split) + fan + 64.0) / 5.4);
                if f > 1.0 - 0.46 * split {
                    continue;
                }
                edge = f < 0.2;
            }
            let dash = tile_noise(px * 1.618 + 0.31, sd, 0.0, 20.0, s0);
            if rand(&[px, s0 + 60.0]) < 0.48 && e < 0.86 && !edge {
                p.blend(
                    x,
                    y + 0.5,
                    if dash > 0.45 {
                        c[4]
                    } else if glass > 0.5 {
                        c[3]
                    } else {
                        c[2]
                    },
                    if dash > 0.45 { 0.7 } else { 0.42 + 0.15 * v },
                );
                continue;
            }
            let broad = tile_noise(libm::floor((x + 40.0) / 3.0) * 2.7, sb, 0.0, 6.0, s0 + 1.0);
            let mut b = -0.34 + 0.46 * v + 0.3 * broad - glass * 0.5
                + 0.35 * smooth(0.7, 1.0, e) * (if x < 0.0 { 1.0 } else { 0.3 })
                - (if x > 0.0 { 0.12 * e } else { 0.0 })
                + (if edge { 0.3 } else { 0.0 });
            if dash > 0.2 - 0.22 * v {
                b += 0.36 + 0.7 * (dash - 0.2);
            }
            if b > -0.2 {
                let col = if b > 0.62 {
                    WHITE
                } else if b > 0.34 {
                    c[6]
                } else if b > 0.08 {
                    c[5]
                } else {
                    c[4]
                };
                p.px(px, y, col);
            } else {
                p.blend(x, y + 0.5, c[3], 0.8);
            }
        }
        y += 1.0;
    }
    // strands peel off the edges and fall free
    cycle(
        ph * D,
        D,
        16,
        s0 + 30.0,
        |_| 0.4,
        |age, life, r, _| {
            let side = r.sign();
            let d0 = r.range(14.0, h * 0.7);
            let d = d0 + 50.0 * age + 260.0 * age * age;
            let k = age / life;
            if d > h - 6.0 {
                return;
            }
            let x = side * (half_w(w, d, h) + 1.5 + age * r.range(6.0, 16.0));
            let y = -h + d;
            let len = 2.0 + 6.0 * k;
            let (c4, c6) = (c[4], c[6]);
            p.line(
                x,
                y - len,
                x,
                y,
                |u: f64| if u > 0.7 { c6 } else { c4 },
                1.0 - k * 0.5,
            );
        },
    );
}

// [x, y, rx, ry]: a big boulder left of the notch, a smaller one right, the sill the water runs over, two feet,
// and a slab behind that joins them, the notch cut into it
fn lumps(w: f64, h: f64) -> [[f64; 4]; 6] {
    [
        [-w - 11.0, -h - 4.0, 13.0, 10.0],
        [w + 8.0, -h - 2.0, 9.0, 7.5],
        [0.0, -h + 3.5, w + 3.0, 4.2],
        [-w - 21.0, -h + 4.0, 7.0, 5.5],
        [w + 15.0, -h + 4.5, 5.0, 4.0],
        [-2.0, -h - 5.0, w + 7.0, 6.5],
    ]
}

const LX: f64 = -0.55;
const LY: f64 = -0.72;
const LZ: f64 = 0.42;

// the outcrop the river pours from: boulders shaded like the explosion's rocks (a lit top edge, shaded faces, a
// dark underside, a dark rim), creased where they meet, cracked; dark and glossy where the water runs over them.
// Which boulder is frontmost at each pixel of the outcrop, and how much light its face catches there.
struct RockMap {
    x0: f64,
    y0: f64,
    w: f64,
    hgt: f64,
    id: Vec<i8>,
    /// pixelfx keeps it in a Float32Array
    lit: Vec<f32>,
}

impl RockMap {
    fn new(h: f64, w: f64, s: f64) -> Self {
        let lumps = lumps(w, h);
        let (x0, x1, y0, y1) = (-w - 29.0, w + 21.0, -h - 15.0, -h + 10.0);
        let (mw, hgt) = (x1 - x0 + 1.0, y1 - y0 + 1.0);
        let n = (mw * hgt) as usize;
        let mut id = vec![-1i8; n];
        let mut lit = vec![0f32; n];
        let mut j = 0.0;
        while j < hgt {
            let mut i = 0.0;
            while i < mw {
                let (x, y) = (x0 + i + 0.5, y0 + j + 0.5);
                let (mut best, mut z, mut l) = (-1i8, -1e9, 0.0);
                for (k, &[cx, cy, rx, ry]) in lumps.iter().enumerate() {
                    let (nx, ny) = ((x - cx) / rx, (y - cy) / ry);
                    let r2 = nx * nx + ny * ny;
                    if r2 >= 1.0 {
                        continue;
                    }
                    let nz = libm::sqrt(1.0 - r2);
                    let depth = nz * ry
                        + match k {
                            5 => -6.0,
                            2 => -3.0,
                            k if k > 2 => -1.0,
                            _ => 0.0,
                        };
                    if depth > z {
                        z = depth;
                        best = k as i8;
                        l = nx * LX + ny * LY + nz * LZ;
                    }
                }
                let at = (j * mw + i) as usize;
                if let Some(v) = id.get_mut(at) {
                    *v = best;
                }
                if let Some(v) = lit.get_mut(at) {
                    *v = (l + 0.2 * tile_noise(x * 0.32, y * 0.32, 0.0, 0.0, s + 70.0)) as f32;
                }
                i += 1.0;
            }
            j += 1.0;
        }
        Self {
            x0,
            y0,
            w: mw,
            hgt,
            id,
            lit,
        }
    }

    fn at(&self, i: f64, j: f64) -> i8 {
        if i < 0.0 || j < 0.0 || i >= self.w || j >= self.hgt {
            return -1;
        }
        self.id
            .get((j * self.w + i) as usize)
            .copied()
            .unwrap_or(-1)
    }

    fn lit(&self, i: f64, j: f64) -> f64 {
        f64::from(
            self.lit
                .get((j * self.w + i) as usize)
                .copied()
                .unwrap_or(0.0),
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn outcrop(p: &mut Painter, c: &Ramp, ph: f64, h: f64, w: f64, s: f64, map: &RockMap) {
    let st = ROCK;
    let lumps = lumps(w, h);
    let (x0, y0, mw, hgt) = (map.x0, map.y0, map.w, map.hgt);
    // a soft shadow under it on whatever is behind
    p.each(-w - 26.0, -h, w + 18.0, -h + 20.0, |p, x, y, sx, sy| {
        let k = 1.0 - js_hypot((x + 4.0) / (w + 22.0), (y + h) / 20.0);
        if k > 0.0 {
            p.blend_px(sx, sy, st[0], 0.4 * js_min(1.0, k * 3.0));
        }
    });
    let mut j = 0.0;
    while j < hgt {
        let mut i = 0.0;
        while i < mw {
            let (ii, jj) = (i, j);
            i += 1.0;
            let k = map.at(ii, jj);
            let (x, y) = (x0 + ii, y0 + jj);
            // the river runs over the sill: nothing of the sill shows through it
            if (k == 2 || k == 5 || k < 0)
                && y >= -h - 5.0
                && y < -h + 1.0
                && libm::fabs(x + 0.5) < w + 1.5
            {
                continue;
            }
            if k < 0 {
                if map.at(ii - 1.0, jj) >= 0
                    || map.at(ii + 1.0, jj) >= 0
                    || map.at(ii, jj - 1.0) >= 0
                    || map.at(ii, jj + 1.0) >= 0
                {
                    p.px(x, y, st[0]);
                }
                continue;
            }
            // a crease where one boulder sits in front of another
            let (r, b) = (map.at(ii + 1.0, jj), map.at(ii, jj + 1.0));
            if (r >= 0 && r != k && r < k) || (b >= 0 && b != k) {
                p.px(x, y, st[1]);
                continue;
            }
            let l = map.lit(ii, jj);
            let wet = libm::fabs(x + 0.5) < w + 6.0 && y > -h - 9.0;
            let mut col = if l > 0.9 {
                st[5]
            } else if l > 0.64 {
                st[4]
            } else if l > 0.3 {
                st[3]
            } else if l > -0.05 {
                st[2]
            } else {
                st[1]
            };
            if k == 5 {
                col = if l > 0.7 {
                    st[3]
                } else if l > 0.2 {
                    st[2]
                } else {
                    st[1]
                };
            }
            if map.at(ii, jj - 1.0) < 0 && l > 0.25 {
                col = st[5];
            }
            if wet {
                // wet stone: a step darker, a cool gloss running down it where the light catches
                col = if l > 0.9 {
                    st[4]
                } else if l > 0.64 {
                    st[3]
                } else if l > 0.3 {
                    st[2]
                } else {
                    st[1]
                };
                if l > 0.3 && tile_noise(x * 0.9, y * 0.6 - ph * 8.0, 0.0, 8.0, s + 71.0) > 0.42 {
                    col = if l > 0.6 { c[6] } else { c[4] };
                }
            }
            p.px(x, y, col);
        }
        j += 1.0;
    }
    // cracks: dark lines with a lit lip above them
    for q in 0..3usize {
        let qf = q as f64;
        let Some(l) = lumps.get(q % 2) else {
            continue;
        };
        let cx = libm::floor(l[0] + (if q == 2 { -0.5 } else { 0.15 }) * l[2]);
        let cy = libm::floor(l[1] + l[3] * (if q == 2 { 0.1 } else { -0.25 }));
        let len = 3.0 + libm::floor(rand(&[qf, s + 74.0]) * 4.0);
        let dx = if rand(&[qf, s + 75.0]) < 0.5 {
            1.0
        } else {
            -1.0
        };
        let mut n = 0.0;
        while n <= len {
            let (x, y) = (cx + js_round(n * 0.6) * dx, cy + n);
            let (i, j) = (x - x0, y - y0);
            if map.at(i, j) < 0 || map.at(i, j) > 1 || map.at(i, j + 1.0) < 0 {
                break;
            }
            p.px(x, y, if n == len { st[1] } else { st[0] });
            if n == 0.0 {
                p.px(x - dx, y - 1.0, st[4]);
            }
            n += 1.0;
        }
    }
    // trickles run down the wet faces either side of the notch
    for sx in [-w - 3.0, w + 3.0] {
        let mut y = -h - 2.0;
        while y < -h + 7.0 {
            let g = wrap1(y * 0.35 - ph * 6.0);
            let (i, j) = (libm::floor(sx) - x0, libm::floor(y) - y0);
            if map.at(i, j) >= 0 {
                p.px(sx, y, if g < 0.3 { c[6] } else { c[3] });
            }
            y += 1.0;
        }
    }
}

// the river sliding out of the notch between the boulders, seen from just above: flow lines sliding to the edge
fn river(p: &mut Painter, c: &Ramp, ph: f64, h: f64, w: f64, s: f64) {
    let top = -h - 6.0;
    p.shade(-w - 3.0, top, w + 3.0, -h, |x, y, o| {
        let back = top + 1.5 + tile_noise(x * 0.3, 4.0, 0.0, 0.0, s + 62.0) * 1.5;
        let v = (y - back) / (-h - back);
        let hw = w + 2.0 - (1.0 - v) * 3.0;
        if y < back || libm::fabs(x) > hw {
            return None;
        }
        let n = tile_noise(x * 0.32, (y - top) * 0.7 - ph * 8.0, 0.0, 8.0, s + 3.0);
        o.c = if n > 0.66 {
            c[6]
        } else if n > 0.4 {
            c[5]
        } else if v < 0.5 {
            c[3]
        } else {
            c[4]
        };
        None
    });
}

// the curl over the edge: glassy blue as it rolls, a hard white line along the bend; its light on the stone
fn curl(p: &mut Painter, c: &Ramp, ph: f64, h: f64, w: f64, s: f64) {
    let mut x = -w;
    while x <= w {
        let g = tile_noise(x * 0.45, ph * 4.0, 0.0, 4.0, s + 4.0);
        p.px(x, -h - 1.0, c[4]);
        p.px(x, -h, WHITE);
        p.px(x, -h + 1.0, if g > 0.2 { c[6] } else { c[5] });
        x += 1.0;
    }
    p.glow_line(-w, -h, w, -h, 6.0, c[6], 0.18);
    p.glow(0.0, -h, 34.0, c[5], 0.2);
    // glints run along it
    cycle(
        ph * D,
        D,
        4,
        s + 5.0,
        |_| 0.36,
        |age, life, r, _| {
            let k = libm::sin((PI * age) / life);
            let x = r.range(-w, w);
            if k > 0.5 {
                for (dx, dy) in CROSS {
                    p.px(x + dx, -h + dy, c[6]);
                }
            }
            p.glow(x, -h, 6.0, c[5], 0.4 * k);
        },
    );
}

// the pool: dark see-through water, a lit far rim, caustic light swimming in it, the falls' reflection
fn pool(p: &mut Painter, c: &Ramp, ph: f64, w: f64, s: f64) {
    let (rx, ry) = POOL;
    let cy = 2.0;
    let t = ph * TAU;
    let mut cols = Vec::new();
    let mut xx = -rx;
    while xx < rx {
        let x = xx + 0.5;
        cols.push((
            libm::sin(x * 0.11 - t * 2.0),
            libm::sin(t + x * 0.05) * 1.4 + x * 0.13,
        ));
        xx += 1.0;
    }
    let mut yy = libm::floor(cy - ry);
    while yy < cy + ry {
        let y = yy + 0.5;
        let dy = (y - cy) / ry;
        let sy = libm::sin(y * 0.5 + t);
        let mut xx = -rx;
        while xx < rx {
            let x = xx + 0.5;
            let idx = (xx + rx) as usize;
            xx += 1.0;
            let dx = x / rx;
            let d = dx * dx + dy * dy;
            if d > 1.0 {
                continue;
            }
            p.blend(
                x,
                y,
                if d < 0.12 { c[2] } else { c[1] },
                0.62 * js_min(1.0, (1.0 - d) * 4.0),
            );
            // caustics: two warped sine nets, light where either crosses zero
            let (cx1, cx2) = cols.get(idx).copied().unwrap_or((0.0, 0.0));
            let u = x * 0.2 + 1.8 * sy;
            let ww = y * 0.55 + 1.8 * cx1;
            let f = (js_max(0.0, 1.0 - libm::fabs(libm::sin(u) + libm::sin(ww)) * 2.6)
                + 0.6 * js_max(0.0, 1.0 - libm::fabs(libm::sin(cx2 + y * 0.3)) * 3.0))
                * (1.0 - d);
            if f > 0.5 {
                p.light(x, y, c[5], 0.18 * f);
            }
        }
        yy += 1.0;
    }
    // a few glints sit on the far shore
    for i in 0..7 {
        let i = f64::from(i);
        let a = PI * (1.12 + 0.76 * ((i + 0.5) / 7.0));
        let (x, y) = (libm::cos(a) * rx * 0.96, cy + libm::sin(a) * ry * 0.96);
        let g = libm::sin(ph * TAU * 2.0 + i * 2.4);
        if g > 0.2 {
            p.px(x, y, if g > 0.8 { c[6] } else { c[4] });
        }
    }
    // the falls mirrored in the water, broken by the chop
    p.shade(-w - 3.0, 3.0, w + 3.0, 10.0, |x, y, o| {
        let n = tile_noise(x * 0.9, (y - 3.0) * 0.35 + ph * 6.0, 0.0, 6.0, s + 6.0);
        let k = 1.0 - (y - 3.0) / 7.0;
        if libm::fabs(x) > w + 2.0 - (y - 3.0) * 0.4 || n < 0.25 + (1.0 - k) * 0.4 {
            return None;
        }
        o.c = if n > 0.6 && k > 0.6 { c[6] } else { c[5] };
        None
    });
}

// the foot: a churning white band, billows rolling out of it, spray thrown clear landing with rings
fn foam(p: &mut Painter, c: &Ramp, t: f64, ph: f64, w: f64, s: f64) {
    let rr = w + 10.0;
    p.shade(-rr, -13.0, rr, 3.0, |x, y, o| {
        let ax = libm::fabs(x);
        let lump = tile_noise(x * 0.22, ph * 6.0, 0.0, 6.0, s + 7.0) * 3.0
            + tile_noise(x * 0.55 + 9.0, ph * 12.0, 0.0, 12.0, s + 8.0) * 1.5;
        let top = -8.0 + lump + libm::pow(ax / rr, 2.0) * 7.0;
        if ax > rr - js_max(0.0, -y - 1.0) * 0.6 || y < top {
            return None;
        }
        // the churn in lumps: each boil lit on top, shadowed under, darker where it meets the pool
        let n = tile_noise(x * 0.5, (y - top) * 0.5 - ph * 10.0, 0.0, 10.0, s + 9.0);
        let boil = tile_noise(x * 0.42 + 3.0, y * 0.42 - ph * 8.0, 0.0, 8.0, s + 12.0);
        let u = y - top;
        o.c = if y > 1.0 {
            if n > 0.0 { c[5] } else { c[4] }
        } else if u < 1.2 {
            WHITE
        } else if boil < -0.3 {
            if u > 4.0 { c[4] } else { c[5] }
        } else if boil < -0.05 {
            c[6]
        } else if n > -0.2 {
            WHITE
        } else {
            c[6]
        };
        None
    });
    cycle(
        t,
        D,
        30,
        s + 10.0,
        |_| 0.7,
        |age, life, r, _| {
            let k = age / life;
            let side = r.sign();
            let x0 = r.range(-w, w) * 0.9;
            let sp = r.range(2.0, 8.0);
            let up = r.range(2.0, 8.0);
            let x = x0 + side * ease::out_cubic(k) * sp;
            let y = -r.range(2.0, 5.0) - ease::out_cubic(k) * up;
            let rad = r.range(1.4, 3.0) * libm::pow(libm::sin(PI * k), 0.7);
            if rad > 0.6 {
                billow(p, x, y, rad, c, 1.0);
            }
        },
    );
    // spray: beads thrown clear of the churn, landing in the pool with a ring
    cycle(
        t,
        D,
        18,
        s + 11.0,
        |_| 1.0,
        |age, _life, r, _| {
            let side = r.sign();
            let x0 = side * r.range(2.0, w + 4.0);
            let vx = side * r.range(12.0, 60.0);
            let vy = -r.range(70.0, 150.0);
            let z = r.range(-3.0, 9.0);
            let sz = if r.next() < 0.3 { 2.0 } else { 1.0 };
            let land = z * SY;
            let tl = (-vy + libm::sqrt(vy * vy + 2.0 * G * (land + 5.0))) / G;
            if age < tl {
                bead(
                    p,
                    x0 + vx * age,
                    -5.0 + vy * age + 0.5 * G * age * age,
                    vx,
                    vy + G * age,
                    sz,
                    c,
                    1.0,
                );
            } else if age < tl + 0.35 {
                ring(
                    p,
                    x0 + vx * tl,
                    land,
                    1.0 + (age - tl) * 12.0,
                    SY,
                    1.0 - (age - tl) / 0.35,
                    c,
                    RingOpts {
                        n: 1.0,
                        ..RingOpts::default()
                    },
                );
            }
        },
    );
}

// mist: soft puffs rolled out of the churn that drift sideways along the pool, swell and thin, with a cool light
fn mist(p: &mut Painter, c: &Ramp, t: f64, s: f64, n: usize, seed: f64) {
    cycle(
        t,
        D,
        n,
        s + seed,
        |g| g.range(1.8, 2.8),
        |age, life, r, _| {
            let k = age / life;
            let side = r.sign();
            let drift = ease::out_cubic(k);
            let x = side * (r.range(3.0, 12.0) + drift * r.range(24.0, 48.0));
            let y = -r.range(2.0, 7.0) - k * r.range(3.0, 12.0);
            let rad = r.range(4.0, 6.0) + k * r.range(5.0, 9.0);
            let a = r.range(0.2, 0.3) * js_min(1.0, k / 0.12) * libm::pow(1.0 - k, 1.7);
            p.disc(x, y, rad, c[6], |d: f64, _| a * (1.0 - d * d));
            if a > 0.05 {
                p.glow(x, y, rad * 1.2, c[4], 0.35 * a);
            }
        },
    );
}

fn rings(p: &mut Painter, c: &Ramp, ph: f64, w: f64, half: f64) {
    for i in 0..3 {
        let i = f64::from(i);
        let f = (ph + i / 3.0) % 1.0;
        let glint = if half > 0.0 && f < 0.4 {
            1.0 - f * 2.5
        } else {
            0.0
        };
        ring(
            p,
            0.0,
            1.0,
            w + 6.0 + f * 38.0,
            SY,
            0.95 * libm::pow(1.0 - f, 1.5),
            c,
            RingOpts {
                n: 1.0,
                half,
                seed: i + 1.0,
                glint,
                ..RingOpts::default()
            },
        );
    }
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let (c, ph) = (&o.c, t / D);
    pool(p, c, ph, o.w, o.s);
    rings(p, c, ph, o.w, -1.0);
    mist(p, c, t, o.s, 7, 20.0);
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (c, ph, h, w, s) = (&o.c, t / D, o.h, o.w, o.s);
    let tt = ph * TAU;
    rings(p, c, ph, w, 1.0);
    river(p, c, ph, h, w, s);
    if let Some(map) = &o.rocks {
        outcrop(p, c, ph, h, w, s, map);
    }
    sheet(p, c, ph, h, w, s);
    curl(p, c, ph, h, w, s);
    foam(p, c, t, ph, w, s);
    mist(p, c, t, s, 5, 21.0);
    // glints wink on the falling water and in the spray
    cycle(
        t,
        D,
        12,
        s + 50.0,
        |g| g.range(0.2, 0.4),
        |age, life, r, _| {
            let k = libm::sin((PI * age) / life);
            let low = r.next() < 0.45;
            let d = if low {
                h - r.range(4.0, 16.0)
            } else {
                r.range(12.0, h - 16.0)
            };
            let x = if low {
                r.range(-w - 10.0, w + 10.0)
            } else {
                r.range(-0.8, 0.8) * half_w(w, d, h)
            };
            let y = -h + d - (age / life) * (if low { 6.0 } else { -10.0 });
            p.px(x, y, WHITE);
            if k > 0.55 {
                for (dx, dy) in CROSS {
                    p.px(x + dx, y + dy, c[6]);
                }
            }
            if k > 0.85 && r.next() < 0.5 {
                for (dx, dy) in CROSS {
                    p.px(x + dx * 2.0, y + dy * 2.0, c[5]);
                }
            }
            p.glow(x, y, 5.0, c[5], 0.35 * k);
        },
    );
    // fine spray hangs in the air off the foot, drifting up and out
    cycle(
        t,
        D,
        34,
        s + 51.0,
        |g| g.range(0.9, 1.6),
        |age, life, r, _| {
            let k = age / life;
            let side = r.sign();
            let x = side * (r.range(0.0, w + 6.0) + k * r.range(6.0, 22.0))
                + libm::sin(age * 5.0 + r.range(0.0, TAU)) * 1.2;
            let y = -r.range(3.0, 10.0) - k * r.range(10.0, 34.0);
            if k > 0.7 && r.next() < (k - 0.7) * 3.3 {
                return;
            }
            p.px(
                x,
                y,
                if k < 0.4 && r.next() < 0.5 {
                    WHITE
                } else {
                    c[6]
                },
            );
        },
    );
    // light: down the sheet, a pool of it where the water lands, on the water around
    p.glow_line(0.0, -h + 6.0, 0.0, -8.0, 26.0, c[4], 0.12);
    p.glow(0.0, -5.0, 40.0, c[5], 0.32 + 0.05 * libm::sin(tt * 2.0));
    p.glow(0.0, 4.0, 50.0, c[3], 0.14);
}
