//! Charge up: port of pixelfx `src/fx/lightning/charge-up.js`.
//!
//! Beyond pixelfx: it fires an impact cue ([`pixfx_core::cue::impact`]) at the release for the game to answer;
//! the `feel` option scales it, 0 for none. The acted-out caster (`act`) is gallery staging and is left out. Wraps
//! its actor ([`Effect::wraps_actor`]): the caster's edge catches the core's light and its body flashes at the
//! release; with no sprite handed to it, it reads pixelfx's stock 2x actor ([`super::lib::stand_in`]), as the
//! pixelfx render does.
//!
//! The charge (streaks spiralling in, a core that swells with an accelerating pulse, little arcs spitting off
//! it) and the release (flash, rings, a spray of motes and stars) follow Sprite Fusion's open-source energy charge
//! (https://www.spritefusion.com/pixel-art-vfx/energy), MIT License, Copyright (c) 2026 Hugo Duprez.

use std::sync::Arc;

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, flare};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_max, js_min, rng, to_uint32};

use super::lib::{PLASMA, StrokeOpts, VOLT, crackle, edge_of, stand_in, stroke, zigzag};

pub struct ChargeUp;

pub static CHARGE_UP: ChargeUp = ChargeUp;

const META: Meta = Meta {
    id: "lightning/charge-up",
    name: "Charge up",
    desc: "Sparks streak and arcs whip in to a crackling core at the caster's hand that swells and pulses faster, then it flashes and bursts: bolts every way, a ring, a spray of sparks.",
    tags: &[
        "lightning",
        "electric",
        "charge",
        "spell",
        "power-up",
        "actor",
        "one-shot",
    ],
    size: (272, 184),
    origin: (124, 148),
    duration: 2.7,
    looping: false,
    fps: 30,
    seed: 4,
    state: None,
    options: &[SPARKS.spec(), CHARGE.spec(), FEEL.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);
/// Seconds of charging before the release.
const CHARGE: NumOpt = NumOpt::new("charge", 1.6);

const PI: f64 = core::f64::consts::PI;
const CX: f64 = 15.0;
const CY: f64 = -21.0;
const BK: [f64; 6] = [1.0, 0.7, 0.95, 0.5, 0.8, 0.35];
const ASH: Rgba = Rgba::hex24(0x5b5566);

impl Effect for ChargeUp {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    /// Beyond pixelfx: the release asks the game for a heavy impact's answer.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(opts.num(&CHARGE), 1.0, CX, CY, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State::new(opts, seed))
    }
}

#[expect(clippy::too_many_arguments, reason = "pixelfx spawn signature")]
fn sp(
    kind: Kind,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    col: Option<Rgba>,
    size: f64,
    delay: f64,
) -> Spawn {
    Spawn {
        kind,
        x,
        y,
        vx,
        vy,
        life,
        col,
        size,
        delay,
        flags: Flags::NONE,
    }
}

/// One bolt of the release: where it ends and when it strikes.
struct Burst {
    x1: f64,
    y1: f64,
    t0: f64,
}

/// An arc whipping in to the core: where it starts, when, and its shape's seed.
struct Whip {
    x: f64,
    y: f64,
    t: f64,
    seed: f64,
}

/// The body it plays on and its edge cells, read once per pose.
struct Body {
    actor: Arc<Actor>,
    edge: Vec<(i32, i32)>,
}

impl Body {
    fn of(actor: Option<Arc<Actor>>) -> Option<Self> {
        actor.or_else(stand_in).map(|actor| Self {
            edge: edge_of(&actor),
            actor,
        })
    }
}

struct State {
    c: [Rgba; 8],
    seed: f64,
    rel: f64,
    r: Rng,
    ps: Particles,
    lights: Lights,
    bolts: Vec<Burst>,
    arcs: Vec<Whip>,
    t: f64,
    released: bool,
    phase: f64,
    beats: Vec<f64>,
    body: Option<Body>,
}

impl State {
    fn new(opts: &Options, seed: u64) -> Self {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&VOLT));
        let sd = seed as f64;
        let rel = opts.num(&CHARGE);
        let mut r = Rng::new(to_uint32(sd));
        let ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: opts.ramp(&SPARKS),
            ..ParticlesOpts::default()
        });
        let bolts = (0..8)
            .map(|i| {
                let a = (f64::from(i) / 8.0) * TAU + r.range(-0.25, 0.25);
                let l = r.range(44.0, 72.0);
                let x1 = CX + libm::cos(a) * l;
                let y1 = js_min(0.0, CY + libm::sin(a) * l);
                Burst {
                    x1,
                    y1,
                    t0: rel + r.range(0.0, 0.05),
                }
            })
            .collect();
        Self {
            c,
            seed: sd,
            rel,
            r,
            ps,
            lights: Lights::new(),
            bolts,
            arcs: Vec::new(),
            t: 0.0,
            released: false,
            phase: 0.0,
            beats: Vec::new(),
            body: Body::of(None),
        }
    }

    fn level(&self) -> f64 {
        clamp(self.t / self.rel, 0.0, 1.0)
    }

    fn draw_back(&self, p: &mut Painter) {
        let k = self.level();
        if self.t < self.rel {
            p.glow(CX * 0.5, -3.0, 40.0, self.c[3], 0.3 * k * k);
        }
        self.ps.draw(p, Layer::Back);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (c, t, seed) = (&self.c, self.t, self.seed);
        let k = self.level();
        let e = k * k;
        let f = libm::floor(t * 30.0 + 1e-4);
        if t < self.rel {
            let jit = if k > 0.85 {
                (if f % 2.0 != 0.0 { 1.0 } else { -1.0 }) * ((k - 0.85) / 0.15)
            } else {
                0.0
            };
            let (x, y) = (CX + jit, CY);
            let pr = 0.5 + 0.5 * libm::sin(self.phase * TAU);
            // the body catches the core's light along its near edge
            if let Some(b) = &self.body {
                for &(ex, ey) in &b.edge {
                    let (ex, ey) = (f64::from(ex), f64::from(ey));
                    let d = libm::hypot(ex + 0.5 - x, ey + 0.5 - y);
                    if d < 6.0 + 10.0 * k && e > 0.08 {
                        p.px(
                            ex,
                            ey,
                            if d < 4.0 + 6.0 * k && e > 0.4 {
                                c[6]
                            } else {
                                c[4]
                            },
                        );
                    }
                }
            }
            // rings fall inward to the core on every beat
            for &b in &self.beats {
                let q = (t - b) / 0.22;
                p.ring(
                    x,
                    y,
                    4.0 + 20.0 * (1.0 - ease::in_quad(q)) * (0.5 + k),
                    1.0,
                    if q > 0.6 { c[6] } else { c[4] },
                    0.4 + 0.6 * q,
                );
            }
            // arcs whipping in to the core
            for a in &self.arcs {
                let b = zigzag(&mut rng(&[a.seed, f]), a.x, a.y, x, y, 0.35, 0.4);
                crackle(p, &b, 0.85, c, 0.4, 1.0);
            }
            // the core: swelling, layered, with arcs racing round its skin
            let r = 1.2 + 6.5 * e + pr * (0.5 + 1.2 * k);
            p.circle(x, y, r + 1.3, c[2], 1.0);
            p.circle(x, y, r, c[4], 1.0);
            p.circle(x - r * 0.12, y - r * 0.12, r * 0.7, c[5], 1.0);
            p.circle(x, y, r * 0.38, WHITE, 1.0);
            if r > 3.0 {
                for s in [0.0, PI] {
                    let mut a = 0.0;
                    while a < 1.3 {
                        let ang = t * 10.0 + s + a;
                        let rr = r * (0.62 + 0.1 * libm::sin(a * 3.0));
                        p.dot(
                            x + libm::cos(ang) * rr,
                            y + libm::sin(ang) * rr,
                            if a < 0.5 { WHITE } else { c[6] },
                            1.0,
                        );
                        a += 0.12;
                    }
                }
            }
            if k > 0.35 {
                let mut q = rng(&[seed, 5.0, f]);
                let a = q.range(0.0, TAU);
                let l = q.range(3.0, 6.0) * (0.5 + k);
                let (ca, sa) = (libm::cos(a), libm::sin(a));
                let b = zigzag(
                    &mut q,
                    x + ca * r,
                    y + sa * r,
                    x + ca * (r + l),
                    y + sa * (r + l),
                    0.4,
                    0.3,
                );
                crackle(p, &b, 0.7, c, 0.3, 1.0);
            }
            p.glow(x, y, 12.0 + 30.0 * k, c[4], 0.35 + 0.45 * k);
            p.glow(x, y, 6.0 + 6.0 * k, c[6], 0.25 + 0.3 * k);
        } else {
            let a = t - self.rel;
            // the release: the caster flashed white, a flare, a white core blown open
            if a < 0.07
                && let Some(b) = &self.body
            {
                let col = if a < 0.035 { WHITE } else { c[6] };
                p.mask(
                    &*b.actor,
                    f64::from(b.actor.x()),
                    f64::from(b.actor.y()),
                    col,
                    1.0,
                );
            }
            if a < 0.22 {
                let q = a / 0.22;
                flare(p, CX, CY, 46.0 * libm::pow(1.0 - q, 1.4), 1.0 - q, c);
                if q < 0.3 {
                    p.circle(CX, CY, 12.0 * (1.0 - q * 2.5) + 1.5, WHITE, 1.0);
                }
            }
            // bolts every way, struck and flickering
            for (i, b) in self.bolts.iter().enumerate() {
                let ba = libm::floor((t - b.t0) * 30.0 + 1e-4);
                if t < b.t0 || ba >= BK.len() as f64 {
                    continue;
                }
                let bi = ba as usize;
                let bolt = zigzag(
                    &mut rng(&[seed, 3.0, i as f64, f64::from((ba as i32) >> 1)]),
                    CX,
                    CY,
                    b.x1,
                    b.y1,
                    0.25,
                    0.75,
                );
                stroke(
                    p,
                    &bolt,
                    BK[bi],
                    c,
                    if bi == 0 { 0.95 } else { 0.75 },
                    StrokeOpts {
                        hero: bi == 0,
                        ..StrokeOpts::default()
                    },
                );
                if b.y1 > -1.0 {
                    p.glow(b.x1, -3.0, 18.0, c[5], 0.5 * BK[bi]);
                }
            }
            // a ring bursting out with a fainter echo inside it
            let q = a / 0.45;
            if q < 1.0 {
                let r = 8.0 + 80.0 * ease::out_cubic(q);
                p.ring(
                    CX,
                    CY,
                    r,
                    js_max(1.0, 3.0 * (1.0 - q)),
                    if q < 0.25 { WHITE } else { c[5] },
                    |_v: f64, g: f64| {
                        if CY + libm::sin(g) * r > -1.0 {
                            0.0
                        } else {
                            1.1 - q
                        }
                    },
                );
                let r2 = 6.0 + 56.0 * ease::out_cubic(js_min(1.0, q * 1.3));
                p.ring(CX, CY, r2, 1.0, c[4], |_v: f64, g: f64| {
                    if CY + libm::sin(g) * r2 > -1.0 {
                        0.0
                    } else {
                        0.8 - q
                    }
                });
                p.glow(
                    CX,
                    CY - 6.0,
                    js_min(r * 0.9, 46.0),
                    c[3],
                    0.42 * (1.0 - q) * (1.0 - q),
                );
            }
            // the charge bleeds off in a few last crackles at the hand
            if a > 0.15 && a < 0.5 {
                let mut qq = rng(&[seed, 8.0, f]);
                let ang = qq.range(0.0, TAU);
                let l = qq.range(5.0, 10.0);
                let b = zigzag(
                    &mut qq,
                    CX,
                    CY,
                    CX + libm::cos(ang) * l,
                    CY + libm::sin(ang) * l,
                    0.45,
                    0.3,
                );
                crackle(p, &b, 0.8 * (1.0 - (a - 0.15) / 0.35) + 0.15, c, 0.3, 1.0);
            }
        }
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.body = Body::of(actor);
    }

    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let c = self.c;
        let k = self.level();
        let e = k * k;
        let r = &mut self.r;
        if t < self.rel {
            // energy rushes in: long streaks spiralling in from all round, more and more of them
            let mut acc = (60.0 + 360.0 * e) * dt;
            while acc > 0.0 {
                if acc < 1.0 && !r.chance(acc) {
                    break;
                }
                let a = r.range(0.0, TAU);
                let d = r.range(30.0, 72.0) * (1.0 - 0.35 * k);
                let life = r.range(0.16, 0.26);
                let v = (d / life) * 0.95;
                let (ca, sa) = (libm::cos(a), libm::sin(a));
                let s = sp(
                    Kind::Mote,
                    CX + ca * d,
                    js_min(-2.0, CY + sa * d),
                    -ca * v - sa * v * 0.4,
                    -sa * v + ca * v * 0.4,
                    life,
                    r.pick(&[c[7], c[6], c[5]]).copied(),
                    1.0,
                    0.0,
                );
                self.ps.spawn(r, s);
                acc -= 1.0;
            }
            // arcs whip in from the air, more and more of them
            if k > 0.2 && r.chance((2.0 + 16.0 * e) * dt) {
                let a = r.range(0.0, TAU);
                let d = r.range(20.0, 40.0);
                let seed = r.int(0.0, 1e6);
                self.arcs.push(Whip {
                    x: CX + libm::cos(a) * d,
                    y: js_min(-3.0, CY + libm::sin(a) * d),
                    t,
                    seed,
                });
            }
            // the pulse quickens
            self.phase += (1.5 + 11.0 * e) * dt;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
                self.beats.push(t);
            }
            if r.chance(4.0 * e * dt) {
                let s = sp(
                    Kind::Star,
                    CX + r.range(-14.0, 14.0),
                    CY + r.range(-14.0, 10.0),
                    0.0,
                    -4.0,
                    r.range(0.2, 0.35),
                    Some(c[6]),
                    r.range(1.0, 2.0),
                    0.0,
                );
                self.ps.spawn(r, s);
            }
        } else if !self.released {
            self.released = true;
            self.lights.add(CX, CY, 110.0, c[4], 0.8, 0.5);
            self.lights.add(CX, CY, 28.0, WHITE, 0.6, 0.08);
            for _ in 0..70 {
                let a = r.range(0.0, TAU);
                let v = r.range(140.0, 330.0) * if libm::sin(a) > 0.4 { 0.55 } else { 1.0 };
                let kind = if libm::sin(a) < -0.2 && r.chance(0.35) {
                    Kind::Mote
                } else {
                    Kind::Spark
                };
                let s = sp(
                    kind,
                    CX,
                    CY,
                    libm::cos(a) * v,
                    libm::sin(a) * v,
                    r.range(0.25, 0.55),
                    r.pick(&[c[7], c[6], c[5]]).copied(),
                    r.range(0.8, 1.4),
                    0.0,
                );
                self.ps.spawn(r, s);
            }
            for _ in 0..10 {
                let a = r.range(0.0, TAU);
                let v = r.range(60.0, 160.0);
                let s = sp(
                    Kind::Star,
                    CX,
                    CY,
                    libm::cos(a) * v,
                    libm::sin(a) * v,
                    r.range(0.4, 0.8),
                    Some(if r.chance(0.5) { c[6] } else { c[5] }),
                    r.range(1.2, 2.4),
                    0.0,
                );
                self.ps.spawn(r, s);
            }
            for _ in 0..4 {
                let s = sp(
                    Kind::Smoke,
                    CX + r.range(-5.0, 5.0),
                    CY + r.range(-4.0, 4.0),
                    r.range(-12.0, 12.0),
                    -r.range(8.0, 18.0),
                    r.range(0.6, 0.85),
                    Some(mix(ASH, c[4], 0.3)),
                    r.range(3.0, 5.0),
                    r.range(0.05, 0.15),
                );
                self.ps.spawn(r, s);
            }
        }
        self.beats.retain(|&b| t - b < 0.22);
        self.arcs.retain(|a| t - a.t <= 0.07);
        self.ps.update(r, dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        match layer {
            Layer::Back => self.draw_back(p),
            Layer::Front => self.draw_front(p),
            Layer::Both => {
                self.draw_back(p);
                self.draw_front(p);
            }
        }
    }
}
