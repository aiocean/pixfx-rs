//! Hit: made here (neither pixelfx nor Sprite Fusion has a blow landing on a body). The spark where a blow lands:
//! a white-hot star stretched along the blow, held as it is through a hit-stop, then a cut flashing across the
//! body, sparks thrown the way the blow pushes, and on a heavy blow a ring that breaks into arcs.
//!
//! It fires impact cues ([`pixfx_core::cue::impact`]) weighed by `power`; the `feel` option scales them, 0 for none.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Kind, Lights, Particles, ParticlesOpts, Spawn};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_max, smooth, to_uint32};

use super::lib::{Seg, Spike, broken_ring, star};

pub struct Hit;

pub static HIT: Hit = Hit;

const META: Meta = Meta {
    id: "impacts/hit",
    name: "Hit",
    desc: "A blow lands: a white-hot star stretched along the blow, a cut flashing across, sparks thrown the way it pushes, and on a heavy blow a ring breaking into arcs.",
    tags: &[
        "impact", "hit", "strike", "blow", "sparks", "melee", "one-shot",
    ],
    size: (112, 80),
    origin: (44, 40),
    duration: 0.45,
    looping: false,
    fps: 30,
    seed: 5,
    state: None,
    options: &[POWER.spec(), FEEL.spec()],
};

// meta.options: metal struck, dark to white
const PALETTE: [Rgba; 7] = [
    Rgba::hex24(0x2a1a08),
    Rgba::hex24(0x6a4410),
    Rgba::hex24(0xb07a1a),
    Rgba::hex24(0xe8b030),
    Rgba::hex24(0xffe066),
    Rgba::hex24(0xfff6c4),
    Rgba::hex24(0xffffff),
];
/// How heavy the blow: 0 a jab, 1 a blow that throws the body.
const POWER: NumOpt = NumOpt::new("power", 0.5).within(0.0, 1.0);

/// The star holds white this long: the frame a hit-stop freezes on.
const HOT: f64 = 0.034;
/// The cut flashes across in this long, and is gone by `CUT * 3`.
const CUT: f64 = 0.05;
/// The cut's slant: rising the way the blow goes.
const SLANT: f64 = -0.55;

impl Effect for Hit {
    fn meta(&self) -> &Meta {
        &META
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(0.0, opts.num(&POWER), 0.0, 0.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(WHITE);
        let w = clamp(opts.num(&POWER), 0.0, 1.0);
        // everything grows with the blow: a jab's star about a head and a half long, a throw's two and a half
        let s = 0.75 + 0.6 * w;
        let mut r = Rng::new(to_uint32(seed as f64));
        let mut ps = Particles::new(ParticlesOpts {
            sparks: vec![c(6), c(5), c(4), c(2)],
            ..ParticlesOpts::default()
        });
        let mut lights = Lights::new();
        // the star: long along the blow (longest ahead of it), short across, shorter still between
        let mut sp = Vec::with_capacity(8);
        for i in 0..8 {
            let a = f64::from(i) * core::f64::consts::PI / 4.0 + r.range(-0.05, 0.05);
            let len = match i {
                0 => r.range(17.0, 20.0),
                4 => r.range(9.0, 11.0),
                2 | 6 => r.range(7.0, 9.0),
                _ => r.range(3.5, 5.0),
            } * s;
            let w = if i % 2 == 1 { 1.0 } else { 2.0 };
            sp.push(Spike { a, len, w });
        }
        let mut segs = Vec::with_capacity(5);
        for i in 0..5 {
            let a = (f64::from(i) / 5.0) * TAU + r.range(-0.15, 0.15);
            segs.push(Seg {
                a,
                gap: r.range(0.5, 0.75),
                dr: r.range(0.1, 0.4),
                w: 1.0,
            });
        }
        lights.add(0.0, 0.0, 34.0 + 20.0 * w, c(4), 0.55, 0.25);
        lights.add(0.0, 0.0, 14.0, WHITE, 0.7, 0.06);
        // sparks thrown the way the blow pushes, a few kicked back off the body
        let n = 6 + (6.0 * w) as usize;
        for i in 0..n {
            let back = i % 5 == 4;
            let a = if back {
                core::f64::consts::PI + r.range(-0.7, 0.7)
            } else {
                r.range(-0.75, 0.55)
            };
            let v = r.range(150.0, 260.0 + 140.0 * w) * if back { 0.6 } else { 1.0 };
            let life = r.range(0.16, 0.3 + 0.1 * w);
            let delay = if i > n / 2 { r.range(0.0, 0.03) } else { 0.0 };
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Spark,
                    x: libm::cos(a) * 2.0,
                    y: libm::sin(a) * 2.0,
                    vx: libm::cos(a) * v,
                    vy: libm::sin(a) * v,
                    life,
                    col: Some(WHITE),
                    size: 1.0,
                    delay,
                    ..Spawn::new(Kind::Spark, 0.0, 0.0, 0.0, 0.0, 0.0)
                },
            );
        }
        for _ in 0..(2 + (3.0 * w) as usize) {
            let a = r.range(-1.2, 1.2);
            let v = r.range(60.0, 130.0);
            let col = r.pick(&[c(5), c(6)]).copied();
            let life = r.range(0.2, 0.35);
            let delay = r.range(0.0, 0.03);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Mote,
                    x: 0.0,
                    y: 0.0,
                    vx: libm::cos(a) * v,
                    vy: libm::sin(a) * v,
                    life,
                    col,
                    size: 0.6,
                    delay,
                    ..Spawn::new(Kind::Mote, 0.0, 0.0, 0.0, 0.0, 0.0)
                },
            );
        }
        Box::new(State {
            c: [c(0), c(1), c(2), c(3), c(4), c(5), c(6)],
            w,
            s,
            r,
            ps,
            lights,
            sp,
            segs,
            t: 0.0,
        })
    }
}

struct State {
    c: [Rgba; 7],
    w: f64,
    s: f64,
    r: Rng,
    ps: Particles,
    lights: Lights,
    sp: Vec<Spike>,
    segs: Vec<Seg>,
    t: f64,
}

impl State {
    /// The cut: a slanted crescent through the point struck, drawn from its back end to its front as it flashes
    /// across, thick in the middle and sharp at both ends, white at its edge, cooling as it thins away.
    fn cut(&self, p: &mut Painter, t: f64) {
        let c = &self.c;
        let reach = smooth(0.0, CUT, t);
        let k = 1.0 - smooth(CUT, CUT * 3.0, t);
        if k <= 0.0 {
            return;
        }
        let half = 15.0 * self.s;
        let bend = 4.0 * self.s;
        let thick = (0.7 + 0.6 * self.w) * k;
        let (ca, sa) = (libm::cos(SLANT), libm::sin(SLANT));
        let (edge, body) = if k > 0.7 {
            (WHITE, c[5])
        } else if k > 0.4 {
            (c[5], c[4])
        } else {
            (c[4], c[2])
        };
        let mut u = -1.0;
        while u <= -1.0 + 2.0 * reach {
            // along the cut, and bowed away from the blow's side
            let (along, off) = (u * half, bend * (1.0 - u * u));
            let (x, y) = (along * ca + off * sa, along * sa - off * ca);
            let w = js_max(0.5, thick * (1.0 - u * u));
            let mut o = -w;
            while o <= w {
                let col = if o < -w * 0.3 { edge } else { body };
                p.dot(x - sa * o, y + ca * o, col, 1.0);
                o += 0.5;
            }
            u += 0.04;
        }
        p.glow_line(
            -half * ca,
            -half * sa,
            half * ca,
            half * sa,
            4.0,
            c[5],
            0.3 * k,
        );
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        self.ps.update(&mut self.r, dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, t, w) = (&self.c, self.t, self.w);
        // a blow of some weight rings out, the ring breaking into arcs that fly on and cool
        if w >= 0.4 {
            let u = (t - 0.03) / 0.26;
            if (0.0..1.0).contains(&u) {
                let r = 5.0 + (10.0 + 14.0 * w) * ease::out_cubic(u);
                let i = if u < 0.2 {
                    6
                } else if u < 0.45 {
                    5
                } else {
                    4
                };
                broken_ring(
                    p,
                    0.0,
                    0.0,
                    r,
                    js_max(1.0, 2.2 * libm::pow(1.0 - u, 1.3)),
                    &self.segs,
                    smooth(0.1, 0.55, u),
                    c[i],
                    c[i - 2],
                    if u < 0.6 { 1.0 } else { (1.0 - u) / 0.4 },
                    1.0,
                    f64::INFINITY,
                );
            }
        }
        self.cut(p, t);
        self.ps.draw(p, Layer::Both);
        // the star: all white for the frame a hit-stop holds, then gold and stretched as it thins, then a glint
        if t < HOT {
            p.circle(0.0, 0.0, 4.0 * self.s, WHITE, 1.0);
            star(
                p,
                0.0,
                0.0,
                &self.sp,
                1.0,
                1.0,
                [c[5], WHITE, WHITE, WHITE],
                1.0,
                3.0,
            );
        } else if t < HOT * 2.0 {
            star(
                p,
                0.0,
                0.0,
                &self.sp,
                1.25,
                0.6,
                [c[3], c[5], c[6], WHITE],
                1.0,
                2.4,
            );
        } else if t < HOT * 3.0 {
            star(
                p,
                0.0,
                0.0,
                &self.sp,
                1.4,
                0.3,
                [c[2], c[4], c[5], c[6]],
                1.0,
                1.6,
            );
        }
        let pop = libm::exp(-t / 0.05);
        if pop > 0.02 {
            p.glow(0.0, 0.0, 24.0 + 10.0 * w, c[4], 0.5 * pop);
            p.glow(0.0, 0.0, 9.0, WHITE, 0.45 * pop);
        }
        self.lights.draw(p);
    }
}
