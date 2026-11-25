//! Flamethrower: port of pixelfx `src/fx/fire/flamethrower.js`.
//!
//! The flame-puff, smoke and spark recipe is adapted from Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{js_max, js_min, js_round, smooth};

use super::lib::{
    Hot, LAYERS, drift, fire_puff, flame_tongue, hot_first, looped, nl, pnoise, smoke_puff, solid,
    spark,
};

pub struct Flamethrower;

pub static FLAMETHROWER: Flamethrower = Flamethrower;

// the stream is drawn a column per pixel of its reach: held to what draws in reasonable time
const REACH: NumOpt = NumOpt::new("reach", 112.0).within(1.0, 1000.0);
const SPEED: NumOpt = NumOpt::new("speed", 280.0);
const META: Meta = Meta {
    id: "fire/flamethrower",
    name: "Flamethrower",
    desc: "A pressurised jet shooting right: a blue pilot at the nozzle, a tight stream with surges racing down it, a sputter, burning drops falling off, a tip that curls up into smoke.",
    tags: &[
        "fire",
        "flamethrower",
        "stream",
        "weapon",
        "directional",
        "loop",
    ],
    size: (184, 104),
    origin: (30, 60),
    duration: 1.6,
    looping: true,
    fps: 30,
    seed: 3,
    state: None,
    options: &[PILOT.spec(), REACH.spec(), SPEED.spec()],
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
const PILOT: RampOpt = RampOpt::new(
    "pilot",
    &[
        Rgba::hex24(0x1b2a8a),
        Rgba::hex24(0x3a62e0),
        Rgba::hex24(0xa8d4ff),
    ],
);

const D: f64 = 1.6;
const PI: f64 = core::f64::consts::PI;
const SOOT: Rgba = Rgba::hex24(0x2a2430);
// the sputter: the jet coughs once a loop, then surges back
const COUGH: f64 = 0.55;

fn flow(te: f64) -> f64 {
    let u = ((te % D) + D) % D;
    1.0 - smooth(COUGH, COUGH + 0.04, u) * (1.0 - smooth(COUGH + 0.1, COUGH + 0.16, u))
        + 0.25 * libm::exp(-libm::pow((u - COUGH - 0.2) / 0.05, 2.0))
}

impl Effect for Flamethrower {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            h: hot_first(&opts.palette_or(&PALETTE)),
            pl: ramp(&opts.ramp(&PILOT)),
            reach: opts.num(&REACH),
            speed: opts.num(&SPEED),
            seed: seed as f64,
        })
    }
}

struct State {
    h: Hot,
    pl: [Rgba; 3],
    reach: f64,
    speed: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

struct Curl {
    a0: f64,
    r: f64,
    w: f64,
    up: f64,
    dx: f64,
    fl: f64,
    sl: f64,
    s: f64,
    f: f64,
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, pl, s, l, v) = (&o.h, &o.pl, o.seed, o.reach, o.speed);
    let n = |rate: f64, seed: f64, off: f64| nl(D, t, rate, seed, off);
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.25);
    let sparks = [h[1], h[2], h[4], h[5]];
    let smoke = mix(SOOT, h[7], 0.3);
    // the stream's centre line: dead straight at the nozzle, sagging and wavering as it runs out
    let cy = |x: f64| {
        5.0 * libm::pow(x / l, 2.0) + n(1.2, s + 9.0, x * 0.02) * 2.2 * libm::pow(x / l, 1.5)
    };
    let width = |x: f64| (2.1 + 5.0 * libm::pow(x / l, 0.7)) * flow(t - x / v);

    // the curl: where the stream runs out of push, burning fuel rolls up and back on itself and turns to smoke
    let mut fire: Vec<(f64, f64, f64, f64, f64, f64)> = Vec::new();
    let mut smokes: Vec<(f64, f64, f64, f64)> = Vec::new();
    looped(
        t,
        D,
        80.0,
        s + 100.0,
        |r, _| {
            let q = Curl {
                a0: r.range(-0.3, 0.5),
                r: r.range(9.0, 15.0),
                w: r.range(5.0, 7.5),
                up: r.range(4.0, 12.0),
                dx: r.range(-6.0, 2.0),
                fl: r.range(0.38, 0.62),
                sl: r.range(0.6, 1.0),
                s: r.range(2.4, 4.4),
                f: r.next(),
            };
            (1.6, q)
        },
        |q, age, _| {
            let tb = t - age;
            let k = flow(tb - l / v);
            if k < 0.4 {
                return;
            }
            // round a curl: forward, up and back over the top, rising as it goes
            let (x0, y0) = (l + q.dx, cy(l));
            let a = js_min(age, q.fl) * q.w + q.a0;
            let x = x0 + q.r * libm::sin(a);
            let y = y0 - q.r * (1.0 - libm::cos(a)) - q.up * age;
            if age < q.fl {
                fire.push((age / q.fl, x, y, q.s, q.f, k));
                return;
            }
            if age - q.fl < q.sl {
                smokes.push((
                    (age - q.fl) / q.sl,
                    x - (age - q.fl) * 6.0,
                    y - (age - q.fl) * 16.0,
                    q.s,
                ));
            }
        },
    );
    for &(u, x, y, sz) in &smokes {
        smoke_puff(p, x, y, 1.0 - u, sz * 1.2, Some(smoke), true, 0.85);
    }
    // oldest first (JS's stable sort, descending)
    fire.sort_by(|a, b| b.0.total_cmp(&a.0));
    for &(u, x, y, sz, f, k) in &fire {
        fire_puff(
            p,
            x,
            y,
            1.0 - u,
            sz * (0.8 + 1.1 * u) * k,
            f,
            h,
            0.1,
            0.0,
            -1.0,
        );
    }

    // flames licking up off the burning fuel, riding along the stream, swept back by the wind of it
    for i in 0..13 {
        let nn = (s * 3.0 + f64::from(i)) * 19.0;
        let ph = (f64::from(i) * 0.618 + t / D) % 1.0;
        let x = 10.0 + ph * (l - 14.0);
        let k = flow(t - x / v);
        if k < 0.35 {
            continue;
        }
        let hh = (2.0 + 11.0 * (x / l))
            * (0.55 + 0.6 * (0.5 + 0.5 * n(5.0, nn + 1.0, 0.0)))
            * k
            * js_min(1.0, (1.0 - ph) * 6.0);
        flame_tongue(
            p,
            x,
            cy(x) - width(x) * 0.6,
            hh,
            1.1 + 1.6 * (x / l),
            -hh * 0.55,
            &n,
            nn,
            h,
            &LAYERS,
            0.5,
            1.1,
        );
    }
    // the stream: a white core in yellow, orange and a red skin, surges racing down it
    let per = js_round(D * 3.0);
    let lv = [h[0], h[2], h[4], h[6], h[7]];
    let mut x = 0.0;
    while x <= l {
        let c = cy(x);
        let w = width(x) * (1.0 + 0.15 * pnoise(x * 0.4 - (t / D) * 24.0, 24.0, s + 4.0));
        if w >= 0.4 {
            let surge = if (((x * 0.035 - (t / D) * per) % 1.0) + 1.0) % 1.0 < 0.12 {
                1.0
            } else {
                0.0
            };
            let mut y = libm::floor(c - w - 1.0);
            let y1 = libm::ceil(c + w + 1.0);
            while y <= y1 {
                let q = libm::fabs(y + 0.5 - c) / w;
                if q <= 1.15 {
                    // a white core near the nozzle cooling to yellow down the stream; ragged red skin
                    let lvl = (if q < 0.3 {
                        0.0
                    } else if q < 0.55 {
                        1.0
                    } else if q < 0.78 {
                        2.0
                    } else if q < 1.0 {
                        3.0
                    } else {
                        4.0
                    }) + if x > l * 0.45 && q < 0.55 { 1.0 } else { 0.0 };
                    let lvl = js_max(0.0, js_min(4.0, lvl - surge));
                    if !(lvl == 4.0
                        && pnoise(x * 0.5 + y * 7.3 - (t / D) * 40.0, 40.0, s + 6.0) < 0.0)
                    {
                        p.paint_cell(
                            x,
                            y,
                            lv.get(lvl as usize).copied().unwrap_or(Rgba::NONE),
                            1.0,
                        );
                    }
                }
                y += 1.0;
            }
            // two hot strands twisting through the fuel, racing downstream
            for ph in [0.0, PI] {
                let a = x * 0.22 - (t / D) * PI * 2.0 * 9.0 + ph;
                let sy = c + libm::sin(a) * w * 0.55;
                if libm::cos(a) > -0.1 && w > 1.2 {
                    p.px(x, sy, if libm::cos(a) > 0.55 { h[0] } else { h[1] });
                }
            }
        }
        x += 1.0;
    }
    // the pilot at the nozzle: a blue cone of gas where it lights
    let pk = flow(t);
    for x in 0..6 {
        for y in -2..=2 {
            let (x, y) = (f64::from(x), f64::from(y));
            if libm::fabs(y) <= 2.0 - x * 0.3 {
                p.paint_cell(
                    x,
                    y,
                    if libm::fabs(y) < 1.0 {
                        if x < 2.0 { pl[2] } else { pl[1] }
                    } else {
                        pl[0]
                    },
                    1.0,
                );
            }
        }
    }
    solid(p, 1.0, 0.0, 1.6 * pk, WHITE);

    // burning drops shaken off the stream: they arc down trailing flame and burn out
    looped(
        t,
        D,
        10.0,
        s + 200.0,
        |r, _| {
            let d = r.range(20.0, 95.0);
            let vx = r.range(0.3, 0.6);
            let vy = r.range(-25.0, 15.0);
            let life = r.range(0.35, 0.6);
            (life, (d, vx, vy, life))
        },
        |&(d, vx, vy, life), age, _| {
            let tb = t - age;
            if flow(tb - d / v) < 0.5 {
                return;
            }
            let at = |a: f64| drift(d, cy(d), v * vx, vy, a, 0.0, 260.0, 1.2, 0.5);
            for j in (1..=4).rev() {
                let j = f64::from(j);
                let a = age - j * 0.035;
                if a < 0.0 {
                    continue;
                }
                let (x, y, _, _) = at(a);
                fire_puff(
                    p,
                    x,
                    y,
                    1.0 - js_min(0.99, j * 0.18 + age / life * 0.4),
                    1.8 - j * 0.25,
                    0.3,
                    h,
                    0.06,
                    0.0,
                    -1.0,
                );
            }
            let (x, y, _, _) = at(age);
            let k = 1.0 - age / life;
            p.px(x, y, if k > 0.5 { h[0] } else { h[2] });
            p.px(x + 1.0, y, h[3]);
            p.glow(x, y, 6.0, h[3], 0.45 * k);
        },
    );
    // the cough: when the jet sputters the nozzle spits sparks and a puff of smoke
    looped(
        t,
        D,
        12.0,
        s + 300.0,
        |r, _| {
            let born = COUGH + r.range(0.0, 0.08);
            let a = r.range(-0.8, 0.8);
            let vm = r.range(120.0, 240.0);
            let life = r.range(0.2, 0.45);
            (life, (born, a, vm, life))
        },
        |&(born, a, vm, life), age, _| {
            let u = ((t - age) % D + D) % D;
            if libm::fabs(u - born) > 0.06 {
                return;
            }
            let (x, y, vx, vy) = drift(
                4.0,
                0.0,
                libm::cos(a) * vm,
                libm::sin(a) * vm,
                age,
                0.0,
                300.0,
                2.5,
                2.5,
            );
            spark(p, x, y, vx, vy, 1.0 - age / life, &sparks);
        },
    );
    // and spits a few blobs of burning fuel that fall short
    looped(
        t,
        D,
        5.0,
        s + 350.0,
        |r, _| {
            let born = COUGH + r.range(0.01, 0.1);
            let vm = r.range(90.0, 170.0);
            let vy = r.range(-30.0, 10.0);
            let life = r.range(0.25, 0.4);
            let sz = r.range(1.6, 2.6);
            (life, (born, vm, vy, life, sz))
        },
        |&(born, vm, vy, life, sz), age, _| {
            let u = ((t - age) % D + D) % D;
            if libm::fabs(u - born) > 0.05 {
                return;
            }
            let (x, y, _, _) = drift(4.0, 0.0, vm, vy, age, 0.0, 200.0, 2.0, 1.0);
            fire_puff(p, x, y, 1.0 - age / life, sz, 0.2, h, 0.1, -0.86, -0.5);
        },
    );
    {
        let u = ((t % D) + D) % D;
        let a = u - COUGH;
        if a > 0.0 && a < 0.7 {
            smoke_puff(
                p,
                8.0 + a * 20.0,
                -a * 18.0,
                1.0 - a / 0.7,
                3.0,
                Some(smoke),
                false,
                0.8,
            );
        }
    }

    // light: the pilot, the stream, the burning tip
    p.glow(2.0, 0.0, 34.0 * pk, glow, 0.8 * pk);
    p.glow(2.0, 0.0, 9.0, pl[2], 0.45 * pk);
    let mut d = 8.0;
    while d <= l {
        p.glow(d, cy(d), 12.0 + d * 0.16, glow, 0.3 * flow(t - d / v));
        d += 8.0;
    }
    p.glow(l, cy(l) - 10.0, 52.0, glow, 0.34 * flow(t - l / v));
}
