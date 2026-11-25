//! Snowflake burst: port of pixelfx `src/fx/ice/snowflake-burst.js`.
//!
//! The pop recipe (a gathering, a white flash with a ring and flares, twinkling motes that linger and fade) is
//! adapted from Sprite Fusion's open-source sparkle (https://www.spritefusion.com/pixel-art-vfx/sparkle),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::sim::{Kind, Lights, Particles, ParticlesOpts, flare};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_max, js_min, js_round, smooth, to_uint32};

use super::lib::{FlakeDesign, PALETTE, emit, end_by, flake_design, snowflake};

pub struct SnowflakeBurst;

pub static SNOWFLAKE_BURST: SnowflakeBurst = SnowflakeBurst;

const META: Meta = Meta {
    id: "ice/snowflake-burst",
    name: "Snowflake burst",
    desc: "A cold pop: frost gathers and flashes white in a ring, then crisp six-armed snowflakes spin out, slow, twinkle and drift down to settle and melt.",
    tags: &["ice", "snow", "snowflake", "burst", "magic", "one-shot"],
    size: (140, 120),
    origin: (70, 106),
    duration: D,
    looping: false,
    fps: 30,
    seed: 8,
    state: None,
    options: &[COUNT.spec()],
};

const COUNT: NumOpt = NumOpt::new("count", 9.0);
const D: f64 = 3.2;
const POP: f64 = 0.16;
const CY: f64 = -48.0;
/// The flakes' sizes in turn: two big, a few middling, the rest small.
const SIZES: [u8; 9] = [2, 0, 1, 0, 2, 0, 1, 0, 1];

impl Effect for SnowflakeBurst {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        let count = opts.num(&COUNT);
        let mut r = Rng::new(to_uint32(seed as f64));
        let ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: vec![c[7], c[6], c[5], c[4]],
            haze: Some(c[5]),
        });
        // the flakes: two big, a few middling, the rest small; small ones fly further
        let mut flakes = Vec::new();
        let mut i = 0.0;
        while i < count {
            let z = SIZES[(i as usize) % SIZES.len()];
            let rad = match z {
                2 => r.range(9.5, 11.5),
                1 => r.range(6.0, 7.5),
                _ => r.range(3.0, 4.5),
            };
            let a = (i / count) * TAU + r.range(-0.3, 0.3) + 0.4;
            let sp = r.range(80.0, 120.0) * (1.4 - rad / 14.0);
            let vx = libm::cos(a) * sp;
            let vy = libm::sin(a) * sp * 0.85 - 20.0;
            let rot = r.range(0.0, TAU);
            let w0 = r.sign() * r.range(7.0, 12.0);
            let w1 = r.sign() * r.range(0.4, 1.0);
            let d = flake_design(&mut r);
            let life = r.range(2.1, 2.8);
            let ph = r.range(0.0, TAU);
            let tw = r.range(0.5, 0.85);
            let tw0 = r.range(0.15, 0.6);
            flakes.push(Flake {
                x: 0.0,
                y: CY,
                vx,
                vy,
                r: rad,
                rot,
                w0,
                w1,
                d,
                life,
                age: 0.0,
                ph,
                tw,
                tw0,
                rest: false,
                popped: false,
            });
            i += 1.0;
        }
        Box::new(State {
            c,
            r,
            ps,
            lights: Lights::new(),
            flakes,
            t: 0.0,
            popped: false,
        })
    }
}

struct Flake {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    r: f64,
    rot: f64,
    w0: f64,
    w1: f64,
    d: FlakeDesign,
    life: f64,
    age: f64,
    ph: f64,
    tw: f64,
    tw0: f64,
    rest: bool,
    popped: bool,
}

/// A flake's radius now: it pops up to size and shrinks away at the end.
fn sz(f: &Flake) -> f64 {
    f.r * js_min(1.0, ease::out_back(clamp(f.age / 0.18, 0.0, 1.0), 2.0))
        * js_min(1.0, (f.life - f.age) / 0.35)
}

struct State {
    c: [Rgba; 8],
    r: Rng,
    ps: Particles,
    lights: Lights,
    flakes: Vec<Flake>,
    t: f64,
    popped: bool,
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        let State {
            c,
            r,
            ps,
            lights,
            flakes,
            t,
            popped,
        } = self;
        *t += dt;
        let t = *t;
        // the cold gathers: streaks of frost spiral in, snow motes drift in
        if t < POP - 0.03 && r.chance(0.8) {
            let a = r.range(0.0, TAU);
            let d = r.range(22.0, 44.0);
            let tt = r.range(0.12, 0.16);
            let v = (d * 3.0) / (libm::exp(3.0 * tt) - 1.0);
            emit!(
                ps,
                r,
                Kind::Streak,
                libm::cos(a) * d,
                CY + libm::sin(a) * d,
                -libm::cos(a) * v,
                -libm::sin(a) * v,
                tt,
                if r.chance(0.5) { c[6] } else { c[5] }
            );
        }
        if !*popped && t >= POP {
            *popped = true;
            lights.add(0.0, CY, 44.0, WHITE, 0.8, 0.1);
            lights.add(0.0, CY, 72.0, c[4], 0.5, 0.6);
            // frost thrown out: streaking motes, snow, glints that come and go round it
            for _ in 0..22 {
                let a = r.range(0.0, TAU);
                let sp = r.range(80.0, 150.0);
                emit!(
                    ps,
                    r,
                    Kind::Mote,
                    0.0,
                    CY,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    r.range(0.25, 0.5),
                    if r.chance(0.5) { c[6] } else { c[5] },
                    r.range(0.6, 1.0)
                );
            }
            for _ in 0..18 {
                let a = r.range(-core::f64::consts::PI, 0.2);
                let sp = r.range(30.0, 110.0);
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    0.0,
                    CY,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    r.range(1.0, 1.6),
                    if r.chance(0.6) { WHITE } else { c[6] },
                    r.range(0.5, 1.5)
                );
            }
            for _ in 0..12 {
                let a = r.range(0.0, TAU);
                let d = r.range(10.0, 44.0);
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    libm::cos(a) * d,
                    CY + libm::sin(a) * d * 0.85,
                    0.0,
                    r.range(2.0, 6.0),
                    r.range(0.3, 0.6),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(0.8, 2.2),
                    r.range(0.08, 1.6)
                );
            }
        }
        if *popped {
            for f in flakes.iter_mut() {
                f.age += dt;
                if f.age >= f.life {
                    continue;
                }
                if !f.rest {
                    f.vx -= f.vx * 2.8 * dt;
                    f.vy -= f.vy * 2.8 * dt - 26.0 * dt;
                    f.x += f.vx * dt + libm::sin(t * 1.8 + f.ph) * 5.0 * dt;
                    f.y += f.vy * dt;
                    f.rot += (f.w1 + (f.w0 - f.w1) * libm::exp(-f.age * 2.5)) * dt;
                    // it settles on the floor, flat and still, and melts there
                    if f.y > -sz(f) * 0.3 {
                        f.rest = true;
                        f.y = -1.0;
                        f.life = js_min(f.life, f.age + 0.6);
                    }
                }
                // a melted or spent flake goes out with a glint or two
                if !f.popped && f.life - f.age < 0.1 {
                    f.popped = true;
                    for _ in 0..2 {
                        emit!(
                            ps,
                            r,
                            Kind::Star,
                            f.x + r.range(-2.0, 2.0),
                            f.y + r.range(-2.0, 2.0),
                            r.range(-6.0, 6.0),
                            -r.range(2.0, 8.0),
                            r.range(0.2, 0.4),
                            if r.chance(0.5) { WHITE } else { c[6] },
                            r.range(0.8, 1.6)
                        );
                    }
                }
            }
        }
        ps.update(r, dt);
        lights.update(dt);
        end_by(t, D, ps, None, None);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, t) = (&self.c, self.t);
        // the gathering: a tiny flake spins up at the heart, faster and brighter
        if t < POP
            && let Some(f0) = self.flakes.first()
        {
            let k = t / POP;
            snowflake(p, 0.0, CY, 1.0 + 3.2 * k * k, t * 20.0, &f0.d, c, 1.0, k);
            p.glow(0.0, CY, 8.0 + 14.0 * k, c[5], 0.3 + 0.4 * k);
        }
        // the pop: a white flash, a flare and a ring of frost running out
        let q = t - POP;
        if (0.0..0.4).contains(&q) {
            if q < 0.07 {
                p.circle(0.0, CY, 6.0 * (1.0 - q / 0.07) + 1.0, WHITE, 1.0);
            }
            flare(
                p,
                0.0,
                CY,
                28.0 * libm::pow(js_max(0.0, 1.0 - q / 0.26), 1.5),
                1.0 - q / 0.26,
                c,
            );
            let e = ease::out_cubic(q / 0.4);
            let rr = 5.0 + 34.0 * e;
            let col = if q < 0.08 {
                WHITE
            } else if q < 0.2 {
                c[6]
            } else {
                c[5]
            };
            p.ring(0.0, CY, rr, js_max(1.0, 2.4 * (1.0 - e)), col, 1.15 - e);
            p.glow_ring(0.0, CY, rr, 5.0, c[5], 0.4 * (1.0 - e));
        }
        // the cold light left where it burst, fading
        if self.popped {
            p.glow(0.0, CY, 46.0, c[4], 0.3 * (1.0 - smooth(POP, POP + 1.6, t)));
        }
        // the flakes: spinning out, twinkling now and then, landing and melting
        for f in &self.flakes {
            if !self.popped || f.age >= f.life {
                continue;
            }
            let r = sz(f);
            let tw = libm::pow(js_max(0.0, libm::sin(((f.age - f.tw0) / f.tw) * TAU)), 8.0)
                * if f.age > f.tw0 { 1.0 } else { 0.0 };
            if f.rest {
                // lying on the floor: flattened to a sparkling smear that shrinks away
                let l = js_max(1.0, js_round(r * 1.2 * ((f.life - f.age) / 0.6)));
                let mut i = -l;
                while i <= l {
                    let col = if libm::fabs(i) == l {
                        c[5]
                    } else if i == 0.0 {
                        WHITE
                    } else {
                        c[6]
                    };
                    p.px(f.x + i, -1.0, col);
                    i += 1.0;
                }
                p.glow(f.x, -1.0, 3.0 + l, c[4], 0.3);
                continue;
            }
            snowflake(
                p,
                f.x,
                f.y,
                r,
                f.rot,
                &f.d,
                c,
                1.0,
                js_max(tw, 1.0 - clamp(f.age / 0.12, 0.0, 1.0)),
            );
            p.glow(f.x, f.y, 6.0 + r * 1.4, c[4], 0.12);
            if tw > 0.5 {
                flare(p, f.x, f.y, (3.0 + r * 0.7) * tw, tw, c);
            }
        }
        self.ps.draw(p, Layer::Both);
        self.lights.draw(p);
    }
}
