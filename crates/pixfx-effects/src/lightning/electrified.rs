//! Electrified: port of pixelfx `src/fx/lightning/electrified.js`.
//!
//! Wraps its actor ([`Effect::wraps_actor`]): the edge of the sprite it is handed is read once per pose in
//! [`EffectState::set_actor`]; with no sprite it plays on pixelfx's stock 2x actor ([`stand_in`]), as pixelfx's
//! render does. pixelfx's `act` (the jolting poses) is gallery staging and is dropped.
//!
//! Arcs leaping between points of the silhouette's edge, re-struck every other frame, grounding to the floor, and
//! the periodic jolt that throws sparks follow Sprite Fusion's open-source lightning crackle
//! (https://www.spritefusion.com/pixel-art-vfx/lightning), MIT License, Copyright (c) 2026 Hugo Duprez.

use std::sync::Arc;

use pixfx_core::particles::cycle;
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, luma, ramp};
use pixfx_draw::math::{TAU, clamp, js_round, rng};

use super::lib::{PLASMA, VOLT, crackle, edge_of, flight, spark, stand_in, zigzag};

pub struct Electrified;

pub static ELECTRIFIED: Electrified = Electrified;

const META: Meta = Meta {
    id: "lightning/electrified",
    name: "Electrified",
    desc: "Current runs through a character: arcs crawl over and leap across the body, sparks spit off, the body flares to an electric silhouette on each jolt, and the feet ground out.",
    tags: &[
        "lightning",
        "electric",
        "status",
        "shock",
        "stun",
        "actor",
        "loop",
    ],
    size: (108, 92),
    origin: (54, 76),
    duration: 2.0,
    looping: true,
    fps: 30,
    seed: 5,
    state: None,
    options: &[SPARKS.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);

const FPS: f64 = 30.0;
const N: f64 = 60.0;
const P: f64 = 2.0;
const JOLTS: [(f64, f64); 3] = [(3.0, 1.0), (22.0, 0.7), (41.0, 0.9)];

impl Effect for Electrified {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let mut s = State {
            c: ramp(&opts.palette_or(&VOLT)),
            s: opts.ramp(&SPARKS),
            seed: seed as f64,
            actor: None,
            edge: Vec::new(),
        };
        s.set_actor(None);
        Box::new(s)
    }
}

struct State {
    c: [Rgba; 8],
    s: Vec<Rgba>,
    seed: f64,
    /// The sprite it plays on (the stand-in without one).
    actor: Option<Arc<Actor>>,
    /// The silhouette's edge pixels' centres, in local space.
    edge: Vec<(f64, f64)>,
}

/// The jolt playing at frame `f`: its frame within the jolt and its strength.
fn jolt(f: f64) -> Option<(f64, f64)> {
    JOLTS
        .iter()
        .find(|&&(f0, _)| f >= f0 && f < f0 + 3.0)
        .map(|&(f0, k)| (f - f0, k))
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.actor = actor.or_else(stand_in);
        self.edge = self.actor.as_deref().map_or_else(Vec::new, |a| {
            edge_of(a)
                .into_iter()
                .map(|(x, y)| (f64::from(x) + 0.5, f64::from(y) + 0.5))
                .collect()
        });
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            self.draw_back(p, t);
        }
        if layer != Layer::Back {
            self.draw_front(p, t);
        }
    }
}

impl State {
    fn draw_back(&self, p: &mut Painter, t: f64) {
        let c = &self.c;
        let f = libm::floor(t * FPS + 1e-6) % N;
        let j = jolt(f);
        let mut r = rng(&[self.seed, 4.0, f]);
        // a cold aura: a broken outline that flickers round the body, light pooling at the feet
        if let Some(a) = &self.actor {
            let col = if j.is_some() {
                c[4]
            } else if r.next() < 0.5 {
                c[2]
            } else {
                c[1]
            };
            p.outline(&**a, f64::from(a.x()), f64::from(a.y()), col, 1.0);
        }
        let k = match j {
            Some((_, k)) => 0.55 * k,
            None => 0.16 + 0.1 * r.next(),
        };
        p.glow(0.0, -18.0, 30.0, c[3], k);
        p.glow(0.0, -2.0, 22.0, c[4], 0.18);
    }

    fn draw_front(&self, p: &mut Painter, t: f64) {
        let (c, seed, e) = (&self.c, self.seed, &self.edge);
        let f = libm::floor(t * FPS + 1e-6) % N;
        let j = jolt(f);
        // the jolt: the body flares to an electric silhouette, then fades back through its own shading
        if let (Some((ja, jk)), Some(a)) = (j, &self.actor) {
            let lift = [1.1, 0.6, -0.2][ja as usize] * jk;
            let rp = [c[1], c[2], c[3], c[4], c[5], c[6], c[7]];
            for (x, y, col) in a.opaque() {
                let i = clamp(js_round(0.4 + luma(col) * 6.0 + lift), 0.0, 6.0);
                p.px(f64::from(x), f64::from(y), rp[i as usize]);
            }
            p.glow(0.0, -19.0, 26.0, c[5], 0.5 * jk * (1.0 - ja / 3.0));
        }
        if e.is_empty() {
            return;
        }
        // arcs: two or three at a time, re-struck every other frame, leaping between nearby points of the edge
        let fi = f as i64;
        let mut r = rng(&[seed, 7.0, (fi >> 1) as f64]);
        let n = 2.0 + if j.is_some() { 2.0 } else { r.int(0.0, 1.0) };
        let mut i = 0.0;
        while i < n {
            let a = r.pick(e).copied().unwrap_or_default();
            let ang = r.range(0.0, TAU);
            let d = r.range(6.0, 14.0);
            let mut b = (a.0 + libm::cos(ang) * d, a.1 + libm::sin(ang) * d);
            for _ in 0..10 {
                let q = r.pick(e).copied().unwrap_or_default();
                if libm::hypot(q.0 - b.0, q.1 - b.1) < 5.0
                    && libm::hypot(q.0 - a.0, q.1 - a.1) > 4.0
                {
                    b = q;
                }
            }
            let k = (if fi & 1 != 0 { 0.75 } else { 0.95 }) * r.range(0.6, 1.0);
            let bolt = zigzag(&mut rng(&[seed, 8.0, f, i]), a.0, a.1, b.0, b.1, 0.42, 0.35);
            crackle(p, &bolt, k, c, 0.4, 1.0);
            // the edge catches the light where an arc touches it
            for &(x, y) in e {
                if libm::hypot(x - a.0, y - a.1) < 3.5 || libm::hypot(x - b.0, y - b.1) < 3.0 {
                    p.px(x - 0.5, y - 0.5, c[5]);
                }
            }
            i += 1.0;
        }
        // the feet ground out to the floor now and then
        if r.next() < if j.is_some() { 0.9 } else { 0.35 } {
            let s = r.sign();
            let x0 = s * r.range(3.0, 8.0);
            let x1 = x0 + s * r.range(5.0, 12.0);
            let bolt = zigzag(&mut rng(&[seed, 9.0, f]), x0, -1.5, x1, -0.5, 0.45, 0.3);
            crackle(p, &bolt, 0.7, c, 0.35, 1.0);
            p.glow(x0, -1.0, 10.0, c[5], 0.35);
        }
        // sparks: a spray off the edge with every jolt, and a few spat out between them
        let len = e.len() as f64;
        let pick = |v: f64| {
            e.get(libm::floor(v * len) as usize)
                .copied()
                .unwrap_or_default()
        };
        for (f0, k) in JOLTS {
            let age = (((t - f0 / FPS) % P) + P) % P;
            for i in 0..9 {
                let mut q = rng(&[seed, 21.0, f0, f64::from(i)]);
                let life = q.range(0.18, 0.4);
                if age >= life {
                    continue;
                }
                let (x0, y0) = pick(q.next());
                let ang = libm::atan2(y0 + 19.0, x0) + q.range(-0.7, 0.7);
                let sp = q.range(60.0, 150.0) * k;
                let (x, y, vx, vy) = flight(
                    x0,
                    y0,
                    libm::cos(ang) * sp,
                    libm::sin(ang) * sp - 30.0,
                    age,
                    0.0,
                );
                spark(p, x, y, vx, vy, 1.0 - age / life, &self.s);
            }
        }
        cycle(
            t,
            P,
            8,
            seed + 3.0,
            |g| g.range(0.15, 0.3),
            |age, life, q, _| {
                let (x0, y0) = pick(q.next());
                let ang = libm::atan2(y0 + 19.0, x0) + q.range(-0.6, 0.6);
                let sp = q.range(50.0, 110.0);
                let (x, y, vx, vy) = flight(
                    x0,
                    y0,
                    libm::cos(ang) * sp,
                    libm::sin(ang) * sp - 20.0,
                    age,
                    0.0,
                );
                spark(p, x, y, vx, vy, 1.0 - age / life, &self.s);
            },
        );
    }
}
