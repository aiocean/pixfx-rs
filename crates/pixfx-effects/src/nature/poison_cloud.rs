//! Poison cloud: port of pixelfx `src/fx/nature/poison-cloud.js`.
//!
//! Beyond pixelfx: the `spread` option is held to 0..=16, so a huge number cannot hang the draw.

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, ramp};
use pixfx_draw::math::{TAU, clamp, js_max, js_min, js_round, js_sign, rng, smooth, tile_noise};

/// Held to 16 at most, the widest cloud drawn (beyond pixelfx, which draws any spread; see the module doc).
const SPREAD: NumOpt = NumOpt::new("spread", 1.0).within(0.0, 16.0);

pub struct PoisonCloud;

pub static POISON_CLOUD: PoisonCloud = PoisonCloud;

const META: Meta = Meta {
    id: "nature/poison-cloud",
    name: "Poison cloud",
    desc: "A miasma hugging the floor: ragged sheets of gas lit yellow-green from within, fading to violet, creep out in tongues that curl over at their tips; plumes twist up as ribbons.",
    tags: &[
        "nature", "poison", "gas", "cloud", "miasma", "toxic", "area", "ground", "loop",
    ],
    size: (256, 100),
    origin: (128, 86),
    duration: 4.0,
    looping: true,
    fps: 30,
    seed: 3,
    state: None,
    options: &[SPREAD.spec()],
};

const PALETTE: [Rgba; 8] = [
    Rgba(0x0e, 0x08, 0x16, 255),
    Rgba(0x24, 0x12, 0x3a, 255),
    Rgba(0x3e, 0x20, 0x56, 255),
    Rgba(0x5a, 0x3a, 0x62, 255),
    Rgba(0x6a, 0x6a, 0x3a, 255),
    Rgba(0x96, 0xaa, 0x30, 255),
    Rgba(0xcc, 0xe0, 0x40, 255),
    Rgba(0xf4, 0xff, 0xa0, 255),
];
const D: f64 = 4.0;

/// A sheet of gas: how tall, how wide, how thick, how much dimmer, how fast its edges flow.
struct Sheet {
    h: f64,
    e: f64,
    a: f64,
    dim: usize,
    flow: f64,
    n: usize,
    back: bool,
}

/// Sheets of gas, back to front.
const SHEETS: [Sheet; 3] = [
    Sheet {
        h: 36.0,
        e: 0.84,
        a: 0.46,
        dim: 2,
        flow: 4.0,
        n: 2,
        back: true,
    },
    Sheet {
        h: 26.0,
        e: 0.96,
        a: 0.5,
        dim: 1,
        flow: 8.0,
        n: 2,
        back: true,
    },
    Sheet {
        h: 16.0,
        e: 1.04,
        a: 0.56,
        dim: 0,
        flow: 8.0,
        n: 3,
        back: false,
    },
];

impl Effect for PoisonCloud {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            spread: opts.num(&SPREAD),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    spread: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        match layer {
            Layer::Back => draw_back(p, t, self),
            Layer::Front => draw(p, t, self),
            Layer::Both => {
                draw_back(p, t, self);
                draw(p, t, self);
            }
        }
    }
}

/// One tongue creeping out along the floor from the middle: which side, how far along, where its front is,
/// how tall.
struct Tongue {
    s: f64,
    q: f64,
    x: f64,
    h: f64,
}

fn tongues(sh: &Sheet, si: usize, e: f64, ph: f64) -> Vec<Tongue> {
    let mut out = Vec::new();
    for s in [-1.0, 1.0] {
        for i in 0..sh.n {
            let n = sh.n as f64;
            let q =
                (ph + i as f64 / n + si as f64 * 0.17 + if s > 0.0 { 0.5 / n } else { 0.0 }) % 1.0;
            out.push(Tongue {
                s,
                q,
                x: s * e * (0.3 + 0.82 * q),
                h: sh.h * (0.75 - 0.45 * q),
            });
        }
    }
    out
}

/// A sheet's height above the floor at x: flat through the middle, falling away at its ends, its top torn by
/// noise that flows outward, raised behind each tongue's front.
fn height(sh: &Sheet, si: usize, e: f64, x: f64, ph: f64, tg: &[Tongue], seed: f64) -> f64 {
    let ax = libm::fabs(x);
    let env = 1.0 - smooth(0.5, 1.0, ax / (e * sh.e));
    let r = tile_noise(
        ax * 0.07 - ph * sh.flow,
        si as f64 * 3.7,
        sh.flow,
        0.0,
        seed + si as f64,
    );
    let mut h = sh.h * env * (0.55 + 0.45 * r);
    for g in tg {
        let behind = g.s * (g.x - x);
        if !(-1.5..=30.0).contains(&behind) {
            continue;
        }
        h = js_max(
            h,
            g.h * libm::sqrt(js_max(0.0, 1.0 - behind / 30.0))
                * smooth(-1.5, 1.5, behind)
                * (1.0 - smooth(0.75, 1.0, g.q)),
        );
    }
    h
}

/// One sheet: brightest down at the floor where it's lit from within, darkening to violet toward its torn
/// top, streaked in thin layers, a pale rim along its edge.
fn sheet(p: &mut Painter, c: &[Rgba; 8], sh: &Sheet, si: usize, e: f64, t: f64, seed: f64) {
    let ph = t / D;
    let tg = tongues(sh, si, e, ph);
    let d = sh.dim;
    let xx = libm::ceil(e * 1.25) as i64;
    for xi in -xx..=xx {
        let x = xi as f64;
        let hh = height(sh, si, e, x + 0.5, ph, &tg, seed);
        if hh < 0.6 {
            continue;
        }
        let top = -js_round(hh);
        let mut y = top;
        while y <= 1.0 {
            let v = clamp(-y / hh, 0.0, 1.0);
            let sx = if x == 0.0 { 1.0 } else { js_sign(x) };
            let streak = 0.7
                + 0.3
                    * tile_noise(
                        x * 0.05 - ph * sh.flow * sx,
                        y * 0.55,
                        sh.flow,
                        0.0,
                        seed + 9.0 + si as f64,
                    );
            let lv = v - (streak - 0.7) * 0.6;
            let col = if y == top {
                c[js_max(1.0, 4.0 - d as f64) as usize]
            } else if lv < 0.16 {
                c[6 - d]
            } else if lv < 0.42 {
                c[5 - d]
            } else if lv < 0.68 {
                c[4 - d]
            } else {
                c[js_max(1.0, 3.0 - d as f64) as usize]
            };
            p.blend(
                x,
                y,
                col,
                sh.a * if y == top {
                    0.65
                } else {
                    (0.6 + 0.4 * (1.0 - v)) * streak
                },
            );
            y += 1.0;
        }
    }
    // the tongues' leading edges roll over in curls as they creep forward
    for g in &tg {
        let k = libm::sin(core::f64::consts::PI * g.q);
        if k < 0.15 || g.h < 4.0 {
            continue;
        }
        let r = 1.8 + g.h * 0.22;
        let cx = g.x - g.s * r * 0.6;
        let cy = -g.h * 0.72;
        let spin = -g.s * g.q * TAU * 2.2;
        // a roll of gas: a thick soft band winding in on itself, lighter along its outer edge
        let mut a = 0.0;
        while a < 4.6 {
            let rr = r * (1.0 - a * 0.12);
            let px = cx + g.s * libm::cos(spin + a) * rr;
            let py = cy + libm::sin(spin + a) * rr;
            let w = 1.6 * (1.0 - a / 6.0);
            let mut o = -w;
            while o <= w {
                let qx = px + g.s * libm::cos(spin + a) * o;
                let qy = py + libm::sin(spin + a) * o;
                let col = if o > w * 0.4 { c[5 - d] } else { c[4 - d] };
                p.blend(qx, qy, col, sh.a * 0.75 * k * (1.0 - a / 5.5));
                o += 0.8;
            }
            a += 0.1;
        }
    }
}

/// Plumes: columns that lift off the sheets and twist into ribbons, thinning as they climb.
fn plumes(p: &mut Painter, c: &[Rgba; 8], e: f64, t: f64, seed: f64) {
    cycle(
        t,
        D,
        5,
        seed,
        |g| g.range(2.4, 3.2),
        |age, l, r, _| {
            let u = age / l;
            let x0 = r.range(-0.55, 0.55) * e;
            let rise = r.range(13.0, 19.0);
            let len = r.range(34.0, 48.0);
            let ph = r.range(0.0, TAU);
            let amp = r.range(3.0, 5.0);
            let head = 14.0 + rise * age;
            let tail = js_max(10.0, head - len);
            let fade = smooth(0.0, 0.15, u) * (1.0 - smooth(0.5, 1.0, u));
            let mut h = tail;
            while h <= head {
                let s = (h - tail) / js_max(1.0, head - tail);
                let tw = libm::cos(h * 0.22 + age * 2.4 + ph);
                let w = 0.6 + 2.2 * libm::fabs(tw) * (1.0 - s * 0.5);
                let x = x0 + amp * libm::sin(h * 0.12 + age + ph) * (0.4 + s);
                let a = fade * libm::sin(core::f64::consts::PI * js_min(1.0, s * 1.2)) * 0.32;
                let mut o = -w;
                while o <= w {
                    let col = if tw > 0.0 {
                        if libm::fabs(o) < w * 0.5 { c[6] } else { c[5] }
                    } else if libm::fabs(o) < w * 0.5 {
                        c[3]
                    } else {
                        c[2]
                    };
                    p.blend(x + o, -h, col, a);
                    o += 1.0;
                }
                h += 0.7;
            }
        },
    );
}

/// Specks drifting out through the gas with its flow, twinkling.
fn specks(p: &mut Painter, c: &[Rgba; 8], e: f64, t: f64, seed: f64) {
    cycle(
        t,
        D,
        30,
        seed,
        |g| g.range(1.5, 2.6),
        |age, l, r, _| {
            let u = age / l;
            let s = r.sign();
            let x = s * (r.range(0.05, 0.7) * e + age * r.range(6.0, 14.0));
            let y = -r.range(2.0, 20.0) - libm::sin(age * r.range(1.5, 3.0) + r.next() * TAU) * 2.0;
            let a = js_min(1.0, u * 5.0)
                * js_min(1.0, (1.0 - u) * 3.0)
                * (0.6 + 0.4 * libm::sin(age * r.range(6.0, 10.0) + r.next() * TAU));
            if a < 0.1 {
                return;
            }
            p.dot(x, y, if a > 0.7 { c[7] } else { c[6] }, a * 1.3);
            p.glow(x, y, 4.0, c[6], 0.3 * a);
        },
    );
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let e = 96.0 * o.spread;
    let pulse = 0.85 + 0.15 * libm::sin((TAU * t * 2.0) / D);
    // lit from within, low down: a soft line of light along the floor under it all
    let mut x = -e;
    while x <= e {
        p.glow(
            x,
            -3.0,
            30.0,
            c[5],
            0.14 * pulse * (1.0 - libm::fabs(x) / (e * 1.3)),
        );
        x += 22.0;
    }
    // pockets of light wandering inside it, low down
    for i in 0..6 {
        let mut r = rng(&[o.seed, i as f64, 7.0]);
        let x = libm::sin((TAU * t) / D + r.range(0.0, TAU)) * e * r.range(0.2, 0.7)
            + r.range(-0.3, 0.3) * e;
        let k = 0.6 + 0.4 * libm::sin((TAU * t * 3.0) / D + i as f64);
        p.glow(x, -r.range(4.0, 10.0), r.range(14.0, 22.0), c[6], 0.2 * k);
    }
    for (i, sh) in SHEETS.iter().enumerate() {
        if sh.back {
            sheet(p, c, sh, i, e, t, o.seed);
        }
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let e = 96.0 * o.spread;
    plumes(p, c, e, t, o.seed * 7.0 + 2.0);
    for (i, sh) in SHEETS.iter().enumerate() {
        if !sh.back {
            sheet(p, c, sh, i, e, t, o.seed);
        }
    }
    specks(p, c, e, t, o.seed * 11.0 + 5.0);
    p.glow(0.0, -6.0, e * 0.9, c[5], 0.1);
}
