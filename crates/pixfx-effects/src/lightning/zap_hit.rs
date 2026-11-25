//! Zap hit: port of pixelfx `src/fx/lightning/zap-hit.js`.
//!
//! Beyond pixelfx: it fires impact cues ([`pixfx_core::cue::impact`]) when the arc lands, for the game to answer;
//! the `feel` option scales them, 0 for none.
//!
//! The arc's halo-body-core inking, its flicker and the light that strobes as it dies follow Sprite Fusion's
//! open-source lightning (https://www.spritefusion.com/pixel-art-vfx/lightning), MIT License, Copyright (c) 2026
//! Hugo Duprez.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Flags, Kind, Lights, Particles, ParticlesOpts, Spawn};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, js_max, js_round, rng, smooth, to_uint32};

use super::lib::{Bolt, StrokeOpts, VOLT, crackle, stroke, zigzag};

pub struct ZapHit;

pub static ZAP_HIT: ZapHit = ZapHit;

const META: Meta = Meta {
    id: "lightning/zap-hit",
    name: "Zap hit",
    desc: "A contact zap: an arc snaps in from the left, the hit point overloads into a knot of tiny crackling zigzags, a fern of burn marks glows outward, sparks jink away, ozone puffs.",
    tags: &[
        "lightning",
        "electric",
        "hit",
        "impact",
        "combat",
        "directional",
        "one-shot",
    ],
    size: (144, 112),
    origin: (76, 56),
    duration: 0.9,
    looping: false,
    fps: 30,
    seed: 3,
    state: None,
    options: &[FEEL.spec()],
};

const SNAP: f64 = 0.05;
const ASH: Rgba = Rgba::hex24(0x6a6680);

impl Effect for ZapHit {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the zap landing asks the game for an impact's answer.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(SNAP, 0.5, 0.0, 0.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&VOLT));
        let mut r = Rng::new(to_uint32(seed as f64));
        let tree = fern(&mut r);
        let y0 = r.range(-14.0, 6.0);
        // sparks that jink: they fly out and jerk to a new heading every few hundredths of a second
        let kinked = (0..16)
            .map(|_| {
                let a = r.range(-1.2, 1.2)
                    + if r.chance(0.7) {
                        0.0
                    } else {
                        core::f64::consts::PI
                    };
                let sp = r.range(130.0, 250.0);
                let life = r.range(0.2, 0.36);
                let next = r.range(0.02, 0.05);
                Jink {
                    x: 0.0,
                    y: 0.0,
                    vx: libm::cos(a) * sp,
                    vy: libm::sin(a) * sp - 40.0,
                    life,
                    age: -SNAP,
                    next,
                    trail: Vec::new(),
                }
            })
            .collect();
        Box::new(State {
            c,
            seed: seed as f64,
            r,
            ps: Particles::new(ParticlesOpts::default()),
            lights: Lights::new(),
            tree,
            y0,
            kinked,
            t: 0.0,
            hit: false,
        })
    }
}

/// One frond of the fern: points `(x, y, distance along)` and a weight.
struct Frond {
    pts: Vec<(f64, f64, f64)>,
    w: f64,
}

/// The burn the current leaves on what it hit: a Lichtenberg fern, main fronds radiating, side fronds alternating.
fn fern(r: &mut Rng) -> Vec<Frond> {
    let mut out = Vec::new();
    for i in 0..6 {
        let a = (f64::from(i) / 6.0) * TAU + r.range(-0.35, 0.35);
        let len = r.range(18.0, 32.0);
        grow(r, &mut out, 0.0, 0.0, a, len, 1.0, 0.0, 0);
    }
    out
}

#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
fn grow(
    r: &mut Rng,
    out: &mut Vec<Frond>,
    mut x: f64,
    mut y: f64,
    mut a: f64,
    len: f64,
    w: f64,
    mut d: f64,
    depth: u32,
) {
    let mut pts = vec![(x, y, d)];
    let mut side = r.sign();
    let mut s = 0.0;
    while s < len {
        a += r.range(-0.3, 0.3);
        x += libm::cos(a) * 1.4;
        y += libm::sin(a) * 1.4;
        d += 1.4;
        pts.push((x, y, d));
        if depth < 2 && s > 2.0 && r.chance(0.24) {
            side = -side;
            let a1 = a + side * r.range(0.55, 1.05);
            let l1 = (len - s) * r.range(0.3, 0.55);
            grow(r, out, x, y, a1, l1, w * 0.6, d, depth + 1);
        }
        s += 1.4;
    }
    out.push(Frond { pts, w });
}

struct Jink {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    age: f64,
    next: f64,
    trail: Vec<(f64, f64)>,
}

struct State {
    c: [Rgba; 8],
    seed: f64,
    r: Rng,
    ps: Particles,
    lights: Lights,
    tree: Vec<Frond>,
    y0: f64,
    kinked: Vec<Jink>,
    t: f64,
    hit: bool,
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let c = &self.c;
        let r = &mut self.r;
        if !self.hit && self.t >= SNAP {
            self.hit = true;
            self.lights.add(0.0, 0.0, 60.0, c[4], 0.7, 0.3);
            self.lights.add(0.0, 0.0, 14.0, WHITE, 0.5, 0.07);
            for _ in 0..3 {
                let x = r.range(-3.0, 5.0);
                let y = r.range(-4.0, 2.0);
                let vx = r.range(-5.0, 8.0);
                let vy = -r.range(8.0, 16.0);
                let life = r.range(0.55, 0.75);
                let col = mix(ASH, c[4], 0.3);
                let size = r.range(2.5, 3.8);
                let delay = r.range(0.08, 0.16);
                self.ps.spawn(
                    r,
                    Spawn {
                        kind: Kind::Smoke,
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
        }
        for s in &mut self.kinked {
            s.age += dt;
            if s.age < 0.0 || s.age >= s.life {
                continue;
            }
            s.next -= dt;
            if s.next <= 0.0 {
                s.next = r.range(0.025, 0.055);
                let sp = libm::hypot(s.vx, s.vy);
                let a = libm::atan2(s.vy, s.vx) + r.sign() * r.range(0.6, 1.3);
                s.vx = libm::cos(a) * sp;
                s.vy = libm::sin(a) * sp;
                s.trail.push((s.x, s.y));
                if s.trail.len() > 4 {
                    s.trail.remove(0);
                }
            }
            s.vx -= s.vx * 3.0 * dt;
            s.vy -= s.vy * 3.0 * dt - 140.0 * dt;
            s.x += s.vx * dt;
            s.y += s.vy * dt;
        }
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

impl State {
    fn draw_back(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        // the fern burns outward from the hit, glowing white, cooling through the palette, then fading
        if t < SNAP {
            return;
        }
        // it cools by stepping down the palette, and only the last embers dither away
        let cool = smooth(0.45, 0.85, t);
        let fade = 1.0 - smooth(0.78, 0.88, t);
        for br in &self.tree {
            for i in 1..br.pts.len() {
                let (x0, y0, _) = br.pts[i - 1];
                let (x, y, d) = br.pts[i];
                let age = t - SNAP - d / 260.0;
                if age < 0.0 {
                    break;
                }
                let k = libm::exp(-age / 0.22) - cool * 0.3;
                let col = if k > 0.75 {
                    WHITE
                } else if k > 0.45 {
                    c[6]
                } else if k > 0.25 {
                    c[5]
                } else if k > 0.1 {
                    c[4]
                } else if k > -0.05 {
                    c[2]
                } else {
                    c[1]
                };
                p.line(x0, y0, x, y, col, fade);
                if br.w >= 1.0 && d < 9.0 {
                    p.line(
                        x0 + 1.0,
                        y0,
                        x + 1.0,
                        y,
                        if k > 0.45 { c[5] } else { c[2] },
                        fade,
                    );
                }
                if i & 3 == 0 && k > 0.0 {
                    p.glow(x, y, 5.0, c[5], 0.35 * k * fade);
                }
            }
        }
        self.ps.draw(p, Layer::Back);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (c, t, seed) = (&self.c, self.t, self.seed);
        let f = libm::floor(t * 30.0 + 1e-4);
        // the arc snaps in from the left in two frames, holds and flickers, then lets go
        if t < 0.32 {
            let mut r = rng(&[seed, 2.0, f]);
            let b = zigzag(&mut r, -66.0, self.y0, 0.0, 0.0, 0.22, 0.35);
            let u = clamp(t / SNAP, 0.0, 1.0);
            let k = if t < SNAP {
                1.0
            } else if t < 0.2 {
                if f % 2.0 != 0.0 { 0.7 } else { 1.0 }
            } else {
                0.55 * (1.0 - (t - 0.2) / 0.12)
            };
            let trunk;
            let shown = if u < 1.0 {
                trunk = Bolt {
                    paths: b.paths.first().cloned().into_iter().collect(),
                };
                &trunk
            } else {
                &b
            };
            stroke(
                p,
                shown,
                k,
                c,
                if t < SNAP + 0.034 { 0.95 } else { 0.7 },
                StrokeOpts {
                    up_to: Some(u),
                    hero: (SNAP..SNAP + 0.034).contains(&t),
                    ..StrokeOpts::default()
                },
            );
            if u < 1.0
                && let Some(tp) = b.paths.first().map(|q| &q.pts)
                && let Some(&(hx, hy)) = tp.get(libm::floor(u * (tp.len() as f64 - 1.0)) as usize)
            {
                p.circle(hx, hy, 2.0, WHITE, 1.0);
                p.glow(hx, hy, 14.0, c[5], 0.6);
            }
        }
        // the hit point overloads: a tight knot of tiny zigzags re-struck every frame round a white heart
        if (SNAP..0.45).contains(&t) {
            let a = (t - SNAP) / 0.4;
            let mut r = rng(&[seed, 5.0, f]);
            let r0 = 8.0 * (1.0 - a * 0.6);
            let m = js_round(11.0 * (1.0 - a * 0.5));
            let mut i = 0.0;
            while i < m {
                let ang = r.range(0.0, TAU);
                let d0 = r.range(0.5, r0 * 0.5);
                let ang2 = ang + r.range(-1.2, 1.2);
                let d1 = r.range(r0 * 0.55, r0);
                let bolt = zigzag(
                    &mut r,
                    libm::cos(ang) * d0,
                    libm::sin(ang) * d0,
                    libm::cos(ang2) * d1,
                    libm::sin(ang2) * d1,
                    0.5,
                    0.0,
                );
                crackle(p, &bolt, 0.75 * (1.0 - a) + 0.25, c, 0.25, 1.0);
                i += 1.0;
            }
            p.circle(0.0, 0.0, 3.0 * (1.0 - a) + 1.0, c[5], 1.0);
            p.circle(0.0, 0.0, 2.0 * (1.0 - a) + 0.5, WHITE, 1.0);
            p.glow(0.0, 0.0, 20.0, c[4], 0.55 * (1.0 - a));
            if t < SNAP + 0.07 {
                p.glow(0.0, 0.0, 9.0, WHITE, 0.4);
            }
        }
        // the sparks: a bright head, the zigzag behind it cooling
        for s in &self.kinked {
            if s.age < 0.0 || s.age >= s.life {
                continue;
            }
            let k = 1.0 - s.age / s.life;
            let mut path = s.trail.clone();
            path.push((s.x, s.y));
            let n = path.len();
            for i in 0..n - 1 {
                let col = if i == n - 2 {
                    if k > 0.5 { WHITE } else { c[6] }
                } else if k > 0.5 {
                    c[5]
                } else {
                    c[4]
                };
                p.line(
                    path[i].0,
                    path[i].1,
                    path[i + 1].0,
                    path[i + 1].1,
                    col,
                    0.6 + 0.4 * (i as f64 / n as f64),
                );
            }
            p.px(s.x, s.y, if k > 0.4 { WHITE } else { c[5] });
            p.glow(s.x, s.y, 4.0, c[5], 0.4 * k);
        }
        // the light strobes as it dies
        if t >= SNAP {
            p.glow(
                0.0,
                0.0,
                40.0 * js_max(0.0, 1.0 - t / 0.85),
                c[3],
                0.45 * libm::exp(-(t - SNAP) * 5.0) * if f % 2.0 != 0.0 { 0.7 } else { 1.0 },
            );
        }
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
