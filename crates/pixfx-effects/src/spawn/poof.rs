//! Poof: port of pixelfx `src/fx/spawn/poof.js`.
//!
//! Beyond pixelfx: it fires impact cues ([`pixfx_core::cue::impact`]) for the game to answer; the `feel` option
//! scales them, 0 for none.
//!
//! The soft, top-lit smoke puffs (a body, a denser core, a lit cap) follow Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Kind, Lights, Particles, ParticlesOpts, Spawn, flare};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_min, smooth, to_uint32};

pub struct Poof;

pub static POOF: Poof = Poof;

const META: Meta = Meta {
    id: "spawn/poof",
    name: "Poof",
    desc: "A cartoon death poof: a white pop, a ring of soft round smoke balls bursting out between speed lines, little stars spinning off, and the cloud lifting and thinning away.",
    tags: &[
        "spawn", "death", "despawn", "smoke", "poof", "cartoon", "one-shot",
    ],
    size: (112, 92),
    origin: (56, 80),
    duration: 1.6,
    looping: false,
    fps: 30,
    seed: 21,
    state: None,
    options: &[STARS.spec(), FEEL.spec()],
};

// meta.options
const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x2a2440),
    Rgba::hex24(0x5e5878),
    Rgba::hex24(0x9a94b4),
    Rgba::hex24(0xcdc8e0),
    Rgba::hex24(0xf2f0fa),
    Rgba::hex24(0xffffff),
];
const STARS: RampOpt = RampOpt::new(
    "stars",
    &[
        Rgba::hex24(0xffd84a),
        Rgba::hex24(0xfff3a6),
        Rgba::hex24(0xffffff),
    ],
);

const CY: f64 = -17.0;

/// A smoke ball: swells out, breathes, then shrinks to nothing.
struct Ball {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    r0: f64,
    r1: f64,
    life: f64,
    delay: f64,
    max: f64,
}

/// A speed line flicked out between the balls.
struct Line {
    a: f64,
    r1: f64,
    d: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Shade,
    Body,
    Cap,
}

impl Effect for Poof {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the pop asks the game for an impact's answer (shake, hit-stop, ring, flash by weight).
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(0.0, 0.15, 0.0, -12.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 6] = ramp(&opts.palette_or(&PALETTE));
        let s: [Rgba; 3] = ramp(&opts.ramp(&STARS));
        let mut r = Rng::new(to_uint32(seed as f64));
        let mut ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: vec![WHITE, s[1], s[0], c[2]],
            ..ParticlesOpts::default()
        });
        let mut lights = Lights::new();
        let mut balls: Vec<Ball> = Vec::new();
        let mut lines: Vec<Line> = Vec::new();
        // a ring of fat balls round a fat core, thrown out fast and stopped by drag, each swelling as it slows
        let ring = 9;
        for i in 0..ring {
            let a = (f64::from(i) / f64::from(ring)) * TAU + r.range(-0.2, 0.2);
            let sp = r.range(95.0, 140.0);
            let rad = r.range(5.0, 7.0);
            let life = r.range(0.7, 1.05);
            let delay = r.range(0.0, 0.03);
            balls.push(Ball {
                x: libm::cos(a) * 3.0,
                y: CY + libm::sin(a) * 3.0,
                vx: libm::cos(a) * sp,
                vy: libm::sin(a) * sp * 0.8 - 12.0,
                r0: rad * 0.5,
                r1: rad,
                life,
                delay,
                max: 0.0,
            });
        }
        for _ in 0..5 {
            let a = r.range(0.0, TAU);
            let sp = r.range(20.0, 60.0);
            let r1 = r.range(6.5, 8.5);
            let life = r.range(0.85, 1.2);
            let delay = r.range(0.02, 0.06);
            balls.push(Ball {
                x: libm::cos(a) * 2.0,
                y: CY + libm::sin(a) * 2.0,
                vx: libm::cos(a) * sp,
                vy: libm::sin(a) * sp - 16.0,
                r0: 4.0,
                r1,
                life,
                delay,
                max: 0.0,
            });
        }
        // a few small puffs flung further out, gone quickly, so the cloud's edge is ragged
        for _ in 0..7 {
            let a = r.range(0.0, TAU);
            let sp = r.range(170.0, 240.0);
            let rad = r.range(2.2, 3.6);
            let life = r.range(0.32, 0.5);
            let delay = r.range(0.01, 0.04);
            balls.push(Ball {
                x: libm::cos(a) * 4.0,
                y: CY + libm::sin(a) * 4.0,
                vx: libm::cos(a) * sp,
                vy: libm::sin(a) * sp * 0.75 - 10.0,
                r0: rad * 0.6,
                r1: rad,
                life,
                delay,
                max: 0.0,
            });
        }
        for b in &mut balls {
            b.max = b.life;
        }
        // speed lines between the balls, flicked out and gone in a few frames
        for i in 0..12 {
            let a = (f64::from(i) / 12.0) * TAU + r.range(-0.12, 0.12) + 0.13;
            let sa = libm::sin(a);
            // the ones that point down stop at the floor
            let reach = r.range(24.0, 34.0);
            let r1 = js_min(reach, if sa > 0.1 { (-2.0 - CY) / sa } else { 99.0 });
            let d = r.range(0.0, 0.04);
            lines.push(Line { a, r1, d });
        }
        // little stars spun off the pop
        for _ in 0..7 {
            let a = r.range(-core::f64::consts::PI, 0.3);
            let sp = r.range(70.0, 125.0);
            let life = r.range(0.55, 0.95);
            let col = r.pick(&[s[2], s[1], s[1], s[0]]).copied();
            let size = r.range(1.3, 2.4);
            let delay = r.range(0.02, 0.1);
            ps.spawn(
                &mut r,
                Spawn {
                    col,
                    size,
                    delay,
                    ..Spawn::new(
                        Kind::Star,
                        libm::cos(a) * 6.0,
                        CY + libm::sin(a) * 6.0,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp - 30.0,
                        life,
                    )
                },
            );
        }
        for _ in 0..10 {
            let a = r.range(0.0, TAU);
            let sp = r.range(40.0, 110.0);
            let life = r.range(0.3, 0.6);
            let col = r.pick(&[WHITE, s[1]]).copied();
            let delay = r.range(0.0, 0.08);
            ps.spawn(
                &mut r,
                Spawn {
                    col,
                    size: 0.6,
                    delay,
                    ..Spawn::new(
                        Kind::Star,
                        libm::cos(a) * 5.0,
                        CY + libm::sin(a) * 5.0,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp - 20.0,
                        life,
                    )
                },
            );
        }
        // the feet go too: a skid of dust along the floor
        for _ in 0..6 {
            let sg = r.sign();
            let x = sg * r.range(1.0, 6.0);
            let vx = sg * r.range(25.0, 70.0);
            let vy = -r.range(2.0, 10.0);
            let life = r.range(0.4, 0.7);
            let size = r.range(1.6, 3.0);
            let delay = r.range(0.02, 0.08);
            ps.spawn(
                &mut r,
                Spawn {
                    col: Some(c[2]),
                    size,
                    delay,
                    ..Spawn::new(Kind::Dust, x, -1.0, vx, vy, life)
                },
            );
        }
        lights.add(0.0, CY, 46.0, c[4], 0.8, 0.4);
        lights.add(0.0, CY, 22.0, WHITE, 0.8, 0.1);
        Box::new(State {
            c,
            r,
            ps,
            lights,
            balls,
            lines,
            t: 0.0,
        })
    }
}

struct State {
    c: [Rgba; 6],
    r: Rng,
    ps: Particles,
    lights: Lights,
    balls: Vec<Ball>,
    lines: Vec<Line>,
    t: f64,
}

/// A ball swells out, breathes, then shrinks to nothing (a cartoon puff goes out ball by ball, it doesn't fade).
fn radius(b: &Ball) -> f64 {
    let age = b.max - b.life;
    let u = age / b.max;
    let grow = b.r0 + (b.r1 - b.r0) * ease::out_back(clamp(age / 0.22, 0.0, 1.0), 2.0);
    let breathe = 1.0 + 0.06 * libm::sin(age * 14.0 + b.r1);
    grow * breathe
        * if u < 0.62 {
            1.0
        } else {
            1.0 - ease::in_back(clamp((u - 0.62) / 0.38, 0.0, 1.0), 1.4)
        }
}

impl State {
    fn draw_ball(&self, p: &mut Painter, b: &Ball, part: Part) {
        let c = &self.c;
        let age = b.max - b.life;
        let r = radius(b);
        let a = js_min(1.0, age * 20.0);
        let lit = age < 0.12;
        if r < 0.6 {
            return;
        }
        // the whole cloud is lit from above: the balls underneath are a step darker
        let hy = (b.y - CY + 6.0) / 14.0;
        let lo = hy > 0.4;
        let hi = hy < -0.3;
        match part {
            Part::Shade => p.disc(
                b.x + r * 0.16,
                b.y + r * 0.2,
                r,
                if lit {
                    c[3]
                } else if lo {
                    c[1]
                } else {
                    c[2]
                },
                0.8 * a,
            ),
            Part::Body => p.disc(
                b.x - r * 0.06,
                b.y - r * 0.08,
                r * 0.86,
                if lit {
                    c[4]
                } else if lo {
                    c[2]
                } else {
                    c[3]
                },
                0.88 * a,
            ),
            Part::Cap => {
                if !lo || lit {
                    p.disc(
                        b.x - r * 0.28,
                        b.y - r * 0.34,
                        r * 0.42,
                        if lit || hi { WHITE } else { c[4] },
                        0.85 * a,
                    );
                    if age < 0.25 {
                        p.disc(
                            b.x - r * 0.3,
                            b.y - r * 0.38,
                            r * 0.2,
                            WHITE,
                            0.9 * a * (1.0 - age / 0.25),
                        );
                    }
                }
            }
        }
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        for i in (0..self.balls.len()).rev() {
            let Some(b) = self.balls.get_mut(i) else {
                continue;
            };
            if b.delay > 0.0 {
                b.delay -= dt;
                continue;
            }
            b.life -= dt;
            if b.life <= 0.0 {
                // each ball goes with a pip of light
                if b.r1 > 4.0 {
                    let (x, y) = (b.x, b.y);
                    self.ps.spawn(
                        &mut self.r,
                        Spawn {
                            col: Some(WHITE),
                            size: 0.6,
                            ..Spawn::new(Kind::Star, x, y, 0.0, -8.0, 0.22)
                        },
                    );
                }
                self.balls.remove(i);
                continue;
            }
            // fast burst, hard drag, then a slow lift
            b.vx -= b.vx * 7.0 * dt;
            b.vy -= b.vy * 7.0 * dt + 30.0 * dt;
            b.x += b.vx * dt;
            b.y = js_min(b.y + b.vy * dt, -radius(b) * 0.7);
        }
        self.ps.update(&mut self.r, dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let (c, t) = (&self.c, self.t);
        if layer == Layer::Back {
            // a soft shadow where the body stood, and the light of the pop on the floor
            let s = 1.0 - smooth(0.1, 0.9, t);
            if s > 0.0 {
                p.glow(0.0, -1.0, 26.0, c[4], 0.25 * s);
            }
            self.ps.draw(p, Layer::Back);
            return;
        }
        // the cloud: shade, body, then caps, so neighbouring balls merge into one lumpy cloud
        for part in [Part::Shade, Part::Body, Part::Cap] {
            for b in self.balls.iter().filter(|b| b.delay <= 0.0) {
                self.draw_ball(p, b, part);
            }
        }
        // the pop: a white ball and an 8-ray flare for a couple of frames, then a ring
        if t < 0.07 {
            let k = t / 0.07;
            p.circle(0.0, CY, 9.0 + 3.0 * k, c[4], 1.0);
            p.circle(0.0, CY, 7.0 + 3.0 * k, WHITE, 1.0);
            flare(p, 0.0, CY, 20.0 * (1.0 - k * 0.5), 1.0 - k, c);
        }
        let rk = (t - 0.03) / 0.2;
        if rk > 0.0 && rk < 1.0 {
            p.ring(
                0.0,
                CY,
                8.0 + 18.0 * ease::out_cubic(rk),
                pixfx_draw::math::js_max(1.0, 2.5 * (1.0 - rk)),
                if rk < 0.3 { WHITE } else { c[4] },
                1.1 - rk,
            );
        }
        for l in &self.lines {
            let u = (t - l.d) / 0.2;
            if u <= 0.0 || u >= 1.0 {
                continue;
            }
            let head = 8.0 + (l.r1 - 8.0) * ease::out_expo(clamp(u / 0.6, 0.0, 1.0));
            let tail = 8.0 + (l.r1 - 8.0) * ease::in_cubic(u);
            let (ca, sa) = (libm::cos(l.a), libm::sin(l.a));
            let hot = c[4];
            p.line(
                ca * tail,
                CY + sa * tail,
                ca * head,
                CY + sa * head,
                |v: f64| if v > 0.6 { WHITE } else { hot },
                1.0,
            );
            p.glow_line(
                ca * tail,
                CY + sa * tail,
                ca * head,
                CY + sa * head,
                3.0,
                c[4],
                0.25,
            );
        }
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
