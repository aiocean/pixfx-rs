//! Campfire: port of pixelfx `src/fx/fire/campfire.js`.
//!
//! The tongue, lick, flame-puff, smoke and ember recipe is adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::PI;

use pixfx_core::color::{Rgba, WHITE, mix, ramp};
use pixfx_core::math::{Rng, clamp, js_max, js_min, js_round, rand, to_uint32};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options, Painter, RampOpt};

use super::lib::{
    BACK_LAYERS, Hot, LAYERS, Layers, Life, LoopNoise, drift, ember, fire_puff_up, flame_tongue,
    floor_light, hot_first, lick, looped, smoke_puff, spark, spark_at,
};

pub struct Campfire;

pub static CAMPFIRE: Campfire = Campfire;

const META: Meta = Meta {
    id: "fire/campfire",
    name: "Campfire",
    desc: "Crossed logs on a bed of coals and a lively fire: tongues that lick and leap, popping sparks, embers drifting up, smoke curling off, a warm pool of light.",
    tags: &[
        "fire", "campfire", "prop", "light", "smoke", "ambient", "loop",
    ],
    size: (128, 128),
    origin: (64, 112),
    duration: 3.0,
    looping: true,
    fps: 30,
    seed: 3,
    state: None,
    options: &[WOOD.spec(), HEIGHT.spec()],
};

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
const WOOD: RampOpt = RampOpt::new(
    "wood",
    &[
        Rgba::hex24(0x0e080a),
        Rgba::hex24(0x24130f),
        Rgba::hex24(0x3b2216),
        Rgba::hex24(0x5a3520),
        Rgba::hex24(0x8a5a34),
        Rgba::hex24(0xb9884f),
    ],
);
const HEIGHT: NumOpt = NumOpt::new("height", 22.0);

const D: f64 = META.duration;
const NL: LoopNoise = LoopNoise(D);
const SMOKE1: Rgba = Rgba::hex24(0x5b5566);
const ASH: [Rgba; 3] = [
    Rgba::hex24(0x2a2530),
    Rgba::hex24(0x4a4350),
    Rgba::hex24(0x6b6270),
];

// the logs, near end first: they cross in an X over the coals, their near ends showing the cut grain and their
// far ends poking out past the flames
type Log = (f64, f64, f64, f64, f64);
const LOGS: [Log; 2] = [
    (-19.0, 2.0, 15.0, -12.0, 3.4),
    (19.0, 2.0, -15.0, -12.0, 3.4),
];
// the crossing, as a share of each log's length
const CROSS: f64 = 0.56;
// big tongues keep more orange and a smaller white heart, so the light doesn't wash them out
const BIG_LAYERS: [(f64, f64, usize); 6] = [
    (1.0, 1.0, 6),
    (0.8, 0.9, 5),
    (0.62, 0.78, 4),
    (0.44, 0.6, 3),
    (0.26, 0.4, 1),
    (0.12, 0.22, 0),
];

// the fire: [x, base, height (share of o.height), half-width, back row?]
const TONGUES: [(f64, f64, f64, f64, bool); 11] = [
    (-7.5, -7.0, 0.42, 1.8, true),
    (7.5, -7.0, 0.46, 1.8, true),
    (-3.0, -9.0, 0.78, 2.0, true),
    (3.2, -9.0, 0.82, 2.0, true),
    (-12.0, -3.0, 0.14, 1.3, false),
    (12.5, -3.0, 0.16, 1.3, false),
    (-9.5, -4.5, 0.3, 1.9, false),
    (9.5, -4.5, 0.34, 1.9, false),
    (-5.0, -6.0, 0.6, 2.6, false),
    (5.2, -6.0, 0.66, 2.6, false),
    (0.0, -7.0, 1.0, 3.4, false),
];

impl Effect for Campfire {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            f: ramp(&opts.palette_or(&PALETTE)),
            w: ramp(&opts.ramp(&WOOD)),
            height: opts.num(&HEIGHT),
            s: seed as f64,
        })
    }
}

struct State {
    f: Hot,
    w: [Rgba; 6],
    height: f64,
    s: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

// a log from its near end (x0, y0) to its far end, radius r, drawn between u0 and u1 along it: a 1 px dark rim,
// a lit top, grooves running along the grain, a shadowed underside; charred with glowing cracks in the fire
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
fn log(p: &mut Painter, l: &Log, w: &[Rgba; 6], h: &Hot, u0: f64, u1: f64, t: f64, s: f64) {
    let &(x0, y0, x1, y1, r) = l;
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = libm::hypot(dx, dy);
    let (ax, ay) = (dx / len, dy / len);
    let (mut nx, mut ny) = (-ay, ax);
    if ny < 0.0 {
        (nx, ny) = (-nx, -ny);
    }
    let rim = 1.0 - 1.05 / r;
    p.shade(
        js_min(x0, x1) - r - 1.0,
        js_min(y0, y1) - r - 1.0,
        js_max(x0, x1) + r + 1.0,
        js_max(y0, y1) + r + 1.0,
        |x, y, out| {
            let along = (x - x0) * ax + (y - y0) * ay;
            let u = along / len;
            let sv = ((x - x0) * nx + (y - y0) * ny) / r;
            if u < u0 || u > u1 || sv.abs() > 1.0 {
                return None;
            }
            let burn = clamp(1.2 - x.abs() / 8.0, 0.0, 1.0);
            let lane = js_round(sv * r * 0.85);
            let run = rand(&[libm::floor(along / 3.0), lane, s]);
            if sv.abs() > rim || along > len - 1.0 {
                return Some(if sv < 0.0 && burn > 0.4 { h[7] } else { w[0] });
            }
            if burn > 0.35 && lane < 2.0 {
                // charred: black, with embers breathing in the cracks along the grain
                if run < 0.55 && lane > -2.0 {
                    let g = 0.5
                        + 0.5
                            * libm::sin(
                                (t / D) * PI * 6.0
                                    + rand(&[libm::floor(along / 2.0), lane, s + 1.0]) * 6.3,
                            );
                    out.k = 0.3 * burn * g;
                    return Some(if g > 0.72 {
                        h[3]
                    } else if g > 0.4 {
                        h[4]
                    } else {
                        h[5]
                    });
                }
                return Some(if lane < -1.0 { w[1] } else { w[0] });
            }
            if lane <= -2.0 {
                return Some(if run < 0.25 {
                    w[2]
                } else {
                    mix(w[3], h[4], 0.45 * burn)
                });
            }
            if lane == -1.0 {
                return Some(if run < 0.45 { w[1] } else { w[2] });
            }
            if lane == 0.0 {
                return Some(if run < 0.2 { w[1] } else { w[2] });
            }
            Some(if run < 0.4 { w[0] } else { w[1] })
        },
    );
    // the near end's cut face: a dark rim, pale sapwood, growth rings round a dark heart
    if u0 <= 0.0 {
        p.ellipse(
            x0,
            y0,
            r * 0.62,
            r + 0.2,
            |d: f64, _: f64| {
                if d > 0.78 {
                    w[0]
                } else if d > 0.56 {
                    w[5]
                } else if d > 0.32 {
                    w[4]
                } else {
                    w[3]
                }
            },
            1.0,
        );
    }
}

// the bed of coals under the logs: coals that breathe in the heat, grey ash round the rim
fn coals(p: &mut Painter, w: &[Rgba; 6], h: &Hot, t: f64, s: f64) {
    p.shade(-15.0, -3.0, 15.0, 2.0, |x, y, out| {
        let (dx, dy) = (x / 14.0, (y + 0.3) / 2.2);
        let d = dx * dx + dy * dy;
        if d > 1.0 {
            return None;
        }
        let hv = rand(&[libm::floor(x / 2.0), libm::floor(y), s + 20.0]);
        if d > 0.55 && hv < 0.55 {
            return Some(
                ASH[if hv < 0.15 {
                    2
                } else if hv < 0.32 {
                    1
                } else {
                    0
                }],
            );
        }
        if hv > 0.5 {
            let g = (0.5 + 0.5 * libm::sin((t / D) * PI * 4.0 + hv * 40.0)) * (1.1 - d);
            out.k = 0.35 * g;
            return Some(if g > 0.7 {
                h[2]
            } else if g > 0.5 {
                h[3]
            } else if g > 0.3 {
                h[4]
            } else {
                h[5]
            });
        }
        Some(if hv > 0.3 { w[1] } else { w[0] })
    });
}

struct Tongue {
    dx: f64,
    y: f64,
    h: f64,
    w: f64,
    back: bool,
    env: f64,
    n: f64,
}

/// The tongues and how they move: pixelfx's `T`, `fla`, `lean`, `height`, `sweep`, `tip` closures.
struct Fire {
    t: Vec<Tongue>,
    s: f64,
}

impl Fire {
    fn fla(&self, tt: f64) -> f64 {
        let s = self.s;
        0.74 + 0.16 * NL.at(tt, 7.3, s + 1.0, 0.0) + 0.1 * NL.at(tt, 19.0, s + 2.0, 0.0)
    }
    fn lean(&self, tt: f64) -> f64 {
        NL.at(tt, 0.67, self.s + 90.0, 0.0) * 0.12
    }
    fn height(&self, q: &Tongue, tt: f64) -> f64 {
        let (h, n) = (q.h, q.n);
        let leap = js_max(0.0, NL.at(tt, 2.6, n + 70.0, 0.0) - 0.55) * 1.4;
        (h + 0.25 * h * NL.at(tt, 3.3, n, 0.0) + h * leap) * (0.9 + 0.1 * self.fla(tt))
    }
    fn sweep(&self, q: &Tongue, tt: f64, h: f64) -> f64 {
        NL.at(tt, 2.2, q.n + 40.0, 0.0) * (1.0 + h * 0.08) + self.lean(tt) * h * 0.9
            - clamp(q.dx * 0.3, -2.5, 2.5)
    }
    fn tip(&self, i: usize, tt: f64) -> (f64, f64) {
        let Some(q) = self.t.get(i) else {
            return (f64::NAN, f64::NAN);
        };
        let h = self.height(q, tt);
        (q.dx + self.sweep(q, tt, h) * 0.85, q.y - h * 0.88 + 3.0)
    }
    fn tongue_at(&self, p: &mut Painter, t: f64, hh: &Hot, q: &Tongue) {
        let h = self.height(q, t);
        let nf = |rate: f64, seed: f64, off: f64| NL.at(t, rate, seed, off);
        let layers: &Layers = if q.back {
            &BACK_LAYERS
        } else if q.w > 2.2 {
            &BIG_LAYERS
        } else {
            &LAYERS
        };
        flame_tongue(
            p,
            q.dx,
            q.y,
            h,
            q.w,
            self.sweep(q, t, h),
            &nf,
            q.n,
            hh,
            layers,
            0.5,
            1.1 + h * 0.04,
        );
    }
    fn env(&self, i: usize) -> f64 {
        self.t.get(i).map_or(f64::NAN, |q| q.env)
    }
}

struct Plume {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    s: f64,
}

impl Life for Plume {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Shed {
    i: usize,
    fire: f64,
    sl: f64,
    ox: f64,
    oy: f64,
    vx: f64,
    vy: f64,
    s: f64,
    f: f64,
    life: f64,
}

impl Life for Shed {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Flame {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    s: f64,
    f: f64,
}

impl Life for Flame {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Lick {
    i: usize,
    vx: f64,
    vy: f64,
    s: f64,
    life: f64,
}

impl Life for Lick {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Pop {
    x: f64,
    y: f64,
    n: f64,
    seed: f64,
    life: f64,
}

impl Life for Pop {
    fn life(&self) -> f64 {
        self.life
    }
}

struct Ember {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    s: f64,
    a: f64,
    b: f64,
}

impl Life for Ember {
    fn life(&self) -> f64 {
        self.life
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, w, s, hc) = (hot_first(&o.f), &o.w, o.s, o.height);
    let mut rr = Rng::new(to_uint32(s));
    let nf = |rate: f64, seed: f64, off: f64| NL.at(t, rate, seed, off);
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.25);
    let smoke = mix(SMOKE1, h[6], 0.25);
    let sparks = [h[1], h[2], h[4], h[5]];
    let emb = [h[1], h[2], h[4], h[6]];

    let fire = Fire {
        t: TONGUES
            .iter()
            .enumerate()
            .map(|(i, &(dx, y, th, tw, back))| Tongue {
                dx: dx + rr.range(-0.4, 0.4),
                y,
                h: 3.0 + hc * th * rr.range(0.92, 1.08),
                w: tw * rr.range(0.9, 1.1),
                back,
                env: th,
                n: (s * 7.0 + i as f64) * 13.0,
            })
            .collect(),
        s,
    };
    let fl = fire.fla(t);
    let n_t = fire.t.len();

    // smoke curling off the top, behind everything: each puff keeps the sway the plume had when it left the
    // flames, so the wiggle climbs the plume, and the draught bends it over as it rises
    looped(
        t,
        D,
        48,
        s + 300.0,
        |r, _| Plume {
            x: r.range(-5.0, 5.0),
            y: -hc * 0.8 - r.range(0.0, 8.0),
            vx: r.range(-2.0, 2.0),
            vy: -r.range(10.0, 17.0),
            life: r.range(2.0, 2.9),
            s: r.range(2.4, 3.8),
        },
        |q, age, _| {
            let [x, y, ..] = drift(q.x, q.y, q.vx, q.vy, age, 4.0, -20.0, 1.6, 1.0);
            smoke_puff(
                p,
                x + NL.at(t - age, 0.8, s + 60.0, 0.0) * (2.0 + 3.0 * age),
                y,
                1.0 - age / q.life,
                q.s,
                Some(smoke),
                false,
                1.0,
            );
        },
    );
    // flame puffs shed from the tips: they burn out above the fire, some into smoke
    let mut puffs: Vec<[f64; 5]> = Vec::new();
    looped(
        t,
        D,
        n_t * 24,
        s + 200.0,
        |r, j| {
            let i = j % n_t;
            let (fire_t, smokes) = (r.range(0.14, 0.3), r.chance(0.45));
            Shed {
                i,
                fire: fire_t,
                sl: r.range(0.8, 1.4),
                ox: r.range(-1.0, 1.0),
                oy: r.range(0.0, 4.0),
                vx: r.range(-4.0, 4.0),
                vy: -r.range(15.0, 30.0),
                s: r.range(0.8, 1.6) * (0.8 + 0.6 * fire.env(i)),
                f: r.next(),
                life: fire_t + if smokes { 1.4 } else { 0.0 },
            }
        },
        |q, age, _| {
            let (tx, ty) = fire.tip(q.i, t - age);
            let [x, y, vx, vy] = drift(
                tx + q.ox,
                ty + q.oy,
                q.vx,
                q.vy,
                js_min(age, q.fire),
                0.0,
                -95.0,
                5.0,
                3.2,
            );
            if age < q.fire {
                puffs.push([x, y, 1.0 - age / q.fire, q.s, q.f]);
                return;
            }
            if age - q.fire > q.sl {
                return;
            }
            let [sx, sy, ..] = drift(
                x,
                y,
                vx * 0.6,
                vy * 0.6 - 6.0,
                age - q.fire,
                2.0,
                -16.0,
                1.6,
                1.2,
            );
            smoke_puff(
                p,
                sx,
                sy,
                1.0 - (age - q.fire) / q.sl,
                q.s * 0.7,
                None,
                false,
                1.0,
            );
        },
    );

    coals(p, w, &h, t, s);
    for l in &LOGS {
        log(p, l, w, &h, CROSS, 1.0, t, s + 11.0);
    }
    for q in fire.t.iter().filter(|q| q.back) {
        fire.tongue_at(p, t, &h, q);
    }
    // flames licking up off the burning wood
    looped(
        t,
        D,
        90,
        s + 210.0,
        |r, _| Flame {
            x: r.range(-10.0, 10.0),
            y: -r.range(3.0, 8.0),
            vx: r.range(-5.0, 5.0),
            vy: -r.range(20.0, 40.0),
            life: r.range(0.16, 0.3),
            s: r.range(1.0, 1.8),
            f: r.next(),
        },
        |q, age, _| {
            let [x, y, ..] = drift(q.x, q.y, q.vx, q.vy, age, 0.0, -95.0, 5.0, 3.2);
            fire_puff_up(p, x, y, 1.0 - age / q.life, q.s, q.f, &h);
        },
    );
    for q in fire.t.iter().filter(|q| !q.back) {
        fire.tongue_at(p, t, &h, q);
    }
    // licks torn off the tips, rising and shrinking
    looped(
        t,
        D,
        n_t * 12,
        s + 220.0,
        |r, j| Lick {
            i: j % n_t,
            vx: r.range(-3.0, 3.0),
            vy: -r.range(12.0, 22.0),
            s: r.range(0.9, 1.6) * (0.8 + 0.5 * fire.env(j % n_t)),
            life: 0.26,
        },
        |q, age, _| {
            let (tx, ty) = fire.tip(q.i, t - age);
            lick(
                p,
                tx + q.vx * age,
                ty - 2.0 + q.vy * age,
                1.0 - age / q.life,
                q.s,
                &h,
            );
        },
    );
    for &[x, y, k, sz, f] in &puffs {
        fire_puff_up(p, x, y, k, sz, f, &h);
    }
    for l in &LOGS {
        log(p, l, w, &h, 0.0, CROSS, t, s + 11.0);
    }

    // pops: a crack, a flash, and a fistful of sparks thrown up out of the wood that bounce on the floor
    looped(
        t,
        D,
        4,
        s + 400.0,
        |r, _| Pop {
            x: r.range(-9.0, 9.0),
            y: -r.range(4.0, 9.0),
            n: r.int(3.0, 5.0),
            seed: r.int(0.0, 1e6),
            life: 0.7,
        },
        |q, age, _| {
            if age < 0.08 {
                p.px(q.x, q.y, WHITE);
                p.glow(q.x, q.y, 12.0, h[1], 0.7 * (1.0 - age / 0.08));
            }
            let mut r = Rng::new(to_uint32(q.seed));
            let mut j = 0.0;
            while j < q.n {
                j += 1.0;
                let a = -PI / 2.0 + r.range(-0.8, 0.8);
                let sp = r.range(110.0, 220.0);
                let life = r.range(0.25, 0.6);
                if age >= life {
                    continue;
                }
                let [x, y, vx, vy] =
                    spark_at(q.x, q.y, libm::cos(a) * sp, libm::sin(a) * sp, age, 0.0);
                spark(p, x, y, vx, vy, 1.0 - age / life, &sparks);
            }
        },
    );
    // embers carried up by the heat, wandering and cooling
    looped(
        t,
        D,
        36,
        s + 500.0,
        |r, _| Ember {
            x: r.range(-7.0, 7.0),
            y: -r.range(8.0, 18.0),
            vx: r.range(-15.0, 15.0),
            vy: -r.range(25.0, 60.0),
            life: r.range(1.1, 2.5),
            s: if r.chance(0.15) {
                2.0
            } else {
                r.range(1.0, 1.6)
            },
            a: r.range(0.0, 6.3),
            b: r.range(0.0, 6.3),
        },
        |q, age, _| {
            let [x, y, ..] = drift(q.x, q.y, q.vx, q.vy, age, 0.0, -30.0, 1.6, 1.6);
            ember(
                p,
                x + 7.0 * (libm::sin(age * 2.3 + q.a) - libm::sin(q.a)),
                y + 2.0 * (libm::sin(age * 3.0 + q.b) - libm::sin(q.b)),
                1.0 - age / q.life,
                q.s,
                &emb,
                age * 40.0 + q.a,
            );
        },
    );

    // light: a big warm room light, a hot heart, a pool on the floor, all breathing with the flames
    let br = 0.92 + 0.08 * nf(0.67, s + 3.0, 0.0);
    let lx = fire.lean(t) * 12.0;
    p.glow(lx, -8.0, 74.0 * br * (0.9 + 0.1 * fl), glow, 0.3 * fl);
    for (x, y, k) in [(-6.0, -17.0, 0.85), (0.0, -21.0, 1.0), (6.0, -17.0, 0.85)] {
        p.glow(x + lx, y, 46.0 * br, glow, 0.3 * k * fl);
        p.glow(x * 0.6, y + 13.0, 15.0, h[2], 0.16 * k * fl);
    }
    p.glow(0.0, -3.0, 9.0, h[0], 0.16 * fl);
    floor_light(p, 0.0, 0.5, 48.0, 8.0, glow, 0.3 * fl);
}
