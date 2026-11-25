//! Icicle fall: port of pixelfx `src/fx/ice/icicle-fall.js`.
//!
//! Beyond pixelfx: the icicle stabbing the floor fires impact cues ([`pixfx_core::cue::impact`]) for the game to
//! answer; the `feel` option scales them, 0 for none.
//!
//! The break recipe (a white frame, the ice flung apart as shards with ice dust and glints) is adapted from
//! Sprite Fusion's open-source ice (https://www.spritefusion.com/pixel-art-vfx/ice), MIT License,
//! Copyright (c) 2026 Hugo Duprez.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{
    BitKind, Bits, GRAVITY, Kind, Lights, Particles, ParticlesOpts, band, sparkle,
};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{
    Rng, TAU, clamp, ease, js_max, js_min, js_round, js_sign, rand, seg, smooth, to_uint32,
};

use super::lib::{
    PALETTE, Pane, PaneEvent, Panes, Pt, bit, centroid, emit, end_by, splinter, split_poly, step,
};

pub struct IcicleFall;

pub static ICICLE_FALL: IcicleFall = IcicleFall;

const META: Meta = Meta {
    id: "ice/icicle-fall",
    name: "Icicle fall",
    desc: "An icicle drips and trembles, snaps off its frosted root, drops, stabs the floor and bursts into skidding shards, splinters and rolling frost.",
    tags: &["ice", "icicle", "fall", "trap", "shatter", "one-shot"],
    size: (152, 120),
    origin: (76, 100),
    duration: D,
    looping: false,
    fps: 30,
    seed: 4,
    state: None,
    options: &[FEEL.spec()],
};

const D: f64 = 2.6;
/// Where it hangs, how long, how wide at the root.
const ROOT: f64 = -80.0;
const LEN: f64 = 36.0;
const W: f64 = 12.0;
const DRIP: f64 = 0.16;
const TREMBLE: f64 = 0.3;
const SNAP: f64 = 0.6;
const FREE: f64 = SNAP + 0.04;

/// When it hits the floor.
fn impact_t() -> f64 {
    FREE + libm::sqrt((2.0 * (-ROOT - LEN)) / GRAVITY)
}

impl Effect for IcicleFall {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the icicle stabbing the floor asks the game for an impact's answer.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(impact_t(), 0.5, 0.5, -3.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        let s = seed as f64;
        // the rime it hangs from: a frosted crust with stubs of older icicles
        let rime = (-18..=18)
            .map(|xi| {
                let x = f64::from(xi);
                let h = rand(&[x, s, 2.0]);
                let d = libm::fabs(x) / 18.0;
                Rime {
                    x,
                    h: 1.0 + js_round((0.6 + h * 1.8) * (1.0 - d * 0.7)),
                    stub: if h < 0.3 && libm::fabs(x) > 7.0 {
                        2.0 + js_round(rand(&[x, 5.0]) * 5.0 * (1.0 - d * 0.5))
                    } else {
                        0.0
                    },
                }
            })
            .collect();
        Box::new(State {
            c,
            r: Rng::new(to_uint32(s)),
            ps: Particles::new(ParticlesOpts {
                ground: 0.0,
                sparks: vec![c[7], c[6], c[5], c[4]],
                haze: Some(c[5]),
            }),
            bits: Bits::new(0.0),
            lights: Lights::new(),
            panes: Panes::new(0.0),
            rime,
            impact: impact_t(),
            t: 0.0,
            snapped: false,
            landed: false,
            dripped: false,
        })
    }
}

struct Rime {
    x: f64,
    h: f64,
    stub: f64,
}

struct State {
    c: [Rgba; 8],
    r: Rng,
    ps: Particles,
    bits: Bits,
    lights: Lights,
    panes: Panes,
    rime: Vec<Rime>,
    impact: f64,
    t: f64,
    snapped: bool,
    landed: bool,
    dripped: bool,
}

/// The icicle's top: hanging, then falling free.
fn top(t: f64) -> f64 {
    ROOT + if t < FREE {
        0.0
    } else {
        0.5 * GRAVITY * libm::pow(t - FREE, 2.0)
    }
}

/// The icicle: a rippled taper hanging from (x, top), lit down its left side with a bright core streak, rings of
/// light at the ripples, deep on the right, a dark rim. `white` flashes it.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
fn icicle(
    p: &mut Painter,
    c: &[Rgba; 8],
    x: f64,
    top: f64,
    len: f64,
    w: f64,
    white: bool,
    glint: f64,
) {
    let hw = |u: f64| {
        (w / 2.0)
            * libm::pow(1.0 - u, 0.75)
            * (1.0 + 0.14 * libm::sin(u * 13.0 + 0.5))
            * (if u < 0.08 { 1.15 } else { 1.0 })
            + 0.3
    };
    let (tilt, cut) = (0.0, 0.0);
    p.shade(
        x - w / 2.0 - 4.0,
        top - 1.0,
        x + w / 2.0 + 4.0 + libm::fabs(tilt) * len,
        top + len + 1.0,
        |px, py, o| {
            let u = (py - top) / len;
            let cx = x + tilt * len * u + 0.7 * libm::sin(u * 6.0 + 1.0);
            let s = px - cx;
            if u < cut || u > 1.0 {
                return None;
            }
            let h = hw(u);
            if libm::fabs(s) > h + 1.0 {
                return None;
            }
            if libm::fabs(s) > h {
                o.c = c[1];
                return None;
            }
            if white {
                return Some(WHITE);
            }
            let q = s / h;
            let ring = libm::sin(u * 13.0 + 0.5) > 0.7;
            let mut col = if u > 0.88 {
                WHITE
            } else if q < -0.72 {
                c[6]
            } else if q < -0.2 {
                if ring { c[7] } else { c[5] }
            } else if q < 0.18 {
                if u < 0.72 { c[7] } else { c[6] }
            } else if q > 0.8 {
                c[2]
            } else if libm::fabs(q - 0.45) < 0.1 && u > 0.15 && u < 0.6 {
                c[5]
            } else if q < 0.6 {
                c[4]
            } else {
                c[3]
            };
            if glint >= 0.0 && q < 0.25 && libm::fabs(u - glint) < 0.06 {
                col = WHITE;
            }
            Some(col)
        },
    );
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let (t, impact) = (self.t, self.impact);
        let State {
            c,
            r,
            ps,
            bits,
            lights,
            panes,
            snapped,
            landed,
            dripped,
            ..
        } = self;
        let c = *c;
        // a drop of meltwater swells at the tip and falls
        if !*dripped && t >= DRIP {
            *dripped = true;
            bit!(bits, BitKind::Drip;
                x = 0.5,
                y = ROOT + LEN + 1.0,
                vy = 20.0,
                g = 0.9,
                life = 1.2,
                c = c[6],
            );
        }
        // it trembles: frost sifts down from the root
        if t > TREMBLE && t < SNAP && r.chance(0.25) {
            emit!(
                ps,
                r,
                Kind::Flake,
                r.range(-6.0, 6.0),
                ROOT + r.range(0.0, 3.0),
                r.range(-8.0, 8.0),
                r.range(5.0, 20.0),
                r.range(0.6, 1.1),
                if r.chance(0.5) { WHITE } else { c[6] },
                r.range(0.5, 1.1)
            );
        }
        // it snaps: a crack flashes across the root, chips and a puff burst off
        if !*snapped && t >= SNAP {
            *snapped = true;
            lights.add(0.0, ROOT + 4.0, 24.0, WHITE, 0.6, 0.1);
            lights.add(0.0, ROOT + 4.0, 32.0, c[5], 0.4, 0.3);
            for _ in 0..7 {
                let a = r.range(0.0, TAU);
                let sp = r.range(30.0, 110.0);
                bit!(bits, BitKind::Shard;
                    x = r.range(-4.0, 4.0),
                    y = ROOT + 2.0,
                    vx = libm::cos(a) * sp,
                    vy = libm::sin(a) * sp - 20.0,
                    g = 0.8,
                    drag = 0.8,
                    bounce = 0.3,
                    life = r.range(0.5, 0.9),
                    a = r.range(0.0, TAU),
                    va = r.range(-20.0, 20.0),
                    s = r.range(1.0, 2.0),
                    c = step(&c, r.int(5.0, 7.0)),
                );
            }
            for _ in 0..3 {
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    r.range(-5.0, 5.0),
                    ROOT + 2.0,
                    r.range(-25.0, 25.0),
                    r.range(-5.0, 5.0),
                    r.range(0.4, 0.7),
                    c[6],
                    r.range(1.6, 2.6)
                );
            }
            for _ in 0..5 {
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    r.range(-6.0, 6.0),
                    ROOT + 2.0,
                    r.range(-30.0, 30.0),
                    r.range(-20.0, 20.0),
                    r.range(0.6, 1.2),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(0.5, 1.3)
                );
            }
        }
        // falling: it sheds frost motes that hang in the air behind it
        if t > FREE && t < impact && r.chance(0.5) {
            let tp = top(t);
            emit!(
                ps,
                r,
                Kind::Mote,
                r.range(-3.0, 3.0),
                tp + r.range(0.0, LEN * 0.6),
                r.range(-10.0, 10.0),
                r.range(-40.0, -10.0),
                r.range(0.2, 0.35),
                if r.chance(0.5) { c[6] } else { c[5] },
                0.6
            );
        }
        // it stabs the floor and bursts
        if !*landed && t >= impact {
            *landed = true;
            let tp = -LEN;
            lights.add(0.0, -16.0, 46.0, WHITE, 0.85, 0.12);
            lights.add(0.0, -16.0, 58.0, c[4], 0.5, 0.5);
            // the icicle in pieces: cut across its length, then down the middle
            let mut parts: Vec<Vec<Pt>> = vec![vec![(-W / 2.0, tp), (W / 2.0, tp), (0.5, 0.0)]];
            for v in [0.3, 0.58, 0.8] {
                let mut next = Vec::new();
                for poly in &parts {
                    let y = tp + LEN * v + r.range(-2.0, 2.0);
                    next.extend(split_poly(poly, 0.0, y, r.range(-0.4, 0.4)));
                }
                parts = next;
            }
            let mut next = Vec::new();
            for poly in parts {
                let m = centroid(&poly);
                if m.area > 22.0 {
                    next.extend(split_poly(
                        &poly,
                        m.x,
                        m.y,
                        core::f64::consts::FRAC_PI_2 + r.range(-0.5, 0.5),
                    ));
                } else {
                    next.push(poly);
                }
            }
            for part in next {
                let m = centroid(&part);
                let side = if m.x < 0.5 { -1.0 } else { 1.0 };
                let low = clamp(-m.y / LEN, 0.0, 1.0);
                let pts = part.iter().map(|&(x, y)| (x - m.x, y - m.y)).collect();
                let mut q = Pane::new(pts, m.x, m.y);
                q.vx = side * r.range(30.0, 110.0) + r.range(-15.0, 15.0);
                q.vy = -r.range(70.0, 200.0) * (0.4 + low);
                q.a = 0.0;
                q.va = r.range(-12.0, 12.0);
                q.f = r.range(-0.7, 0.7);
                q.vf = r.range(-8.0, 8.0);
                q.lit = 0.75;
                q.g = 0.9;
                q.drag = 0.6;
                q.bounce = 0.35;
                q.solid = true;
                q.ridge = Some(core::f64::consts::FRAC_PI_2);
                q.brittle = 2;
                q.crumble = Some(r.range(0.15, 0.45));
                q.life = r.range(1.0, 1.4);
                panes.add(q);
            }
            for _ in 0..52 {
                let a = -core::f64::consts::FRAC_PI_2 + r.range(-1.25, 1.25);
                let sp = r.range(60.0, 230.0);
                let kind = if r.chance(0.15) {
                    BitKind::Chunk
                } else {
                    BitKind::Shard
                };
                bit!(bits, kind;
                    x = r.range(-3.0, 3.0),
                    y = -r.range(1.0, 10.0),
                    vx = libm::cos(a) * sp,
                    vy = libm::sin(a) * sp,
                    g = 0.85,
                    drag = 0.6,
                    bounce = 0.32,
                    life = r.range(0.6, 1.2),
                    a = r.range(0.0, TAU),
                    va = r.range(-22.0, 22.0),
                    s = r.range(1.2, 3.0),
                    c = step(&c, r.int(4.0, 7.0)),
                    ramp = Some(vec![c[1], c[3], c[4], c[6], c[7]]),
                );
            }
            for _ in 0..10 {
                let s = r.sign();
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    s * r.range(0.0, 8.0),
                    -r.range(1.0, 6.0),
                    s * r.range(25.0, 90.0),
                    -r.range(2.0, 14.0),
                    r.range(0.6, 1.1),
                    c[6],
                    r.range(2.2, 3.8)
                );
            }
            for _ in 0..12 {
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    r.range(-6.0, 6.0),
                    -r.range(2.0, 16.0),
                    r.range(-70.0, 70.0),
                    -r.range(30.0, 110.0),
                    r.range(1.0, 1.5),
                    if r.chance(0.6) { WHITE } else { c[6] },
                    r.range(0.5, 1.5)
                );
            }
            for _ in 0..6 {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    r.range(-24.0, 24.0),
                    -r.range(4.0, 34.0),
                    r.range(-10.0, 10.0),
                    -r.range(4.0, 14.0),
                    r.range(0.4, 0.8),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(1.0, 2.2)
                );
            }
        }
        if *landed && t < impact + 0.9 && r.chance(0.08) && bits.count() > 0 {
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
                    r.range(0.6, 1.4)
                );
            }
        }
        ps.update(r, dt);
        bits.update(dt);
        panes.update(dt, |e| match e {
            PaneEvent::Land { vx, x, y, speed } => {
                if speed < 100.0 {
                    return;
                }
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    x,
                    y - 1.0,
                    r.range(-15.0, 15.0),
                    -r.range(2.0, 8.0),
                    r.range(0.4, 0.6),
                    c[6],
                    r.range(1.4, 2.2)
                );
                splinter(bits, r, &c, x, y, vx, 2);
            }
            PaneEvent::Crumble { x, vx } => splinter(bits, r, &c, x, 0.0, vx, 3),
            PaneEvent::Skid { .. } => {}
        });
        lights.update(dt);
        end_by(t, D, ps, Some(bits), Some(panes));
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let (c, t, impact) = (&self.c, self.t, self.impact);
        if layer != Layer::Front {
            // its shadow gathers on the floor as it comes down
            if t > TREMBLE && t < impact + 0.05 {
                let k = if t < FREE {
                    seg(t, TREMBLE, FREE) * 0.35
                } else {
                    0.35 + 0.65 * seg(t, FREE, impact)
                };
                let rx = 3.0 + 7.0 * k;
                let mut x = libm::floor(-rx);
                while x <= rx {
                    let f = 1.0 - libm::pow(x / rx, 2.0);
                    p.blend(x + 0.5, 0.0, c[0], 0.7 * k * f);
                    if libm::fabs(x) < rx * 0.7 {
                        p.blend(x + 0.5, 1.0, c[0], 0.45 * k * f);
                    }
                    x += 1.0;
                }
            }
            // frost splashed across the floor where it struck, melting away
            if t >= impact {
                let g = ease::out_cubic(seg(t, impact, impact + 0.25));
                let out = 1.0 - smooth(impact + 0.8, impact + 1.5, t);
                for xi in -26..=26 {
                    let x = f64::from(xi);
                    let d = libm::fabs(x + 0.5) / 26.0;
                    let h = rand(&[x, 23.0]);
                    let a = out * clamp((g * 1.15 - d) / 0.15, 0.0, 1.0);
                    if a <= 0.0 {
                        continue;
                    }
                    p.dot(
                        x,
                        0.0,
                        if h < 0.35 || d > g - 0.12 {
                            WHITE
                        } else {
                            c[6]
                        },
                        a,
                    );
                    if h < 0.4 {
                        let n = js_round((1.0 + rand(&[x, 3.0]) * 2.5) * (1.0 - d));
                        let mut j = 1.0;
                        while j <= n {
                            p.dot(
                                x + js_sign(x) * if j > 1.0 { 1.0 } else { 0.0 },
                                -j,
                                if j > 1.0 { WHITE } else { c[5] },
                                a,
                            );
                            j += 1.0;
                        }
                    }
                    let n = js_round((2.0 + rand(&[x, 7.0]) * 3.0) * (1.0 - d * 0.6));
                    let mut j = 1.0;
                    while j <= n {
                        p.dot(x, j, if j < 2.0 { c[5] } else { c[4] }, a * (1.1 - j / 6.0));
                        j += 1.0;
                    }
                }
                let k = seg(t, impact, impact + 0.35);
                if k < 1.0 {
                    band(
                        p,
                        0.0,
                        0.5,
                        6.0 + 34.0 * ease::out_cubic(k),
                        0.18,
                        js_max(1.0, 4.0 * (1.0 - k)),
                        1.1 - k,
                        &[c[3], c[4], c[5], c[6]],
                        |a| libm::sin(a) < 0.0,
                        false,
                    );
                }
            }
            self.ps.draw(p, Layer::Back);
        }
        if layer == Layer::Back {
            return;
        }
        // the rime it hangs from, fading at the end
        let ra = 1.0 - smooth(D - 0.7, D - 0.2, t);
        for rm in &self.rime {
            // a lumpy crust: white on top, pale below, a deep underside; little icicles hang off it
            let mut j = 0.0;
            while j < rm.h + 1.0 {
                let col = if j == rm.h {
                    WHITE
                } else if j == rm.h - 1.0 {
                    c[6]
                } else if j == 0.0 {
                    c[3]
                } else {
                    c[5]
                };
                p.dot(rm.x, ROOT - 1.0 - j, col, ra);
                j += 1.0;
            }
            let mut j = 0.0;
            while j < rm.stub {
                let col = if j == rm.stub - 1.0 {
                    WHITE
                } else if j == 0.0 {
                    c[5]
                } else {
                    c[4]
                };
                p.dot(rm.x, ROOT + j, col, ra);
                j += 1.0;
            }
            if rm.stub > 4.0 {
                p.dot(rm.x + 1.0, ROOT, c[3], ra);
            }
        }
        // the snapped root: a jagged stub left behind
        if self.snapped {
            let mut x = -W / 2.0 + 1.0;
            while x <= W / 2.0 - 1.0 {
                let l = 1.0 + js_round(rand(&[x, 77.0]) * 2.5);
                let mut j = 0.0;
                while j < l {
                    let col = if j == l - 1.0 {
                        if t < SNAP + 0.15 { WHITE } else { c[6] }
                    } else if x < 0.0 {
                        c[5]
                    } else {
                        c[3]
                    };
                    p.dot(x, ROOT + j, col, ra);
                    j += 1.0;
                }
                x += 1.0;
            }
        }
        p.glow(0.0, ROOT, 18.0, c[4], 0.3 * ra);
        if t >= impact {
            let k = seg(t, impact, impact + 0.35);
            if k < 1.0 {
                band(
                    p,
                    0.0,
                    0.5,
                    6.0 + 34.0 * ease::out_cubic(k),
                    0.18,
                    js_max(1.0, 4.0 * (1.0 - k)),
                    1.1 - k,
                    &[c[3], c[4], c[5], c[6]],
                    |a| libm::sin(a) >= 0.0,
                    k < 0.15,
                );
            }
            if t < impact + 0.22 {
                sparkle(
                    p,
                    0.5,
                    -3.0,
                    10.0 * ease::out_back(js_min(1.0, (t - impact) / 0.05), ease::BACK_S)
                        * (1.0 - (t - impact) / 0.22),
                    c[6],
                    c[4],
                    0.7,
                );
            }
        }
        if t < impact {
            // the icicle: trembling before it snaps, then falling with a streak of air behind it
            let tp = top(t);
            let jx = if t > TREMBLE && t < SNAP {
                (if libm::floor(t * 36.0) % 2.0 != 0.0 {
                    1.0
                } else {
                    0.0
                }) * js_round(seg(t, TREMBLE, SNAP) * 1.4 + 0.4)
            } else {
                0.0
            };
            let falling = t >= FREE;
            let sp = if falling { GRAVITY * (t - FREE) } else { 0.0 };
            if falling {
                for (dx, k) in [(-3.0, 0.7), (0.5, 1.0), (4.0, 0.6)] {
                    let l = js_min(30.0, sp * 0.07) * k;
                    if l > 2.0 {
                        p.line(
                            dx,
                            tp - l,
                            dx,
                            tp + 3.0,
                            |u: f64| {
                                if u > 0.75 {
                                    c[6]
                                } else if u > 0.4 {
                                    c[4]
                                } else {
                                    c[3]
                                }
                            },
                            1.0,
                        );
                    }
                }
            }
            let gl = (t - 0.04) / 0.3;
            icicle(
                p,
                c,
                jx + 0.5,
                tp,
                LEN,
                W,
                t >= impact - 0.025 || (SNAP..SNAP + 0.03).contains(&t),
                if (0.0..=1.0).contains(&gl) { gl } else { -1.0 },
            );
            if self.snapped && t < SNAP + 0.12 {
                p.line(
                    -W / 2.0 - 1.0,
                    ROOT + 1.0,
                    W / 2.0 + 1.0,
                    ROOT + 1.0,
                    WHITE,
                    1.0,
                );
            }
            p.glow(0.5, tp + LEN * 0.5, 20.0, c[4], 0.28);
            // the drop of meltwater swelling at the tip
            if t < DRIP {
                let s = seg(t, 0.0, DRIP);
                p.dot(0.5, ROOT + LEN + 1.0, c[6], s);
                if s > 0.6 {
                    p.px(0.5, ROOT + LEN + 2.0, WHITE);
                }
            }
        }
        self.panes.draw(p, c);
        self.bits.draw(p);
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
