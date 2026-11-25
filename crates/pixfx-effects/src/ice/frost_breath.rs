//! Frost breath: port of pixelfx `src/fx/ice/frost-breath.js`.
//!
//! Soft puffs drawn as a body, a denser core and a lit cap, and streaks with a hot head and a cool tail, follow
//! the recipes in Sprite Fusion's open-source pixel art VFX (https://www.spritefusion.com/pixel-art-vfx),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::particles::{cycle, fly, fly_v};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{TAU, clamp, js_min, loop_noise, tile_fbm};

use super::lib::PALETTE;

pub struct FrostBreath;

pub static FROST_BREATH: FrostBreath = FrostBreath;

const META: Meta = Meta {
    id: "ice/frost-breath",
    name: "Frost breath",
    desc: "A cone of icy wind blowing right: frost puffs billow out, swell and sink, wind streaks race through, ice crystals tumble and glint off a white-hot mouth.",
    tags: &[
        "ice",
        "breath",
        "frost",
        "cone",
        "wind",
        "directional",
        "loop",
    ],
    size: (184, 104),
    origin: (12, 50),
    duration: D,
    looping: true,
    fps: 30,
    seed: 11,
    state: None,
    options: &[SPREAD.spec(), POWER.spec()],
};

const SPREAD: NumOpt = NumOpt::new("spread", 0.21);
const POWER: NumOpt = NumOpt::new("power", 1.0);
const D: f64 = 1.2;
/// Cold air is heavy: it sags a little as it slows.
const SINK: f64 = 9.0;

impl Effect for FrostBreath {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            spread: opts.num(&SPREAD),
            power: opts.num(&POWER),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    spread: f64,
    power: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        match layer {
            Layer::Back => draw_back(p, t, self),
            Layer::Front => draw(p, t, self),
            Layer::Both => {
                draw_back(p, t, self);
                draw(p, t, self);
            }
        }
    }
}

/// A puff of frost: a soft body, a denser core sitting low, a lit cap while it's young, a cold haze of light.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
fn puff(p: &mut Painter, c: &[Rgba; 8], x: f64, y: f64, r: f64, a: f64, k: f64, front: bool) {
    let body = if front { c[5] } else { c[4] };
    let cap = if front { c[7] } else { c[6] };
    let soft = |q: f64| move |d: f64, _: f64| q * (1.0 - d * d);
    p.disc(x, y, r, body, soft(a * 0.6));
    p.disc(
        x + r * 0.06,
        y + r * 0.12,
        r * 0.68,
        mix(body, c[3], 0.3),
        soft(a * 0.5),
    );
    if k < 0.55 {
        p.disc(
            x - r * 0.25,
            y - r * 0.3,
            r * 0.5,
            cap,
            soft(a * 0.7 * (1.0 - k / 0.55)),
        );
    }
    p.glow(x, y, r * 1.7, c[4], 0.1 * a);
}

/// The body of the gust: a cone of turbulent, translucent cold air streaming right, brightest at its heart. The
/// noise scrolls exactly one tile per loop, so it never seams.
fn gust(p: &mut Painter, c: &[Rgba; 8], t: f64, o: &State) {
    let ph = t / D;
    let sp = libm::tan(o.spread) * 1.08;
    let s = o.seed;
    p.each(2.0, -46.0, 170.0, 46.0, |p, x, y, sx, sy| {
        let u = x / 170.0;
        let hw = 2.5 + x * sp;
        let yc = 7.0 * u * u;
        let v = (y - yc) / hw;
        if v * v >= 1.0 || u >= 1.0 {
            return;
        }
        let n = tile_fbm(
            x * 0.055 - ph * 10.0,
            (y - yc) * 0.15 + 7.0,
            10.0,
            0.0,
            s,
            2,
        );
        let edge =
            (1.0 - v * v) * (if u < 0.08 { u / 0.08 } else { 1.0 }) * libm::pow(1.0 - u, 0.8);
        // denser and brighter down its heart near the mouth
        let d = clamp(
            0.5 + n * 1.3 + (1.0 - libm::fabs(v)) * 0.35 * libm::pow(1.0 - u, 1.5),
            0.0,
            1.0,
        ) * edge;
        if d < 0.08 {
            return;
        }
        let col = if d > 0.72 {
            c[7]
        } else if d > 0.52 {
            c[6]
        } else if d > 0.32 {
            c[5]
        } else {
            c[4]
        };
        p.blend_px(
            sx,
            sy,
            col,
            js_min(0.78, d * 0.95) * clamp((d - 0.08) / 0.3, 0.0, 1.0),
        );
        if d > 0.3 {
            p.add(sx, sy, if d > 0.6 { c[6] } else { c[5] }, 0.2 * d);
        }
    });
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let (c, s, sp) = (&o.c, o.seed, o.spread);
    gust(p, c, t, o);
    // the big slow billows behind
    cycle(
        t,
        D,
        12,
        s,
        |g| g.range(1.0, 1.25),
        |age, life, r, _| {
            let th = r.range(-sp, sp);
            let v = r.range(170.0, 230.0) * o.power;
            let y0 = r.range(-1.0, 1.0);
            let (x, y) = fly(
                3.0,
                y0,
                v * libm::cos(th),
                v * libm::sin(th),
                age,
                SINK,
                1.4,
            );
            let k = age / life;
            let rad = 2.0 + 12.0 * libm::pow(k, 0.75);
            puff(
                p,
                c,
                x,
                js_min(y, 40.0 - rad),
                rad,
                0.75 * libm::pow(1.0 - k, 1.2) * js_min(1.0, age / 0.08),
                k,
                false,
            );
        },
    );
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (c, s, sp) = (&o.c, o.seed, o.spread);
    let ph = t / D;
    // light down the cone
    p.glow_line(4.0, 0.0, 120.0, 6.0, 24.0, c[4], 0.26);
    p.glow_line(2.0, 0.0, 50.0, 1.0, 9.0, c[6], 0.3);
    p.glow(60.0, 4.0, 44.0, c[3], 0.16);
    // brighter puffs in front, smaller and quicker
    cycle(
        t,
        D,
        10,
        s + 1.0,
        |g| g.range(0.7, 0.95),
        |age, life, r, _| {
            let th = r.range(-sp * 0.8, sp * 0.8);
            let v = r.range(190.0, 260.0) * o.power;
            let y0 = r.range(-1.0, 1.0);
            let (x, y) = fly(
                3.0,
                y0,
                v * libm::cos(th),
                v * libm::sin(th),
                age,
                SINK,
                1.6,
            );
            let k = age / life;
            let rad = 1.5 + 8.0 * libm::pow(k, 0.7);
            puff(
                p,
                c,
                x,
                js_min(y, 40.0 - rad),
                rad,
                0.85 * libm::pow(1.0 - k, 1.3) * js_min(1.0, age / 0.06),
                k,
                true,
            );
        },
    );
    // snow carried along, swaying
    cycle(
        t,
        D,
        18,
        s + 2.0,
        |g| g.range(0.8, 1.1),
        |age, life, r, _| {
            let th = r.range(-sp * 1.1, sp * 1.1);
            let v = r.range(120.0, 190.0);
            let (x, y) = fly(
                4.0,
                0.0,
                v * libm::cos(th),
                v * libm::sin(th),
                age,
                SINK * 1.4,
                1.5,
            );
            let k = age / life;
            let sw = libm::sin(age * 9.0 + r.range(0.0, TAU)) * 1.5;
            if k < 0.85 || (age * 30.0) % 2.0 < 1.0 {
                let col = if r.chance(0.5) { WHITE } else { c[6] };
                p.px(x, y + sw, col);
            }
        },
    );
    // wind streaks: a white head, a pale body, a cool tail
    cycle(
        t,
        D,
        26,
        s + 3.0,
        |g| g.range(0.22, 0.36),
        |age, life, r, _| {
            let th = r.range(-sp * 0.9, sp * 0.9);
            let v = r.range(280.0, 380.0);
            let y0 = r.range(-2.0, 2.0);
            let (x, y) = fly(6.0, y0, v * libm::cos(th), v * libm::sin(th), age, 0.0, 0.9);
            let (vx, vy) = fly_v(v * libm::cos(th), v * libm::sin(th), age, 0.0, 0.9);
            let s = libm::hypot(vx, vy);
            let l = 4.0 + s * 0.03;
            let (ux, uy) = (vx / s, vy / s);
            let k = age / life;
            p.line(
                x - ux * l,
                y - uy * l,
                x,
                y,
                |u: f64| {
                    if u > 0.8 {
                        WHITE
                    } else if u > 0.4 {
                        c[6]
                    } else {
                        c[4]
                    }
                },
                |u: f64| {
                    (if u > 0.35 { 1.0 } else { u / 0.35 })
                        * (if k < 0.75 { 1.0 } else { (1.0 - k) / 0.25 })
                },
            );
            p.glow(x, y, 3.0, c[5], 0.3 * (1.0 - k));
        },
    );
    // ice crystals tumbling through: a short shard with a white edge, glinting as it turns
    cycle(
        t,
        D,
        9,
        s + 4.0,
        |g| g.range(0.65, 0.9),
        |age, life, r, _| {
            let th = r.range(-sp, sp);
            let v = r.range(150.0, 230.0);
            let (x, y) = fly(
                5.0,
                0.0,
                v * libm::cos(th),
                v * libm::sin(th),
                age,
                SINK,
                1.1,
            );
            let k = age / life;
            let a = r.range(0.0, TAU) + age * r.range(9.0, 15.0) * r.sign();
            let l = r.range(1.5, 2.6);
            let (ca, sa) = (libm::cos(a), libm::sin(a));
            if k > 0.85 && (age * 30.0) % 2.0 < 1.0 {
                return;
            }
            p.line(x - ca * l, y - sa * l, x + ca * l, y + sa * l, c[5], 1.0);
            p.px(x + ca * l, y + sa * l, WHITE);
            let gl = libm::cos(a * 2.0);
            if gl > 0.92 {
                for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    p.px(x + dx, y + dy, c[6]);
                }
                p.px(x, y, WHITE);
                p.glow(x, y, 6.0, c[5], 0.5);
            } else {
                p.glow(x, y, 3.0, c[5], 0.2);
            }
        },
    );
    // the mouth: a white-hot core, a flickering jet of cold
    for i in 0..3 {
        let fi = f64::from(i);
        let l = 9.0 + 6.0 * loop_noise(ph, 1.2, s + fi * 7.0);
        let dy = (fi - 1.0) * 1.4;
        p.line(
            2.0,
            dy * 0.5,
            2.0 + l,
            dy,
            |u: f64| if u < 0.5 { WHITE } else { c[6] },
            |u: f64| if u < 0.6 { 1.0 } else { 1.0 - (u - 0.6) / 0.4 },
        );
    }
    p.ellipse(2.5, 0.0, 4.6, 3.0, c[6], 1.0);
    p.ellipse(2.0, 0.0, 3.0, 1.8, WHITE, 1.0);
    let fl = 0.85 + 0.15 * libm::sin(ph * TAU * 3.0);
    p.glow(2.0, 0.0, 24.0, c[5], 0.7 * fl);
    p.glow(2.0, 0.0, 9.0, WHITE, 0.5 * fl);
}
