//! Parry: port of pixelfx `src/fx/impacts/parry.js`.
//!
//! The star's key frames and the spark spray are adapted from Sprite Fusion's open-source impact
//! (https://www.spritefusion.com/pixel-art-vfx/impact), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::math::{Rng, TAU, ease, js_max, smooth, to_uint32};
use pixfx_core::sim::{Kind, Lights, Particles, ParticlesOpts, Spawn};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options, Painter};

use super::lib::{Seg, Spike, broken_ring, star};

pub struct Parry;

pub static PARRY: Parry = Parry;

const META: Meta = Meta {
    id: "impacts/parry",
    name: "Parry",
    desc: "Two blades clash: their edges flash in an X, a white four-point star rings out, a ring breaks into arcs and a fan of gold sparks sprays up and rains down.",
    tags: &[
        "impact", "parry", "block", "clash", "sword", "metal", "sparks", "one-shot",
    ],
    size: (200, 112),
    origin: (100, 96),
    duration: 1.0,
    looping: false,
    fps: 30,
    seed: 11,
    state: None,
    options: &[HEIGHT.spec()],
};

// meta.options
const PALETTE: [Rgba; 7] = [
    Rgba::hex24(0x2a1a08),
    Rgba::hex24(0x6a4410),
    Rgba::hex24(0xb07a1a),
    Rgba::hex24(0xe8b030),
    Rgba::hex24(0xffe066),
    Rgba::hex24(0xfff6c4),
    Rgba::hex24(0xffffff),
];
const HEIGHT: NumOpt = NumOpt::new("height", 30.0);

const X: f64 = 0.0;

impl Effect for Parry {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(WHITE);
        let y = -opts.num(&HEIGHT);
        let mut r = Rng::new(to_uint32(seed as f64));
        let mut ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: vec![c(4), c(4), c(3), c(2)],
            ..ParticlesOpts::default()
        });
        let mut lights = Lights::new();
        // a hard four-point star: long arms on the axes, short ones between
        let mut sp = Vec::with_capacity(8);
        for i in 0..8 {
            let a = f64::from(i) * core::f64::consts::PI / 4.0 + r.range(-0.04, 0.04);
            let len = if i % 2 == 1 {
                r.range(6.0, 8.0)
            } else if i % 4 == 0 {
                r.range(19.0, 22.0)
            } else {
                r.range(14.0, 17.0)
            };
            let w = if i % 2 == 1 { 1.2 } else { 2.2 };
            sp.push(Spike { a, len, w });
        }
        let mut segs = Vec::with_capacity(6);
        for i in 0..6 {
            let a = (f64::from(i) / 6.0) * TAU + r.range(-0.1, 0.1);
            let gap = r.range(0.55, 0.8);
            let dr = r.range(0.1, 0.35);
            segs.push(Seg { a, gap, dr, w: 1.0 });
        }
        // the two edges that met: lines crossing in an X
        let e0 = -0.62 + r.range(-0.08, 0.08);
        let e1 = core::f64::consts::PI + 0.62 + r.range(-0.08, 0.08);

        lights.add(X, y, 60.0, c(4), 0.65, 0.4);
        lights.add(X, y, 22.0, WHITE, 0.8, 0.08);
        // the fan: sparks thrown up and out to both sides, arcing over and raining down to bounce on the floor
        for i in 0..24 {
            let s = if i % 2 == 1 { 1.0 } else { -1.0 };
            let a = -core::f64::consts::FRAC_PI_2 + s * r.range(0.3, 1.3);
            let v = r.range(130.0, 290.0);
            let x = X + s * r.range(1.0, 3.0);
            let life = r.range(0.45, 0.85);
            let delay = if i > 17 { r.range(0.02, 0.07) } else { 0.0 };
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Spark,
                    x,
                    y,
                    vx: libm::cos(a) * v,
                    vy: libm::sin(a) * v,
                    life,
                    col: Some(WHITE),
                    size: 1.0,
                    delay,
                    ..Spawn::new(Kind::Spark, 0.0, 0.0, 0.0, 0.0, 0.0)
                },
            );
        }
        for _ in 0..8 {
            let a = r.range(0.0, TAU);
            let v = r.range(40.0, 110.0);
            let life = r.range(0.3, 0.55);
            let col = r.pick(&[c(5), c(6)]).copied();
            let delay = r.range(0.0, 0.04);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Mote,
                    x: X,
                    y,
                    vx: libm::cos(a) * v,
                    vy: libm::sin(a) * v - 20.0,
                    life,
                    col,
                    size: 0.6,
                    delay,
                    ..Spawn::new(Kind::Mote, 0.0, 0.0, 0.0, 0.0, 0.0)
                },
            );
        }
        for _ in 0..3 {
            let a = r.range(0.0, TAU);
            let d = r.range(12.0, 26.0);
            let life = r.range(0.22, 0.34);
            let size = r.range(1.6, 2.6);
            let delay = r.range(0.1, 0.3);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Star,
                    x: X + libm::cos(a) * d,
                    y: y + libm::sin(a) * d,
                    vx: 0.0,
                    vy: -3.0,
                    life,
                    col: Some(WHITE),
                    size,
                    delay,
                    ..Spawn::new(Kind::Star, 0.0, 0.0, 0.0, 0.0, 0.0)
                },
            );
        }
        Box::new(State {
            c: [c(0), c(1), c(2), c(3), c(4), c(5), c(6)],
            y,
            r,
            ps,
            lights,
            sp,
            segs,
            edges: [e0, e1],
            t: 0.0,
        })
    }
}

struct State {
    c: [Rgba; 7],
    y: f64,
    r: Rng,
    ps: Particles,
    lights: Lights,
    sp: Vec<Spike>,
    segs: Vec<Seg>,
    edges: [f64; 2],
    t: f64,
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        self.ps.update(&mut self.r, dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, y, t) = (&self.c, self.y, self.t);
        // the ring: rings out off the star and breaks into arcs that fly on and cool
        let u = (t - 0.03) / 0.36;
        if (0.0..1.0).contains(&u) {
            let r = 8.0 + 24.0 * ease::out_cubic(u);
            let brk = smooth(0.15, 0.6, u);
            let i = if u < 0.15 {
                6
            } else if u < 0.35 {
                5
            } else if u < 0.6 {
                4
            } else {
                3
            };
            broken_ring(
                p,
                X,
                y,
                r,
                js_max(1.0, 2.6 * libm::pow(1.0 - u, 1.2)),
                &self.segs,
                brk,
                c[i],
                c[js_max(2.0, i as f64 - 2.0) as usize],
                if u < 0.6 { 1.0 } else { (1.0 - u) / 0.4 },
                1.0,
                0.0,
            );
            if u < 0.5 {
                p.glow_ring(X, y, r, 5.0, c[4], 0.3 * (1.0 - u / 0.5));
            }
        }
        self.ps.draw(p, Layer::Both);
        // the edges flash where they met
        if t < 0.058 {
            for a in self.edges {
                let (ca, sa) = (libm::cos(a), libm::sin(a));
                let l = if t < 0.025 { 26.0 } else { 32.0 };
                let k = if t < 0.025 { 1.0 } else { 0.6 };
                let mut d = -l;
                while d <= l {
                    let q = libm::fabs(d) / l;
                    let col = if q < 0.4 {
                        WHITE
                    } else if q < 0.75 {
                        c[5]
                    } else {
                        c[4]
                    };
                    let al = if q < 0.6 { 1.0 } else { k * (1.3 - q) };
                    p.dot(X + ca * d, y + sa * d, col, al);
                    d += 0.5;
                }
                p.glow_line(
                    X - ca * l,
                    y - sa * l,
                    X + ca * l,
                    y + sa * l,
                    4.0,
                    c[5],
                    0.35 * k,
                );
            }
        }
        // the star: white, then bigger and gold as it thins, then just a glint
        if t < 0.025 {
            star(
                p,
                X,
                y,
                &self.sp,
                1.0,
                1.0,
                [c[4], WHITE, WHITE, WHITE],
                1.0,
                2.6,
            );
        } else if t < 0.058 {
            star(
                p,
                X,
                y,
                &self.sp,
                1.25,
                0.65,
                [c[2], c[5], c[6], WHITE],
                1.0,
                2.6,
            );
        } else if t < 0.092 {
            star(
                p,
                X,
                y,
                &self.sp,
                1.4,
                0.3,
                [c[2], c[4], c[5], c[6]],
                1.0,
                2.0,
            );
        }
        let pop = libm::exp(-t / 0.05);
        if pop > 0.02 {
            p.glow(X, y, 30.0, c[4], 0.5 * pop);
            p.glow(X, y, 10.0, WHITE, 0.4 * pop);
        }
        self.lights.draw(p);
    }
}
