//! Fountain: port of pixelfx `src/fx/water/fountain.js`.
//!
//! The basin's caustic light (two warped sine nets, light where they cross zero) is adapted from Sprite Fusion's
//! open-source bubbles (https://www.spritefusion.com/pixel-art-vfx/bubbles), MIT License, Copyright (c) 2026 Hugo
//! Duprez.

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{TAU, clamp, js_max, rand, tile_noise};

use super::lib::{CROSS, ROCK, Ramp, RingOpts, WATER, bead, ring};

pub struct Fountain;

pub static FOUNTAIN: Fountain = Fountain;

const META: Meta = Meta {
    id: "water/fountain",
    name: "Fountain",
    desc: "A stone basin; a jet rises from its spout and blooms into glassy arcs that break into beads, fall back and plink into the water with rings.",
    tags: &[
        "water", "fountain", "jet", "basin", "prop", "ambient", "loop",
    ],
    size: (120, 104),
    origin: (60, 96),
    duration: 2.0,
    looping: true,
    fps: 30,
    seed: 3,
    state: None,
    options: &[],
};

const D: f64 = 2.0;
const SY: f64 = 0.34;
const G: f64 = 600.0;
const RIM: f64 = 10.0;
const RX: f64 = 36.0;
const IN: f64 = 29.5;
const JET: f64 = 54.0;
const ARCS: usize = 12;
// the basin stands on the floor: its base ellipse touches y = 0 at the front; heights are measured from that plane
const BY: f64 = -RX * SY;
const LEVEL: f64 = RIM - 1.2;

impl Effect for Fountain {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let s = seed as f64;
        Box::new(State {
            c: ramp(&opts.palette_or(&WATER)),
            s,
            q: arcs(s),
        })
    }
}

struct State {
    c: Ramp,
    s: f64,
    q: [Arc; ARCS],
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

#[derive(Clone, Copy)]
struct Arc {
    a: f64,
    vr: f64,
    vh: f64,
    tl: f64,
    ca: f64,
    sa: f64,
}

// the arcs: water thrown out of the top of the jet in every direction, each a parabola back down to the basin
fn arcs(s: f64) -> [Arc; ARCS] {
    core::array::from_fn(|i| {
        let fi = i as f64;
        let a = ((fi + 0.5) / ARCS as f64) * TAU + rand(&[fi, s]) * 0.25;
        let vr = 38.0 + 14.0 * rand(&[fi, s + 1.0]);
        let vh = 30.0 + 26.0 * rand(&[fi, s + 2.0]);
        let tl = (vh + libm::sqrt(vh * vh + 2.0 * G * (JET - LEVEL))) / G;
        Arc {
            a,
            vr,
            vh,
            tl,
            ca: libm::cos(a),
            sa: libm::sin(a),
        }
    })
}

// a point along an arc tau seconds into its flight: screen x, y and the height above the floor
fn along(q: &Arc, tau: f64) -> (f64, f64) {
    let r = q.vr * tau;
    let h = JET + q.vh * tau - 0.5 * G * tau * tau;
    (q.ca * r, BY + q.sa * r * SY - h)
}

// the stone basin: a wide rim lit toward the light with a dark line round both edges, a wall of stone blocks
// below its near side (lit left, shaded right) standing on the floor
fn in_ell(x: f64, y: f64, rx: f64, cy: f64) -> bool {
    libm::pow(x / rx, 2.0) + libm::pow((y - cy) / (rx * SY), 2.0) <= 1.0
}

fn basin(p: &mut Painter, st: &[Rgba; 6], half: f64) {
    let cy = BY - RIM;
    let ry = RX * SY;
    p.shade(-RX - 1.0, cy - ry - 1.0, RX + 1.0, 1.0, |x, y, out| {
        if in_ell(x, y, RX, cy) {
            if in_ell(x, y, IN, cy) {
                return None;
            }
            let near = y > cy;
            if near != (half > 0.0) {
                return None;
            }
            let outer = !in_ell(x, y - 1.0, RX, cy)
                || !in_ell(x, y + 1.0, RX, cy)
                || !in_ell(x - 1.0, y, RX, cy)
                || !in_ell(x + 1.0, y, RX, cy);
            let inner = in_ell(x, y - 1.0, IN, cy)
                || in_ell(x, y + 1.0, IN, cy)
                || in_ell(x - 1.0, y, IN, cy)
                || in_ell(x + 1.0, y, IN, cy);
            let a = libm::atan2((y - cy) / ry, x / RX);
            let lit = -libm::cos(a) * 0.65 - libm::sin(a) * 0.45;
            out.c = if outer {
                st[0]
            } else if inner {
                if y < cy { st[2] } else { st[1] }
            } else if lit > 0.4 {
                st[5]
            } else if lit > -0.05 {
                st[4]
            } else if lit > -0.45 {
                st[3]
            } else {
                st[2]
            };
            return None;
        }
        // the wall, under the near half of the rim
        if half < 0.0 || libm::fabs(x) >= RX {
            return None;
        }
        let dx = x / RX;
        let top = cy + libm::sqrt(1.0 - dx * dx) * ry;
        let base = top + RIM;
        if y < top || y > base {
            return None;
        }
        let u = libm::asin(clamp(dx, 0.0, 1.0)) * RX * 0.62;
        let row = y - top;
        let course = if row > RIM * 0.5 { 1.0 } else { 0.0 };
        let joint = (((((u + 40.0 + course * 4.5) / 9.0) % 1.0) + 1.0) % 1.0) < 0.1;
        out.c = if y > base - 1.0 || libm::fabs(x) > RX - 1.0 || row < 1.0 {
            st[0]
        } else if joint || libm::fabs(row - RIM * 0.5) < 0.5 {
            st[1]
        } else if dx < -0.4 {
            st[4]
        } else if dx < 0.3 {
            st[3]
        } else {
            st[2]
        };
        if (1.0..2.0).contains(&row) && !joint {
            out.c = if dx < 0.0 { st[2] } else { st[1] };
        }
        None
    });
}

// the water in the basin: dark and see-through, a lit band at the far side, caustic light swimming in it
fn pool(p: &mut Painter, c: &Ramp, ph: f64) {
    let rx = IN;
    let ry = rx * SY;
    let cy = BY - LEVEL;
    let tt = ph * TAU;
    let mut yy = libm::floor(cy - ry);
    while yy < cy + ry {
        let mut xx = -libm::ceil(rx);
        while xx < rx {
            let (x, y) = (xx + 0.5, yy + 0.5);
            xx += 1.0;
            let (dx, dy) = (x / rx, (y - cy) / ry);
            let d = dx * dx + dy * dy;
            if d > 1.0 {
                continue;
            }
            p.blend(
                x,
                y,
                if dy < -0.55 { c[3] } else { c[1] },
                if dy < -0.55 { 0.8 } else { 0.85 },
            );
            let u = x * 0.25 + 1.6 * libm::sin(y * 0.6 + tt);
            let w = y * 0.7 + 1.6 * libm::sin(x * 0.14 - tt * 2.0);
            let f = js_max(0.0, 1.0 - libm::fabs(libm::sin(u) + libm::sin(w)) * 2.6) * (1.0 - d);
            if f > 0.35 {
                p.light(x, y, c[5], 0.3 * f);
            }
        }
        yy += 1.0;
    }
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let ph = t / D;
    basin(p, &ROCK, -1.0);
    pool(p, c, ph);
    // the arcs on the far side, dimmer, behind the jet
    for q in &o.q {
        if q.sa < 0.0 {
            arc(p, c, q, ph, true);
        }
    }
}

// one arc: a glassy stream that thins as it falls and breaks into beads at the end; beads slide down it
fn arc(p: &mut Painter, c: &Ramp, q: &Arc, ph: f64, back: bool) {
    let mut prev: Option<(f64, f64)> = None;
    let mut tau = 0.0;
    while tau <= q.tl {
        let (x, y) = along(q, tau);
        let u = tau / q.tl;
        if let Some((px, py)) = prev
            && u < 0.66
        {
            let col = if back {
                if u < 0.25 { c[5] } else { c[4] }
            } else if u < 0.2 {
                WHITE
            } else if u < 0.45 {
                c[6]
            } else {
                c[5]
            };
            p.line(px, py, x, y, col, 1.0);
        }
        prev = Some((x, y));
        tau += 0.012;
    }
    // the beads it breaks into, sliding down at the water's own speed
    let dim: Ramp = [c[0], c[1], c[2], c[2], c[3], c[4], c[5], c[5]];
    for k in 0..6 {
        let u = ((ph * 2.0 + f64::from(k) / 6.0) % 1.0) * 0.42 + 0.58;
        let tau = u * q.tl;
        let (x, y) = along(q, tau);
        let vy = -(q.vh - G * tau) + q.sa * q.vr * SY;
        let s = if back {
            1.0
        } else if k % 3 == 0 {
            2.0
        } else {
            1.0
        };
        bead(
            p,
            x,
            y,
            q.ca * q.vr,
            vy,
            s,
            if back { &dim } else { c },
            1.0,
        );
    }
    // and the ones sliding along the stream itself catch the light
    let g = ((ph * 4.0 + q.a) % 1.0) * 0.6;
    let (gx, gy) = along(q, g * q.tl);
    p.px(gx, gy, WHITE);
    if !back {
        p.glow(gx, gy, 3.0, c[5], 0.3);
    }
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (c, s) = (&o.c, o.s);
    let st = &ROCK;
    let ph = t / D;
    let tt = ph * TAU;
    // the spout: a little stone stem in the middle of the basin
    p.shade(-3.5, BY - 16.0, 3.5, BY - LEVEL + 0.5, |x, y, out| {
        let w = if y < BY - 14.0 { 3.4 } else { 2.2 };
        if libm::fabs(x) > w {
            return None;
        }
        out.c = if libm::fabs(x) > w - 0.8 || y < BY - 15.5 {
            st[0]
        } else if x < -0.6 {
            st[4]
        } else if x < 0.8 {
            st[3]
        } else {
            st[2]
        };
        None
    });
    // the jet: a glassy column, white down its middle, wobbling a little at the top
    p.shade(-3.0, BY - JET - 1.0, 3.0, BY - 16.0, |x, y, out| {
        let v = (BY - 16.0 - y) / (JET - 16.0);
        let sway = libm::sin(tt * 2.0 + v * 5.0) * 0.5 * v;
        let w = 1.3 + 0.5 * v + 0.3 * libm::sin(tt * 3.0 + y * 0.4);
        let q = (x - sway) / w;
        if libm::fabs(q) > 1.0 {
            return None;
        }
        let n = tile_noise(x * 0.9, y * 0.15 + ph * 16.0, 0.0, 16.0, s);
        out.c = if q < -0.4 {
            c[6]
        } else if q < 0.35 {
            if n > -0.3 { WHITE } else { c[6] }
        } else {
            c[5]
        };
        None
    });
    // the bloom where it opens out at the top
    p.circle(0.0, BY - JET - 1.0, 2.4, c[6], 1.0);
    p.circle(-0.4, BY - JET - 1.5, 1.4, WHITE, 1.0);
    p.glow(0.0, BY - JET, 22.0, c[5], 0.46 + 0.06 * libm::sin(tt * 4.0));
    p.glow(0.0, BY - JET - 1.0, 8.0, WHITE, 0.3);
    // glints wink along the arcs
    cycle(
        t,
        D,
        10,
        s + 30.0,
        |g| g.range(0.2, 0.35),
        |age, life, r, _| {
            let idx = libm::floor(r.next() * ARCS as f64) as usize;
            let Some(q) = o.q.get(idx) else {
                return;
            };
            let (x, y) = along(q, r.range(0.1, 0.7) * q.tl);
            let k = libm::sin((core::f64::consts::PI * age) / life);
            if q.sa < -0.3 {
                return;
            }
            p.px(x, y, WHITE);
            if k > 0.6 {
                for (dx, dy) in CROSS {
                    p.px(x + dx, y + dy, c[6]);
                }
            }
            p.glow(x, y, 5.0, c[5], 0.4 * k);
        },
    );
    // foam where the jet leaves the spout
    for i in 0..4 {
        let fi = f64::from(i);
        p.px(
            libm::sin(tt * 3.0 + fi * 1.7) * 2.5,
            BY - 16.0 - f64::from(i % 2),
            if i % 2 == 1 { c[6] } else { WHITE },
        );
    }
    // the arcs on the near side, in front of the jet
    for q in &o.q {
        if q.sa >= 0.0 {
            arc(p, c, q, ph, false);
        }
    }
    // where they come down: near halves of the rings, flecks and tiny crowns thrown up, light on the water
    for (i, q) in o.q.iter().enumerate() {
        let fi = i as f64;
        let f = (ph * 2.0 + rand(&[fi, s + 9.0])) % 1.0;
        let (x, y) = along(q, q.tl);
        ring(
            p,
            x,
            y,
            2.0 + f * 8.0,
            SY,
            0.8 * libm::pow(1.0 - f, 1.3),
            c,
            RingOpts {
                n: 1.0,
                half: 1.0,
                seed: fi,
                ..RingOpts::default()
            },
        );
        let g = (ph * 6.0 + rand(&[fi, s + 10.0])) % 1.0;
        if g < 0.15 {
            p.px(x, y - 1.0, WHITE);
        }
        if g < 0.3 && q.sa > -0.2 {
            p.line(
                x,
                y - 1.0,
                x + (rand(&[fi, s + 11.0]) - 0.5) * 2.0,
                y - 1.0 - 3.0 * (1.0 - g / 0.3),
                c[6],
                1.0,
            );
        }
        p.glow(x, y, 6.0, c[5], 0.18 * (1.0 - g));
    }
    basin(p, st, 1.0);
    // a fine mist hangs round the bloom
    cycle(
        t,
        D,
        16,
        s + 20.0,
        |g| g.range(0.8, 1.4),
        |age, life, r, _| {
            let k = age / life;
            let a = r.range(0.0, TAU);
            let rr = r.range(4.0, 20.0) + k * 6.0;
            let x = libm::cos(a) * rr;
            let y = BY - JET + r.range(-4.0, 16.0) + k * 10.0;
            if k > 0.75 && r.next() < (k - 0.75) * 4.0 {
                return;
            }
            p.px(x, y, if r.next() < 0.4 { WHITE } else { c[6] });
        },
    );
    // light: the jet, and the basin water lit from above
    p.glow_line(0.0, BY - 16.0, 0.0, BY - JET, 10.0, c[5], 0.18);
    p.glow(
        0.0,
        BY - LEVEL - 2.0,
        46.0,
        c[4],
        0.26 + 0.03 * libm::sin(tt * 2.0),
    );
    p.glow(0.0, BY - LEVEL - 20.0, 30.0, c[5], 0.12);
}
