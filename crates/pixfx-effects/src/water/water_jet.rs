//! Water jet: port of pixelfx `src/fx/water/water-jet.js`.
//!
//! Beyond pixelfx: the `length` option is held to 0..=4096 px, so a huge number cannot stall the column loop.
//!
//! The stream's build (a white core banded out to darker edges, surges running along it, light down its length, a
//! flare where it leaves the source) follows Sprite Fusion's open-source energy beam
//! (https://www.spritefusion.com/pixel-art-vfx/energy), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::color::{Rgba, WHITE, ramp};
use pixfx_core::math::{TAU, js_max, js_min, smooth, tile_noise};
use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{WATER, bead};

pub struct WaterJet;

pub static WATER_JET: WaterJet = WaterJet;

const META: Meta = Meta {
    id: "water/water-jet",
    name: "Water jet",
    desc: "A pressurised stream blasts right: a tight glassy rod with surges running down it that swells, roughens, bends under its weight and breaks into chunks, spray and mist.",
    tags: &[
        "water",
        "jet",
        "stream",
        "hose",
        "spray",
        "directional",
        "loop",
    ],
    size: (188, 80),
    origin: (14, 32),
    duration: 1.0,
    looping: true,
    fps: 30,
    seed: 3,
};

const LENGTH: f64 = 118.0;
/// The longest stream drawn (beyond pixelfx, which draws any length; see the module doc).
const MAX_LENGTH: f64 = 4096.0;
const D: f64 = META.duration;
const G: f64 = 520.0;
const SPEED: f64 = 260.0;

impl Effect for WaterJet {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            length: opts.num_in("length", LENGTH, 0.0, MAX_LENGTH),
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

/// The stream's centre line sags as it goes.
fn sag(x: f64, l: f64) -> f64 {
    libm::pow(x / l, 2.0) * 9.0
}

/// How thick the stream is at x.
fn thick(x: f64, l: f64) -> f64 {
    2.6 + 5.4 * libm::pow(x / l, 1.1)
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let ph = t / D;
    let (l, s) = (o.length, o.seed);
    let big_t = ph * TAU;
    // the stream, column by column: white down its core, banded out to blue edges, shadowed underneath; streaks race
    // along it and surges bulge through; past halfway it roughens, and its far end tears into chunks with gaps
    let mut xi = 0.0;
    while xi <= l + 3.0 {
        let x = xi + 0.5;
        let u = x / l;
        if u > 1.0 {
            break;
        }
        let yc = sag(x, l);
        let rough = smooth(0.25, 1.0, u);
        let sd = (((x - ph * SPEED * 2.0) % 130.0) + 130.0) % 130.0 - 65.0;
        let w0 = thick(x, l) + libm::exp(-(sd * sd) / 90.0) * 1.4 * (1.0 - u * 0.5);
        let up = w0 + tile_noise(x * 0.18 - ph * 30.0, 9.0, 30.0, 0.0, s + 1.0) * 2.2 * rough;
        let dn = w0 + tile_noise(x * 0.18 - ph * 30.0, 3.0, 30.0, 0.0, s + 1.0) * 2.2 * rough;
        let mut yi = libm::floor(yc - up);
        while yi <= libm::ceil(yc + dn) {
            let y = yi + 0.5;
            let q = (y - yc) / js_max(0.8, if y < yc { up } else { dn });
            let aq = libm::fabs(q);
            if aq <= 1.0
                && !(u > 0.55
                    && tile_noise(x * 0.12 - ph * 25.0, (y - yc) * 0.25, 25.0, 0.0, s + 2.0)
                        < -0.9 + 1.6 * smooth(0.55, 1.0, u))
            {
                let streak = tile_noise(
                    x * 0.06 - ph * 40.0,
                    libm::floor(y - yc + 20.0) * 1.7,
                    40.0,
                    0.0,
                    s,
                );
                let mut lvl: usize = if aq < 0.2 {
                    0
                } else if aq < 0.48 {
                    1
                } else if aq < 0.78 {
                    2
                } else {
                    3
                };
                if streak > 0.35 {
                    lvl = lvl.saturating_sub(1);
                }
                if q > 0.4 {
                    lvl = lvl.saturating_add(1).min(3);
                }
                p.px(xi, yi, [WHITE, c[6], c[5], c[4]][lvl]);
                if lvl == 0 {
                    p.light(x, y, c[5], 0.12);
                }
            }
            yi += 1.0;
        }
        xi += 1.0;
    }
    // the nozzle flare: a white burst where it leaves, ringed with light
    let fl = 0.85 + 0.15 * libm::sin(big_t * 6.0);
    p.circle(0.0, 0.0, 4.4 * fl, c[5], 1.0);
    p.circle(0.0, 0.0, 3.4 * fl, c[6], 1.0);
    p.circle(0.0, 0.0, 2.2 * fl, WHITE, 1.0);
    let mut a = 0.0;
    while a < TAU {
        p.dot(
            libm::cos(a + big_t * 2.0) * 5.2 * fl,
            libm::sin(a + big_t * 2.0) * 5.2 * fl,
            c[5],
            0.6,
        );
        a += 0.35;
    }
    p.glow(0.0, 0.0, 24.0, c[5], 0.6 * fl);
    p.glow(0.0, 0.0, 8.0, WHITE, 0.4 * fl);
    // where it pinches apart: beads strung out along the end of the stream, racing on
    for k in 0..6 {
        let kf = f64::from(k);
        let x = l * 0.82 + ((kf * 13.0 + ph * SPEED) % 78.0);
        let y = sag(js_min(x, l), l)
            + if x > l {
                libm::pow((x - l) / 60.0, 2.0) * 10.0
            } else {
                0.0
            }
            + libm::sin(kf * 2.1) * 1.5;
        if x < l + 34.0 {
            bead(
                p,
                x,
                y,
                SPEED,
                40.0,
                if k % 3 == 0 { 3.0 } else { 2.0 },
                c,
                1.0,
            );
        }
    }
    // spray: a widening cone of drops shed from the stream and torn off its end, arcing down
    cycle(
        t,
        D,
        70,
        s + 3.0,
        |g| g.range(0.2, 0.42),
        |age, life, r, _| {
            let f = libm::sqrt(r.next());
            let x0 = l * (0.3 + 0.7 * f);
            let y0 = sag(x0, l) + r.range(-1.0, 1.0) * thick(x0, l);
            let vx = r.range(150.0, 270.0) * (0.55 + 0.45 * f);
            let vy = r.range(-1.0, 1.0) * 110.0 * f + r.range(-10.0, 20.0);
            let x = x0 + vx * age;
            let y = y0 + vy * age + 0.5 * G * age * age;
            let k = age / life;
            if x > l + 50.0 || y > 40.0 || y < -28.0 || (k > 0.75 && r.next() < (k - 0.75) * 4.0) {
                return;
            }
            if r.next() < 0.45 {
                p.px(x, y, if k < 0.5 { WHITE } else { c[6] });
            } else {
                let size = if r.next() < 0.2 { 2.0 } else { 1.0 };
                bead(p, x, y, vx, vy + G * age, size, c, 1.0);
            }
        },
    );
    // mist blooming off the end
    cycle(
        t,
        D,
        12,
        s + 4.0,
        |g| g.range(0.7, 1.0),
        |age, life, r, _| {
            let k = age / life;
            let x = l * r.range(0.7, 1.0) + age * r.range(20.0, 45.0);
            let y = sag(js_min(x, l), l) + r.range(-5.0, 5.0) + age * r.range(4.0, 14.0);
            let rad = r.range(3.0, 5.0) + k * r.range(5.0, 9.0);
            let a = r.range(0.1, 0.18) * js_min(1.0, k / 0.15) * libm::pow(1.0 - k, 1.5);
            p.disc(x, y, rad, c[6], |d: f64, _: f64| a * (1.0 - d * d));
        },
    );
    // light: down the core, brightest near the nozzle
    let mut x = 0.0;
    while x < l {
        p.glow(
            x,
            sag(x, l),
            14.0 + x * 0.1,
            c[4],
            0.26 * (1.0 - (x / l) * 0.5),
        );
        x += 10.0;
    }
    p.glow(l * 0.9, sag(l * 0.9, l), 26.0, c[5], 0.22);
}
