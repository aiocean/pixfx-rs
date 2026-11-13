//! Spirit flame: port of pixelfx `src/fx/fire/spirit-flame.js`.
//!
//! The layered tongue, its noise-torn edges and the shed puffs are adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{TAU, js_max, js_min, js_round};

use super::lib::{Hot, Life, LoopNoise, Noise, drift, hot_first, looped, soft};

pub struct SpiritFlame;

pub static SPIRIT_FLAME: SpiritFlame = SpiritFlame;

const META: Meta = Meta {
    id: "fire/spirit-flame",
    name: "Spirit flame",
    desc: "A cold blue-teal ghost fire floating in the air: a bright rim round a hollow dark heart, a wave climbing its wispy tail, curls breaking off, cold sparks rising.",
    tags: &[
        "fire", "spirit", "ghost", "magic", "cold", "ambient", "loop",
    ],
    size: (80, 124),
    origin: (40, 94),
    duration: 3.0,
    looping: true,
    fps: 30,
    seed: 7,
    state: None,
    options: &[],
};

// meta.options
const PALETTE: [Rgba; 9] = [
    Rgba::hex24(0x040a16),
    Rgba::hex24(0x08203a),
    Rgba::hex24(0x0b3a52),
    Rgba::hex24(0x0f5e6e),
    Rgba::hex24(0x16908e),
    Rgba::hex24(0x2cc4b4),
    Rgba::hex24(0x7ef0dc),
    Rgba::hex24(0xc8fff2),
    Rgba::hex24(0xffffff),
];

const D: f64 = META.duration;
const NL: LoopNoise = LoopNoise(D);
// inverted layers: the rim burns brightest, a teal band inside it, a dark hollow heart, a pale core low down
const GHOST: [(f64, f64, usize); 5] = [
    (1.0, 1.0, 1),
    (0.8, 0.92, 3),
    (0.6, 0.78, 6),
    (0.3, 0.42, 2),
    (0.14, 0.22, 0),
];

impl Effect for SpiritFlame {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            f: ramp(&opts.palette_or(&PALETTE)),
            s: seed as f64,
        })
    }
}

struct State {
    f: Hot,
    s: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

// a tongue whose centre line carries a wave climbing it: drawn row by row in layers like a fire tongue
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
fn ghost_tongue(
    p: &mut Painter,
    x: f64,
    y: f64,
    h: f64,
    w: f64,
    centre: impl Fn(f64, f64) -> f64,
    nf: Noise,
    n: f64,
    hh: &Hot,
    layers: &[(f64, f64, usize)],
    jit: f64,
) {
    for &(lw, lh, ri) in layers {
        let l = h * lh;
        let c = hh.get(ri).copied().unwrap_or(Rgba::NONE);
        let mut r = 0.0;
        while r <= l {
            let u = r / js_max(1.0, l);
            let v = r / h;
            // a teardrop: round at the bottom, swelling, then drawn out thin
            let half = w * lw * libm::pow(1.0 - u, 0.75) * js_min(1.0, (r + 1.5) / 4.0)
                + nf(6.0, n, r * 0.4) * jit * u;
            if half >= 0.3 {
                let cx = x + centre(v, r);
                let yy = js_round(y - r);
                let (mut px, x1) = (js_round(cx - half), js_round(cx + half));
                while px <= x1 {
                    p.px(px, yy, c);
                    px += 1.0;
                }
            }
            r += 1.0;
        }
    }
}

struct Curl {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    s: f64,
    a: f64,
    w: f64,
}

impl Life for Curl {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Spark {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    a: f64,
}

impl Life for Spark {
    fn life(&self) -> f64 {
        self.life
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, s) = (hot_first(&o.f), o.s);
    let ph = t / D;
    let nf = |rate: f64, seed: f64, off: f64| NL.at(t, rate, seed, off);
    let cold = mix(h[3], WHITE, 0.15);
    let bob = libm::sin(ph * TAU) * 1.5;
    // the wave: it climbs the flame twice a loop, growing toward the tip, and the whole flame leans with a slow drift
    let lean = nf(0.67, s + 2.0, 0.0) * 3.0;
    let wave = |v: f64, r: f64, k: f64| {
        (libm::sin(r * 0.24 - ph * TAU * 2.0 + k) * 2.2
            + libm::sin(r * 0.11 - ph * TAU + k * 0.5) * 3.2)
            * v
            * v
            + lean * v * v
    };
    let h0 = 44.0 * (0.92 + 0.08 * nf(1.3, s + 3.0, 0.0));
    let y0 = bob;

    // the glow it casts, cold and slow
    let br = 0.9 + 0.1 * nf(2.0, s + 4.0, 0.0);
    p.glow(0.0, y0 - 16.0, 60.0, cold, 0.36 * br);
    p.glow(wave(0.5, 20.0, 0.0) * 0.5, y0 - 20.0, 26.0, h[2], 0.2 * br);
    // curls breaking off the tips and drifting up, thinning out
    looped(
        t,
        D,
        40,
        s + 300.0,
        |r, _| Curl {
            x: r.range(-4.0, 4.0),
            y: -h0 * r.range(0.55, 0.95),
            vx: r.range(-7.0, 7.0),
            vy: -r.range(10.0, 22.0),
            life: r.range(1.0, 1.9),
            s: r.range(1.4, 2.8),
            a: r.range(0.0, TAU),
            w: r.range(2.0, 3.5),
        },
        |q, age, _| {
            let [x, y, ..] = drift(
                q.x + wave(0.9, 26.0, 0.0) * 0.5,
                q.y + y0,
                q.vx,
                q.vy,
                age,
                0.0,
                -6.0,
                1.2,
                0.6,
            );
            let k = 1.0 - age / q.life;
            let cx = x + libm::sin(age * q.w + q.a) * 3.0 * (age + 0.2);
            let a = js_min(1.0, age * 5.0) * k;
            soft(p, cx, y, q.s * (0.8 + age * 0.8), h[4], 0.35 * a);
            soft(
                p,
                cx - 0.4,
                y - 0.6,
                q.s * 0.5 * (0.8 + age * 0.5),
                h[2],
                0.4 * a * k,
            );
            p.glow(cx, y, 6.0, h[3], 0.15 * a);
        },
    );
    // side wisps that flicker out and curl, now long, now gone
    let wl = js_max(0.0, 0.25 + 0.55 * nf(1.7, s + 6.0, 0.0));
    let wr = js_max(0.0, 0.25 + 0.55 * nf(1.7, s + 7.0, 0.0));
    if wl > 0.12 {
        ghost_tongue(
            p,
            -4.0,
            y0 - 4.0,
            h0 * wl,
            2.4,
            |v, r| wave(v, r, 2.1) - 6.0 * v * v + 3.0 * libm::sin(v * 5.0 - ph * TAU * 2.0) * v,
            &nf,
            s * 11.0 + 1.0,
            &h,
            &GHOST,
            0.5,
        );
    }
    if wr > 0.12 {
        ghost_tongue(
            p,
            4.2,
            y0 - 4.0,
            h0 * wr,
            2.4,
            |v, r| {
                wave(v, r, -1.8) + 6.0 * v * v - 3.0 * libm::sin(v * 5.0 - ph * TAU * 2.0 + 1.0) * v
            },
            &nf,
            s * 11.0 + 2.0,
            &h,
            &GHOST,
            0.5,
        );
    }
    ghost_tongue(
        p,
        0.0,
        y0,
        h0,
        8.6,
        |v, r| wave(v, r, 0.0),
        &nf,
        s * 11.0,
        &h,
        &GHOST,
        0.6,
    );
    // bright strands drifting up through the hollow heart
    for k in [0.0, 1.0] {
        let mut r = 4.0;
        while r < h0 * 0.72 {
            let v = r / h0;
            let x = wave(v, r, 0.0)
                + libm::sin(r * 0.35 - ph * TAU * 3.0 + k * core::f64::consts::PI)
                    * 2.2
                    * (1.0 - v);
            let on = libm::sin(r * 0.2 - ph * TAU * 4.0 + k * 2.0) > 0.2;
            if on {
                p.px(x, y0 - r, if k != 0.0 { h[2] } else { h[1] });
            }
            r += 1.0;
        }
    }
    // light along the body
    for v in [0.2, 0.45, 0.7] {
        p.glow(wave(v, h0 * v, 0.0), y0 - h0 * v, 12.0, h[2], 0.16 * br);
    }
    // the pale heart breathing low in the flame
    p.glow(
        0.0,
        y0 - 4.0,
        9.0,
        h[1],
        0.22 + 0.06 * nf(3.0, s + 5.0, 0.0),
    );
    // cold sparks rising off it, twinkling
    looped(
        t,
        D,
        34,
        s + 400.0,
        |r, _| Spark {
            x: r.range(-7.0, 7.0),
            y: -r.range(4.0, 28.0),
            vx: r.range(-12.0, 12.0),
            vy: -r.range(14.0, 34.0),
            life: r.range(0.9, 1.8),
            a: r.range(0.0, TAU),
        },
        |q, age, _| {
            let [x, y, ..] = drift(q.x, q.y + y0, q.vx, q.vy, age, 0.0, -10.0, 1.2, 1.2);
            let k = 1.0 - age / q.life;
            let tw = 0.5 + 0.5 * libm::sin(age * 14.0 + q.a);
            let sx = x + libm::sin(age * 3.0 + q.a) * 3.0;
            if k * (0.4 + 0.6 * tw) < 0.2 {
                return;
            }
            p.px(
                sx,
                y,
                if tw > 0.7 && k > 0.5 {
                    h[0]
                } else if k > 0.5 {
                    h[1]
                } else {
                    h[3]
                },
            );
            if tw > 0.85 && k > 0.6 {
                p.px(sx - 1.0, y, h[3]);
                p.px(sx + 1.0, y, h[3]);
                p.px(sx, y - 1.0, h[3]);
                p.px(sx, y + 1.0, h[3]);
            }
            p.glow(sx, y, 4.0, h[2], 0.3 * k);
        },
    );
}
