//! Fire tornado: port of pixelfx `src/fx/fire/fire-tornado.js`.
//!
//! The flame-puff, smoke and ember recipe is adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::{TAU, clamp, js_max, js_min, or_one};

use super::lib::{
    BACK_LAYERS, Hot, LAYERS, SMOKE1, drift, fire_puff, flame_tongue, floor_light, hot_first,
    looped, nl, pnoise, smoke_puff,
};

pub struct FireTornado;

pub static FIRE_TORNADO: FireTornado = FireTornado;

// the column is drawn a row per pixel of its height: held to what draws in reasonable time
const HEIGHT: NumOpt = NumOpt::new("height", 80.0).within(0.0, 400.0);
const META: Meta = Meta {
    id: "fire/fire-tornado",
    name: "Fire tornado",
    desc: "A spinning funnel of fire: spiral bands turning round it, licks flung off its sides, a ring of flame whirling at its foot, embers orbiting it in and out of view.",
    tags: &["fire", "tornado", "vortex", "spell", "aoe", "loop"],
    size: (120, 152),
    origin: (60, 136),
    duration: 2.0,
    looping: true,
    fps: 30,
    seed: 4,
    state: None,
    options: &[HEIGHT.spec()],
};

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

const D: f64 = 2.0;
const PI: f64 = core::f64::consts::PI;
// turns of the surface a loop
const SPIN: f64 = 3.0;

impl Effect for FireTornado {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            h: hot_first(&opts.palette_or(&PALETTE)),
            height: opts.num(&HEIGHT),
            seed: seed as f64,
        })
    }
}

struct State {
    h: Hot,
    height: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

struct Orbit {
    x: f64,
    y: f64,
    z: f64,
    k: f64,
    vx: f64,
    vy: f64,
}

// the funnel: narrow at the foot, flaring at the top, its spine snaking as it turns
fn cx(v: f64, tt: f64) -> f64 {
    3.2 * libm::sin(v * 4.2 - (tt / D) * TAU * 2.0) * v + 2.0 * libm::sin((tt / D) * TAU) * v
}

fn rad(v: f64) -> f64 {
    3.5 + 15.0 * libm::pow(v, 1.35)
}

// the surface turns, twisted up the column
fn spin(v: f64, tt: f64) -> f64 {
    (tt / D) * TAU * SPIN + v * 5.5
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, s, ht) = (&o.h, o.seed, o.height);
    let ph = t / D;
    let n = |rate: f64, seed: f64, off: f64| nl(D, t, rate, seed, off);
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.25);
    let smoke = mix(SMOKE1, h[6], 0.25);
    let emb = [h[1], h[2], h[4], h[6]];

    // embers orbiting: behind the column first, in front after it
    let mut orbit: Vec<Orbit> = Vec::new();
    looped(
        t,
        D,
        30.0,
        s + 500.0,
        |r, _| {
            let v0 = r.range(0.02, 0.7);
            let rise = r.range(0.12, 0.3);
            let a = r.range(0.0, TAU);
            let w = r.range(4.0, 7.0);
            let out = r.range(4.0, 12.0);
            let life = r.range(1.2, 2.4);
            // JS keeps `big`, which nothing reads; drawn for the rng
            let _big = r.chance(0.2);
            (life, (v0, rise, a, w, out, life))
        },
        |&(v0, rise, qa, w, out, life), age, _| {
            let v = v0 + rise * age;
            let rr = rad(js_min(1.0, v)) + out + age * 4.0;
            let a = qa + w * age;
            let k = 1.0 - age / life;
            let x = cx(js_min(1.0, v), t) + libm::cos(a) * rr;
            let y = -v * ht + libm::sin(a) * rr * 0.28;
            let z = libm::sin(a);
            orbit.push(Orbit {
                x,
                y,
                z,
                k,
                vx: -libm::sin(a) * rr * w,
                vy: libm::cos(a) * rr * w * 0.28,
            });
        },
    );
    let embers = |p: &mut Painter, front: bool| {
        for e in &orbit {
            if (e.z >= 0.0) != front || e.k <= 0.0 {
                continue;
            }
            let sp = or_one(libm::hypot(e.vx, e.vy));
            let l = js_min(5.0, 1.0 + sp * 0.025);
            let c = if e.k > 0.6 {
                emb[0]
            } else if e.k > 0.3 {
                emb[1]
            } else {
                emb[2]
            };
            let tail = if front { emb[3] } else { h[7] };
            p.line(
                e.x - (e.vx / sp) * l,
                e.y - (e.vy / sp) * l,
                e.x,
                e.y,
                |u: f64| if u > 0.6 { c } else { tail },
                1.0,
            );
            if front {
                p.glow(e.x, e.y, 4.0, c, 0.4 * e.k);
            } else {
                p.glow(e.x, e.y, 3.0, c, 0.15 * e.k);
            }
        }
    };
    // the ring of flame whirling at the foot: tongues round an ellipse, the far half dimmer
    let ring = |p: &mut Painter, front: bool| {
        for i in 0..12 {
            let a = (f64::from(i) / 12.0) * TAU + (t / D) * TAU * 2.0;
            let z = libm::sin(a);
            if (z >= 0.0) != front {
                continue;
            }
            let nn = (s * 3.0 + f64::from(i)) * 19.0;
            let rr = 12.0 + 2.0 * n(2.0, nn, 0.0);
            let x = libm::cos(a) * rr;
            let y = z * rr * 0.25;
            let hh =
                (5.0 + 5.0 * (0.5 + 0.5 * n(5.0, nn + 1.0, 0.0))) * if front { 1.0 } else { 0.8 };
            flame_tongue(
                p,
                x,
                y,
                hh,
                1.6,
                -libm::cos(a + PI / 2.0) * 3.0,
                &n,
                nn,
                h,
                if front { &LAYERS } else { &BACK_LAYERS },
                0.5,
                1.1,
            );
        }
    };

    // smoke spiralling off the top
    looped(
        t,
        D,
        20.0,
        s + 300.0,
        |r, _| {
            let a = r.range(0.0, TAU);
            let life = r.range(1.2, 2.0);
            let sz = r.range(2.5, 4.0);
            let vy = -r.range(12.0, 22.0);
            (life, (a, life, sz, vy))
        },
        |&(qa, life, sz, vy), age, _| {
            let rr = rad(1.0) * (0.8 + age * 0.5);
            let a = qa + age * 3.0;
            let x = cx(1.0, t - age) + libm::cos(a) * rr;
            let y = -ht - 2.0 + vy * age + libm::sin(a) * rr * 0.25;
            smoke_puff(p, x, y, 1.0 - age / life, sz, Some(smoke), false, 1.0);
        },
    );
    floor_light(p, 0.0, 0.5, 40.0, 8.0, glow, 0.4);
    embers(p, false);
    ring(p, false);

    // the column: each pixel's place round the funnel picks its heat; spiral bands turn up it; licks at its edges
    let fl = 0.85 + 0.15 * n(7.0, s + 1.0, 0.0);
    let mut y = 0.0;
    while y >= -ht * 1.2 {
        let v = -y / ht;
        let vv = js_min(1.0, v);
        let c = cx(vv, t);
        let r = rad(vv) * (1.0 + 0.08 * n(3.0, s + 2.0, y * 0.1));
        let sp = spin(v, t);
        // near the top the funnel comes apart into spiral strands of flame climbing past its rim
        let cut = clamp((v - 0.72) / 0.42, 0.0, 1.0);
        let lick_l = 0.3 * js_max(0.0, pnoise(-y * 0.22 + ph * 18.0, 18.0, s + 3.0));
        let lick_r = 0.3 * js_max(0.0, pnoise(-y * 0.22 + ph * 18.0 + 9.0, 18.0, s + 4.0));
        let mut x = libm::floor(c - r * 1.35);
        let x1 = libm::ceil(c + r * 1.35);
        while x <= x1 {
            let px = x;
            x += 1.0;
            let dx = (px + 0.5 - c) / r;
            let edge = 1.0 + if dx < 0.0 { lick_l } else { lick_r };
            if libm::fabs(dx) > edge {
                continue;
            }
            let th = libm::asin(clamp(dx, -1.0, 1.0));
            let depth = libm::cos(th);
            let tear = 0.18 * pnoise(y * 0.45 + dx * 2.0 - ph * 30.0, 30.0, s + 6.0);
            let band = 0.5 + 0.5 * libm::sin((th + sp + tear) * 3.0);
            let streak = band * band * band;
            if cut > 0.0
                && band < 0.35 + cut * 0.6 + 0.1 * pnoise(px * 0.7 + ph * 20.0, 20.0, s + 5.0)
            {
                continue;
            }
            // a darker body with thin bright streaks spiralling up it, hottest where it faces us, cooling up the column
            let mut heat =
                0.22 + 0.36 * depth + 0.5 * streak - 0.22 * vv + 0.1 * (1.0 - vv) - 0.25 * cut;
            if libm::fabs(dx) > 1.0 {
                heat = 0.3 - (libm::fabs(dx) - 1.0) * 1.5;
            }
            heat *= fl;
            let col = if heat > 0.95 {
                h[0]
            } else if heat > 0.82 {
                h[1]
            } else if heat > 0.7 {
                h[2]
            } else if heat > 0.58 {
                h[3]
            } else if heat > 0.46 {
                h[4]
            } else if heat > 0.34 {
                h[5]
            } else if heat > 0.2 {
                h[6]
            } else {
                h[7]
            };
            p.paint_cell(px, y, col, 1.0);
        }
        if y % 10.0 == 0.0 && v <= 1.0 {
            p.glow(c, y, r * 2.0 + 8.0, glow, 0.22 * fl);
        }
        y -= 1.0;
    }

    // flame flung off the spinning sides, cooling as it flies, some into smoke
    looped(
        t,
        D,
        70.0,
        s + 200.0,
        |r, _| {
            let v = r.range(0.05, 0.95);
            let side = r.sign();
            let sp = r.range(20.0, 50.0);
            let life = r.range(0.25, 0.5);
            let sz = r.range(1.4, 2.8);
            let f = r.next();
            (life, (v, side, sp, life, sz, f))
        },
        |&(v, side, sp, life, sz, f), age, _| {
            let tb = t - age;
            let x0 = cx(v, tb) + side * rad(v);
            let y0 = -v * ht;
            let (x, y, _, _) = drift(x0, y0, side * sp, -sp * 0.6, age, 0.0, -60.0, 3.0, 3.0);
            fire_puff(p, x, y, 1.0 - age / life, sz, f, h, 0.1, -side * 0.6, -0.8);
        },
    );
    ring(p, true);
    embers(p, true);
    // light: the column's own glow and a wash round it
    p.glow(0.0, -ht * 0.5, 80.0, glow, 0.28 * fl);
    p.glow(0.0, -6.0, 18.0, h[2], 0.3 * fl);
}
