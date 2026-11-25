//! Chain lightning: port of pixelfx `src/fx/lightning/chain-lightning.js`.
//!
//! The acted-out caster and targets of the gallery (`act`) are left out. Beyond pixelfx: it fires impact cues
//! ([`pixfx_core::cue::impact`]) as the bolt lands on each target, the last one heavier, for the game to answer;
//! the `feel` option scales them, 0 for none.
//!
//! The hop timing, the link shapes that re-strike every other frame, the glowing nodes and the crawling arcs
//! follow Sprite Fusion's open-source lightning (https://www.spritefusion.com/pixel-art-vfx/lightning),
//! MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, sparkle};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{
    Rng, TAU, clamp, ease, js_min, js_round, lerp, rng, smooth, to_int32, to_uint32,
};

use super::lib::{
    Bolt, PLASMA, StrokeOpts, VOLT, bowed, crackle, scorch, stroke, trace_all, zigzag,
};

pub struct ChainLightning;

pub static CHAIN_LIGHTNING: ChainLightning = ChainLightning;

const META: Meta = Meta {
    id: "lightning/chain-lightning",
    name: "Chain lightning",
    desc: "A bolt leaps from the caster's hand through three targets in turn; each flashes, sparks and grounds itself to the floor while the arcs between them flicker, then the chain snaps.",
    tags: &[
        "lightning",
        "electric",
        "chain",
        "spell",
        "combat",
        "directional",
        "one-shot",
    ],
    size: (248, 104),
    origin: (40, 82),
    duration: 1.8,
    looping: false,
    fps: 30,
    seed: 3,
    state: None,
    options: &[SPARKS.spec(), FEEL.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);

const HAND: (f64, f64) = (10.0, -16.0);
const HOPS: [(f64, f64); 3] = [(58.0, -22.0), (104.0, -30.0), (150.0, -19.0)];
const PTS: [(f64, f64); 4] = [HAND, HOPS[0], HOPS[1], HOPS[2]];
const WIND: f64 = 0.12;
const LEAP: f64 = 0.05;
const GAP: f64 = 0.075;
const HOLD: f64 = 0.78;
const BREAK: f64 = 0.16;
const ASH: Rgba = Rgba::hex24(0x5b5566);

/// When link `i` lands on its target.
fn on(i: usize) -> f64 {
    WIND + i as f64 * GAP + LEAP
}

/// When link `i` starts to give out.
fn off(i: usize) -> f64 {
    HOLD + i as f64 * 0.05
}

/// A link's strength: a hard first flash, a restless hum, then it stutters out.
fn level(i: usize, t: f64) -> f64 {
    let a = t - on(i);
    if a < 0.0 {
        return 0.0;
    }
    let fi = i as f64;
    let hum = if a < 0.1 {
        [1.0, 0.55, 1.0]
            .get(libm::floor(a * 30.0) as usize)
            .copied()
            .unwrap_or(1.0)
    } else {
        0.78 + 0.22 * libm::sin(t * 61.0 + fi) * libm::sin(t * 23.0 + fi * 2.0)
    };
    let b = t - off(i);
    if b < 0.0 {
        hum
    } else if b < BREAK {
        hum * (1.0 - b / BREAK)
            * if libm::floor(b * 60.0) % 3.0 == 1.0 {
                0.5
            } else {
                1.0
            }
    } else {
        0.0
    }
}

impl Effect for ChainLightning {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: each target struck asks the game for an impact's answer, the last one heavier.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        HOPS.iter()
            .enumerate()
            .flat_map(|(i, &(x, y))| impact(on(i), if i == 2 { 0.6 } else { 0.35 }, x, y, opts))
            .collect()
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&VOLT));
        let mut r = Rng::new(to_uint32(seed as f64));
        // where each target grounds itself: a spot on the floor below it
        let ground = HOPS.map(|(x, _)| x + r.range(-5.0, 5.0));
        Box::new(State {
            c,
            seed: seed as f64,
            r,
            ps: Particles::new(ParticlesOpts {
                ground: 0.0,
                sparks: opts.ramp(&SPARKS),
                ..ParticlesOpts::default()
            }),
            hot: Particles::new(ParticlesOpts {
                ground: 0.0,
                ..ParticlesOpts::default()
            }),
            lights: Lights::new(),
            hit: [false; 3],
            popped: [false; 3],
            ground,
            t: 0.0,
        })
    }
}

struct State {
    c: [Rgba; 8],
    seed: f64,
    r: Rng,
    ps: Particles,
    hot: Particles,
    lights: Lights,
    hit: [bool; 3],
    popped: [bool; 3],
    ground: [f64; 3],
    t: f64,
}

/// A spawn with every argument given, in pixelfx's order (its draws are made before this is called).
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
fn spawn(
    ps: &mut Particles,
    r: &mut Rng,
    kind: Kind,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    col: Rgba,
    size: f64,
    delay: f64,
) {
    ps.spawn(
        r,
        Spawn {
            kind,
            x,
            y,
            vx,
            vy,
            life,
            col: Some(col),
            size,
            delay,
            flags: Flags::NONE,
        },
    );
}

/// Link `i`'s shape on frame `f`, the same shape for the same frame.
fn shape(seed: f64, i: usize, f: f64) -> Bolt {
    let mut r = rng(&[seed, 31.0, i as f64, f]);
    let ((x0, y0), (x1, y1)) = (PTS[i], PTS[i + 1]);
    let l = libm::hypot(x1 - x0, y1 - y0);
    let bow = -l * r.range(0.05, 0.15);
    bowed(&mut r, x0, y0, x1, y1, bow, 0.2, 0.08)
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let (c, t) = (&self.c, self.t);
        let r = &mut self.r;
        let (ps, hot) = (&mut self.ps, &mut self.hot);
        // the charge gathers at the hand
        if t < WIND && r.chance(70.0 * dt) {
            let a = r.range(0.0, TAU);
            let d = r.range(10.0, 24.0);
            let tt = r.range(0.06, 0.1);
            let v = (d * 3.0) / (libm::exp(3.0 * tt) - 1.0);
            let col = if r.chance(0.5) { c[6] } else { c[5] };
            spawn(
                ps,
                r,
                Kind::Streak,
                HAND.0 + libm::cos(a) * d,
                HAND.1 + libm::sin(a) * d,
                -libm::cos(a) * v,
                -libm::sin(a) * v,
                tt,
                col,
                1.0,
                0.0,
            );
        }
        for (i, &(x, y)) in HOPS.iter().enumerate() {
            if !self.hit[i] && t >= on(i) {
                self.hit[i] = true;
                let (px, py) = PTS[i];
                let dir = libm::atan2(y - py, x - px);
                self.lights.add(x, y, 56.0, c[4], 0.85, 0.22);
                self.lights.add(x, y, 16.0, WHITE, 0.7, 0.08);
                if i == 0 {
                    self.lights.add(HAND.0, HAND.1, 40.0, c[4], 0.6, 0.15);
                }
                for _ in 0..18 {
                    let a = if r.chance(0.6) {
                        dir + r.range(-1.1, 1.1)
                    } else {
                        r.range(0.0, TAU)
                    };
                    let sp = r.range(70.0, 230.0);
                    let life = r.range(0.15, 0.4);
                    spawn(
                        ps,
                        r,
                        Kind::Spark,
                        x,
                        y,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp - 40.0,
                        life,
                        WHITE,
                        1.0,
                        0.0,
                    );
                }
                for _ in 0..5 {
                    let sx = x + r.range(-2.0, 2.0);
                    let vx = r.range(-50.0, 50.0);
                    let vy = r.range(-60.0, 20.0);
                    let life = r.range(0.5, 0.9);
                    spawn(
                        hot,
                        r,
                        Kind::Spark,
                        sx,
                        y + 2.0,
                        vx,
                        vy,
                        life,
                        WHITE,
                        1.0,
                        0.0,
                    );
                }
                let sx = x + r.range(-4.0, 4.0);
                let sy = y - r.range(3.0, 7.0);
                let life = r.range(0.25, 0.4);
                let size = r.range(1.5, 2.5);
                spawn(ps, r, Kind::Star, sx, sy, 0.0, -6.0, life, c[6], size, 0.0);
                for _ in 0..2 {
                    let sx = x + r.range(-4.0, 4.0);
                    let sy = y + r.range(-2.0, 3.0);
                    let vx = r.range(-6.0, 6.0);
                    let vy = -r.range(8.0, 16.0);
                    let life = r.range(0.7, 1.0);
                    let col = mix(ASH, c[2], 0.35);
                    let size = r.range(2.0, 3.5);
                    let delay = r.range(0.1, 0.25);
                    spawn(ps, r, Kind::Smoke, sx, sy, vx, vy, life, col, size, delay);
                }
            }
            // while the chain holds: sparks shed off the links, spit from the floor where each target grounds
            if self.hit[i] && t < off(i) {
                if r.chance(9.0 * dt) {
                    let u = r.next();
                    let (x0, y0) = PTS[i];
                    let vx = r.range(-60.0, 60.0);
                    let vy = r.range(-90.0, 20.0);
                    let life = r.range(0.1, 0.25);
                    spawn(
                        ps,
                        r,
                        Kind::Spark,
                        lerp(x0, x, u),
                        lerp(y0, y, u) - 2.0,
                        vx,
                        vy,
                        life,
                        WHITE,
                        1.0,
                        0.0,
                    );
                }
                if r.chance(7.0 * dt) {
                    let vx = r.range(-70.0, 70.0);
                    let vy = -r.range(40.0, 140.0);
                    let life = r.range(0.12, 0.3);
                    let g = self.ground[i];
                    spawn(ps, r, Kind::Spark, g, -1.0, vx, vy, life, WHITE, 1.0, 0.0);
                }
                if r.chance(2.0 * dt) {
                    let sx = x + r.range(-3.0, 3.0);
                    let vx = r.range(-30.0, 30.0);
                    let vy = r.range(-30.0, 10.0);
                    let life = r.range(0.5, 0.8);
                    spawn(
                        hot,
                        r,
                        Kind::Spark,
                        sx,
                        y + 3.0,
                        vx,
                        vy,
                        life,
                        WHITE,
                        1.0,
                        0.0,
                    );
                }
            }
            // when it gives out, the target pops once more and smokes
            if !self.popped[i] && t >= off(i) + BREAK {
                self.popped[i] = true;
                for _ in 0..7 {
                    let a = r.range(0.0, TAU);
                    let sp = r.range(40.0, 140.0);
                    let life = r.range(0.12, 0.3);
                    spawn(
                        ps,
                        r,
                        Kind::Spark,
                        x,
                        y,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp - 30.0,
                        life,
                        WHITE,
                        1.0,
                        0.0,
                    );
                }
                let vx = r.range(-4.0, 4.0);
                let vy = -r.range(10.0, 16.0);
                let life = r.range(0.55, 0.7);
                let col = mix(ASH, c[2], 0.35);
                let size = r.range(2.5, 3.5);
                spawn(ps, r, Kind::Smoke, x, y, vx, vy, life, col, size, 0.0);
                self.lights.add(x, y, 30.0, c[4], 0.5, 0.12);
            }
        }
        ps.update(r, dt);
        hot.update(r, dt);
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

impl State {
    fn draw_back(&self, p: &mut Painter) {
        let t = self.t;
        // the floor scorches where the targets ground themselves
        for (i, &g) in self.ground.iter().enumerate() {
            let a = t - on(i);
            if a < 0.0 {
                continue;
            }
            let t0 = js_min(1.0, a / 0.2)
                * libm::pow(clamp(1.0 - (t - off(i)) / 0.7, 0.0, 1.0), 1.3)
                * 0.9;
            scorch(p, g, 0.0, 7.0, t0, 1.0 - smooth(1.35, 1.75, t));
        }
        self.ps.draw(p, Layer::Back);
        self.hot.draw(p, Layer::Back);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (c, t, seed) = (&self.c, self.t, self.seed);
        let f = libm::floor(t * 30.0 + 1e-4);
        let f2 = f64::from(to_int32(f) >> 1);
        // the hand: a node that swells while the charge gathers and holds the chain
        let kh = if t < WIND { t / WIND } else { level(0, t) };
        if kh > 0.02 {
            let r = (1.2 + 1.6 * kh + libm::sin(t * 53.0) * 0.4) * js_min(1.0, kh * 1.5);
            p.circle(HAND.0, HAND.1, r + 1.2, c[4], 1.0);
            p.circle(HAND.0, HAND.1, r, c[6], 1.0);
            p.circle(HAND.0, HAND.1, r * 0.5, WHITE, 1.0);
            p.glow(HAND.0, HAND.1, 18.0, c[4], 0.6 * kh);
            if t > WIND * 0.5 {
                let mut r2 = rng(&[seed, 5.0, f]);
                let a = r2.range(0.0, TAU);
                let d = r2.range(4.0, 8.0);
                let b = zigzag(
                    &mut r2,
                    HAND.0,
                    HAND.1,
                    HAND.0 + libm::cos(a) * d,
                    HAND.1 + libm::sin(a) * d,
                    0.4,
                    0.3,
                );
                crackle(p, &b, 0.6 * kh, c, 0.3, 1.0);
            }
        }
        for (i, &(x, y)) in HOPS.iter().enumerate() {
            let t0 = WIND + i as f64 * GAP;
            // the leap: the bolt races out from the last node with a hot head
            if t >= t0 && t < on(i) {
                let u = ease::out_quad((t - t0) / LEAP);
                let b = shape(seed, i, -1.0);
                let trunk = Bolt {
                    paths: b.paths.first().cloned().into_iter().collect(),
                };
                stroke(
                    p,
                    &trunk,
                    1.0,
                    c,
                    0.75,
                    StrokeOpts {
                        up_to: Some(u),
                        ..StrokeOpts::default()
                    },
                );
                if let Some(tp) = b.paths.first().map(|q| &q.pts) {
                    let n = tp.len();
                    let j = js_min(n as f64 - 1.0, libm::floor(u * (n as f64 - 1.0)));
                    if let Some(&(hx, hy)) = tp.get(j as usize) {
                        p.circle(hx, hy, 2.0, WHITE, 1.0);
                        p.glow(hx, hy, 16.0, c[5], 0.7);
                    }
                }
            }
            let k = level(i, t);
            if k > 0.0 {
                stroke(p, &shape(seed, i, f2), k, c, 0.75, StrokeOpts::default());
            } else if t >= off(i) + BREAK && t < off(i) + BREAK + 0.35 {
                // the spent channel: a dotted trace that cools and goes
                let g = 1.0 - (t - off(i) - BREAK) / 0.35;
                let b = shape(seed, i, libm::floor((off(i) + BREAK) * 15.0));
                if let Some(q) = b.paths.first() {
                    trace_all(p, &q.pts, if g > 0.5 { c[4] } else { c[2] }, g * 0.9);
                }
            }
            // the target: a flash as the bolt lands, then a node with arcs crawling over it and down to the floor
            let a = t - on(i);
            if (0.0..0.09).contains(&a) {
                let q = a / 0.09;
                p.circle(x, y, 6.0 * (1.0 - q) + 1.5, c[5], 1.0);
                p.circle(x, y, 5.0 * (1.0 - q), WHITE, 1.0);
                sparkle(p, x, y, js_round(10.0 * (1.0 - q)), c[6], c[4], 0.6);
            }
            if k > 0.05 {
                let fi = i as f64;
                let r = (2.6 + libm::sin(t * 47.0 + fi) * 0.7) * k;
                p.circle(x, y, r + 1.5, c[2], 1.0);
                p.circle(x, y, r + 0.7, c[5], 1.0);
                p.circle(x, y, r * 0.6, WHITE, 1.0);
                p.glow(x, y, 26.0, c[4], 0.6 * k);
                p.glow(x, y, 8.0, c[6], 0.5 * k);
                // arcs crawl over the target as if over a body caught in the current
                let mut r2 = rng(&[seed, 11.0, fi, f]);
                let n = if t - on(i) < 0.12 { 2 } else { 1 };
                for _ in 0..n {
                    let a0 = r2.range(0.0, TAU);
                    let a1 = a0 + r2.sign() * r2.range(0.5, 1.0);
                    let d0 = r2.range(4.0, 6.0);
                    let d1 = r2.range(5.0, 8.0);
                    let b = zigzag(
                        &mut r2,
                        x + libm::cos(a0) * d0,
                        y + libm::sin(a0) * d0,
                        x + libm::cos(a1) * d1,
                        y + libm::sin(a1) * d1,
                        0.35,
                        0.0,
                    );
                    let kk = r2.range(0.45, 0.7) * k;
                    crackle(p, &b, kk, c, 0.25, 1.0);
                }
                if r2.next() < 0.3 * k {
                    let gx = self.ground[i] + r2.range(-3.0, 3.0);
                    let sx = x + r2.range(-2.0, 2.0);
                    let b = zigzag(&mut r2, sx, y + 3.0, gx, -0.5, 0.3, 0.35);
                    crackle(p, &b, 0.75 * k, c, 0.35, 1.0);
                    p.glow(gx, -1.0, 12.0, c[5], 0.5 * k);
                    p.px(gx, -1.0, WHITE);
                }
            }
        }
        self.ps.draw(p, Layer::Front);
        self.hot.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
