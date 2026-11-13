//! Burn away: port of pixelfx `src/fx/transitions/burn-away.js`.
//!
//! The layered flame tongues and the licks, embers and smoke shed off a burning edge follow Sprite Fusion's
//! open-source fire (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Beyond pixelfx: the number options `holeX` and `holeY` (screen pixels) set where the hole starts burning
//! open. Each replaces pixelfx's seeded centre on its own axis; the seeded centre is still drawn, so every
//! other random draw is unchanged.

use pixfx_core::color::{Rgba, WHITE, mix, ramp};
use pixfx_core::math::{
    Rng, TAU, clamp, ease, fbm, js_max, js_min, js_round, noise, noise2, rand, smooth, to_uint32,
};
use pixfx_core::sim::{Flags, Kind, Lights, Particles, ParticlesOpts, SMOKE_RAMP, Spawn};
use pixfx_core::{Effect, EffectState, Layer, Meta, OptValue, Options, Painter};

use super::lib::{COLOR, Field, H, W, clip_to, field, life_left, pixels, rect, screen_meta};

pub struct BurnAway;

pub static BURN_AWAY: BurnAway = BurnAway;

const META: Meta = screen_meta(
    "transitions/burn-away",
    "Burn away",
    "The screen burns in from its edges behind a ragged ember line of licking flames, embers and smoke; then the black smoulders and burns open from the middle.",
    &[
        "transition",
        "screen",
        "burn",
        "fire",
        "embers",
        "smoke",
        "wipe",
        "one-shot",
    ],
    3.0,
    3,
);

// meta.options
const PALETTE: [Rgba; 9] = [
    Rgba::hex24(0x4a1712),
    Rgba::hex24(0x8a1f17),
    Rgba::hex24(0xc9281f),
    Rgba::hex24(0xf2561d),
    Rgba::hex24(0xff8a2a),
    Rgba::hex24(0xffc23d),
    Rgba::hex24(0xffe26b),
    Rgba::hex24(0xfff3a6),
    Rgba::hex24(0xffffff),
];

const WF: f64 = W as f64;
const HF: f64 = H as f64;
const D: f64 = META.duration;
const CLOSE: f64 = 1.18;
/// The screen is fully covered from here (the docs site relies on it).
const COVER: f64 = 1.35;
/// The black starts smouldering here (the docs site starts playing from here).
const SMOULDER: f64 = 1.5;
const POP: f64 = 1.8;
const OPEN: f64 = 2.62;
const SC: f64 = 90.0;
const HALO: f64 = 24.0;
const POOL: f64 = 48.0;
const ASH: [Rgba; 4] = [
    Rgba::hex24(0xa39cab),
    Rgba::hex24(0x7b7484),
    Rgba::hex24(0x5c5664),
    Rgba::hex24(0x433e4b),
];
const SMOKE_BASE: Rgba = Rgba::hex24(0x8a7276);
// a tongue's layers: [width, height, palette step], outermost (red) to the white core
const LAYERS: [(f64, f64, usize); 4] = [
    (1.0, 1.0, 2),
    (0.76, 0.86, 4),
    (0.54, 0.68, 6),
    (0.3, 0.42, 8),
];
// what a patch of edge gives off per pixel crossed: [flame, ember, smoke, smoke on the hottest stretches, ash,
// light], as the cover burns in and as the black burns open
const RATE_IN: [f64; 6] = [0.006, 0.0014, 0.0006, 0.0024, 0.0012, 0.0004];
const RATE_OUT: [f64; 6] = [0.006, 0.0005, 0.0004, 0.0012, 0.0004, 0.0004];

/// The value at `i`, or the type's zero off the screen.
fn at<T: Copy + Default>(a: &[T], i: usize) -> T {
    a.get(i).copied().unwrap_or_default()
}

// where the fire reaches when: rising from the edges' midpoints (the corners go first), ragged at two scales,
// and the bottom burns a little faster because fire climbs
fn in_field(seed: f64) -> Field {
    field(|x, y| {
        let (ux, uy) = ((x - WF / 2.0) / (WF / 2.0), (y - HF / 2.0) / (HF / 2.0));
        let m = libm::pow(libm::pow(ux, 4.0) + libm::pow(uy, 4.0), 0.25);
        (1.0 - m) * SC
            + 14.0 * fbm(x / 44.0, y / 44.0, seed, 3)
            + 2.4 * fbm(x / 7.0, y / 7.0, seed + 9.0, 2)
            + 7.0 * (0.5 - y / HF)
    })
}

/// A spot near the hole that catches a beat later.
struct Sat {
    x: f64,
    y: f64,
    d: f64,
}

// the hole: from a spot near the middle (and two smaller ones that catch a beat later) out to the corners.
// `hole` overrides the seeded centre per axis; the centre is drawn either way.
fn out_field(seed: f64, hole: (Option<f64>, Option<f64>)) -> (Field, f64, f64) {
    let mut r = Rng::new(to_uint32(seed * 31.0 + 5.0));
    let ox = WF / 2.0 + r.range(-26.0, 26.0);
    let oy = HF / 2.0 + r.range(-14.0, 6.0);
    let (ox, oy) = (hole.0.unwrap_or(ox), hole.1.unwrap_or(oy));
    let sats = [-1.0, 1.0].map(|s| {
        let x = ox + s * r.range(70.0, 110.0);
        let y = oy + r.range(-40.0, 40.0);
        let d = r.range(30.0, 44.0);
        Sat { x, y, d }
    });
    let f = field(|x, y| {
        let mut v = libm::hypot((x - ox) / (WF / 2.0), (y - oy) / (HF / 2.0)) * SC;
        for s in &sats {
            v = js_min(
                v,
                s.d + libm::hypot((x - s.x) / (WF / 2.0), (y - s.y) / (HF / 2.0)) * SC * 1.15,
            );
        }
        v + 13.0 * fbm(x / 40.0, y / 40.0, seed + 21.0, 3)
            + 2.4 * fbm(x / 7.0, y / 7.0, seed + 33.0, 2)
    });
    (f, ox, oy)
}

/// A position option the caller set to a finite number, if any (anything else leaves pixelfx's centre).
fn num_given(opts: &Options, key: &str) -> Option<f64> {
    match opts.extra.get(key) {
        Some(OptValue::Num(v)) if v.is_finite() => Some(*v),
        _ => None,
    }
}

// a flame standing on (x, y): four layers of the ramp, each narrower, shorter and hotter, swaying at the tip
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
fn tongue(
    p: &mut Painter,
    f: &[Rgba; 9],
    x: f64,
    y: f64,
    h: f64,
    w: f64,
    sway: f64,
    t: f64,
    n: f64,
) {
    for (lw, lh, ri) in LAYERS {
        let ht = h * lh;
        let col = f.get(ri).copied().unwrap_or(WHITE);
        let mut r = 0.0;
        while r <= ht {
            let u = r / js_max(1.0, ht);
            let half = w * lw * libm::pow(1.0 - u, 0.85) * js_min(1.0, (r + 1.5) / 3.5)
                + noise(t * 9.0 + r * 0.45, n) * 0.5;
            if half >= 0.3 {
                let cx = x + sway * u * u * lh + noise(t * 4.0 + r * 0.2, n + 7.0) * u * 1.1;
                let mut px = js_round(cx - half);
                while px <= js_round(cx + half) {
                    p.px(px, js_round(y - r), col);
                    px += 1.0;
                }
            }
            r += 1.0;
        }
    }
}

impl Effect for BurnAway {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let f: [Rgba; 9] = ramp(&opts.palette_or(&PALETTE));
        let ink = opts.color_or("color", COLOR);
        let r = Rng::new(to_uint32(seed as f64));
        let fseed = seed as f64 * 13.0;
        let a = in_field(fseed);
        let hole = (num_given(opts, "holeX"), num_given(opts, "holeY"));
        let (b, ox, oy) = out_field(fseed + 1.0, hole);
        // per pixel: how hot the ember line burns there (two smooth fields), and the odd fleck that glows on
        // (ph: a smooth phase 0..63 that the flicker tables are read with; jit: a ragged char rim)
        // (vein: near 0 along a web of thin curving cracks, where the char smoulders before it burns open)
        let n = W * H;
        let (mut heat, mut ph, mut fleck, mut jit, mut vein) = (
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
        );
        for i in 0..n {
            let (x, y) = ((i % W) as f64, (i / W) as f64);
            heat.push((0.5 + 0.5 * noise2(x / 13.0, y / 13.0, fseed + 41.0)) as f32);
            ph.push(js_min(
                63.0,
                libm::floor((0.5 + 0.5 * noise2(x / 23.0, y / 23.0, fseed + 43.0)) * 64.0),
            ) as u8);
            fleck.push(if rand(&[x, y, fseed + 45.0]) < 0.04 {
                (9.0 + 22.0 * rand(&[x, y, fseed + 47.0])) as f32
            } else {
                0.0
            });
            jit.push(((rand(&[x, y, fseed + 49.0]) - 0.5) * 1.5) as f32);
            // off the pixel grid, so cracks don't run square
            let (rx, ry) = (x * 0.8 + y * 0.6, y * 0.8 - x * 0.6);
            vein.push(js_min(
                libm::fabs(noise2(rx / 11.0, ry / 9.0, fseed + 61.0)),
                libm::fabs(noise2(ry / 5.5, rx / 5.0, fseed + 63.0)) * 1.5,
            ) as f32);
        }
        Box::new(State {
            f,
            ink,
            char_: mix(ink, f[0], 0.45),
            scorch: mix(ink, Rgba::hex24(0x3a2216), 0.6),
            warm: mix(f[4], f[5], 0.4),
            smoke: mix(SMOKE_BASE, f[2], 0.2),
            lit: mix(SMOKE_BASE, f[3], 0.25),
            seed: fseed,
            a,
            b,
            ox,
            oy,
            heat,
            ph,
            fleck,
            jit,
            vein,
            r,
            ps: Particles::new(ParticlesOpts {
                sparks: vec![f[7], f[6], f[4], f[2]],
                ..ParticlesOpts::default()
            }),
            ash: Particles::new(ParticlesOpts {
                sparks: ASH.to_vec(),
                ..ParticlesOpts::default()
            }),
            lights: Lights::new(),
            t: 0.0,
            pin: 0,
            pout: 0,
            closed: false,
            popped: false,
        })
    }
}

struct State {
    f: [Rgba; 9],
    ink: Rgba,
    char_: Rgba,
    scorch: Rgba,
    warm: Rgba,
    smoke: Rgba,
    lit: Rgba,
    /// The fields' seed (pixelfx `o.seed * 13`).
    seed: f64,
    /// Where the cover reaches when, and where the hole does.
    a: Field,
    b: Field,
    /// Where the hole starts.
    ox: f64,
    oy: f64,
    heat: Vec<f32>,
    ph: Vec<u8>,
    fleck: Vec<f32>,
    jit: Vec<f32>,
    vein: Vec<f32>,
    r: Rng,
    ps: Particles,
    ash: Particles,
    lights: Lights,
    t: f64,
    pin: usize,
    pout: usize,
    closed: bool,
    popped: bool,
}

impl State {
    fn k_in(&self, t: f64) -> f64 {
        self.a.lo - 8.0 + (self.a.hi - self.a.lo + 8.0) * libm::pow(js_max(0.0, t) / CLOSE, 1.25)
    }

    fn k_out(&self, t: f64) -> f64 {
        let b = &self.b;
        if t < POP {
            b.lo - 24.0 + 24.0 * ease::in_quad(clamp((t - SMOULDER) / (POP - SMOULDER), 0.0, 1.0))
        } else {
            b.lo + (b.hi - b.lo + 16.0) * ease::out_quad(clamp((t - POP) / (OPEN - POP), 0.0, 1.0))
        }
    }

    // what a patch of edge gives off as the front crosses it: licks of flame, embers, ash, now and then a cloud
    // of smoke (a run of puffs from one spot, so it rolls up as a wisp, not a dot) and a flicker of light. The
    // black burning open sheds fewer embers and less ash, so the picture it uncovers stays clear.
    fn shed(&mut self, i: usize, in_a: bool) {
        // what burns in is gone before the black burns open, so the uncovered picture starts clean
        let [fl, em, sm, sh, as_, li] = if in_a { RATE_IN } else { RATE_OUT };
        let (x, y) = ((i % W) as f64 + 0.5, (i / W) as f64 + 0.5);
        let end = if in_a { POP + 0.1 } else { D - 0.05 };
        let (t, r, ps) = (self.t, &mut self.r, &mut self.ps);
        let mut roll = r.next();
        roll -= fl;
        if roll < 0.0 {
            let life = life_left(r.range(0.18, 0.4), t, end);
            if life > 0.08 {
                let s = Spawn {
                    kind: Kind::Fire,
                    x,
                    y: y - 3.0,
                    vx: r.range(-8.0, 8.0),
                    vy: -r.range(25.0, 50.0),
                    life,
                    col: None,
                    size: r.range(0.8, 1.7),
                    delay: 0.0,
                    flags: Flags::NONE,
                };
                ps.spawn(r, s);
            }
            return;
        }
        roll -= em;
        if roll < 0.0 {
            let life = life_left(
                if in_a {
                    r.range(0.6, 1.3)
                } else {
                    r.range(0.5, 1.0)
                },
                t,
                end,
            );
            if life > 0.2 {
                let s = Spawn {
                    kind: Kind::Ember,
                    x,
                    y: y - 2.0,
                    vx: r.range(-30.0, 30.0),
                    vy: -r.range(40.0, 100.0),
                    life,
                    col: None,
                    size: if r.chance(0.25) { 2.0 } else { 1.0 },
                    delay: 0.0,
                    flags: Flags::NONE,
                };
                ps.spawn(r, s);
            }
            return;
        }
        roll -= sm + sh * libm::pow(f64::from(at(&self.heat, i)), 2.0);
        if roll < 0.0 {
            // smoke lives long only where the front chases it upward; elsewhere the front leaves it, so it's a wisp
            let n = r.int(4.0, 6.0) as u32;
            let vx = r.range(-6.0, 6.0);
            let up = f64::from(at(if in_a { &self.a.ny } else { &self.b.ny }, i)) < -0.25;
            for j in 0..n {
                let j = f64::from(j);
                let life = life_left(
                    if up {
                        r.range(0.8, 1.3)
                    } else {
                        r.range(0.35, 0.65)
                    },
                    t + j * 0.05,
                    end,
                );
                if life > 0.4 {
                    let s = Spawn {
                        kind: Kind::Smoke,
                        x: x + r.range(-3.5, 3.5),
                        y: y - 4.0 - j * 1.2,
                        vx: vx + r.range(-3.0, 3.0),
                        vy: -r.range(18.0, 34.0),
                        life,
                        col: Some(self.smoke),
                        size: r.range(2.8, 4.8),
                        delay: j * 0.05,
                        flags: Flags::NONE,
                    };
                    ps.spawn(r, s);
                }
            }
            return;
        }
        roll -= as_;
        if roll < 0.0 {
            let life = life_left(r.range(0.9, 1.6), t, end);
            if life > 0.3 {
                let s = Spawn {
                    kind: Kind::Ember,
                    x,
                    y: y - 2.0,
                    vx: r.range(-18.0, 18.0),
                    vy: -r.range(25.0, 60.0),
                    life,
                    ..Spawn::new(Kind::Ember, 0.0, 0.0, 0.0, 0.0, 0.0)
                };
                self.ash.spawn(r, s);
            }
            return;
        }
        roll -= li;
        if roll < 0.0 {
            let rad = r.range(20.0, 32.0);
            let k = r.range(0.14, 0.26);
            let life = r.range(0.18, 0.4);
            self.lights.add(x, y - 4.0, rad, self.warm, k, life);
        }
    }

    // smoke shows where firelight catches it: lit warm near the burning line, dark over the picture, all but lost
    // in the black away from the fire (the sim's smoke puff, three soft discs, shaded by that light)
    fn draw_smoke(&self, p: &mut Painter) {
        let t = self.t;
        let in_phase = t < COVER;
        let open = (SMOULDER..OPEN + 0.25).contains(&t);
        let k = if in_phase {
            self.k_in(t)
        } else {
            self.k_out(t)
        };
        let f = if in_phase { &self.a } else { &self.b };
        let dim = SMOKE_RAMP[3];
        for q in self.ps.list() {
            if q.kind != Kind::Smoke || q.delay > 0.0 {
                continue;
            }
            let kk = q.life / q.max;
            let age = 1.0 - kk;
            let r = q.size * (0.65 + js_min(1.0, age * 2.5) * 0.45 + age * 0.4);
            let i = clamp(libm::floor(q.y), 0.0, HF - 1.0) * WF
                + clamp(libm::floor(q.x), 0.0, WF - 1.0);
            let fi = f.at(i as usize);
            let e = if in_phase || open {
                if in_phase { k - fi } else { fi - k }
            } else {
                99.0
            };
            let lit = clamp(1.0 - libm::fabs(e) / 30.0, 0.0, 1.0);
            let a = js_min(1.0, age * 12.0)
                * js_min(1.0, kk * 2.4)
                * 0.58
                * if e > 0.0 {
                    0.05 + 0.95 * lit
                } else if in_phase {
                    1.0
                } else {
                    0.2 + 0.8 * lit
                };
            if a < 0.04 {
                continue;
            }
            // pixelfx draws a puff wholly on screen without the clip, which keeps every pixel anyway
            let c = mix(mix(dim, self.lit, lit), SMOKE_RAMP[2], age * 0.5);
            p.disc(q.x, q.y, r, c, a * 0.45);
            p.disc(q.x + r * 0.05, q.y + r * 0.06, r * 0.74, c, a * 0.62);
            if age < 0.5 {
                p.disc(
                    q.x - r * 0.22,
                    q.y - r * 0.3,
                    r * 0.5,
                    mix(c, WHITE, 0.15),
                    a * 0.4 * (1.0 - age * 2.0),
                );
            }
        }
    }

    /// The cover, the burning line and the flames standing on it, while the screen burns in or open.
    fn draw_burn(&self, p: &mut Painter) {
        let t = self.t;
        let (f_, ink, warm) = (&self.f, self.ink, self.warm);
        let c = |i: usize| f_.get(i).copied().unwrap_or(WHITE);
        let in_phase = t < COVER;
        let k = if in_phase {
            self.k_in(t)
        } else {
            self.k_out(t)
        };
        let fd = if in_phase { &self.a } else { &self.b };
        let fade = if in_phase {
            1.0 - smooth(COVER - 0.15, COVER, t)
        } else {
            1.0
        };
        let (mut fl, mut hv_t, mut tw) = ([0.0_f32; 64], [0.0_f32; 64], [0_u8; 64]);
        for j in 0..64 {
            let q = j as f64 / 63.0;
            fl[j] = (0.75 + 0.25 * libm::sin(t * 7.0 + q * 9.0)) as f32;
            hv_t[j] = (0.3 * libm::sin(t * 5.5 + q * 11.0)) as f32;
            tw[j] = u8::from(libm::sin(t * 13.0 + q * 40.0) > -0.5);
        }
        let table = |tab: &[f32; 64], i: usize| f64::from(tab[usize::from(at(&self.ph, i)) & 63]);
        let mut line = Vec::new();
        // before the black burns open it smoulders: a web of cracks in the hole's own shape glows, spreads and
        // heats from dark red, then the hole bursts through it and it cools
        let su = clamp((t - SMOULDER) / (POP - SMOULDER), 0.0, 1.0);
        let sr = 5.0 + 17.0 * su;
        let sk = if in_phase || t < SMOULDER {
            0.0
        } else if t < POP {
            0.3 + 0.7 * su
        } else {
            1.0 - (t - POP) / 0.3
        };
        let b_lo = self.b.lo;
        // only the pixels near the burning line need working out (a slice of the field's order): the cover
        // behind it is plain ink, the picture far ahead of it untouched
        let ord = &fd.order;
        let n = ord.len();
        let lo = fd.below(k - if in_phase { 33.0 } else { POOL });
        let hi = fd.below(js_max(
            k + if in_phase { POOL } else { 33.0 },
            if sk > 0.0 { b_lo + sr } else { -1e9 },
        ));
        let (c0, c1, a0, a1) = if in_phase {
            (0, lo, lo, hi)
        } else {
            (hi, n, lo, hi)
        };
        let mut px = pixels(p);
        let shade_at = |px: &mut super::lib::Pixels, i: usize, line: &mut Vec<usize>| {
            // e: how far into the cover this pixel is in field units (smooth, for light); s: in px from the
            // burning line (sharp near it, for the line itself)
            let fi = fd.at(i);
            let e = if in_phase { k - fi } else { fi - k };
            let s = e * f64::from(at(&fd.g, i));
            px.at(i);
            if e < 0.0 {
                // the picture just ahead of the fire: scorched, and lit by it
                if in_phase && s > -2.5 {
                    px.blend(self.scorch, 0.5 * (1.0 + s / 2.5));
                }
                let u = js_max(0.0, 1.0 + e / HALO);
                let v = 1.0 + e / POOL;
                px.add(warm, (0.3 * u * u + 0.12 * v * v) * table(&fl, i));
                return;
            }
            let hv = clamp(f64::from(at(&self.heat, i)) + table(&hv_t, i), 0.0, 1.0);
            let th = 0.6 + 0.6 * hv;
            let u = s / th;
            if u < 6.4 {
                // the ember line: white-hot at the edge, cooling down the ramp into char
                if u < 0.9 {
                    line.push(i);
                }
                px.put(if u < 0.9 {
                    if hv > 0.6 { c(8) } else { c(7) }
                } else if u < 1.7 {
                    c(6)
                } else if u < 2.4 {
                    c(5)
                } else if u < 3.1 {
                    c(4)
                } else if u < 3.8 {
                    c(3)
                } else if u < 4.6 {
                    c(2)
                } else if u < 5.5 {
                    c(1)
                } else {
                    c(0)
                });
                if u < 4.6 {
                    px.add(warm, 0.45 * (1.0 - u / 4.6) * (0.7 + 0.3 * hv));
                }
                return;
            }
            if sk > 0.0 && fi < b_lo + sr {
                let hm = (1.0 - (fi - b_lo) / sr) * sk * (0.7 + 0.6 * hv);
                if f64::from(at(&self.vein, i)) < 0.04 + 0.08 * hm {
                    px.put(if hm > 0.95 {
                        c(6)
                    } else if hm > 0.75 {
                        c(5)
                    } else if hm > 0.55 {
                        c(4)
                    } else if hm > 0.38 {
                        c(3)
                    } else if hm > 0.22 {
                        c(2)
                    } else {
                        c(1)
                    });
                    px.add(warm, 0.4 * hm);
                    return;
                }
                px.add(warm, 0.2 * hm * hm);
            }
            // char, with the odd fleck still glowing in it
            let l = f64::from(at(&self.fleck, i)) * (0.8 + 0.4 * hv);
            let jit = f64::from(at(&self.jit, i));
            if l > 0.0 && s < l {
                let v = (s - 6.4 * th) / js_max(1.0, l - 6.4 * th);
                let twinkle = tw[(usize::from(at(&self.ph, i)) * 5) & 63] != 0;
                if v < 1.0 && twinkle && !(fade < 1.0 && fade <= jit / 1.5 + 0.5) {
                    px.put(if v < 0.3 {
                        c(4)
                    } else if v < 0.55 {
                        c(3)
                    } else if v < 0.8 {
                        c(2)
                    } else {
                        c(1)
                    });
                    px.add(warm, 0.3 * (1.0 - v));
                    return;
                }
            }
            if e < 22.0 {
                px.add(warm, 0.2 * libm::pow(1.0 - e / 22.0, 2.0));
            }
            px.put(if u - 6.4 < 1.6 * th + jit {
                self.char_
            } else {
                ink
            });
        };
        // [c0, c1): deep in the cover; [a0, a1): near the line
        for &i in ord.get(c0..c1).unwrap_or_default() {
            let i = i as usize;
            if at(&self.fleck, i) != 0.0 {
                shade_at(&mut px, i, &mut line);
            } else {
                px.at(i);
                px.put(ink);
            }
        }
        for &i in ord.get(a0..a1).unwrap_or_default() {
            shade_at(&mut px, i as usize, &mut line);
        }
        // flames stand on the white-hot line: tall and many where the char lies below it and they rise clear,
        // short and few where they'd only lick up over the char
        for i in line {
            let (xi, yi) = (i % W, i / W);
            let (x, y) = (xi as f64, yi as f64);
            let ny = f64::from(at(&fd.ny, i));
            let dn = if in_phase { -ny } else { ny };
            let h0 = rand(&[x, (yi >> 3) as f64, self.seed + 51.0]);
            if h0 > 0.12 + 0.6 * js_max(0.0, dn) {
                continue;
            }
            let n = ((xi * 7 + (yi >> 4) * 131) % 977) as f64;
            let hv = f64::from(at(&self.heat, i));
            let leap = 1.0 + js_max(0.0, noise(t * 2.6 + n * 3.1, n + 70.0) - 0.5) * 1.8;
            let h = (5.0 + 7.0 * rand(&[x, 1.0, self.seed + 53.0]))
                * (0.6 + 0.4 * noise(t * 3.3 + n * 1.7, n))
                * (0.6 + 0.6 * hv)
                * leap
                * fade
                * if dn > 0.0 { 0.65 + 0.5 * dn } else { 0.5 };
            if h >= 1.5 {
                tongue(
                    p,
                    f_,
                    x + 0.5,
                    y + 1.0,
                    h,
                    (1.1 + 0.7 * hv) * (0.8 + h / 16.0),
                    noise(t * 2.2 + n, n + 40.0) * 1.5,
                    t,
                    n,
                );
            }
        }
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        // the front eats inward: everything it passes this step gives off flame, embers, ash and smoke
        if t < COVER {
            let k = self.k_in(t);
            while self.pin < self.a.order.len()
                && self.a.at(at(&self.a.order, self.pin) as usize) <= k
            {
                let i = at(&self.a.order, self.pin) as usize;
                self.pin += 1;
                self.shed(i, true);
            }
            if !self.closed && t >= CLOSE {
                // the last of the picture goes up in a puff
                self.closed = true;
                let i = self.a.order.last().copied().unwrap_or(0) as usize;
                let (x, y) = ((i % W) as f64, (i / W) as f64);
                let r = &mut self.r;
                self.lights.add(x, y, 64.0, self.warm, 0.5, 0.4);
                for _ in 0..12 {
                    let s = Spawn {
                        kind: Kind::Ember,
                        x: x + r.range(-6.0, 6.0),
                        y: y + r.range(-3.0, 3.0),
                        vx: r.range(-50.0, 50.0),
                        vy: -r.range(40.0, 120.0),
                        life: r.range(0.5, 0.75),
                        col: None,
                        size: if r.chance(0.3) { 2.0 } else { 1.0 },
                        delay: 0.0,
                        flags: Flags::NONE,
                    };
                    self.ps.spawn(r, s);
                }
                for _ in 0..5 {
                    let s = Spawn {
                        kind: Kind::Smoke,
                        x: x + r.range(-8.0, 8.0),
                        y: y + r.range(-4.0, 2.0),
                        vx: r.range(-10.0, 10.0),
                        vy: -r.range(12.0, 28.0),
                        life: r.range(1.1, 1.6),
                        col: Some(self.smoke),
                        size: r.range(3.5, 5.5),
                        delay: r.range(0.0, 0.15),
                        flags: Flags::NONE,
                    };
                    self.ps.spawn(r, s);
                }
            }
        }
        // the black smoulders in the middle, then burns open
        if (SMOULDER..POP).contains(&t) && self.r.chance(0.1) {
            let r = &mut self.r;
            let i = at(&self.b.order, r.int(0.0, 400.0) as usize) as usize;
            let (x, y) = ((i % W) as f64 + 0.5, (i / W) as f64 + 0.5);
            let s = if r.chance(0.6) {
                Spawn {
                    kind: Kind::Smoke,
                    x,
                    y,
                    vx: r.range(-4.0, 4.0),
                    vy: -r.range(8.0, 16.0),
                    life: life_left(r.range(0.9, 1.4), t, D - 0.05),
                    col: Some(self.smoke),
                    size: r.range(2.0, 3.5),
                    delay: 0.0,
                    flags: Flags::NONE,
                }
            } else {
                Spawn {
                    kind: Kind::Ember,
                    x,
                    y,
                    vx: r.range(-12.0, 12.0),
                    vy: -r.range(15.0, 40.0),
                    life: r.range(0.4, 0.8),
                    ..Spawn::new(Kind::Ember, 0.0, 0.0, 0.0, 0.0, 0.0)
                }
            };
            self.ps.spawn(r, s);
        }
        if !self.popped && t >= POP {
            self.popped = true;
            let (ox, oy, r) = (self.ox, self.oy, &mut self.r);
            self.lights.add(ox, oy, 90.0, self.warm, 0.6, 0.45);
            self.lights.add(ox, oy, 34.0, self.f[7], 0.7, 0.12);
            for _ in 0..12 {
                let a = r.range(0.0, TAU);
                let sp = r.range(50.0, 150.0);
                let s = Spawn {
                    kind: Kind::Ember,
                    x: ox + libm::cos(a) * 3.0,
                    y: oy + libm::sin(a) * 2.0,
                    vx: libm::cos(a) * sp,
                    vy: libm::sin(a) * sp * 0.7 - 40.0,
                    life: r.range(0.4, 0.9),
                    col: None,
                    size: if r.chance(0.3) { 2.0 } else { 1.0 },
                    delay: 0.0,
                    flags: Flags::NONE,
                };
                self.ps.spawn(r, s);
            }
            for _ in 0..10 {
                let a = r.range(0.0, TAU);
                let sp = r.range(120.0, 260.0);
                let life = r.range(0.15, 0.35);
                let s = Spawn::new(
                    Kind::Spark,
                    ox,
                    oy,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp - 50.0,
                    life,
                );
                self.ps.spawn(r, s);
            }
        }
        if (POP..OPEN + 0.2).contains(&t) {
            let k = self.k_out(t);
            while self.pout < self.b.order.len()
                && self.b.at(at(&self.b.order, self.pout) as usize) <= k
            {
                let i = at(&self.b.order, self.pout) as usize;
                self.pout += 1;
                self.shed(i, false);
            }
        }
        self.ps.update(&mut self.r, dt);
        self.ash.update(&mut self.r, dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let t = self.t;
        // pixelfx draws through clipTo(p) and writes the cover straight to p; the cover is all on screen, so
        // one clipped painter draws the same pixels
        clip_to(p);
        if (COVER..SMOULDER).contains(&t) {
            rect(p, 0.0, 0.0, WF, HF, self.ink);
        } else if t < OPEN + 0.25 {
            self.draw_burn(p);
        }
        self.draw_smoke(p);
        self.ash.draw(p, Layer::Front);
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
