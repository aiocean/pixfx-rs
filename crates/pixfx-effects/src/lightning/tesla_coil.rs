//! Tesla coil: port of pixelfx `src/fx/lightning/tesla-coil.js`.
//!
//! The arcs (forking bolts re-struck every frame, inked with halo, body, core and light), their flicker and the
//! zaps that strike the floor in a spray of sparks follow Sprite Fusion's open-source lightning and energy orb
//! (https://www.spritefusion.com/pixel-art-vfx/lightning), MIT License, Copyright (c) 2026 Hugo Duprez.

use std::sync::OnceLock;

use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_max, js_min, js_round, rng};

use super::lib::{PLASMA, StrokeOpts, VOLT, crackle, flight, puff, scorch, spark, stroke, zigzag};

pub struct TeslaCoil;

pub static TESLA_COIL: TeslaCoil = TeslaCoil;

const META: Meta = Meta {
    id: "lightning/tesla-coil",
    name: "Tesla coil",
    desc: "A copper coil under a steel torus throws branching arcs that lash out in volleys and now and then strike the floor in a spray of sparks, lighting the metal and the ground.",
    tags: &["lightning", "electric", "tesla", "machine", "prop", "loop"],
    size: (224, 136),
    origin: (112, 120),
    duration: 2.4,
    looping: true,
    fps: 30,
    seed: 4,
    state: None,
    options: &[SPARKS.spec(), COPPER.spec(), STEEL.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);
const COPPER: RampOpt = RampOpt::new(
    "copper",
    &[
        Rgba::hex24(0x2a120c),
        Rgba::hex24(0x5a2814),
        Rgba::hex24(0x8e4a1e),
        Rgba::hex24(0xc4772f),
        Rgba::hex24(0xeaa65a),
        Rgba::hex24(0xffd9a0),
    ],
);
const STEEL: RampOpt = RampOpt::new(
    "steel",
    &[
        Rgba::hex24(0x0f121c),
        Rgba::hex24(0x1f2433),
        Rgba::hex24(0x353d52),
        Rgba::hex24(0x556078),
        Rgba::hex24(0x7f8ba3),
        Rgba::hex24(0xb8c3d6),
        Rgba::hex24(0xf2f6ff),
    ],
);

const FPS: f64 = 30.0;
const N: f64 = 72.0;
const P: f64 = N / FPS;
const TY: f64 = -57.0;
const RR: f64 = 12.0;
const TR: f64 = 3.6;
const SY: f64 = 0.45;
const CTOP: f64 = -50.0;
const CBOT: f64 = -12.0;
const IRON: [Rgba; 5] = [
    Rgba::hex24(0x120f18),
    Rgba::hex24(0x1e1a27),
    Rgba::hex24(0x2c2637),
    Rgba::hex24(0x3f374d),
    Rgba::hex24(0x5a5068),
];
const ASH: Rgba = Rgba::hex24(0x5b5566);
/// An arc's brightness over the frames it lives: struck, flickered, re-struck, gone.
const FL: [f64; 8] = [1.0, 0.7, 0.95, 0.55, 0.85, 0.45, 0.7, 0.35];

impl Effect for TeslaCoil {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let seed = seed as f64;
        let (arcs, strikes) = plan(seed);
        Box::new(State {
            c: ramp(&opts.palette_or(&VOLT)),
            cu: ramp(&opts.ramp(&COPPER)),
            st: ramp(&opts.ramp(&STEEL)),
            s: opts.ramp(&SPARKS),
            seed,
            arcs,
            strikes,
        })
    }
}

struct State {
    c: [Rgba; 8],
    cu: [Rgba; 6],
    st: [Rgba; 7],
    s: Vec<Rgba>,
    seed: f64,
    arcs: Vec<Arc>,
    strikes: Vec<Strike>,
}

/// An arc off the torus: the frame it strikes, how many frames it lives, the side, where it roots, whether it
/// goes in front of the coil, its swing from `th0` to `th1`, its length.
struct Arc {
    f0: f64,
    len: f64,
    side: f64,
    phi: f64,
    front: bool,
    th0: f64,
    th1: f64,
    l: f64,
}

/// A zap to the floor: the frame it strikes, the side, where it lands, where it roots.
struct Strike {
    f0: f64,
    side: f64,
    x: f64,
    phi: f64,
}

/// The loop's events, frame by frame: volleys of arcs off the torus, and two zaps to the floor.
fn plan(seed: f64) -> (Vec<Arc>, Vec<Strike>) {
    let mut r = rng(&[seed, 1.0]);
    let mut arcs = Vec::new();
    let mut strikes = Vec::new();
    for (v, n) in [
        (2.0, 3.0),
        (20.0, 3.0),
        (30.0, 1.0),
        (38.0, 3.0),
        (56.0, 3.0),
        (66.0, 1.0),
    ] {
        // the JS loop bound draws again on every test
        let mut j = 0.0;
        while j < n + r.int(0.0, 1.0) {
            let side = if r.chance(0.5) { 1.0 } else { -1.0 };
            let front = r.chance(0.55);
            let a = r.range(0.15, 1.1);
            let pi = core::f64::consts::PI;
            let phi = if side > 0.0 {
                if front { a } else { -a }
            } else if front {
                pi - a
            } else {
                pi + a
            };
            let th0 = r.range(-1.15, 0.35);
            let f0 = (v + j * r.int(2.0, 4.0)) % N;
            let len = r.int(3.0, 7.0);
            let th1 = th0 + r.range(-0.6, 0.6);
            let l = r.range(24.0, 52.0);
            arcs.push(Arc {
                f0,
                len,
                side,
                phi,
                front,
                th0,
                th1,
                l,
            });
            j += 1.0;
        }
    }
    for (f0, side) in [(14.0, 1.0), (47.0, -1.0)] {
        let x = side * r.range(38.0, 56.0);
        let phi = if side > 0.0 {
            0.4
        } else {
            core::f64::consts::PI - 0.4
        };
        strikes.push(Strike { f0, side, x, phi });
    }
    (arcs, strikes)
}

fn age_of(f: f64, f0: f64) -> f64 {
    (((f - f0) % N) + N) % N
}

fn root(phi: f64, up: f64) -> (f64, f64) {
    (
        libm::cos(phi) * (RR + TR * 0.6),
        TY + libm::sin(phi) * RR * SY - up,
    )
}

fn at<const K: usize>(list: &[Rgba; K], i: f64) -> Rgba {
    list[clamp(i, 0.0, (K - 1) as f64) as usize]
}

/// The coil: an iron plinth, two thick primary turns, the secondary wound in copper wire, catching arc light.
fn draw_coil(p: &mut Painter, cu: &[Rgba; 6], lit: [f64; 2], c: &[Rgba; 8]) {
    let n = cu.len() as f64;
    p.shade(-14.0, -8.0, 14.0, 0.0, |x, y, o| {
        if x < -13.0 || x > 13.0 || y < -7.0 {
            return None;
        }
        let edge = if x < -12.0 {
            2
        } else if x > 12.0 {
            0
        } else {
            1
        };
        o.c = if y < -6.0 {
            IRON[4]
        } else if y < -5.0 {
            IRON[3]
        } else if y > -1.0 {
            IRON[0]
        } else {
            IRON[if edge + 1 > 2 {
                2
            } else {
                edge + usize::from(y < -3.0)
            }]
        };
        None
    });
    for rx in [-10.0, 10.0] {
        p.px(rx, -4.0, IRON[4]);
        p.px(rx, -3.0, IRON[0]);
    }
    let turn = |p: &mut Painter, y: f64, front: bool| {
        let mut a = 0.0;
        while a < TAU {
            if (libm::sin(a) >= 0.0) == front {
                let x = libm::cos(a) * 10.0;
                let yy = y + libm::sin(a) * 2.2;
                let l = 0.5 - 0.4 * libm::cos(a) + if front { 0.25 } else { -0.2 };
                p.px(x, yy, at(cu, js_round(l * (n - 1.0))));
                p.px(x, yy + 1.0, at(cu, js_round(l * (n - 1.0)) - 2.0));
            }
            a += 0.04;
        }
    };
    turn(p, -10.5, false);
    turn(p, -13.5, false);
    // the secondary: wire on every other row, lit from the left, a bright band down the front, arc light on the rims
    p.shade(-6.0, CTOP, 6.0, CBOT, |x, y, o| {
        if x < -5.0 || x > 5.0 {
            return None;
        }
        let u = (x + 5.5) / 11.0;
        let nx = u * 2.0 - 1.0;
        let wire = (libm::floor(y) as i64) & 1 == 0;
        let mut l = 0.62 - 0.55 * nx - 0.35 * nx * nx
            + if libm::fabs(nx + 0.35) < 0.12 {
                0.3
            } else {
                0.0
            };
        if !wire {
            l -= 0.32;
        }
        o.c = at(cu, js_round(l * (n - 1.0)));
        if wire && ((nx < -0.75 && lit[0] > 0.25) || (nx > 0.75 && lit[1] > 0.25)) {
            o.c = if nx < 0.0 {
                if lit[0] > 0.6 { c[6] } else { c[4] }
            } else if lit[1] > 0.6 {
                c[6]
            } else {
                c[4]
            };
        }
        None
    });
    p.line(-5.0, CTOP - 1.0, 5.0, CTOP - 1.0, IRON[3], 1.0);
    p.line(-4.0, CTOP - 2.0, 4.0, CTOP - 2.0, IRON[2], 1.0);
    turn(p, -10.5, true);
    turn(p, -13.5, true);
}

/// The torus, solved once: for every pixel, the normal of the nearest point of a ring tube seen from a little
/// above. Depths and normals are held as `f32`, as pixelfx's `Float32Array`s hold them.
struct Torus {
    x0: f64,
    y0: f64,
    w: usize,
    h: usize,
    z: Vec<f32>,
    n: Vec<f32>,
    sa: f64,
    ca: f64,
}

fn torus() -> &'static Torus {
    static TORUS: OnceLock<Torus> = OnceLock::new();
    TORUS.get_or_init(|| {
        let sa = SY;
        let ca = libm::sqrt(1.0 - SY * SY);
        let x0 = -libm::ceil(RR + TR) - 1.0;
        let y0 = libm::floor(TY - RR * SY - TR) - 2.0;
        let w = 2.0 * -x0 + 2.0;
        let h = libm::ceil(TY + RR * SY + TR) + 3.0 - y0;
        let (wu, hu) = (w as usize, h as usize);
        let mut z = vec![-1e9_f32; wu * hu];
        let mut n = vec![0.0_f32; wu * hu * 3];
        for i in 0..480 {
            let phi = (f64::from(i) / 480.0) * TAU;
            let (cp, sp) = (libm::cos(phi), libm::sin(phi));
            for j in 0..96 {
                let psi = (f64::from(j) / 96.0) * TAU;
                let (cs, ss) = (libm::cos(psi), libm::sin(psi));
                let rr = RR + TR * cs;
                let (xx, yy, zz) = (rr * cp, TR * ss, rr * sp);
                let px = libm::floor(xx) - x0;
                let py = libm::floor(TY - yy * ca + zz * sa) - y0;
                let d = zz * ca + yy * sa;
                if px < 0.0 || py < 0.0 || px >= w || py >= h {
                    continue;
                }
                let k = py as usize * wu + px as usize;
                if d > f64::from(z[k]) {
                    z[k] = d as f32;
                    n[k * 3] = (cs * cp) as f32;
                    n[k * 3 + 1] = ss as f32;
                    n[k * 3 + 2] = (cs * sp) as f32;
                }
            }
        }
        Torus {
            x0,
            y0,
            w: wu,
            h: hu,
            z,
            n,
            sa,
            ca,
        }
    })
}

/// JS `Math.hypot` of three numbers as V8 computes it: scaled by the largest, summed with Kahan compensation.
fn js_hypot3(v: [f64; 3]) -> f64 {
    let max = v.iter().fold(0.0, |m: f64, &x| js_max(m, libm::fabs(x)));
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut comp) = (0.0, 0.0);
    for x in v {
        let n = libm::fabs(x) / max;
        let summand = n * n - comp;
        let pre = sum + summand;
        comp = (pre - sum) - summand;
        sum = pre;
    }
    libm::sqrt(sum) * max
}

fn norm(x: f64, y: f64, z: f64) -> [f64; 3] {
    let l = js_hypot3([x, y, z]);
    [x / l, y / l, z / l]
}

/// Steel shaded by a key light from the upper left, a hard specular, dark inner faces, the arcs' light on the
/// sides.
fn draw_torus(p: &mut Painter, st: &[Rgba; 7], c: &[Rgba; 8], lit: [f64; 2]) {
    let t = torus();
    let m = st.len() as f64;
    let key = norm(-0.55, 0.75, 0.45);
    let half = norm(key[0], key[1] + t.sa, key[2] + t.ca);
    let on = |x: usize, y: usize| x < t.w && y < t.h && f64::from(t.z[y * t.w + x]) > -1e8;
    for y in 0..t.h {
        for x in 0..t.w {
            let k = y * t.w + x;
            if f64::from(t.z[k]) < -1e8 {
                continue;
            }
            let nx = f64::from(t.n[k * 3]);
            let ny = f64::from(t.n[k * 3 + 1]);
            let nz = f64::from(t.n[k * 3 + 2]);
            let dif = js_max(0.0, nx * key[0] + ny * key[1] + nz * key[2]);
            let spec = libm::pow(
                js_max(0.0, nx * half[0] + ny * half[1] + nz * half[2]),
                28.0,
            );
            let lx = x as f64 + t.x0 + 0.5;
            let inner = if nx * lx + nz * 0.0 < 0.0 && libm::fabs(lx) < RR {
                0.75
            } else {
                1.0
            };
            let mut col = if spec > 0.55 {
                st[6]
            } else {
                st[clamp(
                    libm::floor((0.12 + 0.8 * dif * inner + spec) * (m - 1.01)),
                    0.0,
                    m - 2.0,
                ) as usize]
            };
            for (s, l) in [(-1.0, lit[0]), (1.0, lit[1])] {
                let r = js_max(0.0, s * nx * 0.85 + ny * 0.25 + nz * 0.45) * js_min(1.0, l);
                if r > 0.88 {
                    col = c[6];
                } else if r > 0.62 {
                    col = mix(col, c[5], 0.6);
                } else if r > 0.38 {
                    col = mix(col, c[4], 0.3);
                }
            }
            if !on(x, y + 1) && ny < 0.3 {
                col = st[0];
            }
            p.px(x as f64 + t.x0, y as f64 + t.y0, col);
        }
    }
}

impl State {
    fn draw_arc(&self, p: &mut Painter, a: &Arc, f: f64) {
        let c = &self.c;
        let age = age_of(f, a.f0);
        let u = age / js_max(1.0, a.len - 1.0);
        let th = a.th0 + (a.th1 - a.th0) * ease::out_quad(u);
        let ang = if a.side > 0.0 {
            th
        } else {
            core::f64::consts::PI - th
        };
        let (x0, y0) = root(a.phi, 0.0);
        let l = a.l * if age == 0.0 { 0.75 } else { 1.0 };
        let mut r: Rng = rng(&[self.seed, 21.0, a.f0, f]);
        let k = FL[age as usize];
        let bolt = zigzag(
            &mut r,
            x0,
            y0,
            x0 + libm::cos(ang) * l,
            y0 + libm::sin(ang) * l,
            0.32,
            0.75,
        );
        stroke(
            p,
            &bolt,
            k,
            c,
            0.6,
            StrokeOpts {
                light: 0.4,
                ..StrokeOpts::default()
            },
        );
        p.glow(x0, y0, 10.0, c[5], 0.5 * k);
    }
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, seed) = (&self.c, self.seed);
        let f = libm::floor(t * FPS + 1e-6) % N;
        let mut lit = [0.0, 0.0];
        let live: Vec<&Arc> = self
            .arcs
            .iter()
            .filter(|a| age_of(f, a.f0) < a.len)
            .collect();
        for a in &live {
            lit[usize::from(a.side > 0.0)] += FL[age_of(f, a.f0) as usize] * 0.6;
        }
        for s in &self.strikes {
            if age_of(f, s.f0) < 6.0 {
                lit[usize::from(s.side > 0.0)] += 1.0;
            }
        }
        let act = js_min(1.0, (lit[0] + lit[1]) * 0.5);

        // the floor: a cold pool of light under the coil, scorches and smoke where zaps landed
        p.glow(0.0, -10.0, 58.0, c[3], 0.14 + 0.25 * act);
        p.ellipse(0.0, 0.5, 15.0, 1.6, IRON[0], 1.0);
        for s in &self.strikes {
            let age = (((t - s.f0 / FPS) % P) + P) % P;
            scorch(
                p,
                s.x,
                0.0,
                7.0,
                libm::pow(clamp(1.0 - age / 1.1, 0.0, 1.0), 1.3),
                1.0 - clamp((age - 1.4) / 0.6, 0.0, 1.0),
            );
            for j in 0..2 {
                let fj = f64::from(j);
                let u = (age - 0.12 - fj * 0.18) / 1.2;
                if u > 0.0 && u < 1.0 {
                    puff(
                        p,
                        s.x + (if j != 0 { 3.0 } else { -2.0 }) + u * 5.0 * s.side,
                        -3.0 - u * 16.0 - fj * 2.0,
                        2.6 + fj,
                        u,
                        mix(ASH, c[2], 0.3),
                    );
                }
            }
        }

        // arcs off the back of the torus go behind the coil
        for a in &live {
            if !a.front {
                self.draw_arc(p, a, f);
            }
        }
        draw_coil(p, &self.cu, lit, c);
        draw_torus(p, &self.st, c, lit);
        // corona: a faint violet fuzz crawling over the torus
        let mut rc = rng(&[seed, 9.0, f]);
        for _ in 0..6 {
            let phi = rc.range(0.0, TAU);
            let d = rc.range(1.0, 3.0);
            let x = libm::cos(phi) * (RR + TR + d);
            let y = TY + libm::sin(phi) * (RR * SY + TR + d * 0.7);
            p.px(x, y, if rc.chance(0.3) { c[5] } else { c[3] });
            if rc.chance(0.4) {
                p.px(x + libm::cos(phi), y + libm::sin(phi) * 0.6, c[2]);
            }
            p.glow(x, y, 4.0, c[3], 0.3);
        }
        // brush discharge: short streamers hiss off the rim every frame
        for _ in 0..2 {
            let phi = rc.range(0.0, TAU);
            let x0 = libm::cos(phi) * (RR + TR * 0.8);
            let y0 = TY + libm::sin(phi) * (RR * SY + TR * 0.6);
            let a = libm::atan2(libm::sin(phi) * 0.6 - 0.5, libm::cos(phi)) + rc.range(-0.5, 0.5);
            let l = rc.range(4.0, 10.0);
            let bolt = zigzag(
                &mut rc,
                x0,
                y0,
                x0 + libm::cos(a) * l,
                y0 + libm::sin(a) * l,
                0.4,
                0.4,
            );
            let k = rc.range(0.35, 0.65);
            crackle(p, &bolt, k, c, 0.3, 1.0);
        }
        p.glow(0.0, TY, 34.0, c[3], 0.18 + 0.3 * act);
        for a in &live {
            if a.front {
                self.draw_arc(p, a, f);
            }
        }

        // zaps to the floor: a full bolt, a flash where it lands, sparks thrown and bouncing
        for s in &self.strikes {
            let age = age_of(f, s.f0);
            if age < 6.0 {
                let k = [1.0, 0.75, 1.0, 0.6, 0.85, 0.4][age as usize];
                let (x0, y0) = root(s.phi, -2.0);
                let mut r = rng(&[seed, 41.0, s.f0, f]);
                let bolt = zigzag(&mut r, x0, y0, s.x, 0.0, 0.22, 0.55);
                stroke(
                    p,
                    &bolt,
                    k,
                    c,
                    if age == 0.0 { 0.95 } else { 0.8 },
                    StrokeOpts {
                        hero: age == 0.0,
                        ..StrokeOpts::default()
                    },
                );
                p.circle(s.x, -1.0, 3.0 * k + 1.5, c[2], 1.0);
                p.circle(s.x, -1.0, 3.0 * k + 0.5, c[5], 1.0);
                p.circle(s.x, -1.0, 2.0 * k, WHITE, 1.0);
                p.glow(s.x, -10.0, 40.0, c[4], 0.55 * k);
                p.glow(s.x, -3.0, 12.0, c[6], 0.5 * k);
                if age < 2.0 {
                    p.glow(0.0, -50.0, 120.0, c[3], 0.25 * k);
                }
            }
            let st = (((t - s.f0 / FPS) % P) + P) % P;
            for j in 0..18 {
                let mut r = rng(&[seed, 51.0, s.f0, f64::from(j)]);
                let life = r.range(0.3, 0.65);
                if st >= life {
                    continue;
                }
                let a = -core::f64::consts::FRAC_PI_2 + r.range(-1.3, 1.3);
                let sp = r.range(50.0, 190.0);
                let (x, y, vx, vy) =
                    flight(s.x, -1.0, libm::cos(a) * sp, libm::sin(a) * sp, st, 0.0);
                spark(p, x, y, vx, vy, 1.0 - st / life, &self.s);
            }
        }
        // sparks spat off the arc tips as they strike
        for a in &self.arcs {
            let age = (((t - a.f0 / FPS) % P) + P) % P;
            let ang = if a.side > 0.0 {
                a.th0
            } else {
                core::f64::consts::PI - a.th0
            };
            let (x0, y0) = root(a.phi, 0.0);
            for j in 0..3 {
                let mut r = rng(&[seed, 61.0, a.f0, f64::from(j)]);
                let life = r.range(0.15, 0.35);
                if age >= life {
                    continue;
                }
                let tx = x0 + libm::cos(ang) * a.l * 0.75;
                let ty = y0 + libm::sin(ang) * a.l * 0.75;
                let b = ang + r.range(-0.9, 0.9);
                let sp = r.range(50.0, 150.0);
                let (x, y, vx, vy) = flight(tx, ty, libm::cos(b) * sp, libm::sin(b) * sp, age, 0.0);
                spark(p, x, y, vx, vy, 1.0 - age / life, &self.s);
            }
        }
    }
}
