//! Fireflies: port of pixelfx `src/fx/weather/fireflies.js`.
//!
//! The firefly (a wandering point that glows up and fades on its own rhythm, close ones drawn with a dark body,
//! beating wings and a lantern that throws a big soft light, grass blades swaying underneath) is adapted from
//! Sprite Fusion's open-source fireflies (https://www.spritefusion.com/pixel-art-vfx/fireflies), MIT License,
//! Copyright (c) 2026 Hugo Duprez.

use pixfx_core::color::{Rgba, WHITE, mix, ramp};
use pixfx_core::math::{TAU, js_max, js_min, js_round, loop_noise, rand};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{clip, rows};

pub struct Fireflies;

pub static FIREFLIES: Fireflies = Fireflies;

const META: Meta = Meta {
    id: "weather/fireflies",
    name: "Fireflies",
    desc: "Fireflies wander over a meadow at night, each glowing up and fading on its own rhythm, trailing light and lighting the grass tips beneath them.",
    tags: &[
        "weather",
        "fireflies",
        "night",
        "glow",
        "meadow",
        "nature",
        "ambient",
        "loop",
    ],
    size: (320, 180),
    origin: (160, 170),
    duration: 12.0,
    looping: true,
    fps: 30,
    seed: 2,
};

// meta.options
const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x2a3a10),
    Rgba::hex24(0x5a7a1a),
    Rgba::hex24(0x9ad02a),
    Rgba::hex24(0xd6ff5a),
    Rgba::hex24(0xf4ffb8),
    Rgba::hex24(0xffffff),
];
const GRASS: [Rgba; 6] = [
    Rgba::hex24(0x070f0b),
    Rgba::hex24(0x0c1a12),
    Rgba::hex24(0x13281a),
    Rgba::hex24(0x1d3a24),
    Rgba::hex24(0x2b5232),
    Rgba::hex24(0x3f6e40),
];

const D: f64 = META.duration;
const FLIES: usize = 32;

/// `2 pi n t / D + ph`: `n` whole cycles a loop.
fn cyc(t: f64, n: f64, ph: f64) -> f64 {
    (TAU * n * t) / D + ph
}

/// A firefly's path and glow, both whole cycles a loop. `depth` 0 far .. 2 near.
struct Fly {
    i: f64,
    depth: u8,
    glow: f64,
    k: f64,
    cx: f64,
    cy: f64,
    ax: f64,
    ay: f64,
    n1: f64,
    n2: f64,
    ph1: f64,
    ph2: f64,
}

impl Fly {
    fn new(s: f64, i: usize, t: f64) -> Fly {
        let i = i as f64;
        let r = |n: f64| rand(&[s, i, n]);
        let depth: u8 = if i < 14.0 {
            0
        } else if i < 26.0 {
            1
        } else {
            2
        };
        let k = [0.5, 0.8, 1.0][usize::from(depth)];
        let cx = -150.0 + r(1.0) * 300.0;
        let cy = -16.0 - r(2.0) * if depth == 2 { 70.0 } else { 130.0 };
        let ax = (14.0 + r(3.0) * 26.0) * k;
        let ay = (8.0 + r(4.0) * 14.0) * k;
        let n1 = 1.0 + libm::floor(r(5.0) * 2.0);
        let n2 = 1.0 + libm::floor(r(6.0) * 3.0);
        // its rhythm: a quick rise, a hold, a slow fade, dark; some flash twice
        let per = D / (3.0 + libm::floor(r(9.0) * 4.0));
        let u = (((t / per + r(10.0)) % 1.0) + 1.0) % 1.0;
        let two = r(11.0) < 0.35;
        let pulse = |v: f64| {
            if v < 0.06 {
                v / 0.06
            } else if v < 0.14 {
                1.0
            } else if v < 0.4 {
                1.0 - (v - 0.14) / 0.26
            } else {
                0.0
            }
        };
        let glow = js_max(pulse(u), if two { pulse(u - 0.22) * 0.8 } else { 0.0 });
        Fly {
            i,
            depth,
            glow,
            k,
            cx,
            cy,
            ax,
            ay,
            n1,
            n2,
            ph1: r(7.0) * TAU,
            ph2: r(8.0) * TAU,
        }
    }

    fn pos(&self, tt: f64) -> (f64, f64) {
        (
            self.cx
                + self.ax * libm::sin(cyc(tt, self.n1, self.ph1))
                + 10.0 * self.k * loop_noise(tt / D, 1.5, self.i * 3.0),
            self.cy
                + self.ay * libm::sin(cyc(tt, self.n2, self.ph2))
                + 6.0 * self.k * loop_noise(tt / D, 1.5, self.i * 3.0 + 1.0),
        )
    }
}

impl Effect for Fireflies {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            g: ramp(&opts.ramp_or("grass", &GRASS)),
            s: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 6],
    g: [Rgba; 6],
    s: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        clip(p, &META);
        let (s, c, g) = (self.s, &self.c, &self.g);
        let flies: Vec<Fly> = (0..FLIES).map(|i| Fly::new(s, i, t)).collect();
        if layer == Layer::Back {
            // a cool night haze, so the sky's stars don't read as fireflies
            rows(p, &META, -170.0, 9.0, |y, row| {
                let k = (y + 170.0) / 180.0;
                let haze = mix(g[0], g[1], k);
                let a = (0.48 + 0.14 * k) as f32;
                for cell in row.iter_mut() {
                    cell.c = haze;
                    cell.a = a;
                    cell.b = true;
                }
            });
            grass(p, t, s, g, c, &flies, false);
            for f in flies.iter().filter(|f| f.depth < 2) {
                firefly(p, t, f, c);
            }
        } else {
            grass(p, t, s, g, c, &flies, true);
            for f in flies.iter().filter(|f| f.depth == 2) {
                firefly(p, t, f, c);
            }
        }
    }
}

fn firefly(q: &mut Painter, t: f64, f: &Fly, c: &[Rgba; 6]) {
    let (x, y) = f.pos(t);
    let g = f.glow;
    let (xi, yi) = (js_round(x), js_round(y));
    if f.depth == 0 {
        if g < 0.08 {
            return;
        }
        q.dot(
            xi,
            yi,
            if g > 0.6 { c[4] } else { c[3] },
            js_min(1.0, g * 1.6),
        );
        q.glow(x, y, 6.0, c[2], 0.45 * g);
        return;
    }
    // a short trail of light where it has just been, while it's lit
    if g > 0.3 {
        for j in 1..=4 {
            let j = f64::from(j);
            let (tx, ty) = f.pos(t - j * 0.05);
            q.dot(tx, ty, c[2], (g - 0.3) * (1.0 - j / 5.0) * 1.3);
        }
    }
    if f.depth == 1 {
        q.px(
            xi,
            yi,
            if g > 0.5 {
                c[5]
            } else if g > 0.15 {
                c[3]
            } else {
                c[1]
            },
        );
        if g > 0.4 {
            q.px(xi, yi + 1.0, c[2]);
        }
        q.glow(x, y, 9.0, c[2], 0.5 * g);
        q.glow(x, y, 3.0, c[4], 0.5 * g);
        return;
    }
    // a close one: a dark body, wings a blur above it, a lantern at its tail; a glint at the peak
    let (nx, _) = f.pos(t + 0.05);
    let dir = if nx >= x { 1.0 } else { -1.0 };
    let beat = libm::floor(t * 30.0) % 2.0;
    q.px(xi + dir, yi, c[0]);
    q.px(xi + dir * 2.0, yi, c[0]);
    let wing = mix(c[1], WHITE, 0.4);
    q.dot(xi, yi - 1.0 - beat, wing, 0.7);
    q.dot(xi + dir, yi - 1.0 - beat, wing, 0.7);
    q.px(xi, yi, if g > 0.3 { c[5] } else { c[1] });
    q.px(
        xi - dir,
        yi,
        if g > 0.5 {
            c[4]
        } else if g > 0.15 {
            c[3]
        } else {
            c[1]
        },
    );
    if g > 0.6 {
        q.px(xi - dir, yi + 1.0, c[3]);
    }
    if g > 0.7 {
        for (dx, dy) in [(0.0, -2.0), (0.0, 2.0), (-2.0, 0.0), (2.0, 0.0)] {
            q.dot(xi - dir + dx, yi + dy, c[3], (g - 0.7) * 3.0);
        }
    }
    q.glow(x, y, 28.0, c[2], 0.45 * g);
    q.glow(x, y, 10.0, c[4], 0.5 * g);
    q.glow(x, y, 4.0, c[5], 0.5 * g);
}

// the meadow: blades in two rows (the front row drawn over the low fireflies), swaying as the breeze rolls
// across, a few seed heads; blade tips near a lit firefly catch its light
fn grass(
    q: &mut Painter,
    t: f64,
    s: f64,
    g: &[Rgba; 6],
    c: &[Rgba; 6],
    flies: &[Fly],
    front: bool,
) {
    let lit: Vec<(f64, f64, f64)> = flies
        .iter()
        .filter(|f| f.glow > 0.25 && f.depth > 0)
        .map(|f| {
            let (x, y) = f.pos(t);
            (x, y, f.glow)
        })
        .collect();
    let n: usize = if front { 150 } else { 230 };
    let sd = s * 3.0 + if front { 1.0 } else { 0.0 };
    for i in 0..n {
        let i = i as f64;
        let r = |k: f64| rand(&[sd, i, k]);
        let x = -162.0 + (i + r(1.0)) * (324.0 / n as f64);
        let h = if front { 4.0 } else { 5.0 }
            + r(2.0) * if front { 8.0 } else { 11.0 }
            + if r(3.0) < 0.1 { 6.0 } else { 0.0 };
        let base = if front { 9.0 } else { 2.0 } - r(4.0) * 3.0;
        let sway = (1.2 + r(5.0)) * libm::sin(cyc(t, 2.0, -x * 0.03) + r(6.0))
            + 0.6 * libm::sin(cyc(t, 5.0, x * 0.07));
        let tone = if front {
            1 + libm::floor(r(7.0) * 3.0) as usize
        } else {
            1 + libm::floor(r(7.0) * 2.0) as usize
        };
        let mut tip_light = 0.0;
        for &(fx, fy, glow) in &lit {
            let d = libm::hypot(fx - (x + sway), fy - (base - h));
            if d < 28.0 {
                tip_light = js_max(tip_light, glow * (1.0 - d / 28.0));
            }
        }
        let mut row = 0.0;
        while row < h {
            let u = row / h;
            let col = if u > 0.8 && tip_light > 0.2 {
                if tip_light > 0.55 && u > 0.9 {
                    c[3]
                } else {
                    c[2]
                }
            } else if u > 0.7 {
                g.get((tone + 1).min(5)).copied().unwrap_or(Rgba::NONE)
            } else {
                g.get(tone).copied().unwrap_or(Rgba::NONE)
            };
            q.px(x + sway * u * u, base - row, col);
            row += 1.0;
        }
        if r(8.0) < 0.07 {
            let (sx, sy) = (x + sway, base - h);
            q.px(sx, sy - 1.0, if tip_light > 0.2 { c[3] } else { g[5] });
            q.px(sx - 1.0, sy, g[4]);
            q.px(sx + 1.0, sy, g[4]);
        }
    }
}
