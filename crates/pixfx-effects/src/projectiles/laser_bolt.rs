//! Laser bolt: port of pixelfx `src/fx/projectiles/laser-bolt.js`.
//!
//! The banded body, travelling pulses and spiralling sparks follow Sprite Fusion's open-source energy beam
//! (https://www.spritefusion.com/pixel-art-vfx/energy), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::color::{Rgba, WHITE, ramp};
use pixfx_core::math::{TAU, js_max, js_min, js_round, rand};
use pixfx_core::sim::sparkle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::beat;

pub struct LaserBolt;

pub static LASER_BOLT: LaserBolt = LaserBolt;

const META: Meta = Meta {
    id: "projectiles/laser-bolt",
    name: "Laser bolt",
    desc: "A red laser bolt: a white-hot core in a rounded head, red bands tapering back, pulses racing down it, a tail that shimmers apart into wavering dashes.",
    tags: &[
        "projectile",
        "laser",
        "energy",
        "sci-fi",
        "directional",
        "loop",
    ],
    size: (128, 32),
    origin: (108, 16),
    duration: 0.5,
    looping: true,
    fps: 30,
    seed: 2,
};

// meta.options
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x2a0610),
    Rgba::hex24(0x5c0d1c),
    Rgba::hex24(0x9a1428),
    Rgba::hex24(0xd8243a),
    Rgba::hex24(0xff4d5e),
    Rgba::hex24(0xff8a94),
    Rgba::hex24(0xffd0d4),
    Rgba::hex24(0xffffff),
];
const LENGTH: f64 = 40.0;

impl Effect for LaserBolt {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            length: opts.num_or("length", LENGTH),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    length: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let (l, s, d) = (o.length, o.seed, META.duration);
    let ph = t / d;
    let fr = libm::floor(t * 30.0 + 1e-6);
    let left = -f64::from(META.origin.0) + 2.0;

    // the shimmering tail: dashes shed off the bolt's end, wavering, flickering and cooling as they fall behind
    beat(
        t,
        d,
        14,
        s,
        |r| r.range(0.18, 0.3),
        |age, life, r, _| {
            let k = age / life;
            let x0 = -l * 0.8 - age * r.range(150.0, 230.0);
            let len = r.range(3.0, 7.0) * (1.0 - k * 0.5);
            let y = 0.5
                + libm::sin(x0 * 0.25 + ph * TAU * 2.0) * (0.5 + k * 1.5)
                + r.range(-0.6, 0.6) * k;
            if x0 - len < left || rand(&[fr, r.int(0.0, 999.0)]) < k * 0.5 {
                return;
            }
            let col = if k < 0.3 {
                c[5]
            } else if k < 0.6 {
                c[4]
            } else {
                c[3]
            };
            p.line(x0, y, x0 - len, y, col, 1.0);
            if k < 0.4 {
                p.glow(x0 - len / 2.0, y, 4.0, c[4], 0.3 * (1.0 - k));
            }
        },
    );

    // the body: bands round a white core, thickest just behind the rounded head and tapering to a thread, a
    // pulse running back down it every so often; the edge breaks up a little as the light shimmers
    p.shade(-l - 1.0, -5.0, 4.0, 6.0, |x, y, _| {
        let d = -x + 3.0;
        let u = d / (l + 3.0);
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let head = if d < 3.0 {
            libm::sqrt(js_max(0.0, 1.0 - libm::pow((3.0 - d) / 3.0, 2.0)))
        } else {
            1.0
        };
        let w = (3.4 * libm::pow(1.0 - u, 0.9) + 0.5)
            * head
            * (1.0 + 0.08 * libm::sin(d * 0.9 - ph * TAU * 4.0));
        let v = libm::fabs(y - 0.5) / w;
        if v > 1.2 {
            return None;
        }
        if v > 1.0 && (u > 0.5 || rand(&[libm::floor(x), fr]) > 0.6) {
            return None;
        }
        let pulse = if (((d * 0.05 - ph * 3.0) % 1.0) + 1.0) % 1.0 < 0.12 {
            1.0
        } else {
            0.0
        };
        let lvl = if v < 0.22 {
            if u < 0.6 { 7.0 } else { 6.0 }
        } else if v < 0.45 {
            6.0 - if u > 0.5 { 1.0 } else { 0.0 }
        } else if v < 0.72 {
            4.0
        } else if v <= 1.0 {
            3.0
        } else {
            2.0
        };
        let i = js_min(7.0, lvl + if v < 0.72 { pulse } else { 0.0 });
        c.get(i as usize).copied()
    });
    // sparks spiralling round the core, a bright one on the near side and a dim one behind
    for s in [0.0, core::f64::consts::PI] {
        let mut d = 2.0;
        while d < l * 0.7 {
            let a = d * 0.45 - ph * TAU * 3.0 + s;
            let y = 0.5 + libm::sin(a) * (2.2 * (1.0 - d / l) + 0.6);
            if libm::cos(a) > 0.3 {
                p.dot(-d, y, if libm::cos(a) > 0.8 { WHITE } else { c[6] }, 1.0);
            }
            d += 1.0;
        }
    }
    // the head's glint, flickering, and the light
    let g = 0.75 + 0.25 * libm::sin(ph * TAU * 2.0);
    sparkle(p, 1.0, 0.5, 3.0 + js_round(g * 2.0), c[6], c[4], 0.5);
    p.glow_line(1.0, 0.5, -l, 0.5, 9.0, c[3], 0.4);
    p.glow(-2.0, 0.5, 20.0, c[3], 0.5 * g);
    p.glow(1.0, 0.5, 6.0, WHITE, 0.25);
}
