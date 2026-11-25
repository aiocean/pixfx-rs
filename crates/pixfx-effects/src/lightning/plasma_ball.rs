//! Plasma ball: port of pixelfx `src/fx/lightning/plasma-ball.js`.
//!
//! The re-struck filaments, the glass shell with its specular arc and the core follow Sprite Fusion's open-source
//! energy orb (https://www.spritefusion.com/pixel-art-vfx/energy), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{TAU, js_max, js_min, js_round, loop_noise, rng};

pub struct PlasmaBall;

pub static PLASMA_BALL: PlasmaBall = PlasmaBall;

const META: Meta = Meta {
    id: "lightning/plasma-ball",
    name: "Plasma ball",
    desc: "A glass globe on a dark pedestal: wandering plasma tendrils reach from the core to the glass and fan out in bright feet where they touch it, lighting the stand and floor.",
    tags: &["lightning", "electric", "plasma", "lamp", "prop", "loop"],
    size: (112, 104),
    origin: (56, 96),
    duration: 3.0,
    looping: true,
    fps: 30,
    seed: 6,
    state: None,
    options: &[TENDRILS.spec()],
};

const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x1a0828),
    Rgba::hex24(0x3b0f5e),
    Rgba::hex24(0x6a1d9c),
    Rgba::hex24(0xa03ad6),
    Rgba::hex24(0xd46cf2),
    Rgba::hex24(0xf2a8ff),
    Rgba::hex24(0xffe2ff),
    Rgba::hex24(0xffffff),
];
const TENDRILS: NumOpt = NumOpt::new("tendrils", 9.0);

const FPS: f64 = 30.0;
const N: f64 = 90.0;
const GX: f64 = 0.0;
const GY: f64 = -55.0;
const RG: f64 = 24.0;
const BASE: [Rgba; 6] = [
    Rgba::hex24(0x0d0b12),
    Rgba::hex24(0x17131f),
    Rgba::hex24(0x231d2d),
    Rgba::hex24(0x342b43),
    Rgba::hex24(0x4b3f60),
    Rgba::hex24(0x6b5c87),
];

impl Effect for PlasmaBall {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            n: opts.num(&TENDRILS),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    n: f64,
    seed: f64,
}

/// A tendril at phase `ph`: its end wanders over the glass, front ones end on the near face, back ones on the far.
struct Tendril {
    a: f64,
    front: bool,
    ex: f64,
    ey: f64,
    rim: bool,
    w1: f64,
    w2: f64,
}

fn tendril(i: f64, n: f64, ph: f64, seed: f64) -> Tendril {
    let a = ((i + 0.5) / n) * TAU + 0.9 * loop_noise(ph, 0.9, seed + i * 7.0);
    let z = loop_noise(ph, 0.7, seed + i * 7.0 + 3.0);
    let rho = RG * (0.72 + 0.28 * libm::sqrt(1.0 - js_min(1.0, z * z * 1.6)));
    Tendril {
        a,
        front: z > -0.15,
        ex: GX + libm::cos(a) * rho,
        ey: GY + libm::sin(a) * rho,
        rim: rho > RG * 0.93,
        w1: loop_noise(ph, 1.1, seed + i * 7.0 + 5.0),
        w2: loop_noise(ph, 1.3, seed + i * 7.0 + 6.0),
    }
}

/// The pedestal: a flared stand lit from above by the globe, a collar under the glass.
fn pedestal(p: &mut Painter, c: &[Rgba; 8], glow: f64) {
    p.shade(-18.0, -32.0, 18.0, 0.0, |x, y, _| {
        let hw = if y < -27.0 {
            7.0
        } else if y < -2.0 {
            9.0 + ((y + 27.0) / 25.0) * 6.0
        } else {
            16.0
        };
        if libm::fabs(x) > hw {
            return None;
        }
        let u = (x / hw + 1.0) / 2.0;
        if y < -27.0 {
            return Some(if y < -30.0 {
                if u < 0.6 { c[4] } else { BASE[5] }
            } else if u < 0.3 {
                BASE[5]
            } else if u < 0.75 {
                BASE[3]
            } else {
                BASE[1]
            });
        }
        if y >= -2.0 {
            return Some(if y < -1.0 {
                if u < 0.55 { BASE[4] } else { BASE[2] }
            } else {
                BASE[1]
            });
        }
        // glossy body: a lit flank, a specular stripe, the far side in shadow, plasma light on the upper rim
        let i: usize = if u < 0.1 {
            2
        } else if u < 0.2 {
            3
        } else if u < 0.3 {
            5
        } else if u < 0.42 {
            4
        } else if u < 0.6 {
            3
        } else if u < 0.8 {
            2
        } else if u < 0.92 {
            1
        } else {
            0
        };
        Some(if y < -24.0 && u < 0.5 {
            mix(BASE[i], c[4], 0.3 * glow)
        } else if y > -5.0 {
            BASE[i.saturating_sub(1)]
        } else {
            BASE[i]
        })
    });
    p.px(9.0, -9.0, c[5]);
    p.glow(9.0, -9.0, 3.0, c[5], 0.5);
}

fn draw_tendril(
    p: &mut Painter,
    c: &[Rgba; 8],
    q: &Tendril,
    i: f64,
    bright: bool,
    seed: f64,
    f: f64,
) {
    let sx = GX + libm::cos(q.a) * 4.5;
    let sy = GY + libm::sin(q.a) * 4.5;
    let (dx, dy) = (q.ex - sx, q.ey - sy);
    let l = libm::hypot(dx, dy);
    let (nx, ny) = (-dy / l, dx / l);
    let mut r = rng(&[seed, 17.0, i, f]);
    let m = js_max(4.0, js_round(l / 2.2));
    let mut pts = Vec::new();
    let mut j = 0.0;
    while j <= m {
        let u = j / m;
        let w =
            3.2 * q.w1 * libm::sin(core::f64::consts::PI * u) + 1.6 * q.w2 * libm::sin(TAU * u + i);
        let jit = if j != 0.0 && j < m {
            r.range(-0.7, 0.7)
        } else {
            0.0
        };
        pts.push((sx + dx * u + nx * (w + jit), sy + dy * u + ny * (w + jit)));
        j += 1.0;
    }
    // the foot: the filament fans into short fingers where it meets the glass
    let (bx, by) = pts[m as usize - 2];
    let mut paths: Vec<(Vec<(f64, f64)>, f64)> = vec![(pts, 1.0)];
    for k in 0..3 {
        let a = q.a + f64::from(k - 1) * r.range(0.25, 0.45);
        let l = r.range(2.5, 5.0);
        let mx = bx + libm::cos(a) * l * 0.5 + r.range(-0.6, 0.6);
        let my = by + libm::sin(a) * l * 0.5 + r.range(-0.6, 0.6);
        paths.push((
            vec![
                (bx, by),
                (mx, my),
                (bx + libm::cos(a) * l, by + libm::sin(a) * l),
            ],
            0.5,
        ));
    }
    let hot = r.next() < 0.2;
    for (q2, w) in &paths {
        let len = q2.len();
        for j in 0..len - 1 {
            let ((ax, ay), (bx, by)) = (q2[j], q2[j + 1]);
            let u = j as f64 / (len - 1) as f64;
            if bright && *w >= 1.0 {
                p.line(ax + 1.0, ay + 1.0, bx + 1.0, by + 1.0, c[3], 1.0);
            }
            let col = if !bright {
                if *w >= 1.0 { c[3] } else { c[2] }
            } else if *w < 1.0 || hot || u > 0.75 || u < 0.2 {
                c[6]
            } else {
                c[5]
            };
            p.line(ax, ay, bx, by, col, 1.0);
            if *w >= 1.0 && j % 3 == 0 {
                let k = if bright {
                    if hot { 0.4 } else { 0.24 }
                } else {
                    0.12
                };
                p.glow(ax, ay, if bright { 6.0 } else { 4.0 }, c[4], k);
            }
        }
    }
    // where it touches, the glass flares
    if bright {
        p.circle(q.ex, q.ey, 1.6, c[6], 1.0);
        p.px(q.ex, q.ey, WHITE);
        p.glow(q.ex, q.ey, 9.0, c[5], 0.55);
        if q.rim {
            for s in [-1.0, 1.0] {
                p.px(
                    GX + libm::cos(q.a + s * 0.09) * (RG - 0.5),
                    GY + libm::sin(q.a + s * 0.09) * (RG - 0.5),
                    c[6],
                );
            }
        }
    } else {
        p.glow(q.ex, q.ey, 6.0, c[4], 0.25);
    }
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, seed, n) = (&self.c, self.seed, self.n);
        let f = libm::floor(t * FPS + 1e-6) % N;
        let ph = f / N;
        let mut r = rng(&[seed, 3.0, f]);
        let pulse = 0.85 + 0.15 * libm::sin(ph * TAU * 6.0) + 0.1 * (r.next() - 0.5);
        // JS Array.from({ length: n }): the length is n truncated, none for a NaN or a negative n
        let count = if n > 0.0 { libm::floor(n) as usize } else { 0 };
        let ts: Vec<Tendril> = (0..count).map(|i| tendril(i as f64, n, ph, seed)).collect();
        // light the floor and the stand, then the stand
        p.glow(0.0, -6.0, 52.0, c[3], 0.22 * pulse);
        pedestal(p, c, pulse);
        // the glass: a faint tint over what's behind, the plasma's glow inside it
        p.each(GX - RG, GY - RG, GX + RG, GY + RG, |p, x, y, _, _| {
            let d = libm::hypot(x - GX, y - GY);
            if d < RG - 0.5 {
                p.blend(
                    x,
                    y,
                    if d > RG - 4.0 { c[1] } else { c[0] },
                    0.22 + 0.4 * libm::pow(d / RG, 4.0),
                );
            }
        });
        p.glow(GX, GY, RG + 14.0, c[3], 0.32 * pulse);
        p.glow_ring(GX, GY, RG - 1.0, 5.0, c[3], 0.22 * pulse);
        for (i, q) in ts.iter().enumerate() {
            if !q.front {
                draw_tendril(p, c, q, i as f64, false, seed, f);
            }
        }
        // the core: an electrode ball, pulsing
        let cr = 3.6 + 0.5 * pulse;
        p.circle(GX, GY, cr + 1.3, c[3], 1.0);
        p.circle(GX, GY, cr, c[5], 1.0);
        p.circle(GX - 0.6, GY - 0.6, cr * 0.55, c[6], 1.0);
        p.circle(GX - 0.8, GY - 0.8, cr * 0.25, WHITE, 1.0);
        p.glow(GX, GY, 16.0, c[5], 0.65 * pulse);
        for (i, q) in ts.iter().enumerate() {
            if q.front {
                draw_tendril(p, c, q, i as f64, true, seed, f);
            }
        }
        // the glass shell: a thin rim, a bright specular arc upper left, a faint reflection lower right
        p.ring(GX, GY, RG + 1.6, 1.0, c[1], 1.0);
        p.ring(
            GX,
            GY,
            RG + 0.6,
            1.1,
            |_v: f64, a: f64| {
                if a > 2.2 && a < 3.3 {
                    c[5]
                } else if a > 0.3 && a < 1.3 {
                    c[4]
                } else {
                    c[3]
                }
            },
            1.0,
        );
        p.arc(
            GX,
            GY,
            RG - 1.6,
            1.3,
            3.55,
            4.55,
            |u: f64, _: f64| if u > 0.25 && u < 0.7 { WHITE } else { c[6] },
            1.0,
        );
        let mut a = 0.35;
        while a < 1.15 {
            p.dot(
                GX + libm::cos(a) * (RG - 2.6),
                GY + libm::sin(a) * (RG - 2.6),
                c[4],
                0.85,
            );
            a += 0.045;
        }
        p.px(GX - RG * 0.42, GY - RG * 0.62, WHITE);
    }
}
