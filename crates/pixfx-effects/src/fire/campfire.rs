//! Campfire: port of pixelfx `src/fx/fire/campfire.js`.
//!
//! The tongue, lick, flame-puff, smoke and ember recipe is adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp};
use pixfx_draw::math::{Rng, clamp, js_max, js_min, js_round, rand, to_uint32};

use super::lib::{
    BACK_LAYERS, BIG_LAYERS, Ember, Hot, LAYERS, Layers, SMOKE1, Shed, draw_ember, drift,
    fire_puff, flame_tongue, floor_light, hot_first, lick, looped, nl, smoke_puff, spark, spark_at,
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

pub(super) const FIRE_PALETTE: [Rgba; 9] = [
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
// the fire's height; it sizes no loop
const HEIGHT: pixfx_core::NumOpt = pixfx_core::NumOpt::new("height", 22.0);

const D: f64 = 3.0;
const ASH: [Rgba; 3] = [
    Rgba::hex24(0x2a2530),
    Rgba::hex24(0x4a4350),
    Rgba::hex24(0x6b6270),
];

// the logs, near end first: they cross in an X over the coals, their near ends showing the cut grain and their
// far ends poking out past the flames
const LOGS: [[f64; 5]; 2] = [
    [-19.0, 2.0, 15.0, -12.0, 3.4],
    [19.0, 2.0, -15.0, -12.0, 3.4],
];
// the crossing, as a share of each log's length
const CROSS: f64 = 0.56;

// the fire: (x, base, height (share of o.height), half-width, back row?)
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
            h: hot_first(&opts.palette_or(&FIRE_PALETTE)),
            w: ramp(&opts.ramp(&WOOD)),
            height: opts.num(&HEIGHT),
            seed: seed as f64,
        })
    }
}

struct State {
    h: Hot,
    w: [Rgba; 6],
    height: f64,
    seed: f64,
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
fn log(p: &mut Painter, l: &[f64; 5], w: &[Rgba; 6], h: &Hot, u0: f64, u1: f64, t: f64, s: f64) {
    let [x0, y0, x1, y1, r] = *l;
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = libm::hypot(dx, dy);
    let (ax, ay) = (dx / len, dy / len);
    let (mut nx, mut ny) = (-ay, ax);
    if ny < 0.0 {
        nx = -nx;
        ny = -ny;
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
            if u < u0 || u > u1 || libm::fabs(sv) > 1.0 {
                return None;
            }
            let burn = clamp(1.2 - libm::fabs(x) / 8.0, 0.0, 1.0);
            let lane = js_round(sv * r * 0.85);
            let run = rand(&[libm::floor(along / 3.0), lane, s]);
            if libm::fabs(sv) > rim || along > len - 1.0 {
                return Some(if sv < 0.0 && burn > 0.4 { h[7] } else { w[0] });
            }
            if burn > 0.35 && lane < 2.0 {
                // charred: black, with embers breathing in the cracks along the grain
                if run < 0.55 && lane > -2.0 {
                    let g = 0.5
                        + 0.5
                            * libm::sin(
                                (t / D) * core::f64::consts::PI * 6.0
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
            Some(if lane <= -2.0 {
                if run < 0.25 {
                    w[2]
                } else {
                    mix(w[3], h[4], 0.45 * burn)
                }
            } else if lane == -1.0 {
                if run < 0.45 { w[1] } else { w[2] }
            } else if lane == 0.0 {
                if run < 0.2 { w[1] } else { w[2] }
            } else if run < 0.4 {
                w[0]
            } else {
                w[1]
            })
        },
    );
    // the near end's cut face: a dark rim, pale sapwood, growth rings round a dark heart
    if u0 <= 0.0 {
        p.ellipse(
            x0,
            y0,
            r * 0.62,
            r + 0.2,
            |d: f64, _| {
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
            let g = (0.5 + 0.5 * libm::sin((t / D) * core::f64::consts::PI * 4.0 + hv * 40.0))
                * (1.1 - d);
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

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, w, s, hc) = (&o.h, &o.w, o.seed, o.height);
    let mut r = Rng::new(to_uint32(s));
    let nlt = |tt: f64, rate: f64, seed: f64, off: f64| nl(D, tt, rate, seed, off);
    let n = |rate: f64, seed: f64, off: f64| nl(D, t, rate, seed, off);
    let fla =
        |tt: f64| 0.74 + 0.16 * nlt(tt, 7.3, s + 1.0, 0.0) + 0.1 * nlt(tt, 19.0, s + 2.0, 0.0);
    let fl = fla(t);
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.25);
    let smoke = mix(SMOKE1, h[6], 0.25);
    let sparks = [h[1], h[2], h[4], h[5]];
    let emb = [h[1], h[2], h[4], h[6]];

    let tt_: Vec<Tongue> = TONGUES
        .iter()
        .enumerate()
        .map(|(i, &(dx, y, hh, wv, back))| {
            let dx = dx + r.range(-0.4, 0.4);
            let hv = 3.0 + hc * hh * r.range(0.92, 1.08);
            let wv = wv * r.range(0.9, 1.1);
            Tongue {
                dx,
                y,
                h: hv,
                w: wv,
                back,
                env: hh,
                n: (s * 7.0 + i as f64) * 13.0,
            }
        })
        .collect();
    let nt = tt_.len();
    let lean = |tt: f64| nlt(tt, 0.67, s + 90.0, 0.0) * 0.12;
    let height = |q: &Tongue, tt: f64| {
        let leap = js_max(0.0, nlt(tt, 2.6, q.n + 70.0, 0.0) - 0.55) * 1.4;
        (q.h + 0.25 * q.h * nlt(tt, 3.3, q.n, 0.0) + q.h * leap) * (0.9 + 0.1 * fla(tt))
    };
    let sweep = |q: &Tongue, tt: f64, hh: f64| {
        nlt(tt, 2.2, q.n + 40.0, 0.0) * (1.0 + hh * 0.08) + lean(tt) * hh * 0.9
            - clamp(q.dx * 0.3, -2.5, 2.5)
    };
    let tip = |i: usize, tt: f64| -> (f64, f64) {
        let Some(q) = tt_.get(i) else {
            return (f64::NAN, f64::NAN);
        };
        let hh = height(q, tt);
        (q.dx + sweep(q, tt, hh) * 0.85, q.y - hh * 0.88 + 3.0)
    };
    let tongue_at = |p: &mut Painter, q: &Tongue| {
        let hh = height(q, t);
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
            hh,
            q.w,
            sweep(q, t, hh),
            &n,
            q.n,
            h,
            layers,
            0.5,
            1.1 + hh * 0.04,
        );
    };

    // smoke curling off the top, behind everything: each puff keeps the sway the plume had when it left the
    // flames, so the wiggle climbs the plume, and the draught bends it over as it rises
    looped(
        t,
        D,
        48.0,
        s + 300.0,
        |r, _| {
            let x = r.range(-5.0, 5.0);
            let y = -hc * 0.8 - r.range(0.0, 8.0);
            let vx = r.range(-2.0, 2.0);
            let vy = -r.range(10.0, 17.0);
            let life = r.range(2.0, 2.9);
            let sz = r.range(2.4, 3.8);
            (life, (x, y, vx, vy, life, sz))
        },
        |&(qx, qy, vx, vy, life, sz), age, _| {
            let (x, y, _, _) = drift(qx, qy, vx, vy, age, 4.0, -20.0, 1.6, 1.0);
            smoke_puff(
                p,
                x + nlt(t - age, 0.8, s + 60.0, 0.0) * (2.0 + 3.0 * age),
                y,
                1.0 - age / life,
                sz,
                Some(smoke),
                false,
                1.0,
            );
        },
    );
    // flame puffs shed from the tips: they burn out above the fire, some into smoke
    let mut puffs: Vec<(f64, f64, f64, f64, f64)> = Vec::new();
    looped(
        t,
        D,
        nt as f64 * 24.0,
        s + 200.0,
        |r, j| {
            let i = j % nt;
            let env = tt_.get(i).map_or(0.0, |q| q.env);
            let fire = r.range(0.14, 0.3);
            let smokes = r.chance(0.45);
            let q = Shed {
                i,
                fire,
                sl: r.range(0.8, 1.4),
                ox: r.range(-1.0, 1.0),
                oy: r.range(0.0, 4.0),
                vx: r.range(-4.0, 4.0),
                vy: -r.range(15.0, 30.0),
                s: r.range(0.8, 1.6) * (0.8 + 0.6 * env),
                f: r.next(),
            };
            (fire + if smokes { 1.4 } else { 0.0 }, q)
        },
        |q, age, _| {
            let (tx, ty) = tip(q.i, t - age);
            let (x, y, vx, vy) = drift(
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
                puffs.push((x, y, 1.0 - age / q.fire, q.s, q.f));
                return;
            }
            if age - q.fire > q.sl {
                return;
            }
            let (sx, sy, _, _) = drift(
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

    coals(p, w, h, t, s);
    for l in &LOGS {
        log(p, l, w, h, CROSS, 1.0, t, s + 11.0);
    }
    for q in tt_.iter().filter(|q| q.back) {
        tongue_at(p, q);
    }
    // flames licking up off the burning wood
    looped(
        t,
        D,
        90.0,
        s + 210.0,
        |r, _| {
            let x = r.range(-10.0, 10.0);
            let y = -r.range(3.0, 8.0);
            let vx = r.range(-5.0, 5.0);
            let vy = -r.range(20.0, 40.0);
            let life = r.range(0.16, 0.3);
            let sz = r.range(1.0, 1.8);
            let f = r.next();
            (life, (x, y, vx, vy, life, sz, f))
        },
        |&(qx, qy, vx, vy, life, sz, f), age, _| {
            let (x, y, _, _) = drift(qx, qy, vx, vy, age, 0.0, -95.0, 5.0, 3.2);
            fire_puff(p, x, y, 1.0 - age / life, sz, f, h, 0.18, 0.0, -1.0);
        },
    );
    for q in tt_.iter().filter(|q| !q.back) {
        tongue_at(p, q);
    }
    // licks torn off the tips, rising and shrinking
    looped(
        t,
        D,
        nt as f64 * 12.0,
        s + 220.0,
        |r, j| {
            let i = j % nt;
            let env = tt_.get(i).map_or(0.0, |q| q.env);
            let vx = r.range(-3.0, 3.0);
            let vy = -r.range(12.0, 22.0);
            let sz = r.range(0.9, 1.6) * (0.8 + 0.5 * env);
            (0.26, (i, vx, vy, sz))
        },
        |&(i, vx, vy, sz), age, _| {
            let (tx, ty) = tip(i, t - age);
            lick(
                p,
                tx + vx * age,
                ty - 2.0 + vy * age,
                1.0 - age / 0.26,
                sz,
                h,
            );
        },
    );
    for &(x, y, k, sz, f) in &puffs {
        fire_puff(p, x, y, k, sz, f, h, 0.18, 0.0, -1.0);
    }
    for l in &LOGS {
        log(p, l, w, h, 0.0, CROSS, t, s + 11.0);
    }

    // pops: a crack, a flash, and a fistful of sparks thrown up out of the wood that bounce on the floor
    looped(
        t,
        D,
        4.0,
        s + 400.0,
        |r, _| {
            let x = r.range(-9.0, 9.0);
            let y = -r.range(4.0, 9.0);
            let n = r.int(3.0, 5.0);
            let seed = r.int(0.0, 1e6);
            (0.7, (x, y, n, seed))
        },
        |&(qx, qy, qn, seed), age, _| {
            if age < 0.08 {
                p.px(qx, qy, WHITE);
                p.glow(qx, qy, 12.0, h[1], 0.7 * (1.0 - age / 0.08));
            }
            let mut r = Rng::new(to_uint32(seed));
            let mut j = 0.0;
            while j < qn {
                j += 1.0;
                let a = -core::f64::consts::FRAC_PI_2 + r.range(-0.8, 0.8);
                let sp = r.range(110.0, 220.0);
                let life = r.range(0.25, 0.6);
                if age >= life {
                    continue;
                }
                let (x, y, vx, vy) =
                    spark_at(qx, qy, libm::cos(a) * sp, libm::sin(a) * sp, age, 0.0);
                spark(p, x, y, vx, vy, 1.0 - age / life, &sparks);
            }
        },
    );
    // embers carried up by the heat, wandering and cooling
    looped(
        t,
        D,
        36.0,
        s + 500.0,
        |r, _| {
            let x = r.range(-7.0, 7.0);
            let y = -r.range(8.0, 18.0);
            let vx = r.range(-15.0, 15.0);
            let vy = -r.range(25.0, 60.0);
            let life = r.range(1.1, 2.5);
            let sz = if r.chance(0.15) {
                2.0
            } else {
                r.range(1.0, 1.6)
            };
            let a = r.range(0.0, 6.3);
            let b = r.range(0.0, 6.3);
            (
                life,
                Ember {
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    s: sz,
                    a,
                    b,
                },
            )
        },
        |q, age, _| draw_ember(p, 0.0, 0.0, q, age, &emb),
    );

    // light: a big warm room light, a hot heart, a pool on the floor, all breathing with the flames
    let br = 0.92 + 0.08 * n(0.67, s + 3.0, 0.0);
    let lx = lean(t) * 12.0;
    p.glow(lx, -8.0, 74.0 * br * (0.9 + 0.1 * fl), glow, 0.3 * fl);
    for (x, y, k) in [(-6.0, -17.0, 0.85), (0.0, -21.0, 1.0), (6.0, -17.0, 0.85)] {
        p.glow(x + lx, y, 46.0 * br, glow, 0.3 * k * fl);
        p.glow(x * 0.6, y + 13.0, 15.0, h[2], 0.16 * k * fl);
    }
    p.glow(0.0, -3.0, 9.0, h[0], 0.16 * fl);
    floor_light(p, 0.0, 0.5, 48.0, 8.0, glow, 0.3 * fl);
}
