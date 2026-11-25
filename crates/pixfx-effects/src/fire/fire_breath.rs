//! Fire breath: port of pixelfx `src/fx/fire/fire-breath.js`.
//!
//! The flame-puff, smoke and spark recipe is adapted from Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::{clamp, js_min};

use super::lib::{
    BACK_LAYERS, Hot, LAYERS, Layers, SMOKE1, drift, ember, fire_puff, flame_blade, flame_tongue,
    hot_first, looped, nl, smoke_puff, spark,
};

pub struct FireBreath;

pub static FIRE_BREATH: FireBreath = FireBreath;

const SPREAD: NumOpt = NumOpt::new("spread", 0.24);
const SPEED: NumOpt = NumOpt::new("speed", 320.0);
const META: Meta = Meta {
    id: "fire/fire-breath",
    name: "Fire breath",
    desc: "A roaring cone of fire streaming right from the mouth: white-hot at the lips, billowing out and cooling to red and smoke at its edges, sparks thrown out ahead.",
    tags: &[
        "fire",
        "breath",
        "dragon",
        "stream",
        "cone",
        "directional",
        "loop",
    ],
    size: (160, 116),
    origin: (12, 72),
    duration: 1.5,
    looping: true,
    fps: 30,
    seed: 6,
    state: None,
    options: &[SPREAD.spec(), SPEED.spec()],
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

const D: f64 = 1.5;
// the dying licks: red and deep orange, no white heart
const COOL_LAYERS: [(f64, f64, usize); 3] = [(1.0, 1.0, 7), (0.72, 0.82, 6), (0.44, 0.58, 5)];
// the jet at the lips: (length, half-width, angle)
const JET: [(f64, f64, f64); 4] = [
    (42.0, 4.2, 0.0),
    (32.0, 3.8, -0.08),
    (30.0, 3.8, 0.08),
    (20.0, 4.6, 0.0),
];

impl Effect for FireBreath {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            h: hot_first(&opts.palette_or(&PALETTE)),
            spread: opts.num(&SPREAD),
            speed: opts.num(&SPEED),
            seed: seed as f64,
        })
    }
}

struct State {
    h: Hot,
    spread: f64,
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

struct Puff {
    e: f64,
    sp: f64,
    fl: f64,
    sm: bool,
    sl: f64,
    s: f64,
    f: f64,
    ox: f64,
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, s, a_, v_) = (&o.h, o.seed, o.spread, o.speed);
    let n = |rate: f64, seed: f64, off: f64| nl(D, t, rate, seed, off);
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.25);
    let smoke = mix(SMOKE1, h[6], 0.3);
    let sparks = [h[1], h[2], h[4], h[5]];
    let emb = [h[1], h[2], h[4], h[6]];
    // the roar: the stream surges and the cone sways a little
    let roar = |tt: f64| 0.85 + 0.15 * nl(D, tt, 3.0, s + 1.0, 0.0);
    let sway = |tt: f64| nl(D, tt, 1.3, s + 2.0, 0.0) * 0.05;

    // the cone: puffs of flame blown out of the mouth that slow, swell, rise and cool; those near the edges
    // cool first, and many burn out into rolling smoke
    let mut fire: Vec<(f64, f64, f64, f64, f64, f64, f64)> = Vec::new();
    let mut smokes: Vec<(f64, f64, f64, f64, f64)> = Vec::new();
    looped(
        t,
        D,
        260.0,
        s + 100.0,
        |r, _| {
            let e = r.range(-1.0, 1.0);
            let fl = r.range(0.42, 0.8) * (1.0 - 0.3 * libm::fabs(e));
            let sm = r.chance(0.2 + 0.4 * libm::fabs(e));
            let q = Puff {
                e,
                sp: r.range(0.75, 1.1),
                fl,
                sm,
                sl: r.range(0.5, 0.85),
                s: r.range(3.0, 5.2),
                f: r.next(),
                ox: r.range(4.0, 10.0),
            };
            (fl + if sm { 0.85 } else { 0.0 }, q)
        },
        |q, age, _| {
            let tb = t - age;
            let a = q.e * a_ + sway(tb) - 0.04;
            let v = v_ * q.sp * roar(tb);
            let ta = js_min(age, q.fl);
            let (x, y, vx, vy) = drift(
                q.ox,
                q.e * 1.5,
                libm::cos(a) * v,
                libm::sin(a) * v,
                ta,
                0.0,
                -100.0,
                2.0,
                2.4,
            );
            if age < q.fl {
                fire.push((age, x, y, q.fl, q.e, q.s, q.f));
                return;
            }
            if !q.sm || age - q.fl > q.sl {
                return;
            }
            let sa = age - q.fl;
            let (sx, sy, _, _) = drift(x, y, vx * 0.5, vy * 0.6 - 8.0, sa, 0.0, -26.0, 1.8, 1.2);
            smokes.push((sa, sx, sy, q.sl, q.s));
        },
    );
    for &(sa, x, y, sl, sz) in &smokes {
        smoke_puff(p, x, y, 1.0 - sa / sl, sz * 1.1, Some(smoke), true, 0.7);
    }
    // oldest first (JS's stable sort by age, descending)
    fire.sort_by(|a, b| b.0.total_cmp(&a.0));
    for &(age, x, y, fl, e, sz, f) in &fire {
        let u = age / fl;
        let edge = libm::fabs(e);
        let size = sz * (0.75 + 1.6 * u);
        // the heart of the stream stays hot longer than its edges
        let heat = js_min(
            0.999,
            libm::pow(u, 1.5) * (0.85 + 0.2 * edge) + 0.3 * edge * edge,
        );
        // near the mouth the fire rolls in round billows; further out each billow tears into a tongue licking up
        // and back, thinning to a lick as it cools, before the smoke takes it
        let tear = clamp((u - 0.32) / 0.2, 0.0, 1.0);
        fire_puff(
            p,
            x,
            y,
            1.0 - heat,
            size * (1.0 - 0.55 * tear),
            f,
            h,
            0.1,
            -0.5,
            -0.86,
        );
        if tear > 0.0 {
            let nn = libm::floor(f * 1e5);
            let hh = size * (1.5 + 0.5 * n(4.0, nn, 0.0)) * (1.2 - heat * 0.6) * tear;
            let layers: &Layers = if heat < 0.45 {
                &LAYERS
            } else if heat < 0.72 {
                &BACK_LAYERS
            } else {
                &COOL_LAYERS
            };
            flame_tongue(
                p,
                x,
                y + size * 0.3,
                hh,
                size * 0.5 * (1.1 - heat * 0.5),
                -hh * 0.4 + n(2.5, nn + 1.0, 0.0) * 2.0,
                &n,
                nn,
                h,
                layers,
                0.5,
                1.1,
            );
        }
    }
    // licks curling off the edges of the cone
    for i in 0..6 {
        let nn = (s * 3.0 + f64::from(i)) * 19.0;
        let side = if i % 2 == 1 { 1.0 } else { -1.0 };
        let d = 18.0 + f64::from(i >> 1) * 16.0 + n(2.0, nn, 0.0) * 4.0;
        let l = (9.0 + 5.0 * (0.5 + 0.5 * n(5.0, nn + 1.0, 0.0))) * (1.0 - d / 120.0);
        let ex = d;
        let ey = side * d * a_ * 0.85 - d * 0.06;
        flame_blade(
            p,
            ex,
            ey,
            0.75,
            side * 0.45 - 0.48,
            l,
            1.6 + l * 0.08,
            -side * 3.0,
            &n,
            nn,
            h,
            &LAYERS,
            0.5,
            1.1,
        );
    }
    // the roar: bright streaks racing out through the heart of the cone
    looped(
        t,
        D,
        70.0,
        s + 250.0,
        |r, _| {
            let e = r.range(-0.6, 0.6);
            let v = r.range(330.0, 470.0);
            let life = r.range(0.12, 0.26);
            let l = r.range(4.0, 9.0);
            (life, (e, v, life, l))
        },
        |&(e, v, life, ql), age, _| {
            let a = e * a_ - 0.03;
            let (x, y, vx, vy) = drift(
                6.0,
                e * 2.0,
                libm::cos(a) * v,
                libm::sin(a) * v,
                age,
                0.0,
                -60.0,
                2.0,
                2.0,
            );
            let sp = libm::hypot(vx, vy);
            let k = 1.0 - age / life;
            let l = ql * (sp / 400.0);
            p.line(
                x - (vx / sp) * l,
                y - (vy / sp) * l,
                x,
                y,
                |u: f64| {
                    if u > 0.6 {
                        if k > 0.5 { h[0] } else { h[1] }
                    } else if k > 0.5 {
                        h[2]
                    } else {
                        h[3]
                    }
                },
                1.0,
            );
        },
    );
    // the jet at the lips: white-hot, overlapping tongues that flutter as the breath roars
    for (i, &(len, w, a)) in JET.iter().enumerate() {
        let nn = (s * 5.0 + i as f64) * 17.0;
        let l = len * roar(t) * (0.85 + 0.2 * n(5.0, nn, 0.0));
        let aa = a + sway(t);
        flame_blade(
            p,
            0.0,
            0.0,
            libm::cos(aa),
            libm::sin(aa),
            l,
            w,
            n(3.0, nn + 3.0, 0.0) * 3.0,
            &n,
            nn,
            h,
            &LAYERS,
            0.5,
            1.5,
        );
    }

    // sparks flung out ahead and falling; embers lifting off the far end
    looped(
        t,
        D,
        40.0,
        s + 300.0,
        |r, _| {
            let d = r.range(4.0, 60.0);
            let e = r.range(-1.0, 1.0);
            let v = r.range(140.0, 280.0);
            let life = r.range(0.25, 0.55);
            (life, (d, e, v, life))
        },
        |&(d, e, v, life), age, _| {
            let a = e * a_ * 1.8 - 0.12;
            let (x, y, vx, vy) = drift(
                d,
                e * d * a_,
                libm::cos(a) * v,
                libm::sin(a) * v - 30.0,
                age,
                0.0,
                190.0,
                2.0,
                2.0,
            );
            spark(p, x, y, vx, vy, 1.0 - age / life, &sparks);
        },
    );
    looped(
        t,
        D,
        18.0,
        s + 400.0,
        |r, _| {
            let x = r.range(50.0, 100.0);
            let y = r.range(-14.0, 6.0);
            let vx = r.range(10.0, 40.0);
            let vy = -r.range(20.0, 50.0);
            let life = r.range(0.8, 1.4);
            let sz = if r.chance(0.2) { 2.0 } else { 1.0 };
            let ph = r.range(0.0, 6.3);
            (life, (x, y, vx, vy, life, sz, ph))
        },
        |&(qx, qy, vx, vy, life, sz, ph), age, _| {
            let (x, y, _, _) = drift(qx, qy, vx, vy, age, 0.0, -30.0, 1.6, 1.6);
            ember(
                p,
                x + 5.0 * (libm::sin(age * 2.3 + ph) - libm::sin(ph)),
                y,
                1.0 - age / life,
                sz,
                &emb,
                age * 40.0 + ph,
            );
        },
    );

    // light: a flare at the lips, light all along the cone widening with it, a broad halo
    let k = roar(t);
    p.glow(3.0, 0.0, 34.0 * k, glow, 0.75 * k);
    p.glow(3.0, 0.0, 10.0, h[0], 0.55);
    let mut d = 10.0;
    while d < 126.0 {
        p.glow(
            d,
            -d * 0.1,
            10.0 + d * 0.2,
            glow,
            0.32 * k * (1.0 - d / 150.0),
        );
        d += 8.0;
    }
    p.glow(64.0, -12.0, 96.0, glow, 0.16 * k);
}
