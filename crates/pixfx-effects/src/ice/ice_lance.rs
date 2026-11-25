//! Ice lance: port of pixelfx `src/fx/ice/ice-lance.js`.
//!
//! The projectile recipe (a hot head, a trail that thins behind it, motes shed along the way, light riding the
//! head) is adapted from Sprite Fusion's open-source energy bolt (https://www.spritefusion.com/pixel-art-vfx/energy),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::particles::{cycle, fly};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{TAU, js_max};

use super::lib::{BladeOpts, PALETTE, blade};

pub struct IceLance;

pub static ICE_LANCE: IceLance = IceLance;

const META: Meta = Meta {
    id: "ice/ice-lance",
    name: "Ice lance",
    desc: "A faceted ice spear flying right: a light pulse runs through its core, rings of cold air shear off the tip, frost peels off into a wake and crystals shed behind it.",
    tags: &[
        "ice",
        "lance",
        "spear",
        "projectile",
        "directional",
        "magic",
        "loop",
    ],
    size: (176, 64),
    origin: (146, 32),
    duration: D,
    looping: true,
    fps: 30,
    seed: 13,
    state: None,
    options: &[LENGTH.spec()],
};

const LENGTH: NumOpt = NumOpt::new("length", 40.0);
const D: f64 = 0.8;

impl Effect for IceLance {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            len: opts.num(&LENGTH),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    len: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

/// The spear: a hexagonal shaft seen from the side, row by row (a white top edge, a lit upper facet, a bright
/// ridge, a cool lower facet, a deep keel, a dark rim), a long cutting point, a light pulse running through its
/// ridge to the tip, a glint sliding back along the top.
fn spear(p: &mut Painter, c: &[Rgba; 8], l: f64, pulse: f64, glint: f64) {
    let (tip, hh) = (16.0, 3.5);
    let half = |x: f64| {
        if x > -tip {
            hh * (-x / tip) + 0.3
        } else if x < -l + 3.0 {
            hh * (0.7 + 0.3 * ((x + l) / 3.0))
        } else {
            hh
        }
    };
    p.shade(-l - 1.0, -hh - 2.0, 1.0, hh + 2.0, |x, y, o| {
        if x > 0.0 || x < -l {
            return None;
        }
        let h = half(x);
        if libm::fabs(y) > h + 0.5 {
            if libm::fabs(y) < h + 1.5 {
                o.c = c[1];
            }
            return None;
        }
        let v = y / js_max(0.5, h);
        if x > -1.5 {
            return Some(WHITE);
        }
        // three faces lit from above: a pale top facet, a mid front facet with a bright ridge, a deep underside
        if v > -0.3 && v < 0.35 && libm::fabs(x - pulse) < 2.5 {
            return Some(WHITE);
        }
        if v < -0.3 && libm::fabs(x - glint) < 1.0 {
            return Some(WHITE);
        }
        Some(if v < -0.72 {
            WHITE
        } else if v < -0.36 {
            c[6]
        } else if v < -0.18 {
            c[7]
        } else if v < 0.38 {
            c[4]
        } else if v < 0.78 {
            c[2]
        } else {
            c[1]
        })
    });
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (c, s, l) = (&o.c, o.seed, o.len);
    let ph = t / D;
    // the wake: a ribbon of frost narrowing behind the spear, with cold light in it
    p.each(-l - 100.0, -12.0, -l + 4.0, 12.0, |p, x, y, sx, sy| {
        let u = (-l + 4.0 - x) / 104.0;
        let w = 4.5 * libm::pow(1.0 - u, 0.7) + 0.5 + 1.5 * libm::sin(u * 9.0 - ph * TAU * 2.0) * u;
        let v = libm::fabs(y) / w;
        if v >= 1.0 {
            return;
        }
        let a = (1.0 - v * v) * libm::pow(1.0 - u, 1.4);
        if a < 0.04 {
            return;
        }
        let col = if v < 0.25 && u < 0.4 {
            c[6]
        } else if v < 0.6 {
            c[5]
        } else {
            c[4]
        };
        p.blend_px(sx, sy, col, a * 0.7);
        if a > 0.3 {
            p.add(sx, sy, c[5], 0.12 * a);
        }
    });
    p.glow_line(-l - 60.0, 0.0, 0.0, 0.0, 14.0, c[4], 0.22);
    // frost peeling off the shaft into soft puffs left behind
    cycle(
        t,
        D,
        14,
        s,
        |g| g.range(0.35, 0.55),
        |age, life, r, _| {
            let x0 = r.range(-l + 2.0, -8.0);
            let y0 = r.range(-2.5, 2.5);
            let vx = -r.range(150.0, 230.0);
            let vy = r.sign() * r.range(8.0, 30.0);
            let (x, y) = fly(x0, y0, vx, vy, age, 0.0, 2.2);
            let k = age / life;
            let rad = 1.2 + 5.5 * k;
            let a = 0.5 * libm::pow(1.0 - k, 1.3);
            p.disc(
                x,
                y,
                rad,
                if k < 0.3 { c[6] } else { c[5] },
                |d: f64, _: f64| a * (1.0 - d * d),
            );
            p.glow(x, y, rad * 1.6, c[4], 0.1 * (1.0 - k));
        },
    );
    // crystals shed along the way: short tumbling shards with a white edge, glinting, falling back
    cycle(
        t,
        D,
        9,
        s + 1.0,
        |g| g.range(0.4, 0.6),
        |age, life, r, _| {
            let x0 = r.range(-l, -6.0);
            let y0 = r.range(-3.0, 3.0);
            let vx = -r.range(170.0, 260.0);
            let vy = r.range(-35.0, 35.0);
            let (x, y) = fly(x0, y0, vx, vy, age, 60.0, 1.2);
            let k = age / life;
            let a = r.range(0.0, TAU) + age * r.range(10.0, 18.0) * r.sign();
            let ll = r.range(1.2, 2.2);
            let (ca, sa) = (libm::cos(a), libm::sin(a));
            if k > 0.85 && (age * 30.0) % 2.0 < 1.0 {
                return;
            }
            p.line(
                x - ca * ll,
                y - sa * ll,
                x + ca * ll,
                y + sa * ll,
                c[5],
                1.0,
            );
            p.px(x + ca * ll, y + sa * ll, WHITE);
            if libm::cos(a * 2.0) > 0.9 {
                for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    p.px(x + dx, y + dy, c[6]);
                }
                p.glow(x, y, 6.0, c[5], 0.45);
            }
        },
    );
    // speed lines: the air streaming past above and below
    cycle(
        t,
        D,
        12,
        s + 2.0,
        |g| g.range(0.16, 0.26),
        |age, life, r, _| {
            let y = r.sign() * r.range(4.0, 13.0);
            let x = r.range(-20.0, 0.0) - age * r.range(280.0, 380.0);
            let ll = r.range(6.0, 16.0);
            let k = age / life;
            p.line(
                x,
                y,
                x + ll,
                y,
                |u: f64| if u > 0.7 { c[6] } else { c[4] },
                |u: f64| u * (1.0 - k) * 1.4,
            );
        },
    );
    // rings of cold air shearing off the point and sliding back along the shaft: the near arc bright, the far dim
    for i in 0..3 {
        let q = (ph * 3.0 + f64::from(i) / 3.0) % 1.0;
        let x = -4.0 - q * (l + 20.0);
        let ry = 4.5 + q * 8.0;
        let rx = 1.2 + q * 2.6;
        if q > 0.8 {
            continue;
        }
        for k in 0..40 {
            let an = (f64::from(k) / 40.0) * TAU;
            let (px, py) = (x + libm::cos(an) * rx, libm::sin(an) * ry);
            if libm::fabs(py) < 4.6 && px > -l - 2.0 {
                continue;
            }
            let col = if libm::cos(an) > 0.0 {
                if q < 0.35 { WHITE } else { c[6] }
            } else if q < 0.4 {
                c[5]
            } else {
                c[4]
            };
            p.px(px, py, col);
        }
        p.glow_ring(x, 0.0, ry * 0.75, 3.0, c[5], 0.14 * (1.0 - q));
    }
    // the ice fletching at the back: three shards fanning out behind the shaft
    for (dy, ll, w) in [(-1.0, 10.0, 4.0), (1.0, 9.0, 4.0), (-0.2, 6.0, 3.0)] {
        blade(
            p,
            -l + 3.0,
            dy * 1.5,
            -l + 3.0 - ll,
            dy * (2.0 + ll * 0.5),
            w,
            c,
            BladeOpts::default(),
        );
    }
    // the spear itself, a pulse of light running through it, a glint sliding back along the top
    let pulse = -l + ((ph * 2.0) % 1.0) * (l + 6.0);
    let glint = -((ph * 2.0 + 0.5) % 1.0) * l * 1.2;
    spear(p, c, l, pulse, glint);
    p.glow_line(-l, 0.0, 0.0, 0.0, 10.0, c[4], 0.1);
    p.glow(pulse, 0.0, 8.0, c[6], 0.3);
    p.glow(0.0, 0.0, 8.0, WHITE, 0.4);
    p.glow(-l * 0.4, 0.0, 30.0, c[3], 0.16);
}
