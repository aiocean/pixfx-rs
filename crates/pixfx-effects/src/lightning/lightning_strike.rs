//! Lightning strike: port of pixelfx `src/fx/lightning/lightning-strike.js`.
//!
//! Beyond pixelfx: it fires impact cues ([`pixfx_core::cue::impact`]) at each stroke for the game to answer; the
//! `feel` option scales them, 0 for none.
//!
//! The stepped leader, the stroke flicker, the cooling channel and the scorch follow Sprite Fusion's open-source
//! lightning (https://www.spritefusion.com/pixel-art-vfx/lightning), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, sparkle};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{
    Rng, TAU, clamp, ease, js_max, js_min, js_round, rand, rng, smooth, to_uint32,
};

use super::lib::{
    Bolt, PLASMA, Path, SOOT, SOOT2, StrokeOpts, VOLT, bend, crackle, heat, scorch, stroke, trace,
    trace_all, zigzag,
};

pub struct LightningStrike;

pub static LIGHTNING_STRIKE: LightningStrike = LightningStrike;

const META: Meta = Meta {
    id: "lightning/lightning-strike",
    name: "Lightning strike",
    desc: "A targeted strike: a sigil charges on the floor, the scene darkens, a thick bolt slams into it, arcs race out along the cracks it splits, the channel crumbles to a scorch.",
    tags: &[
        "lightning",
        "electric",
        "strike",
        "spell",
        "target",
        "impact",
        "ground",
        "one-shot",
    ],
    size: (240, 264),
    origin: (120, 228),
    duration: 3.0,
    looping: false,
    fps: 30,
    seed: 5,
    state: None,
    options: &[SPARKS.spec(), FEEL.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);

const PI: f64 = core::f64::consts::PI;
const W: f64 = 120.0;
const TOP: f64 = -224.0;
const RX: f64 = 28.0;
const SY: f64 = 0.22;
const REACH: f64 = 52.0;
const LEAD0: f64 = 0.48;
const STEPS: f64 = 9.0;
const MEET: f64 = -30.0;
/// The strokes: when each lands and how strong it is.
const STROKES: [(f64, f64); 3] = [(0.76, 1.0), (1.04, 0.8), (1.26, 0.55)];
const FLICK: [f64; 7] = [1.0, 0.72, 0.3, 0.95, 0.5, 0.8, 0.22];
const NIGHT: Rgba = Rgba::hex24(0x04030a);
const DUST: Rgba = Rgba::hex24(0x6e6382);
const ASH: Rgba = Rgba::hex24(0x5b5566);
const STONE: [Rgba; 3] = [
    Rgba::hex24(0x655b7a),
    Rgba::hex24(0x4a4263),
    Rgba::hex24(0x342d45),
];
const S1: f64 = STROKES[0].0;

fn level(a: f64) -> f64 {
    if a < 0.0 {
        return 0.0;
    }
    FLICK
        .get(libm::floor(a * 30.0 + 1e-4) as usize)
        .copied()
        .unwrap_or(0.0)
}

impl Effect for LightningStrike {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: each stroke asks the game for an impact's answer, weighed by its strength.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        STROKES
            .iter()
            .flat_map(|&(t, k)| impact(t, k, 0.0, -1.0, opts))
            .collect()
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State::new(opts, seed))
    }
}

/// A crack in the floor: points (x, y, distance along) and its weight.
struct Crack {
    pts: Vec<(f64, f64, f64)>,
    w: f64,
}

/// The cracks the strike splits in the floor: Lichtenberg branches lying on the floor's face.
fn cracks(r: &mut Rng) -> Vec<Crack> {
    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    fn grow(
        r: &mut Rng,
        out: &mut Vec<Crack>,
        mut x: f64,
        mut y: f64,
        mut a: f64,
        len: f64,
        w: f64,
        mut d: f64,
        depth: u32,
    ) {
        let mut pts = vec![(x, y, d)];
        let mut s = 0.0;
        while s < len {
            a += r.range(-0.32, 0.32);
            x += libm::cos(a) * 1.5;
            y = clamp(y + libm::sin(a) * 0.45, 0.0, 5.0);
            d += 1.5;
            pts.push((x, y, d));
            if depth < 2 && s > 3.0 && r.chance(0.09) {
                let na = a + r.sign() * r.range(0.5, 1.2);
                let nl = (len - s) * r.range(0.3, 0.6);
                grow(r, out, x, y, na, nl, w * 0.6, d, depth + 1);
            }
            s += 1.5;
        }
        out.push(Crack { pts, w });
    }
    let mut out = Vec::new();
    for i in 0..6 {
        let x = r.range(-1.0, 1.0);
        let y = r.range(0.0, 1.0);
        let a = if i % 2 != 0 {
            r.range(-0.5, 0.7)
        } else {
            PI + r.range(-0.7, 0.5)
        };
        let len = REACH * r.range(0.55, 1.0);
        grow(r, &mut out, x, y, a, len, 1.0, 0.0, 0);
    }
    out
}

/// The sigil on the floor: a ring of dashes turning slowly, crosshair ticks, an inner ring and a heart; drawn in
/// from the front, charging brighter; `near` picks the near (in front) or far (behind) part.
fn sigil(p: &mut Painter, t: f64, c: &[Rgba; 8], k: f64, near: bool) {
    let reveal = clamp(t / 0.22, 0.0, 1.0);
    let spin = t * 0.8;
    let burnt = if t > S1 { t - S1 } else { -1.0 };
    if burnt > 0.45 {
        return;
    }
    let a0 = if burnt < 0.0 || burnt < 0.07 {
        1.0
    } else {
        1.0 - (burnt - 0.07) / 0.38
    };
    let hot = (0.0..0.07).contains(&burnt);
    let on = |a: f64| (libm::sin(a) >= 0.0) == near;
    let from = |a: f64| libm::fabs((((a - PI / 2.0) % TAU) + TAU + PI) % TAU - PI) / PI;
    for i in 0..22 {
        let a = (f64::from(i) / 22.0) * TAU + spin;
        if !on(a) || from(a) > reveal {
            continue;
        }
        for da in [0.0, 0.06, 0.12] {
            let x = libm::cos(a + da) * RX;
            let y = libm::sin(a + da) * RX * SY;
            let col = if hot {
                WHITE
            } else if near {
                if k > 0.7 { c[6] } else { c[5] }
            } else if k > 0.7 {
                c[4]
            } else {
                c[3]
            };
            p.dot(x, y, col, a0);
            if near {
                p.dot(x, y + 1.0, if hot { c[6] } else { c[2] }, a0);
            }
        }
        if i % 3 == 0 {
            p.glow(
                libm::cos(a) * RX,
                libm::sin(a) * RX * SY,
                5.0,
                c[4],
                0.3 * k * a0,
            );
        }
    }
    for j in 0..4 {
        let a = spin * -0.5 + (f64::from(j) * TAU) / 4.0 + PI / 4.0;
        if !on(a) || reveal < 1.0 {
            continue;
        }
        p.line(
            libm::cos(a) * (RX - 5.0),
            libm::sin(a) * (RX - 5.0) * SY,
            libm::cos(a) * (RX + 5.0),
            libm::sin(a) * (RX + 5.0) * SY,
            if hot {
                WHITE
            } else if near {
                c[6]
            } else {
                c[4]
            },
            a0,
        );
    }
    if t > 0.3 {
        let mut a = 0.0;
        while a < TAU {
            if on(a) {
                let col = if hot {
                    WHITE
                } else if near {
                    c[4]
                } else {
                    c[2]
                };
                p.dot(
                    libm::cos(a) * 14.0,
                    libm::sin(a) * 14.0 * SY,
                    col,
                    a0 * clamp((t - 0.3) / 0.12, 0.0, 1.0),
                );
            }
            a += 0.09;
        }
    }
    if near && burnt < 0.0 {
        let pulse = 0.6 + 0.4 * libm::sin(t * (10.0 + 30.0 * k));
        p.px(0.0, 0.0, if k > 0.5 { WHITE } else { c[6] });
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, -1.0)] {
            p.px(dx, dy, c[5]);
        }
        p.glow(0.0, -1.0, 10.0 + 10.0 * k, c[5], 0.3 + 0.4 * k * pulse);
    }
}

/// Keep forks on the canvas and above the floor.
fn keep(b: Bolt) -> Bolt {
    let mut paths = Vec::new();
    for q in b.paths {
        let mut pts = Vec::new();
        for &pt in &q.pts {
            if libm::fabs(pt.0) > W - 5.0 || pt.1 > -3.0 || pt.1 < TOP + 6.0 {
                break;
            }
            pts.push(pt);
        }
        if q.w >= 1.0 || pts.len() > 3 {
            let w = q.w;
            paths.push(Path {
                w,
                pts: if w >= 1.0 { q.pts } else { pts },
            });
        }
    }
    Bolt { paths }
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

/// The stroke now showing: its index, how long since it landed and its brightness.
struct Now {
    i: usize,
    a: f64,
    k: f64,
}

struct State {
    c: [Rgba; 8],
    seed: f64,
    r: Rng,
    ps: Particles,
    hot: Particles,
    lights: Lights,
    x0: f64,
    /// The main bolt and its two drifted channels, one per stroke.
    bolts: [Bolt; 3],
    tree: Vec<Crack>,
    /// When each piece of the spent channel crumbles.
    gone: Vec<f64>,
    t: f64,
    struck: i32,
}

impl State {
    fn new(opts: &Options, seed: u64) -> Self {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&VOLT));
        let sd = seed as f64;
        let mut r = Rng::new(to_uint32(sd));
        let ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: opts.ramp(&SPARKS),
            ..ParticlesOpts::default()
        });
        let hot = Particles::new(ParticlesOpts {
            ground: 0.0,
            ..ParticlesOpts::default()
        });
        let x0 = r.range(8.0, 18.0) * if r.chance(0.5) { 1.0 } else { -1.0 };
        let main = keep(zigzag(&mut r, x0, TOP, 0.0, -1.0, 0.17, 0.5));
        let trunk = Bolt {
            paths: vec![Path {
                pts: main
                    .paths
                    .first()
                    .map(|q| q.pts.clone())
                    .unwrap_or_default(),
                w: 1.0,
            }],
        };
        let drift = |dx: f64| move |x: f64, y: f64| (x + dx * libm::sin((PI * y) / TOP), y);
        let b1 = bend(&trunk, drift(1.5));
        let b2 = bend(&trunk, drift(3.0));
        let bolts = [main, b1, b2];
        let tree = cracks(&mut r);
        let last_t = STROKES[2].0;
        let gone = chan(&bolts, 2)
            .iter()
            .enumerate()
            .map(|(i, &(_, y))| {
                last_t + 0.25 + 0.75 * (0.55 * rand(&[sd, i as f64, 5.0]) + 0.45 * (1.0 - y / TOP))
            })
            .collect();
        Self {
            c,
            seed: sd,
            r,
            ps,
            hot,
            lights: Lights::new(),
            x0,
            bolts,
            tree,
            gone,
            t: 0.0,
            struck: -1,
        }
    }

    fn now(&self) -> Option<Now> {
        let t = self.t;
        STROKES
            .iter()
            .enumerate()
            .rev()
            .find(|&(_, &(st, _))| t >= st)
            .map(|(i, &(st, sk))| Now {
                i,
                a: t - st,
                k: level(t - st) * sk,
            })
    }

    fn charge(&self) -> f64 {
        libm::pow(clamp(self.t / S1, 0.0, 1.0), 1.5)
    }

    fn heat_at(&self) -> f64 {
        if self.struck < 0 {
            return 0.0;
        }
        let mut m = f64::NEG_INFINITY;
        for &(st, sk) in STROKES.iter().take(self.struck as usize + 1) {
            m = js_max(
                m,
                sk * libm::pow(clamp(1.0 - (self.t - st) / 1.6, 0.0, 1.0), 1.4),
            );
        }
        m
    }

    fn draw_back(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        // the scene darkens a beat before the strike: a soft veil, deepest round the bolt's path, gone at the flash
        let dark = 0.5 * smooth(0.46, 0.66, t) * (1.0 - smooth(S1 - 0.01, S1 + 0.02, t));
        if dark > 0.01 {
            for y in -228..36 {
                for x in -120..120 {
                    let (x, y) = (f64::from(x), f64::from(y));
                    let d = libm::hypot(x / W, (y + 96.0) / 132.0);
                    let a = dark * (1.0 - smooth(0.55, 1.0, d));
                    if a > 0.01 {
                        p.blend(x + 0.5, y + 0.5, NIGHT, a);
                    }
                }
            }
        }
        sigil(p, t, c, self.charge(), false);
        // the cracks: arcs race out along them white, they glow with heat, cool to soot and fade
        if self.struck >= 0
            && let Some(s) = self.now()
        {
            let front = REACH * ease::out_cubic(clamp((t - S1) / 0.35, 0.0, 1.0));
            let t0 = self.heat_at();
            let fade = 1.0 - smooth(2.3, 2.95, t);
            let elec = if s.a < 0.14 {
                STROKES[s.i].1 * (1.0 - s.a / 0.14)
            } else {
                0.0
            };
            for br in &self.tree {
                for &(x, y, d) in &br.pts {
                    if d > front {
                        break;
                    }
                    let q = d / REACH;
                    let n = rand(&[js_round(x * 2.0), js_round(y * 2.0), 9.0]);
                    let fresh = d > front - 6.0 && front < REACH;
                    let tt = t0 * (1.15 - q) * (if br.w < 1.0 { 0.8 } else { 1.0 }) - n * 0.25;
                    let col = if fresh || (elec > 0.3 && n < 0.85) {
                        if q < 0.5 { WHITE } else { c[6] }
                    } else {
                        heat(tt).unwrap_or(SOOT2)
                    };
                    p.dot(x, y + 1.0, SOOT, fade);
                    p.dot(x, y, col, fade);
                    if br.w >= 1.0 && d < 9.0 {
                        p.dot(x, y + 2.0, SOOT, fade);
                    }
                    let every4 = (js_round(d) as i64) & 3 == 0;
                    if (fresh || elec > 0.3) && every4 {
                        p.glow(x, y, 7.0, c[5], 0.4);
                    } else if tt > 0.2 && every4 {
                        p.glow(x, y, 5.0, col, 0.2 * tt);
                    }
                }
            }
            scorch(p, 0.0, 0.0, 12.0, t0, fade);
            if t0 > 0.0
                && let Some(h) = heat(0.55)
            {
                p.glow(0.0, -2.0, 34.0, h, 0.3 * t0);
            }
        }
        self.ps.draw(p, Layer::Back);
        self.hot.draw(p, Layer::Back);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (c, t, seed) = (&self.c, self.t, self.seed);
        let s = self.now();
        let f = libm::floor(t * 30.0 + 1e-4);
        let k = self.charge();
        sigil(p, t, c, k, true);
        // static rising off the charging sigil, more and more of it
        if t > 0.15 && t < S1 {
            let mut r = rng(&[seed, 61.0, f]);
            let mut i = 0.0;
            while i < js_round(1.0 + 4.0 * k) {
                let a = r.range(0.0, TAU);
                let x = libm::cos(a) * RX;
                let y = libm::sin(a) * RX * SY;
                let x1 = x + r.range(-2.5, 2.5);
                let y1 = y - r.range(4.0, 9.0) * (0.6 + k);
                let b = zigzag(&mut r, x, y, x1, y1, 0.45, 0.3);
                crackle(p, &b, 0.45 + 0.4 * k, c, 0.3, 1.0);
                i += 1.0;
            }
            p.glow(0.0, -2.0, 36.0, c[4], 0.12 + 0.3 * k);
        }
        // the stepped leader feels down from above; a streamer climbs out of the sigil to meet it
        if (LEAD0..S1).contains(&t) {
            let u = clamp((t - LEAD0) / (S1 - LEAD0), 0.0, 1.0);
            let st = js_min(STEPS, libm::floor(u * STEPS) + 1.0);
            let fresh = 1.0 - ((u * STEPS) % 1.0);
            let reach = TOP + (MEET - TOP) * libm::pow(st / STEPS, 1.2);
            let prev = TOP + (MEET - TOP) * libm::pow((st - 1.0) / STEPS, 1.2);
            for q in &self.bolts[0].paths {
                let tk = q.w >= 1.0;
                for i in 0..q.pts.len().saturating_sub(1) {
                    let ((ax, ay), (bx, by)) = (q.pts[i], q.pts[i + 1]);
                    if by > reach {
                        if ay <= reach && i > 0 {
                            p.px(ax, ay, WHITE);
                            p.glow(ax, ay, 7.0, c[5], 0.5 * fresh + 0.15);
                        }
                        break;
                    }
                    let young = by > prev;
                    let col = if young {
                        if fresh > 0.4 {
                            if tk { c[6] } else { c[5] }
                        } else {
                            c[4]
                        }
                    } else if tk {
                        c[3]
                    } else {
                        c[2]
                    };
                    p.line(ax, ay, bx, by, col, 1.0);
                    if i % 4 == 0 && ay > TOP + 14.0 {
                        p.glow(
                            ax,
                            ay,
                            if young { 8.0 } else { 5.0 },
                            c[4],
                            if young { 0.3 * fresh + 0.1 } else { 0.12 },
                        );
                    }
                }
            }
            if t > 0.62 {
                let h = -MEET * ease::out_quad((t - 0.62) / (S1 - 0.62));
                let mut r = rng(&[seed, 77.0, f]);
                let x1 = r.range(-2.0, 2.0);
                let b = zigzag(&mut r, 0.0, -1.0, x1, -js_max(3.0, h), 0.3, 0.3);
                crackle(p, &b, 0.85, c, 0.45, 1.0);
                if let Some(&(tx, ty)) = b.paths.first().and_then(|q| q.pts.last()) {
                    p.px(tx, ty, WHITE);
                    p.glow(tx, ty, 9.0, c[5], 0.5);
                }
            }
        }
        // the strokes, the dart that runs down the channel before each flicker, the channel cooling and crumbling
        let next = STROKES.iter().position(|&(st, _)| st > t);
        let dart = next.filter(|&n| n != 0 && t > STROKES[n].0 - 0.06);
        if let Some(n) = dart {
            let tr = chan(&self.bolts, n);
            let u = (t - (STROKES[n].0 - 0.06)) / 0.06;
            let i1 = libm::floor(u * tr.len() as f64) as i64;
            trace_all(p, tr, c[3], 1.0);
            trace(p, tr, c[6], 1.0, i1 - 18, i1);
            let mut i = (i1 - 18).max(0);
            while i < (tr.len() as i64).min(i1) {
                let (x, y) = tr[i as usize];
                if y > TOP + 14.0 {
                    p.glow(x, y, 9.0, c[4], 0.35);
                }
                i += 4;
            }
        } else if let Some(s) = s.as_ref().filter(|s| s.k > 0.0) {
            let hero = s.i == 0 && s.a < 1.0 / 30.0;
            stroke(
                p,
                &self.bolts[s.i],
                s.k,
                c,
                if s.i == 0 { 1.5 } else { 1.2 },
                StrokeOpts {
                    hero,
                    core: if s.k > 0.6 {
                        if hero { 1.2 } else { 0.8 }
                    } else {
                        0.0
                    },
                    min_y: Some(TOP + 14.0),
                    ..StrokeOpts::default()
                },
            );
            let r = 4.5 * s.k;
            p.circle(0.0, -1.0, r + 1.5, c[2], 1.0);
            p.circle(0.0, -1.0, r + 0.5, c[5], 1.0);
            p.circle(0.0, -1.0, r * 0.65, WHITE, 1.0);
            if s.a < 0.07 {
                sparkle(
                    p,
                    0.0,
                    -2.0,
                    js_round(12.0 * s.k * (1.0 - s.a / 0.07)),
                    c[6],
                    c[4],
                    0.25,
                );
            }
            p.glow(0.0, -16.0, 96.0, c[3], 0.42 * s.k);
            p.glow(0.0, -3.0, 22.0, c[5], 0.3 * s.k);
            if hero {
                p.glow(self.x0 * 0.5, -110.0, 170.0, c[3], 0.3);
            }
        } else if let Some(s) = s.as_ref() {
            let last_t = STROKES[2].0;
            let crumble = s.i == STROKES.len() - 1;
            let cool = clamp((s.a - 0.23) / 0.45, 0.0, 1.0);
            let tr = chan(&self.bolts, s.i);
            for i in 0..tr.len().saturating_sub(1) {
                let g = self.gone.get(i).copied().unwrap_or(0.0);
                if crumble && t >= g {
                    continue;
                }
                let h = if crumble {
                    1.0 - clamp((t - last_t - 0.23) / (g - last_t - 0.23), 0.0, 1.0)
                } else {
                    1.0 - cool
                };
                let col = if h > 0.7 {
                    c[5]
                } else if h > 0.4 {
                    c[4]
                } else if h > 0.15 {
                    c[3]
                } else {
                    c[2]
                };
                let ((x0, y0), (x1, y1)) = (tr[i], tr[i + 1]);
                p.line(x0, y0, x1, y1, col, 1.0);
                if i % 6 == 0 && h > 0.3 && y0 > TOP + 14.0 {
                    p.glow(x0, y0, 6.0, c[4], 0.15 * h);
                }
            }
        }
        // arcs crawl out along the cracks as they split, and again with every flicker
        if let Some(s) = s.as_ref().filter(|s| s.a < 0.4) {
            let mut r = rng(&[seed, 91.0, f]);
            let run = if s.i == 0 {
                REACH * ease::out_cubic(clamp(s.a / 0.35, 0.0, 1.0))
            } else {
                REACH * clamp(s.a / 0.22, 0.0, 1.0)
            };
            for br in &self.tree {
                if br.w < 1.0 || r.next() < 0.3 {
                    continue;
                }
                let j1 = br
                    .pts
                    .iter()
                    .position(|&(_, _, d)| d > run)
                    .unwrap_or(br.pts.len());
                let j0 = js_max(0.0, j1 as f64 - r.int(7.0, 13.0)) as usize;
                let pts: Vec<(f64, f64)> = br.pts[j0..j1]
                    .iter()
                    .map(|&(x, y, _)| {
                        let nx = x + r.range(-0.6, 0.6);
                        (nx, y - 1.5 - r.range(0.0, 2.5))
                    })
                    .collect();
                if pts.len() > 1 {
                    let b = Bolt {
                        paths: vec![Path { pts, w: 1.0 }],
                    };
                    crackle(p, &b, 0.9 * (1.0 - s.a / 0.4) + 0.1, c, 0.35, 1.0);
                }
            }
        }
        self.ps.draw(p, Layer::Front);
        self.hot.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}

/// Stroke `i`'s channel: its bolt's trunk.
fn chan(bolts: &[Bolt; 3], i: usize) -> &[(f64, f64)] {
    bolts
        .get(i)
        .and_then(|b| b.paths.first())
        .map_or(&[], |q| q.pts.as_slice())
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let c = self.c;
        let r = &mut self.r;
        for (i, &(st, k)) in STROKES.iter().enumerate() {
            if self.struck >= i as i32 || t < st {
                continue;
            }
            self.struck = i as i32;
            self.lights.add(0.0, -50.0, 120.0, c[4], 0.55 * k, 0.4);
            self.lights.add(0.0, -3.0, 24.0, WHITE, 0.5 * k, 0.1);
            for _ in 0..js_round(44.0 * k) as i64 {
                let a = -PI / 2.0 + r.range(-1.3, 1.3);
                let v = r.range(110.0, 320.0) * (0.65 + 0.35 * k);
                let s = sp(
                    Kind::Spark,
                    r.range(-1.5, 1.5),
                    -2.0,
                    libm::cos(a) * v,
                    libm::sin(a) * v,
                    r.range(0.2, 0.55),
                    Some(WHITE),
                    1.0,
                    0.0,
                );
                self.ps.spawn(r, s);
            }
            for _ in 0..js_round(12.0 * k) as i64 {
                let a = -PI / 2.0 + r.range(-1.2, 1.2);
                let v = r.range(70.0, 220.0);
                let s = sp(
                    Kind::Spark,
                    r.range(-2.0, 2.0),
                    -1.0,
                    libm::cos(a) * v,
                    libm::sin(a) * v,
                    r.range(0.5, 1.0),
                    Some(WHITE),
                    1.0,
                    0.0,
                );
                self.hot.spawn(r, s);
            }
            for _ in 0..js_round(5.0 * k) as i64 {
                let s = sp(
                    Kind::Ember,
                    r.range(-4.0, 4.0),
                    -2.0,
                    r.range(-25.0, 25.0),
                    -r.range(30.0, 80.0),
                    r.range(0.8, 1.5),
                    Some(WHITE),
                    1.0,
                    0.0,
                );
                self.hot.spawn(r, s);
            }
            for _ in 0..js_round(18.0 * k) as i64 {
                let a = -PI / 2.0 + r.range(-1.1, 1.1);
                let v = r.range(50.0, 190.0);
                let s = sp(
                    Kind::Grit,
                    r.range(-3.0, 3.0),
                    -1.0,
                    libm::cos(a) * v,
                    libm::sin(a) * v,
                    r.range(1.0, 1.5),
                    r.pick(&STONE).copied(),
                    if r.chance(0.35) { 2.0 } else { 1.0 },
                    0.0,
                );
                self.ps.spawn(r, s);
            }
            for _ in 0..8 {
                let s = sp(
                    Kind::Dust,
                    r.range(-3.0, 3.0),
                    -1.0,
                    r.sign() * r.range(30.0, 110.0),
                    -r.range(4.0, 24.0),
                    r.range(0.4, 0.8),
                    Some(DUST),
                    r.range(1.5, 3.0),
                    0.0,
                );
                self.ps.spawn(r, s);
            }
            for _ in 0..js_round(4.0 * k) as i64 {
                let s = sp(
                    Kind::Smoke,
                    r.range(-8.0, 8.0),
                    -4.0,
                    r.range(-10.0, 10.0),
                    -r.range(10.0, 25.0),
                    r.range(1.1, 1.6),
                    Some(mix(ASH, c[2], 0.3)),
                    r.range(3.0, 6.0),
                    r.range(0.05, 0.25),
                );
                self.ps.spawn(r, s);
            }
        }
        if self.struck >= 0 && t < 1.7 && r.chance(6.0 * dt) {
            let s = sp(
                Kind::Smoke,
                r.range(-8.0, 8.0),
                -2.0,
                r.range(-4.0, 4.0),
                -r.range(9.0, 17.0),
                r.range(1.0, 1.3),
                None,
                r.range(2.0, 3.5),
                0.0,
            );
            self.ps.spawn(r, s);
        }
        let end = chan(&self.bolts, 2);
        for i in 0..self.gone.len().saturating_sub(1) {
            let g = self.gone[i];
            if g <= t && g > t - dt && rand(&[self.seed, i as f64, 6.0]) < 0.5 {
                let (x, y) = end[i];
                let s = sp(
                    Kind::Mote,
                    x,
                    y,
                    r.range(-8.0, 8.0),
                    r.range(6.0, 22.0),
                    r.range(0.4, 0.8),
                    Some(if r.chance(0.5) { c[5] } else { c[4] }),
                    r.range(0.5, 0.9),
                    0.0,
                );
                self.ps.spawn(r, s);
            }
        }
        self.ps.update(r, dt);
        self.hot.update(r, dt);
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
