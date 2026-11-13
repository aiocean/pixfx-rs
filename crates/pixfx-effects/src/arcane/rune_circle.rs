//! Rune circle: port of pixelfx `src/fx/arcane/rune-circle.js`.
//!
//! The floor glow and the specks rising off it follow Sprite Fusion's open-source mana heal
//! (https://www.spritefusion.com/pixel-art-vfx/heal), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::color::{Rgba, WHITE, pal, ramp};
use pixfx_core::math::{Rng, TAU, ease, js_min, smooth};
use pixfx_core::particles::cycle;
use pixfx_core::sim::sparkle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{floor_ring, glyph, mote};

pub struct RuneCircle;

pub static RUNE_CIRCLE: RuneCircle = RuneCircle;

const META: Meta = Meta {
    id: "arcane/rune-circle",
    name: "Rune circle",
    desc: "A rune circle turns on the floor in perspective: a carousel of runes lit by chasing light, a turning square, ticks, motes rising off it.",
    tags: &[
        "arcane", "rune", "circle", "magic", "aura", "ground", "loop",
    ],
    size: (136, 100),
    origin: (68, 80),
    duration: 4.0,
    looping: true,
    fps: 30,
    seed: 8,
    options: &[],
};

// turning a quarter a loop keeps every part seamless: 16 ticks, a square, and 12 runes in a pattern of three
const D: f64 = 4.0;
const R: f64 = 46.0;
const RR: f64 = 39.0;
const RI: f64 = 23.0;
const RC: f64 = 7.0;
const SY: f64 = 0.3;
const NR: usize = 12;
const PI: f64 = core::f64::consts::PI;

impl Effect for RuneCircle {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&pal::ARCANE)),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
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

fn at(g: f64, r: f64) -> (f64, f64) {
    (libm::cos(g) * r, libm::sin(g) * r * SY)
}

fn gap(a: f64, b: f64) -> f64 {
    libm::atan2(libm::sin(a - b), libm::cos(a - b)).abs()
}

fn circle(p: &mut Painter, c: &[Rgba; 8], t: f64, far: bool) {
    let ph = t / D;
    let spin = (ph * TAU) / 4.0;
    let keep = |g: f64| (libm::sin(g) < 0.0) == far;
    // two sparks of light chase round the outer ring, twice a loop
    let chase = ph * TAU * 2.0;
    let lit = |g: f64, w: f64| gap(g, chase) < w || gap(g, chase + PI) < w;
    let tone = |g: f64, hi: Rgba, lo: Rgba| if libm::sin(g) < 0.0 { lo } else { hi };
    floor_ring(
        p,
        0.0,
        0.0,
        R,
        SY,
        |g| {
            if lit(g, 0.22) {
                WHITE
            } else if lit(g, 0.5) {
                c[6]
            } else {
                tone(g, c[6], c[5])
            }
        },
        1.0,
        keep,
    );
    floor_ring(p, 0.0, 0.0, RI, SY, |g| tone(g, c[5], c[4]), 1.0, keep);
    floor_ring(p, 0.0, 0.0, RC, SY, |g| tone(g, c[6], c[5]), 1.0, keep);
    // studs set round the outside of the ring, turning with it
    for j in 0..16 {
        let g = spin + ((f64::from(j) + 0.5) * TAU) / 16.0;
        if (libm::sin(g) < 0.0) != far {
            continue;
        }
        let (x, y) = at(g, R + 3.5);
        let hot = lit(g, 0.3);
        let body = if j % 2 == 1 { c[5] } else { c[6] };
        p.px(x, y, if hot { WHITE } else { tone(g, body, c[4]) });
        if j % 2 == 0 {
            for dx in [1.0, -1.0] {
                p.px(x + dx, y, if hot { c[6] } else { tone(g, c[4], c[3]) });
            }
        }
    }
    // a square turning the other way inside the inner ring, its corners touching it
    let pts: [(f64, f64); 4] =
        core::array::from_fn(|k| at(-spin + (k as f64 * TAU) / 4.0 + PI / 4.0, RI));
    for k in 0..4 {
        let (mut ax, mut ay) = pts[k];
        let (mut bx, mut by) = pts[(k + 1) % 4];
        if (ay < 0.0) != (by < 0.0) {
            let u = ay / (ay - by);
            let mx = ax + (bx - ax) * u;
            if (ay < 0.0) == far {
                bx = mx;
                by = 0.0;
            } else {
                ax = mx;
                ay = 0.0;
            }
        } else if (ay < 0.0) != far {
            continue;
        }
        p.line(ax, ay, bx, by, if far { c[4] } else { c[5] }, 1.0);
    }
    // the runes stand in the band, riding round; bright at the front, dim behind, flaring as the light passes
    let mut gs: Vec<(usize, f64)> = (0..NR)
        .map(|k| (k, spin + ((k as f64 + 0.5) * TAU) / NR as f64))
        .filter(|&(_, g)| (libm::sin(g) < 0.0) == far)
        .collect();
    gs.sort_by(|a, b| libm::sin(a.1).total_cmp(&libm::sin(b.1)));
    for (k, g) in gs {
        let (x, y) = at(g, RR);
        let hot = lit(g, 0.2);
        let bob = libm::sin(ph * TAU * 4.0 + 4.0 * g) * 0.5;
        let body = if hot {
            WHITE
        } else if far {
            c[5]
        } else {
            c[6]
        };
        let under = if hot {
            c[5]
        } else if far {
            c[2]
        } else {
            c[3]
        };
        glyph(p, [0, 3, 5][k % 3], x, y - 1.0 + bob, body, 1.0, under);
        let (r, a) = if hot {
            (10.0, 0.5)
        } else if far {
            (6.0, 0.12)
        } else {
            (6.0, 0.22)
        };
        p.glow(x, y - 3.0, r, c[5], a);
        p.glow(x, y, 5.0, c[4], 0.15);
    }
}

// motes rising off the circle on a slow helix round its axis, swaying in; twinkles hanging over it
fn motes(p: &mut Painter, c: &[Rgba; 8], t: f64, s: f64, far: bool) {
    cycle(
        t,
        D,
        76,
        s * 5.0 + 1.0,
        |g| g.range(1.1, 1.9),
        |age, life, r: &mut Rng, _| {
            let g0 = r.range(0.0, TAU);
            let r0 = r.pick(&[R, R, RR, RR, RI]).copied().unwrap_or(R);
            let q = age / life;
            let g = g0 + q * r.range(0.6, 1.4);
            if (libm::sin(g) < 0.0) != far {
                return;
            }
            let rr = r0 * (1.0 - 0.35 * q);
            let h = 2.0 + ease::out_quad(q) * r.range(26.0, 52.0);
            let x = libm::cos(g) * rr;
            let y = libm::sin(g) * rr * SY - h;
            let a = js_min(1.0, q * 5.0) * (1.0 - smooth(0.65, 1.0, q));
            let big = r.chance(0.2);
            let col = if r.chance(0.3) {
                WHITE
            } else if r.chance(0.6) {
                c[6]
            } else {
                c[5]
            };
            let size = if big { 1.1 } else { r.range(0.55, 0.9) };
            mote(
                p,
                x,
                y,
                -libm::sin(g) * rr * 0.8,
                -36.0 * (1.0 - q) - 10.0,
                col,
                size,
                a,
                if big { 0.5 } else { 0.3 },
            );
        },
    );
    cycle(
        t,
        D,
        14,
        s * 5.0 + 2.0,
        |_| 0.45,
        |age, life, r: &mut Rng, _| {
            let g = r.range(0.0, TAU);
            if (libm::sin(g) < 0.0) != far {
                return;
            }
            let rr = r.range(8.0, R);
            let x = libm::cos(g) * rr;
            let y = libm::sin(g) * rr * SY - r.range(6.0, 34.0);
            let k = libm::sin((age / life) * PI);
            sparkle(p, x, y, 1.0 + 2.4 * k, c[6], c[5], 0.35 * k);
        },
    );
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let pulse = 0.85 + 0.15 * libm::sin((t / D) * TAU * 4.0);
    // the floor glows under the circle, and a soft light stands over it
    p.glow(0.0, 0.0, R * 1.4, c[4], 0.24 * pulse);
    for s in [-1.0, 1.0] {
        p.glow(s * R * 0.55, 1.0, R * 0.7, c[5], 0.12 * pulse);
    }
    // a soft column of light standing over it, fading as it rises
    let mut h = 0.0;
    while h < 66.0 {
        p.glow(0.0, -h, 26.0, c[4], 0.11 * pulse * (1.0 - h / 80.0));
        h += 6.0;
    }
    circle(p, c, t, true);
    motes(p, c, t, o.seed, true);
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let chase = (t / D) * TAU * 2.0;
    circle(p, c, t, false);
    motes(p, c, t, o.seed, false);
    // the two sparks of light running round the ring
    for g in [chase, chase + PI] {
        let (x, y) = at(g, R);
        sparkle(p, x, y, 2.5 + libm::sin(t * 30.0) * 0.8, c[6], c[5], 0.5);
        p.glow(x, y, 14.0, c[5], 0.4);
    }
}
