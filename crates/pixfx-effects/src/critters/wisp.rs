//! Wisp: port of pixelfx `src/fx/critters/wisp.js`.
//!
//! The flame tail, puffs and embers are adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::color::{Rgba, ramp};
use pixfx_core::math::{TAU, clamp, js_min, js_round, wrap};
use pixfx_core::particles::{cycle, fly};
use pixfx_core::sim::{Flags, Kind};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options, Painter};

use super::lib::{Swarm, fire_puff, lnoise, shadow, shift, tongue};

pub struct Wisp;

pub static WISP: Wisp = Wisp;

const META: Meta = Meta {
    id: "critters/wisp",
    name: "Wisp",
    desc: "A will-o'-the-wisp loops a figure-eight over the floor: a little soul-flame with a face, its flame tail licking up and streaming behind, shedding puffs and sparks.",
    tags: &[
        "critter", "wisp", "spirit", "flame", "magic", "flying", "loop",
    ],
    size: (128, 104),
    origin: (64, 96),
    duration: 3.2,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[HOVER.spec()],
};

const PALETTE: [Rgba; 7] = [
    Rgba(0x0a, 0x1f, 0x2e, 255),
    Rgba(0x0f, 0x3d, 0x52, 255),
    Rgba(0x14, 0x70, 0x7e, 255),
    Rgba(0x1f, 0xa8, 0xa8, 255),
    Rgba(0x4f, 0xe0, 0xc8, 255),
    Rgba(0xa8, 0xff, 0xf0, 255),
    Rgba(255, 255, 255, 255),
];
const HOVER: NumOpt = NumOpt::new("hover", 36.0);
const D: f64 = 3.2;

impl Effect for Wisp {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            hover: opts.num(&HOVER),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 7],
    hover: f64,
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

/// Where the wisp is `t` seconds into its loop, and how fast it moves.
struct Pose {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

fn at(t: f64, hover: f64) -> Pose {
    let a = (t / D) * TAU;
    Pose {
        x: 34.0 * libm::sin(a),
        y: -hover - 10.0 * libm::sin(2.0 * a),
        vx: ((34.0 * TAU) / D) * libm::cos(a),
        vy: ((-20.0 * TAU) / D) * libm::cos(2.0 * a),
    }
}

fn draw(p: &mut Painter, time: f64, o: &State) {
    let t = shift(time, D, o.seed);
    let c = &o.c;
    let s = at(t, o.hover);
    let heat = [c[6], c[6], c[5], c[5], c[4], c[4], c[3], c[3], c[3]];
    let lean = clamp(-s.vx / 80.0, -0.9, 0.9);
    let stretch = clamp(s.vy / 120.0, -0.3, 0.4);
    let (x0, y0) = (s.x, s.y);
    // flame puffs shed along its path, cooling down the ramp as they drift up
    cycle(
        t,
        D,
        30,
        o.seed * 7.0 + 1.0,
        |r| r.range(0.3, 0.6),
        |age, life, r, _| {
            let s0 = at(wrap(t - age, D), o.hover);
            let px = s0.x + r.range(-3.0, 3.0);
            let py = s0.y - r.range(0.0, 4.0);
            let vx = r.range(-8.0, 8.0);
            let vy = -r.range(6.0, 16.0);
            let (x, y) = fly(px, py, vx, vy, age, -40.0, 2.5);
            let size = r.range(1.2, 2.6);
            let seed = r.next() * 100.0;
            fire_puff(p, x, y, 1.0 - age / life, size, seed, &heat);
        },
    );
    // the tail: three tongues off the top of its head, streaming back as it flies
    let tails: [(f64, f64, f64, f64); 3] = [
        (-3.5, 13.0, 2.5, 3.0),
        (3.5, 14.0, 2.5, 7.0),
        (0.0, 20.0, 3.2, 11.0),
    ];
    for li in 0..4 {
        for (dx, h, w, n) in tails {
            let hh = (h + 0.25 * h * lnoise(t, D, 9.0, n, 5.0)) * (1.0 + stretch);
            tongue(
                p,
                x0 + dx,
                y0 - 2.0,
                hh,
                w,
                lnoise(t, D, 6.0, n, 40.0) * 1.5 + lean * hh * 0.9,
                t,
                D,
                n,
                &heat,
                Some(li),
            );
        }
    }
    // the head: a ball of cold fire, white at the heart, its rim flickering
    let rad = 7.5;
    p.shade(
        x0 - rad - 2.0,
        y0 - rad - 2.0,
        x0 + rad + 2.0,
        y0 + rad + 2.0,
        |x, y, _| {
            let dx = x - x0;
            let dy = y - y0;
            let ang = libm::atan2(dy, dx);
            let rr = rad
                + 0.7 * lnoise(t, D, 16.0, ang * 1.6 + 10.0, 3.0)
                + if dy > 0.0 { -0.6 } else { 0.0 };
            let d = libm::hypot(dx, dy) / rr;
            if d > 1.0 {
                return None;
            }
            Some(if d < 0.42 {
                c[6]
            } else if d < 0.68 {
                c[5]
            } else if d < 0.88 {
                c[4]
            } else {
                c[3]
            })
        },
    );
    // its face: eyes looking where it's going, a blink, a little mouth
    let look = js_round(clamp(s.vx / 60.0, -1.0, 1.0));
    let ph = wrap(t, D);
    let blink = ph > 2.2 && ph < 2.28;
    for sd in [-1.0, 1.0] {
        let ex = js_round(x0) + sd * 2.0 - if sd > 0.0 { 0.0 } else { 1.0 } + look;
        let ey = js_round(y0) - 1.0;
        if blink {
            p.px(ex, ey + 1.0, c[1]);
            p.px(ex + 1.0, ey + 1.0, c[1]);
        } else {
            for k in 0..3 {
                p.px(ex, ey + f64::from(k), c[1]);
                p.px(ex + 1.0, ey + f64::from(k), c[1]);
            }
            p.px(ex + if look > 0.0 { 1.0 } else { 0.0 }, ey, c[6]);
        }
    }
    p.px(js_round(x0) + look, js_round(y0) + 3.0, c[2]);
    p.px(js_round(x0) - 1.0 + look, js_round(y0) + 3.0, c[2]);
    // sparks wander up off it
    let mut sw = Swarm::new(t, vec![c[6], c[5], c[4], c[3]]);
    cycle(
        t,
        D,
        26,
        o.seed * 7.0 + 2.0,
        |r| r.range(0.8, 1.6),
        |age, life, r, _| {
            let s0 = at(wrap(t - age, D), o.hover);
            let sd = r.next() * 100.0;
            let px = s0.x + r.range(-5.0, 5.0);
            let py = s0.y - r.range(4.0, 12.0);
            let vx = r.range(-20.0, 20.0);
            let vy = -r.range(20.0, 45.0);
            let (x, y) = fly(px, py, vx, vy, age, -15.0, 1.6);
            let sx = x + libm::sin(age * 2.3 + sd) * 3.0 * js_min(1.0, age);
            let size = if r.chance(0.3) { 2.0 } else { 1.0 };
            sw.add(
                Kind::Ember,
                sx,
                y,
                0.0,
                0.0,
                age,
                life,
                c[4],
                size,
                Flags::NONE,
                sd,
            );
        },
    );
    sw.draw(p, Layer::Both);
    // light: a cold glow round it, a hot heart
    let fl = 0.85 + 0.15 * lnoise(t, D, 22.0, 0.0, 9.0);
    p.glow(x0 + lean * 3.0, y0 - 6.0, 46.0 * fl, c[4], 0.38);
    p.glow(x0, y0 - 1.0, 13.0, c[5], 0.35 * fl);
}

fn draw_back(p: &mut Painter, time: f64, o: &State) {
    let t = shift(time, D, o.seed);
    let c = &o.c;
    let s = at(t, o.hover);
    let near = clamp(1.0 - (-s.y - 24.0) / 30.0, 0.0, 1.0);
    // the floor lit cold beneath it, brighter as it dips
    shadow(p, s.x, 0.0, 7.0, 1.2, c[0], 0.4);
    let mut x = -24.0;
    while x <= 24.0 {
        p.glow(
            s.x + x,
            -1.0,
            16.0,
            c[4],
            (0.08 + 0.1 * near) * (1.0 - libm::fabs(x) / 32.0),
        );
        x += 6.0;
    }
}
