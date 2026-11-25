//! Water orb: port of pixelfx `src/fx/water/water-orb.js`.
//!
//! The orb's build (a see-through body with a lit rim, an inner current drawn as streaks on a tilted plane, motes
//! on tilted orbits split behind and in front, a pulsing light) follows Sprite Fusion's open-source energy orb
//! (https://www.spritefusion.com/pixel-art-vfx/energy), MIT License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::{PI, SQRT_2};

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{WHITE, ramp};
use pixfx_draw::math::{TAU, js_max, smooth};

use super::lib::{Ramp, RingOpts, WATER, bead, js_hypot, ring, shade_with};

pub struct WaterOrb;

pub static WATER_ORB: WaterOrb = WaterOrb;

const RADIUS: NumOpt = NumOpt::new("radius", 19.0).within(2.0, 40.0);
const META: Meta = Meta {
    id: "water/water-orb",
    name: "Water orb",
    desc: "A sphere of water floats and wobbles: a current circles inside it, the floor shows upside down through it, it shines, light gathers on the floor under it, and drips fall and plink.",
    tags: &[
        "water", "orb", "sphere", "magic", "float", "ambient", "loop",
    ],
    size: (100, 108),
    origin: (50, 100),
    duration: 3.0,
    looping: true,
    fps: 30,
    seed: 2,
    state: None,
    options: &[RADIUS.spec()],
};

const D: f64 = 3.0;
const SY: f64 = 0.3;
const G: f64 = 700.0;
const DRIP: f64 = 1.5;
const LET: f64 = 0.5;

impl Effect for WaterOrb {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            radius: opts.num(&RADIUS),
            seed: seed as f64,
        })
    }
}

struct State {
    c: Ramp,
    radius: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            draw_back(p, t, self);
        }
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

fn center(ph: f64) -> (f64, f64) {
    (0.0, -52.0 + libm::sin(ph * TAU) * 2.0)
}

// the skin wobbles: radius at angle a, and the belly sags into a drip before each one falls
fn skin(a: f64, r: f64, ph: f64) -> f64 {
    let at = ((ph * D) % DRIP) / DRIP;
    let sag = libm::pow(js_max(0.0, libm::sin(a)), 6.0)
        * 2.2
        * smooth(0.1, LET, at)
        * (if at < LET { 1.0 } else { 0.0 });
    r + libm::sin(a * 3.0 + ph * TAU * 2.0) * 0.6 + libm::sin(a * 5.0 - ph * TAU * 3.0) * 0.35 + sag
}

struct Mote {
    x: f64,
    y: f64,
    z: f64,
    i: usize,
}

// motes of spray on tilted orbits round it: position, and whether it's in front
fn motes(ph: f64, r: f64) -> [Mote; 7] {
    core::array::from_fn(|i| {
        let fi = i as f64;
        let a = (fi / 7.0) * TAU + ph * TAU * (if i % 2 == 1 { 1.0 } else { -1.0 });
        let rr = r + 5.0 + ((i % 3) as f64) * 3.0;
        let tilt = (fi - 3.0) * 0.22;
        let (x, z) = (libm::cos(a) * rr, libm::sin(a));
        Mote {
            x: x * libm::cos(tilt),
            y: x * libm::sin(tilt) + z * rr * 0.28,
            z,
            i,
        }
    })
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let ph = t / D;
    let r = o.radius;
    let (cx, cy) = center(ph);
    // light gathers on the floor under it, the orb's caustic
    p.glow(cx, 0.0, 22.0, c[5], 0.32);
    p.shade(cx - 9.0, -2.0, cx + 9.0, 2.0, |x, y, out| {
        let d = libm::pow(x - cx, 2.0) / 64.0 + libm::pow(y * 2.5, 2.0) / 9.0;
        if d > 1.0 {
            return None;
        }
        out.c = if d < 0.25 {
            WHITE
        } else if d < 0.6 {
            c[6]
        } else {
            c[5]
        };
        out.a = 1.2 - d;
        None
    });
    for m in motes(ph, r) {
        if m.z < 0.0 {
            p.px(cx + m.x, cy + m.y, c[4]);
        }
    }
    p.glow(
        cx,
        cy,
        r * 3.2,
        c[4],
        0.3 + 0.05 * libm::sin(ph * TAU * 2.0),
    );
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let ph = t / D;
    let rad = o.radius;
    let (cx, cy) = center(ph);
    let tt = ph * TAU;
    // the body: see-through water, deeper toward the bottom; the floor seen upside down through its lower half
    shade_with(
        p,
        cx - rad - 4.0,
        cy - rad - 4.0,
        cx + rad + 4.0,
        cy + rad + 6.0,
        |p, x, y, out| {
            let (dx, dy) = (x - cx, y - cy);
            let d = js_hypot(dx, dy);
            let a = libm::atan2(dy, dx);
            let r = skin(a, rad, ph);
            if d > r {
                return;
            }
            let q = d / r;
            let lit = (-dx - dy) / SQRT_2 / js_max(0.5, d);
            if q > 0.9 {
                out.c = if lit > 0.35 {
                    WHITE
                } else if lit > -0.2 {
                    c[5]
                } else {
                    c[3]
                };
                return;
            }
            // a lens of water: dark on the side the light comes from, the light gathering low on the far side
            let gather = js_max(0.0, (dx + dy) / SQRT_2 / r) * q;
            let col = if gather > 0.45 {
                c[4]
            } else if gather > 0.25 {
                c[3]
            } else if q > 0.7 {
                c[2]
            } else {
                c[1]
            };
            p.blend(x, y, col, if gather > 0.25 { 0.7 } else { 0.35 + 0.3 * q });
        },
    );
    // the current inside: streaks circling on a tilted plane, bright where they pass in front
    for k in 0..3 {
        let kf = f64::from(k);
        let rr = rad * (0.38 + 0.2 * kf);
        let tilt = -0.5 + kf * 0.35;
        let spin = tt * (if k % 2 == 1 { -2.0 } else { 3.0 }) + kf * 2.0;
        let mut s = 0.0;
        while s < 1.0 {
            let a = spin + s * TAU;
            let (x, z) = (libm::cos(a) * rr, libm::sin(a));
            let y = x * libm::sin(tilt) * 0.6 + z * rr * 0.62;
            let dash = (s * 3.0 + kf * 0.3) % 1.0;
            s += 0.012;
            if dash > 0.62 {
                continue;
            }
            let col = if z > 0.0 {
                if dash < 0.1 {
                    WHITE
                } else if dash < 0.4 {
                    c[6]
                } else {
                    c[5]
                }
            } else {
                c[3]
            };
            p.px(cx + x * libm::cos(tilt), cy + y, col);
        }
    }
    // bubbles drift up inside it
    cycle(
        t,
        D,
        8,
        o.seed,
        |g| g.range(1.0, 1.8),
        |age, life, r, _| {
            let k = age / life;
            let x = r.range(-0.5, 0.5) * rad + libm::sin(age * 4.0 + r.range(0.0, TAU)) * 1.5;
            let y = rad * 0.6 - k * rad * 1.2;
            if js_hypot(x, y) > rad * 0.8 {
                return;
            }
            p.px(cx + x, cy + y, if k < 0.8 { c[6] } else { WHITE });
            if r.next() < 0.35 {
                p.px(cx + x + 1.0, cy + y, c[5]);
            }
        },
    );
    // the shine: a bright crescent up on the left, a small refracted glint low on the right
    let (c6, white) = (c[6], WHITE);
    p.arc(
        cx,
        cy,
        rad * 0.74,
        1.8,
        PI * 1.12,
        PI * 1.4,
        move |u: f64, v: f64| {
            if u > 0.25 && u < 0.75 && v > 0.4 {
                white
            } else {
                c6
            }
        },
        1.0,
    );
    p.px(cx - rad * 0.45, cy - rad * 0.62, WHITE);
    p.px(cx + rad * 0.48, cy + rad * 0.5, c[6]);
    p.px(cx + rad * 0.48 + 1.0, cy + rad * 0.5, c[5]);
    // motes in front
    for m in motes(ph, rad) {
        if m.z >= 0.0 {
            p.px(cx + m.x, cy + m.y, WHITE);
            p.px(
                cx + m.x - (if m.i % 2 == 1 { -1.0 } else { 1.0 }),
                cy + m.y,
                c[5],
            );
            p.glow(cx + m.x, cy + m.y, 4.0, c[5], 0.35);
        }
    }
    // drips: one swells on its belly and lets go every DRIP seconds, landing with a plink and a ring
    let dt = (t % DRIP) / DRIP;
    let fall = (t % DRIP) - DRIP * LET;
    let y0 = cy + rad + 3.0;
    let land = libm::sqrt((2.0 * -y0) / G);
    if fall < 0.0 && dt > 0.2 {
        let s = smooth(0.2, LET, dt);
        p.circle(cx, cy + rad + 1.0 + s * 1.5, 0.8 + s * 1.2, c[5], 1.0);
        p.px(cx - 0.5, cy + rad + s * 1.5, c[6]);
    } else if fall >= 0.0 && fall < land {
        bead(
            p,
            cx,
            y0 + 0.5 * G * fall * fall,
            0.0,
            G * fall,
            2.0,
            c,
            1.0,
        );
    } else if fall >= land {
        let since = fall - land;
        if since < 0.08 {
            p.px(cx, 0.0, WHITE);
        }
        ring(
            p,
            cx,
            0.0,
            1.5 + since * 18.0,
            SY,
            1.1 * (1.0 - since / 0.4),
            c,
            RingOpts {
                n: 1.0,
                half: 1.0,
                seed: 3.0,
                hot: since < 0.05,
                ..RingOpts::default()
            },
        );
    }
    // where the lens brings the light to a point, low on the far side: a bright fleck, pulsing
    let (fx, fy) = (cx + rad * 0.36, cy + rad * 0.4);
    let pulse = 0.8 + 0.2 * libm::sin(tt * 4.0);
    p.circle(fx, fy, 1.6, c[6], 1.0);
    p.px(fx, fy, WHITE);
    p.glow(fx, fy, 9.0, c[5], 0.45 * pulse);
    p.glow(cx, cy, rad * 1.4, c[5], 0.22);
    p.glow(cx - rad * 0.4, cy - rad * 0.5, 6.0, WHITE, 0.25);
}
