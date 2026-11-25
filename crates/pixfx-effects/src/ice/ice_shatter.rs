//! Ice shatter: port of pixelfx `src/fx/ice/ice-shatter.js`.
//!
//! Beyond pixelfx: the blow from the left and the burst fire impact cues ([`pixfx_core::cue::impact`]) for the
//! game to answer; the `feel` option scales them, 0 for none.
//!
//! The crack-and-burst recipe (cracks walking out from the point of impact, a white frame, the ice flung apart as
//! shards with ice dust and glints) is adapted from Sprite Fusion's open-source ice
//! (https://www.spritefusion.com/pixel-art-vfx/ice), MIT License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::PI;

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{BitKind, Bits, Kind, Lights, Particles, ParticlesOpts, flare, sparkle};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_min, js_round, rand, seg, smooth, to_uint32};

use super::lib::{
    Crack, FractureOpts, PALETTE, Pane, PaneEvent, Panes, Pt, bit, centroid, draw_fractures, emit,
    end_by, fractures, in_poly, split_poly, step,
};

pub struct IceShatter;

pub static ICE_SHATTER: IceShatter = IceShatter;

const META: Meta = Meta {
    id: "ice/ice-shatter",
    name: "Ice shatter",
    desc: "A block of ice takes a hit from the left: it jolts, cracks race over its faces, it flashes white and bursts into chunks that tumble, bounce and skid off in a roll of ice dust.",
    tags: &[
        "ice",
        "shatter",
        "break",
        "destroy",
        "impact",
        "directional",
        "one-shot",
    ],
    size: (224, 112),
    origin: (68, 90),
    duration: D,
    looping: false,
    fps: 30,
    seed: 6,
    state: None,
    options: &[FEEL.spec()],
};

const D: f64 = 2.8;
const HIT: f64 = 0.26;
const CRACK2: f64 = 0.4;
const SHIVER: f64 = 0.54;
const FLASH: f64 = 0.62;
const BURST: f64 = 0.68;
// the block, seen from the front and a little above: a front face, a top, a side in shadow
const FRONT: [Pt; 4] = [(-16.0, 0.0), (-16.0, -28.0), (12.0, -28.0), (12.0, 0.0)];
const TOP: [Pt; 4] = [(-16.0, -28.0), (-8.0, -34.0), (20.0, -34.0), (12.0, -28.0)];
const SIDE: [Pt; 4] = [(12.0, 0.0), (12.0, -28.0), (20.0, -34.0), (20.0, -6.0)];
const SIL: [Pt; 6] = [
    (-16.0, 0.0),
    (-16.0, -28.0),
    (-8.0, -34.0),
    (20.0, -34.0),
    (20.0, -6.0),
    (12.0, 0.0),
];
const HIT_AT: Pt = (-16.0, -15.0);
/// A fracture plane inside, lighter.
const PLANE: [Pt; 4] = [(-16.0, -21.0), (-5.0, -28.0), (3.0, -13.0), (-16.0, -9.0)];
/// The far corner in shadow.
const SHADE: [Pt; 3] = [(5.0, -28.0), (12.0, -28.0), (12.0, -10.0)];

fn inside(x: f64, y: f64) -> bool {
    in_poly(&SIL, x, y)
}

impl Effect for IceShatter {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the blow that cracks it and the burst ask the game for an impact's answer.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let mut cues = impact(HIT, 0.4, HIT_AT.0, HIT_AT.1, opts);
        cues.extend(impact(BURST, 0.7, 2.0, -17.0, opts));
        cues
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        let mut r = Rng::new(to_uint32(seed as f64));
        // the ice's insides: veins of frost and trapped bubbles in the front face
        let mut veins = Vec::new();
        for _ in 0..3 {
            let mut x = r.range(-12.0, 8.0);
            let mut y = r.range(-24.0, -6.0);
            let mut a = r.range(0.0, TAU);
            for _ in 0..9 {
                a += r.range(-0.6, 0.6);
                x += libm::cos(a) * 1.5;
                y += libm::sin(a) * 1.2;
                if !(-14.0..=10.0).contains(&x) || !(-26.0..=-2.0).contains(&y) {
                    break;
                }
                veins.push((libm::floor(x), libm::floor(y)));
            }
        }
        let mut bubbles = Vec::new();
        for _ in 0..5 {
            let x = r.int(-12.0, 8.0);
            let y = r.int(-23.0, -4.0);
            bubbles.push((x, y, r.chance(0.4)));
        }
        let cracks = fractures(
            &mut r,
            &inside,
            HIT_AT.0 + 1.0,
            HIT_AT.1,
            4,
            FractureOpts {
                len: 8.0,
                a0: Some(-0.9),
                spread: 1.9,
                ..FractureOpts::default()
            },
        );
        let cracks2 = fractures(
            &mut r,
            &inside,
            HIT_AT.0 + 1.0,
            HIT_AT.1,
            6,
            FractureOpts {
                len: 18.0,
                a0: Some(-1.3),
                spread: 2.7,
                fork: 0.16,
                ..FractureOpts::default()
            },
        );
        Box::new(State {
            c,
            r,
            ps: Particles::new(ParticlesOpts {
                ground: 0.0,
                sparks: vec![c[7], c[6], c[5], c[4]],
                haze: Some(c[5]),
            }),
            bits: Bits::new(0.0),
            lights: Lights::new(),
            panes: Panes::new(0.0),
            scuffs: Vec::new(),
            veins,
            bubbles,
            cracks,
            cracks2,
            t: 0.0,
            struck: false,
            burst: false,
            wave2: false,
            white: false,
        })
    }
}

/// Frost a sliding chunk scuffed onto the floor: where, how old.
struct Scuff {
    x: f64,
    t: f64,
}

struct State {
    c: [Rgba; 8],
    r: Rng,
    ps: Particles,
    bits: Bits,
    lights: Lights,
    panes: Panes,
    scuffs: Vec<Scuff>,
    veins: Vec<Pt>,
    bubbles: Vec<(f64, f64, bool)>,
    cracks: Vec<Crack>,
    cracks2: Vec<Crack>,
    t: f64,
    struck: bool,
    burst: bool,
    wave2: bool,
    white: bool,
}

/// Splinters thrown off where a chunk hits the floor.
fn splinter(bits: &mut Bits, r: &mut Rng, c: &[Rgba; 8], x: f64, y: f64, vx: f64, n: u32) {
    for _ in 0..n {
        bit!(bits, BitKind::Shard;
            x = x + r.range(-2.0, 2.0),
            y = y - 1.0,
            vx = vx * 0.6 + r.range(-35.0, 45.0),
            vy = -r.range(30.0, 100.0),
            g = 0.8,
            drag = 0.6,
            bounce = 0.35,
            life = r.range(0.4, 0.8),
            a = r.range(0.0, TAU),
            va = r.range(-18.0, 18.0),
            s = r.range(1.2, 2.4),
            c = step(c, r.int(4.0, 7.0)),
        );
    }
}

/// Chunks of a face, cut at random until each is smaller than `max`.
fn chop(r: &mut Rng, pts: Vec<Pt>, max: f64, depth: u32) -> Vec<Vec<Pt>> {
    let c = centroid(&pts);
    if c.area < max || depth > 4 {
        return vec![pts];
    }
    let x = c.x + r.range(-1.0, 1.0);
    let y = c.y + r.range(-1.0, 1.0);
    let a = r.range(0.0, PI);
    let mut out = Vec::new();
    for q in split_poly(&pts, x, y, a) {
        out.extend(chop(r, q, max, depth + 1));
    }
    out
}

impl State {
    fn jolt(&self) -> f64 {
        let t = self.t;
        if (HIT..HIT + 0.05).contains(&t) {
            2.0
        } else if (HIT + 0.05..HIT + 0.12).contains(&t) {
            1.0
        } else if (SHIVER..FLASH).contains(&t) {
            if libm::floor(t * 40.0) % 2.0 != 0.0 {
                1.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// The block: shaded faces, frost veins and bubbles inside, lit edges, a dark rim.
    fn draw_block(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        if t >= BURST {
            return;
        }
        let jx = self.jolt();
        let wh = t >= FLASH;
        let sweep = -40.0 + 70.0 * seg(t, 0.04, 0.3);
        let sh = |pts: &[Pt]| -> Vec<Pt> { pts.iter().map(|&(x, y)| (x + jx, y)).collect() };
        // rim
        p.poly(
            &sh(&[
                (-17.0, 1.0),
                (-17.0, -28.5),
                (-8.3, -35.2),
                (21.0, -35.2),
                (21.0, -5.6),
                (12.5, 1.0),
            ]),
            c[0],
            1.0,
        );
        if wh {
            p.poly(&sh(&SIL), WHITE, 1.0);
            return;
        }
        // the front: lit at the top, deep at the foot, the far edges showing through, a fracture plane catching
        // light, reflections streaked across the upper left
        p.poly(
            &sh(&FRONT),
            |x: f64, y: f64| {
                let lx = x - jx;
                let d = lx + y * 0.8;
                if libm::fabs(d - sweep) < 1.3 && y < -6.0 {
                    return WHITE;
                }
                if y < -9.0 && (libm::fabs(d + 21.0) < 1.1 || libm::fabs(d + 16.5) < 0.6) {
                    return c[6];
                }
                if ((libm::fabs(lx + 7.5) < 0.5 && y < -6.0)
                    || (libm::fabs(y + 5.5) < 0.5 && lx > -8.0))
                    && (libm::floor(lx) + libm::floor(y)) % 2.0 == 0.0
                {
                    return c[5];
                }
                if in_poly(&PLANE, lx, y) {
                    return if y < -25.5 { c[6] } else { c[5] };
                }
                if in_poly(&SHADE, lx, y) {
                    return c[3];
                }
                if y < -25.5 {
                    c[5]
                } else if y < -8.0 {
                    c[4]
                } else if y < -2.5 {
                    c[3]
                } else {
                    c[2]
                }
            },
            1.0,
        );
        p.poly(
            &sh(&TOP),
            |x: f64, y: f64| {
                if y > -29.5 {
                    c[7]
                } else if y < -32.5 {
                    c[5]
                } else if rand(&[libm::floor(x), libm::floor(y), 3.0]) < 0.08 {
                    c[7]
                } else {
                    c[6]
                }
            },
            1.0,
        );
        p.poly(
            &sh(&SIDE),
            |x: f64, y: f64| {
                let lx = x - jx;
                if lx < 13.5 || (libm::fabs(lx - 16.5) < 0.6 && y < -8.0) {
                    c[3]
                } else if y > (lx - 12.0) * -0.75 - 3.0 {
                    c[1]
                } else {
                    c[2]
                }
            },
            1.0,
        );
        for &(x, y) in &self.veins {
            p.px(x + jx, y, c[5]);
        }
        for &(x, y, big) in &self.bubbles {
            p.px(x + jx, y, c[6]);
            if big {
                p.px(x + jx + 1.0, y, c[5]);
                p.px(x + jx, y + 1.0, c[5]);
            }
        }
        let edges: [(Pt, Pt, Rgba); 8] = [
            ((-16.0, -28.0), (12.0, -28.0), WHITE),
            ((-16.0, -28.0), (-8.0, -34.0), c[6]),
            ((-8.0, -34.0), (20.0, -34.0), c[5]),
            ((12.0, -28.0), (20.0, -34.0), c[7]),
            ((-16.0, -1.0), (-16.0, -28.0), c[5]),
            ((12.0, -1.0), (12.0, -28.0), c[6]),
            ((20.0, -6.0), (20.0, -34.0), c[3]),
            ((13.0, -1.0), (20.0, -6.0), c[1]),
        ];
        for (a, b, col) in edges {
            p.line(a.0 + jx, a.1, b.0 + jx, b.1, col, 1.0);
        }
        if t >= HIT {
            draw_fractures(p, &self.cracks, (t - HIT) * 170.0, c, jx);
        }
        if t >= CRACK2 {
            draw_fractures(p, &self.cracks2, (t - CRACK2) * 200.0, c, jx);
        }
    }

    /// The floor: a frost patch where the block stands, frost scuffed along by sliding chunks.
    fn draw_floor(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        let out = 1.0 - smooth(BURST + 0.6, BURST + 1.5, t);
        if out > 0.0 {
            for xi in -22..=26 {
                let x = f64::from(xi);
                let d = libm::fabs(x - 2.0) / 24.0;
                let h = rand(&[x, 41.0]);
                let a = out * clamp((1.1 - d) / 0.25, 0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                p.dot(x, 0.0, if h < 0.35 { WHITE } else { c[6] }, a);
                if h < 0.4 && (x < -16.0 || x > 20.0) {
                    p.dot(x, -1.0, c[5], a);
                }
                let n = 1.0 + js_round(rand(&[x, 7.0]) * 3.0 * (1.0 - d));
                let mut j = 1.0;
                while j <= n {
                    p.dot(x, j, if j < 2.0 { c[5] } else { c[4] }, a * (1.0 - j / 5.0));
                    j += 1.0;
                }
            }
        }
        for s in &self.scuffs {
            let a = 1.0 - s.t / 0.9;
            if a > 0.0 {
                p.dot(s.x, 0.0, if s.t < 0.15 { WHITE } else { c[6] }, a);
                p.dot(s.x, 1.0, c[4], a * 0.6);
            }
        }
    }

    /// Chunks of each face flung away from the hit, mostly right and up; shards, ice dust, a flurry, glints.
    fn explode(&mut self) {
        let State {
            c,
            r,
            ps,
            bits,
            lights,
            panes,
            ..
        } = self;
        let c = *c;
        for (face, tone, max) in [(&FRONT, 0.6, 46.0), (&TOP, 1.0, 40.0), (&SIDE, 0.3, 46.0)] {
            for part in chop(r, face.to_vec(), max, 0) {
                let m = centroid(&part);
                let (dx, dy) = (m.x - HIT_AT.0, m.y - HIT_AT.1);
                let d = libm::hypot(dx, dy);
                let d = if d == 0.0 { 1.0 } else { d };
                let sp = r.range(50.0, 150.0) * (0.7 + 0.3 * (1.0 - d / 40.0));
                let pts = part.iter().map(|&(x, y)| (x - m.x, y - m.y)).collect();
                let mut q = Pane::new(pts, m.x, m.y);
                q.vx = (dx / d) * sp * 0.9 + r.range(40.0, 110.0);
                q.vy = (dy / d) * sp * 0.4 - r.range(60.0, 200.0);
                q.a = 0.0;
                q.va = r.range(-10.0, 10.0);
                q.f = r.range(-0.9, 0.9);
                q.vf = r.range(-7.0, 7.0);
                q.tone = Some(tone);
                q.lit = tone;
                q.g = 0.9;
                q.drag = 0.5;
                q.bounce = 0.38;
                q.solid = true;
                q.brittle = 3;
                q.crumble = Some(r.range(0.25, 0.6));
                q.life = r.range(1.5, 1.9);
                panes.add(q);
            }
        }
        for _ in 0..46 {
            let x = r.range(-15.0, 19.0);
            let y = -r.range(1.0, 33.0);
            let (dx, dy) = (x - HIT_AT.0, y - HIT_AT.1);
            let d = libm::hypot(dx, dy);
            let d = if d == 0.0 { 1.0 } else { d };
            let sp = r.range(60.0, 200.0);
            if !inside(x, y) {
                continue;
            }
            let kind = if r.chance(0.15) {
                BitKind::Chunk
            } else {
                BitKind::Shard
            };
            bit!(bits, kind;
                x = x,
                y = y,
                vx = (dx / d) * sp * 0.8 + r.range(20.0, 90.0),
                vy = (dy / d) * sp * 0.4 - r.range(60.0, 240.0),
                g = 0.85,
                drag = 0.6,
                bounce = 0.32,
                life = r.range(0.8, 1.4),
                a = r.range(0.0, TAU),
                va = r.range(-22.0, 22.0),
                s = r.range(1.2, 3.4),
                c = step(&c, r.int(4.0, 7.0)),
                ramp = Some(vec![c[1], c[3], c[4], c[6], c[7]]),
            );
        }
        // ice dust rolls out along the floor, vapour hangs where it stood, a flurry, glints, a flash of light
        for _ in 0..9 {
            emit!(
                ps,
                r,
                Kind::Vapour,
                r.range(-6.0, 18.0),
                -r.range(1.0, 6.0),
                r.range(10.0, 130.0),
                -r.range(2.0, 12.0),
                r.range(0.6, 1.1),
                c[6],
                r.range(2.2, 3.8)
            );
        }
        for _ in 0..5 {
            emit!(
                ps,
                r,
                Kind::Dust,
                r.range(-6.0, 16.0),
                -r.range(1.0, 4.0),
                r.range(30.0, 110.0),
                -r.range(2.0, 8.0),
                r.range(0.5, 0.9),
                c[5],
                r.range(1.6, 2.8)
            );
        }
        for _ in 0..4 {
            emit!(
                ps,
                r,
                Kind::Vapour,
                r.range(-8.0, 18.0),
                -r.range(8.0, 28.0),
                r.range(-10.0, 50.0),
                -r.range(8.0, 24.0),
                r.range(0.6, 1.0),
                c[6],
                r.range(2.6, 4.0)
            );
        }
        for _ in 0..16 {
            let a = r.range(-PI * 0.9, -0.1);
            let sp = r.range(40.0, 140.0);
            emit!(
                ps,
                r,
                Kind::Flake,
                r.range(-8.0, 16.0),
                -r.range(6.0, 28.0),
                libm::cos(a) * sp + 20.0,
                libm::sin(a) * sp,
                r.range(1.0, 1.6),
                if r.chance(0.6) { WHITE } else { c[6] },
                r.range(0.5, 1.6)
            );
        }
        for _ in 0..7 {
            emit!(
                ps,
                r,
                Kind::Star,
                r.range(-14.0, 40.0),
                -r.range(4.0, 40.0),
                r.range(-8.0, 16.0),
                -r.range(4.0, 16.0),
                r.range(0.4, 0.8),
                if r.chance(0.5) { WHITE } else { c[6] },
                r.range(1.0, 2.4)
            );
        }
        lights.add(2.0, -17.0, 64.0, WHITE, 0.85, 0.14);
        lights.add(16.0, -18.0, 86.0, c[4], 0.5, 0.55);
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let c = self.c;
        // the hit: a flash on the left face, chips and sparks thrown back, the block jolts
        if !self.struck && t >= HIT {
            self.struck = true;
            let State {
                r,
                ps,
                bits,
                lights,
                ..
            } = self;
            lights.add(HIT_AT.0, HIT_AT.1, 24.0, WHITE, 0.6, 0.08);
            lights.add(HIT_AT.0 + 6.0, HIT_AT.1, 40.0, c[5], 0.4, 0.3);
            for _ in 0..12 {
                let a = PI + r.range(-0.9, 0.9);
                let sp = r.range(50.0, 120.0);
                emit!(
                    ps,
                    r,
                    Kind::Spark,
                    HIT_AT.0 - 1.0,
                    HIT_AT.1,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp - 40.0,
                    r.range(0.15, 0.4)
                );
            }
            for _ in 0..6 {
                let a = PI + r.range(-1.0, 0.8);
                let sp = r.range(40.0, 100.0);
                bit!(bits, BitKind::Shard;
                    x = HIT_AT.0 - 1.0,
                    y = HIT_AT.1 + r.range(-2.0, 2.0),
                    vx = libm::cos(a) * sp,
                    vy = libm::sin(a) * sp - 60.0,
                    g = 0.85,
                    drag = 0.6,
                    bounce = 0.3,
                    life = r.range(0.5, 0.9),
                    a = r.range(0.0, TAU),
                    va = r.range(-20.0, 20.0),
                    s = r.range(1.2, 2.4),
                    c = step(&c, r.int(5.0, 7.0)),
                );
            }
        }
        if !self.wave2 && t >= CRACK2 {
            self.wave2 = true;
            let State {
                r, bits, lights, ..
            } = self;
            lights.add(0.0, -16.0, 36.0, c[6], 0.35, 0.15);
            for _ in 0..3 {
                bit!(bits, BitKind::Shard;
                    x = r.range(-14.0, 18.0),
                    y = -r.range(4.0, 30.0),
                    vx = r.range(-30.0, 30.0),
                    vy = -r.range(10.0, 50.0),
                    g = 0.85,
                    drag = 0.6,
                    bounce = 0.3,
                    life = r.range(0.4, 0.7),
                    a = r.range(0.0, TAU),
                    va = r.range(-20.0, 20.0),
                    s = r.range(1.0, 2.0),
                    c = step(&c, r.int(5.0, 7.0)),
                );
            }
        }
        if !self.white && t >= FLASH {
            self.white = true;
            self.lights.add(2.0, -17.0, 44.0, WHITE, 0.7, 0.08);
        }
        if !self.burst && t >= BURST {
            self.burst = true;
            self.explode();
        }
        let State {
            r,
            ps,
            bits,
            lights,
            panes,
            scuffs,
            burst,
            ..
        } = self;
        // glints wink on the settling debris
        if *burst && t < BURST + 1.0 && r.chance(0.08) && bits.count() > 0 {
            let i = libm::floor(r.next() * bits.count() as f64) as usize;
            if let Some(b) = bits.list().get(i).map(|q| (q.bit.x, q.bit.y)) {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    b.0,
                    b.1 - 1.0,
                    0.0,
                    -3.0,
                    r.range(0.2, 0.4),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(0.6, 1.5)
                );
            }
        }
        for i in (0..scuffs.len()).rev() {
            if let Some(s) = scuffs.get_mut(i) {
                s.t += dt;
                if s.t > 0.9 {
                    scuffs.remove(i);
                }
            }
        }
        ps.update(r, dt);
        bits.update(dt);
        panes.update(dt, |e| match e {
            PaneEvent::Land { vx, x, y, speed } => {
                // a chunk hitting the floor: a puff of ice dust, a few splinters, sometimes a glint
                if speed < 100.0 {
                    return;
                }
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    x,
                    y - 1.0,
                    vx * 0.3 + r.range(-15.0, 15.0),
                    -r.range(2.0, 10.0),
                    r.range(0.4, 0.7),
                    c[6],
                    r.range(1.6, 2.8)
                );
                splinter(bits, r, &c, x, y, vx, 2);
                if r.chance(0.3) {
                    emit!(
                        ps,
                        r,
                        Kind::Star,
                        x,
                        y - 3.0,
                        0.0,
                        -5.0,
                        r.range(0.2, 0.4),
                        WHITE,
                        r.range(0.8, 1.6)
                    );
                }
            }
            PaneEvent::Skid { x, .. } => {
                // sliding chunks scuff frost onto the floor
                if r.chance(0.5) {
                    scuffs.push(Scuff {
                        x: js_round(x + r.range(-1.0, 1.0)),
                        t: 0.0,
                    });
                }
            }
            PaneEvent::Crumble { x, vx } => {
                splinter(bits, r, &c, x, 0.0, vx, 3);
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    x,
                    -1.0,
                    r.range(-10.0, 14.0),
                    -r.range(2.0, 6.0),
                    r.range(0.4, 0.7),
                    c[6],
                    r.range(1.2, 2.0)
                );
            }
        });
        lights.update(dt);
        end_by(t, D, ps, Some(bits), Some(panes));
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer != Layer::Front {
            self.draw_floor(p);
            self.ps.draw(p, Layer::Back);
        }
        if layer == Layer::Back {
            return;
        }
        let (c, t) = (&self.c, self.t);
        self.draw_block(p);
        // the hit: a star and a short flare on the left face
        let h = t - HIT;
        if (0.0..0.2).contains(&h) {
            sparkle(
                p,
                HIT_AT.0 - 1.0,
                HIT_AT.1,
                9.0 * ease::out_back(js_min(1.0, h / 0.04), ease::BACK_S) * (1.0 - h / 0.2),
                c[6],
                c[4],
                0.7,
            );
            if h < 0.08 {
                flare(
                    p,
                    HIT_AT.0 - 1.0,
                    HIT_AT.1,
                    14.0 * (1.0 - h / 0.08),
                    1.0 - h / 0.08,
                    c,
                );
            }
        }
        // cold light in the ice, brighter as it cracks; a pool on the floor
        let k = 1.0 - smooth(BURST, BURST + 0.4, t);
        let crk = smooth(HIT, FLASH, t);
        if k > 0.0 {
            p.glow(2.0, -17.0, 34.0, c[4], (0.16 + 0.22 * crk) * k);
            p.glow(2.0, -2.0, 30.0, c[4], 0.14 * k);
        }
        if (FLASH..BURST + 0.12).contains(&t) {
            let a = clamp((t - FLASH) / 0.18, 0.0, 1.0);
            flare(p, 2.0, -17.0, 26.0 * (1.0 - a), 1.0 - a, c);
        }
        self.panes.draw(p, c);
        self.bits.draw(p);
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
