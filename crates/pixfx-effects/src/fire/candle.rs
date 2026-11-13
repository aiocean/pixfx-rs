//! Candle: port of pixelfx `src/fx/fire/candle.js`.
//!
//! The layered flame tongue is adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::color::{Rgba, WHITE, mix, ramp};
use pixfx_core::math::{clamp, js_max, js_min, js_round, smooth};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter, RampOpt};

use super::lib::{
    Hot, Life, LoopNoise, drift, flame_tongue, floor_light, hot_first, looped, smoke_puff,
};

pub struct Candle;

pub static CANDLE: Candle = Candle;

const META: Meta = Meta {
    id: "fire/candle",
    name: "Candle",
    desc: "A wax candle in a brass dish: a calm flame with a blue root that sways and stretches, a halo of light, motes drifting in it, a bead of wax running down a drip.",
    tags: &["fire", "candle", "prop", "light", "calm", "ambient", "loop"],
    size: (56, 76),
    origin: (28, 64),
    duration: 4.0,
    looping: true,
    fps: 30,
    seed: 2,
    state: None,
    options: &[ROOT.spec(), WAX.spec(), BRASS.spec()],
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
const ROOT: RampOpt = RampOpt::new(
    "root",
    &[
    Rgba::hex24(0x1b2a8a),
    Rgba::hex24(0x3a62e0),
    Rgba::hex24(0x7fb4ff),
]);
const WAX: RampOpt = RampOpt::new(
    "wax",
    &[
    Rgba::hex24(0x2e201e),
    Rgba::hex24(0x5e4436),
    Rgba::hex24(0x8e6e58),
    Rgba::hex24(0xb8957a),
    Rgba::hex24(0xd8b896),
    Rgba::hex24(0xf0dcc0),
]);
const BRASS: RampOpt = RampOpt::new(
    "brass",
    &[
    Rgba::hex24(0x2a1a10),
    Rgba::hex24(0x6a4a1e),
    Rgba::hex24(0xb4862e),
    Rgba::hex24(0xe8c060),
    Rgba::hex24(0xfff0b0),
]);

const D: f64 = META.duration;
const NL: LoopNoise = LoopNoise(D);
const TOP: f64 = -16.0;
const WICK: f64 = TOP - 3.0;
// a calm flame: an orange rim round gold, yellow, a pale heart and a small white core low down
const CALM: [(f64, f64, usize); 5] = [
    (1.0, 1.0, 5),
    (0.8, 0.88, 3),
    (0.6, 0.72, 2),
    (0.38, 0.52, 1),
    (0.18, 0.3, 0),
];
// the bead runs down this drip, from the rim to its bulb
const DRIP_X: f64 = 4.0;
const DRIP_Y0: f64 = TOP + 2.0;
const DRIP_Y1: f64 = -7.0;

impl Effect for Candle {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            f: ramp(&opts.palette_or(&PALETTE)),
            rt: ramp(&opts.ramp(&ROOT)),
            x: ramp(&opts.ramp(&WAX)),
            b: ramp(&opts.ramp(&BRASS)),
            s: seed as f64,
        })
    }
}

struct State {
    f: Hot,
    rt: [Rgba; 3],
    x: [Rgba; 6],
    b: [Rgba; 5],
    s: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

fn dish(p: &mut Painter, b: &[Rgba; 5], h: &Hot, fl: f64) {
    p.ellipse(
        0.0,
        1.0,
        10.0,
        2.6,
        |d: f64, a: f64| {
            if d > 0.82 {
                b[0]
            } else if libm::sin(a) < -0.2 {
                if d > 0.6 {
                    mix(b[3], h[3], 0.3 * fl)
                } else {
                    b[2]
                }
            } else if d > 0.55 {
                b[1]
            } else {
                b[2]
            }
        },
        1.0,
    );
    p.rect(-8.0, 2.0, 17.0, 1.0, b[1], 1.0);
    p.rect(-7.0, 3.0, 15.0, 1.0, b[0], 1.0);
}

// the candle: a wax pillar with a dark outline, a highlight band on the lit side, a shadow side, a pool of
// melted wax lit by the flame, a long glossy drip down the right ending in a bulb and a short one on the left
fn candle(p: &mut Painter, x: &[Rgba; 6], h: &Hot, fl: f64) {
    let l = TOP;
    let cols = [x[1], x[4], x[5], x[4], x[4], x[3], x[3], x[2], x[0]];
    for (i, &c) in cols.iter().enumerate() {
        let xx = i as f64 - 4.0;
        p.rect(xx, l + 1.0, 1.0, -l - 2.0, c, 1.0);
        if i == 2 {
            p.rect(xx, -8.0, 1.0, 7.0, x[4], 1.0);
        }
    }
    p.rect(-3.0, -1.0, 7.0, 1.0, x[2], 1.0);
    p.px(-4.0, -1.0, x[0]);
    p.px(4.0, -1.0, x[0]);
    // the rim and the pool of melted wax round the wick, lit by the flame
    p.rect(-3.0, l, 7.0, 1.0, mix(x[5], h[2], 0.25 * fl), 1.0);
    p.px(-4.0, l, x[1]);
    p.px(4.0, l, x[1]);
    p.rect(-2.0, l + 1.0, 5.0, 1.0, mix(x[5], h[1], 0.5 * fl), 1.0);
    p.rect(-1.0, l + 1.0, 3.0, 1.0, mix(x[4], h[2], 0.45 * fl), 1.0);
    // the long drip: a lighter, glossy run down the right that bulges past the side into a bulb
    let y1 = DRIP_Y1;
    p.rect(3.0, l + 1.0, 1.0, y1 - l - 1.0, x[4], 1.0);
    p.rect(4.0, l + 2.0, 1.0, y1 - l - 2.0, x[3], 1.0);
    p.px(5.0, l + 3.0, x[0]);
    p.rect(5.0, l + 4.0, 1.0, y1 - l - 4.0, x[0], 1.0);
    p.rect(3.0, y1, 2.0, 2.0, x[4], 1.0);
    p.px(5.0, y1, x[3]);
    p.px(5.0, y1 + 1.0, x[2]);
    p.px(6.0, y1, x[0]);
    p.px(6.0, y1 + 1.0, x[0]);
    p.rect(3.0, y1 + 2.0, 3.0, 1.0, x[0], 1.0);
    p.px(3.0, l + 2.0, x[5]);
    // the short drip on the left
    p.rect(-4.0, l + 1.0, 1.0, 4.0, x[4], 1.0);
    p.px(-5.0, l + 3.0, x[1]);
    p.px(-5.0, l + 4.0, x[1]);
    p.px(-4.0, l + 5.0, x[3]);
}

struct Wisp {
    vx: f64,
    vy: f64,
    life: f64,
    s: f64,
}

impl Life for Wisp {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Mote {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    ph: f64,
}

impl Life for Mote {
    fn life(&self) -> f64 {
        self.life
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, rt, x, b, s) = (hot_first(&o.f), &o.rt, &o.x, &o.b, o.s);
    // calm: everything moves slowly, the flame now and then stretches tall
    let nf = |rate: f64, seed: f64, off: f64| NL.at(t, rate * 0.35, seed, off);
    let fl = 0.86 + 0.08 * NL.at(t, 2.0, s + 1.0, 0.0) + 0.06 * NL.at(t, 7.0, s + 2.0, 0.0);
    let stretch = js_max(0.0, NL.at(t, 0.75, s + 3.0, 0.0) - 0.3) * 0.6;
    let fh = 12.0 * (1.0 + stretch) * (0.94 + 0.06 * fl);
    let sway = NL.at(t, 0.5, s + 4.0, 0.0) * 1.6 + NL.at(t, 1.5, s + 5.0, 0.0) * 0.5;
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.3);
    let cy = WICK - fh * 0.4;

    // a pool of warm light on the floor and the halo behind the flame
    floor_light(p, 0.0, 1.0, 26.0, 5.0, glow, 0.3 * fl);
    p.glow(sway * 0.3, cy, 38.0 * fl, glow, 0.42 * fl);
    p.glow_ring(sway * 0.3, cy, 10.0, 4.0, h[2], 0.12 * fl);
    dish(p, b, &h, fl);
    candle(p, x, &h, fl);
    // the bead of wax: it swells at the rim, runs down the drip and sets into its bulb
    let run = (t % D) / D;
    let (swell, slide, set) = (
        smooth(0.0, 0.3, run),
        smooth(0.3, 0.85, run),
        smooth(0.85, 1.0, run),
    );
    if set < 1.0 && swell > 0.15 {
        let by = js_round(DRIP_Y0 + (DRIP_Y1 - DRIP_Y0) * slide);
        p.px(DRIP_X, by, mix(x[5], h[1], 0.4 * fl));
        if swell > 0.6 && set < 0.5 {
            p.px(DRIP_X + 1.0, by, x[4]);
            p.px(DRIP_X + 1.0, by + 1.0, x[2]);
        }
    }
    // the wick, glowing at its tip
    p.line(0.0, TOP, 0.0, WICK + 1.0, x[0], 1.0);
    p.px(0.0, WICK + 1.0, h[4]);
    // the flame and its blue root
    flame_tongue(
        p,
        0.0,
        WICK + 1.0,
        fh,
        2.6,
        sway,
        &nf,
        s * 31.0,
        &h,
        &CALM,
        0.25,
        0.6,
    );
    for dx in -2..=2 {
        let dx = f64::from(dx);
        let a = 1.0 - dx.abs() * 0.2;
        p.dot(
            dx,
            WICK + 1.0,
            if dx.abs() == 2.0 { rt[0] } else { rt[1] },
            a,
        );
        if dx.abs() <= 1.0 {
            p.dot(dx, WICK + 2.0, rt[0], 0.7);
        }
    }
    p.px(0.0, WICK, rt[2]);
    // a thread of smoke now and then off the tip
    looped(
        t,
        D,
        6,
        s + 300.0,
        |r, _| Wisp {
            vx: r.range(-1.5, 1.5),
            vy: -r.range(8.0, 12.0),
            life: r.range(1.4, 2.2),
            s: r.range(0.8, 1.2),
        },
        |q, age, _| {
            let [x, y, ..] = drift(sway, WICK - fh, q.vx, q.vy, age, 1.0, -6.0, 1.0, 0.8);
            smoke_puff(
                p,
                x + NL.at(t - age, 0.6, s + 60.0, 0.0) * (1.0 + age * 2.0),
                y,
                1.0 - age / q.life,
                q.s,
                None,
                false,
                0.6,
            );
        },
    );
    // motes drifting in the candlelight, catching it as they pass close
    looped(
        t,
        D,
        10,
        s + 400.0,
        |r, _| Mote {
            x: r.range(-20.0, 20.0),
            y: r.range(-44.0, -8.0),
            vx: r.range(-3.0, 3.0),
            vy: r.range(-4.0, 2.0),
            life: r.range(2.5, 4.0),
            ph: r.range(0.0, 6.3),
        },
        |q, age, _| {
            let x = q.x + q.vx * age + libm::sin(age * 1.3 + q.ph) * 2.0;
            let y = q.y + q.vy * age;
            let d = libm::hypot(x, y - cy);
            let k = js_min(js_min(1.0, age * 2.0), (q.life - age) * 2.0)
                * clamp(1.2 - d / 26.0, 0.0, 1.0);
            if k > 0.15 {
                p.dot(x, y, if k > 0.6 { h[1] } else { h[3] }, k);
                p.glow(x, y, 3.0, h[2], 0.2 * k);
            }
        },
    );
    // the hot core of the light
    p.glow(sway * 0.5, WICK - fh * 0.3, 10.0, h[1], 0.35 * fl);
}
