//! Brazier: port of pixelfx `src/fx/fire/brazier.js`.
//!
//! The tongue, lick, flame-puff, smoke, ember and spark recipe is adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::PI;

use pixfx_core::color::{Rgba, WHITE, mix};
use pixfx_core::math::{js_max, js_min, js_round, rand};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{
    Burn, Hot, Life, LoopNoise, Tongue, burn, drift, ember, floor_light, hot_first, looped, ramp,
    spark,
};

pub struct Brazier;

pub static BRAZIER: Brazier = Brazier;

const META: Meta = Meta {
    id: "fire/brazier",
    name: "Brazier",
    desc: "An iron bowl on a tripod heaped with glowing coals and a big roaring fire, sparks and embers streaming up off it, light on the iron and the floor.",
    tags: &[
        "fire", "brazier", "prop", "light", "sparks", "ambient", "loop",
    ],
    size: (112, 148),
    origin: (56, 136),
    duration: 3.0,
    looping: true,
    fps: 30,
    seed: 8,
};

// meta.options
const PALETTE: [Rgba; 9] = [
    Rgba::hex24(0x4a1712),
    Rgba::hex24(0x8a1f17),
    Rgba::hex24(0xc9281f),
    Rgba::hex24(0xf2561d),
    Rgba::hex24(0xff8a2a),
    Rgba::hex24(0xffc23d),
    Rgba::hex24(0xffe26b),
    Rgba::hex24(0xfff3a6),
    Rgba::hex24(0xffffff),
];
const IRON: [Rgba; 5] = [
    Rgba::hex24(0x0b0910),
    Rgba::hex24(0x1c1924),
    Rgba::hex24(0x302c3a),
    Rgba::hex24(0x4c4658),
    Rgba::hex24(0x78708a),
];

const D: f64 = META.duration;
const NL: LoopNoise = LoopNoise(D);
const RIM: f64 = -25.0;
const RX: f64 = 15.0;
const RY: f64 = 3.0;
// the fire: [x, y from the rim, height share, half-width, back row?]
const TONGUES: [Tongue; 11] = [
    (-7.0, -1.5, 0.5, 2.0, true),
    (7.2, -1.5, 0.55, 2.0, true),
    (-3.0, -2.5, 0.84, 2.2, true),
    (3.3, -2.5, 0.86, 2.2, true),
    (-11.5, 0.5, 0.2, 1.5, false),
    (11.5, 0.5, 0.22, 1.5, false),
    (-8.3, 0.0, 0.34, 2.0, false),
    (8.5, 0.0, 0.38, 2.0, false),
    (-4.7, -0.5, 0.6, 2.8, false),
    (4.9, -0.5, 0.64, 2.8, false),
    (0.0, -1.0, 1.0, 3.6, false),
];

impl Effect for Brazier {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            f: ramp(&opts.palette_or(&PALETTE)),
            i: ramp(&opts.ramp_or("iron", &IRON)),
            s: seed as f64,
        })
    }
}

struct State {
    f: Hot,
    i: [Rgba; 5],
    s: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

// the tripod: two legs splayed out and one coming at us, curled feet, a ring holding them together
fn stand(p: &mut Painter, i: &[Rgba; 5], h: &Hot, fl: f64) {
    let leg = |p: &mut Painter, x0: f64, y0: f64, x1: f64, y1: f64| {
        p.line(x0, y0, x1, y1, i[0], 1.0);
        p.line(x0 + 1.0, y0, x1 + 1.0, y1, i[2], 1.0);
        p.line(x0 + 2.0, y0, x1 + 2.0, y1, i[0], 1.0);
    };
    // the ring: its far half behind the front leg
    p.ellipse(
        0.0,
        -9.0,
        9.0,
        1.8,
        |d: f64, _: f64| if d > 0.6 { i[2] } else { Rgba::NONE },
        1.0,
    );
    leg(p, -6.0, -17.0, -12.0, -1.0);
    leg(p, 4.0, -17.0, 10.0, -1.0);
    p.rect(-14.0, -1.0, 5.0, 1.0, i[2], 1.0);
    p.rect(-15.0, 0.0, 6.0, 1.0, i[0], 1.0);
    p.px(-15.0, -1.0, i[3]);
    p.rect(9.0, -1.0, 5.0, 1.0, i[2], 1.0);
    p.rect(9.0, 0.0, 6.0, 1.0, i[0], 1.0);
    p.px(13.0, -1.0, i[3]);
    p.rect(-1.0, -16.0, 3.0, 15.0, i[0], 1.0);
    p.rect(0.0, -16.0, 1.0, 15.0, mix(i[3], h[4], 0.15 * fl), 1.0);
    p.rect(-2.0, -1.0, 5.0, 1.0, i[2], 1.0);
    p.rect(-2.0, 0.0, 5.0, 1.0, i[0], 1.0);
    p.ellipse(
        0.0,
        -9.0,
        9.0,
        1.8,
        |d: f64, a: f64| {
            if libm::sin(a) > 0.0 && d > 0.55 {
                if d > 0.85 { i[0] } else { i[3] }
            } else {
                Rgba::NONE
            }
        },
        1.0,
    );
}

// the bowl, front half: a band of rivets under a rim lit by the fire, rounding down to a dark belly
fn bowl(p: &mut Painter, i: &[Rgba; 5], h: &Hot, fl: f64) {
    p.shade(-RX - 1.0, RIM - 1.0, RX + 1.0, RIM + 9.0, |x, y, _| {
        let v = (y - RIM) / 9.0;
        let hw = RX * libm::sqrt(js_max(0.0, 1.0 - v * v * 0.9))
            - if v > 0.5 { (v - 0.5) * 6.0 } else { 0.0 };
        if x.abs() > hw + 0.5 || y < RIM {
            return None;
        }
        if x.abs() > hw - 0.5 || v > 0.92 {
            return Some(i[0]);
        }
        if y < RIM + 1.0 {
            return Some(mix(i[3], h[4], 0.5 * fl));
        }
        if y < RIM + 2.0 {
            return Some(i[2]);
        }
        if y < RIM + 3.0 {
            return Some(if js_round(x) % 4.0 == 0.0 {
                mix(i[4], h[4], 0.25 * fl)
            } else {
                i[0]
            });
        }
        Some(if x < -RX * 0.45 {
            i[2]
        } else if x < RX * 0.25 {
            i[1]
        } else {
            i[0]
        })
    });
}

// coals heaped in the bowl, showing over the rim, breathing in the heat
fn coals(p: &mut Painter, i: &[Rgba; 5], h: &Hot, t: f64, s: f64) {
    p.shade(-RX, RIM - 6.0, RX, RIM + 1.0, |x, y, out| {
        // a mound: highest in the middle, standing proud of the rim
        let top = RIM - 1.0 - 4.0 * js_max(0.0, 1.0 - libm::pow(x / (RX - 1.0), 2.0));
        if y < top || y > RIM + 0.5 || x.abs() > RX - 0.5 {
            return None;
        }
        let lump = rand(&[
            libm::floor((x + libm::floor(y) * 1.5) / 3.0),
            libm::floor(y / 2.0),
            s + 20.0,
        ]);
        let edge = y < top + 1.0;
        // the gaps between lumps stay dark; the lumps breathe, hottest deep in the heap
        if rand(&[libm::floor(x), libm::floor(y), s + 21.0]) < 0.16 || (edge && lump < 0.3) {
            return Some(i[0]);
        }
        let g = (0.5 + 0.5 * libm::sin((t / D) * PI * 4.0 + lump * 40.0))
            * if edge { 0.7 } else { 1.0 };
        out.k = 0.3 * g;
        Some(if g > 0.72 {
            h[2]
        } else if g > 0.5 {
            h[3]
        } else if g > 0.3 {
            h[4]
        } else if g > 0.15 {
            h[5]
        } else {
            h[6]
        })
    });
}

struct Spark {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    a: f64,
    w: f64,
    m: f64,
    big: bool,
}

impl Life for Spark {
    fn life(&self) -> f64 {
        self.life
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, i, s) = (hot_first(&o.f), &o.i, o.s);
    let fl = 0.74 + 0.16 * NL.at(t, 7.3, s + 1.0, 0.0) + 0.1 * NL.at(t, 19.0, s + 2.0, 0.0);
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.25);
    let sparks = [h[1], h[2], h[4], h[5]];
    let emb = [h[1], h[2], h[4], h[6]];
    floor_light(p, 0.0, 0.5, 40.0, 7.0, glow, 0.32 * fl);
    stand(p, i, &h, fl);
    // the back of the rim, behind the fire
    p.ellipse(
        0.0,
        RIM,
        RX,
        RY,
        |d: f64, a: f64| {
            if libm::sin(a) < 0.0 && d > 0.7 {
                if d > 0.88 {
                    i[0]
                } else {
                    mix(i[3], h[4], 0.3 * fl)
                }
            } else {
                Rgba::NONE
            }
        },
        1.0,
    );
    let o = Burn {
        h: &h,
        d: D,
        nl: NL,
        s,
        x: 0.0,
        y: RIM,
        height: 32.0,
        lean: 0.1,
        spread: 7.0,
        puffs: 8.0,
        licks: 4.0,
        smoke: 14.0,
        smoke_size: 3.0,
        smoke_life: (1.6, 2.4),
        embers: 10.0,
        glow_r: 58.0,
        glow: 1.4,
        wind: 3.0,
    };
    burn(p, t, &TONGUES, &o, |p, _| {
        coals(p, i, &h, t, s);
        bowl(p, i, &h, fl);
    });
    // sparks streaming up in the draught: fast streaks that slow, wander and cool to embers high above
    looped(
        t,
        D,
        60,
        s + 600.0,
        |r, _| {
            let side = r.range(-1.0, 1.0);
            Spark {
                x: side * 8.0,
                y: RIM - r.range(4.0, 18.0),
                vx: side * r.range(10.0, 40.0),
                vy: -r.range(70.0, 150.0),
                life: r.range(0.7, 1.7),
                a: r.range(0.0, 6.3),
                w: r.range(1.8, 3.2),
                m: r.range(3.0, 8.0),
                big: r.chance(0.12),
            }
        },
        |q, age, _| {
            let [x, y, vx, vy] = drift(q.x, q.y, q.vx, q.vy, age, 0.0, -22.0, 1.9, 1.9);
            let k = 1.0 - age / q.life;
            // they bend and wander as the draught slows them
            let ramp_in = js_min(1.0, age * 1.5);
            let wob = q.m * (libm::sin(age * q.w + q.a) - libm::sin(q.a)) * ramp_in;
            let wv = q.m * q.w * libm::cos(age * q.w + q.a) * ramp_in;
            if libm::hypot(vx + wv, vy) > 40.0 {
                spark(p, x + wob, y, vx + wv, vy, k, &sparks);
            } else {
                ember(
                    p,
                    x + wob,
                    y,
                    k,
                    if q.big { 2.0 } else { 1.0 },
                    &emb,
                    age * 40.0 + q.a,
                );
            }
        },
    );
}
