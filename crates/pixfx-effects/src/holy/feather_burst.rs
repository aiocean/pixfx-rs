//! Feather burst: port of pixelfx `src/fx/holy/feather-burst.js`.

use pixfx_core::color::{Rgba, WHITE, mix, ramp};
use pixfx_core::math::{Rng, TAU, ease, js_max, js_min, smooth, to_uint32};
use pixfx_core::sim::{Kind, Lights, Particles, ParticlesOpts, Spawn, sparkle};
use pixfx_core::{ColorOpt, Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{Feathers, NewFeather};

pub struct FeatherBurst;

pub static FEATHER_BURST: FeatherBurst = FeatherBurst;

const META: Meta = Meta {
    id: "holy/feather-burst",
    name: "Feather burst",
    desc: "A white puff pops and throws feathers that brake hard, then rock from side to side as they sink and fade, with down drifting after them.",
    tags: &[
        "holy", "feather", "burst", "poof", "angel", "bird", "one-shot",
    ],
    size: (160, 160),
    origin: (80, 60),
    duration: 3.0,
    looping: false,
    fps: 30,
    seed: 4,
    state: None,
    options: &[GLOW.spec()],
};

// meta.options
const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x4a4560),
    Rgba::hex24(0x7a7494),
    Rgba::hex24(0xaaa5c2),
    Rgba::hex24(0xd6d3e6),
    Rgba::hex24(0xf4f3fa),
    Rgba::hex24(0xffffff),
];
const GLOW: ColorOpt = ColorOpt::new("glow", Rgba::hex24(0xfff0c0));

/// One lobe of the puff: soft, thrown out from the pop, swelling and thinning.
struct Lobe {
    x: f64,
    y: f64,
    r0: f64,
    r1: f64,
    life: f64,
}

impl Effect for FeatherBurst {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let f: [Rgba; 6] = ramp(&opts.palette_or(&PALETTE));
        let g = opts.color(&GLOW);
        let mut r = Rng::new(to_uint32(seed as f64));
        let mut ps = Particles::new(ParticlesOpts {
            ground: f64::INFINITY,
            sparks: vec![WHITE, f[4], f[3], f[2]],
            ..ParticlesOpts::default()
        });
        let mut lights = Lights::new();
        let mut loose = Feathers::new(1e6);
        // the puff: soft lobes thrown out from the pop, swelling and thinning
        let mut lobes = Vec::new();
        for i in 0..11 {
            let a = (f64::from(i) / 11.0) * TAU + r.range(-0.25, 0.25);
            let d = r.range(5.0, 14.0);
            let x = libm::cos(a) * d;
            let y = libm::sin(a) * d * 0.8;
            let r0 = r.range(2.5, 4.0);
            let r1 = r.range(6.0, 9.5);
            let life = r.range(0.45, 0.8);
            lobes.push(Lobe { x, y, r0, r1, life });
        }
        for i in 0..20 {
            let a = (f64::from(i) / 20.0) * TAU + r.range(-0.15, 0.15);
            let sp = r.range(110.0, 230.0);
            let up = if libm::sin(a) < 0.0 { 0.7 } else { 0.9 };
            let (x, y) = (libm::cos(a) * 3.0, libm::sin(a) * 3.0);
            let (vx, vy) = (libm::cos(a) * sp, libm::sin(a) * sp * up - 20.0);
            let a = a + r.range(-0.5, 0.5);
            let len = r.range(10.0, 14.0);
            let wid = r.range(2.6, 3.4);
            let life = r.range(1.9, 2.6);
            let delay = r.range(0.0, 0.03);
            let sink = r.range(26.0, 38.0);
            let amp = r.range(8.0, 13.0);
            loose.add(
                &mut r,
                NewFeather {
                    x,
                    y,
                    vx,
                    vy,
                    a,
                    len,
                    wid,
                    life,
                    delay,
                    back: false,
                    sink: Some(sink),
                    amp: Some(amp),
                },
            );
        }
        // small feathers and down, slower
        for _ in 0..10 {
            let a = r.range(0.0, TAU);
            let sp = r.range(50.0, 130.0);
            let (vx, vy) = (libm::cos(a) * sp, libm::sin(a) * sp - 20.0);
            let a = r.range(0.0, TAU);
            let len = r.range(6.0, 8.0);
            let wid = r.range(1.8, 2.2);
            let life = r.range(1.6, 2.4);
            let sink = r.range(18.0, 28.0);
            loose.add(
                &mut r,
                NewFeather {
                    x: 0.0,
                    y: 0.0,
                    vx,
                    vy,
                    a,
                    len,
                    wid,
                    life,
                    delay: 0.0,
                    back: true,
                    sink: Some(sink),
                    amp: None,
                },
            );
        }
        for _ in 0..26 {
            let a = r.range(0.0, TAU);
            let sp = r.range(30.0, 140.0);
            let x = r.range(-3.0, 3.0);
            let y = r.range(-3.0, 3.0);
            let life = r.range(1.4, 2.6);
            let col = if r.chance(0.6) { WHITE } else { f[4] };
            let size = r.range(0.6, 1.6);
            ps.spawn(
                &mut r,
                Spawn {
                    col: Some(col),
                    size,
                    ..Spawn::new(
                        Kind::Flake,
                        x,
                        y,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp - 15.0,
                        life,
                    )
                },
            );
        }
        for _ in 0..10 {
            let a = r.range(0.0, TAU);
            let sp = r.range(60.0, 160.0);
            let life = r.range(0.3, 0.6);
            let col = if r.chance(0.5) { WHITE } else { g };
            let size = r.range(0.8, 2.2);
            ps.spawn(
                &mut r,
                Spawn {
                    col: Some(col),
                    size,
                    ..Spawn::new(
                        Kind::Star,
                        0.0,
                        0.0,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp,
                        life,
                    )
                },
            );
        }
        lights.add(0.0, 0.0, 50.0, g, 0.75, 0.45);
        lights.add(0.0, 0.0, 20.0, WHITE, 0.7, 0.1);
        Box::new(State {
            f,
            g,
            r,
            ps,
            lights,
            loose,
            lobes,
            t: 0.0,
        })
    }
}

struct State {
    f: [Rgba; 6],
    g: Rgba,
    r: Rng,
    ps: Particles,
    lights: Lights,
    loose: Feathers,
    lobes: Vec<Lobe>,
    t: f64,
}

impl State {
    fn puff(&self, p: &mut Painter) {
        let (f, t) = (&self.f, self.t);
        for l in &self.lobes {
            let u = t / l.life;
            if u >= 1.0 {
                continue;
            }
            let e = ease::out_cubic(u);
            let x = l.x * (0.4 + e * 1.3);
            let y = l.y * (0.4 + e * 1.3) - u * 5.0;
            let r = (l.r0 + (l.r1 - l.r0) * ease::out_cubic(js_min(1.0, u * 3.0)))
                * (1.0 - ease::in_quad(u));
            if r < 0.6 {
                continue;
            }
            p.circle(x, y, r + 0.8, f[2], 1.0);
            let (white, f3, f4) = (WHITE, f[3], f[4]);
            p.circle(
                x,
                y,
                r,
                move |d: f64, a: f64| {
                    if d > 0.55 && libm::sin(a + 0.8) > 0.3 {
                        f3
                    } else if d < 0.45 && libm::sin(a + 0.8) < -0.2 {
                        white
                    } else {
                        f4
                    }
                },
                1.0,
            );
        }
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        // feathers catch the light as they turn
        if self.t > 0.3 && self.t < 2.2 && self.loose.count() > 0 && self.r.chance(0.14) {
            let picked = self.loose.pick(&mut self.r);
            if let Some((qx, qy, _)) = picked.filter(|&(_, _, delay)| delay <= 0.0) {
                let x = qx + self.r.range(-2.0, 2.0);
                let y = qy + self.r.range(-1.0, 1.0);
                let life = self.r.range(0.2, 0.4);
                let col = if self.r.chance(0.5) { WHITE } else { self.g };
                let size = self.r.range(1.0, 2.2);
                self.ps.spawn(
                    &mut self.r,
                    Spawn {
                        col: Some(col),
                        size,
                        ..Spawn::new(Kind::Star, x, y, 0.0, 0.0, life)
                    },
                );
            }
        }
        self.ps.update(&mut self.r, dt);
        self.loose.update(dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let (f, g, t) = (&self.f, self.g, self.t);
        if layer == Layer::Back {
            self.loose.draw(p, f, Layer::Back);
            self.ps.draw(p, Layer::Back);
            self.puff(p);
            return;
        }
        // the pop: a white flash and a ring of air
        if t < 0.07 {
            p.circle(0.0, 0.0, 9.0 * (1.0 - t / 0.14), WHITE, 1.0);
        }
        if t < 0.25 {
            let u = t / 0.25;
            let air = mix(f[4], g, 0.4);
            p.ring(
                0.0,
                0.0,
                6.0 + 24.0 * ease::out_cubic(u),
                js_max(1.0, 2.5 * (1.0 - u)),
                move |v: f64, _: f64| if v > 0.5 { WHITE } else { air },
                1.1 - u,
            );
        }
        if t < 0.2 {
            sparkle(p, 0.0, 0.0, 10.0 * (1.0 - t / 0.2), WHITE, g, 0.6);
        }
        // the burst's warmth lingers a moment
        p.glow(0.0, 4.0, 46.0, g, 0.28 * (1.0 - smooth(0.0, 1.3, t)));
        self.loose.draw(p, f, Layer::Front);
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
