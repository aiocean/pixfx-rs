//! Ice crystal: port of pixelfx `src/fx/ice/ice-crystal.js`.
//!
//! A floating pickup that bobs, glints, glows and has motes circling in front and behind follows the recipe in
//! Sprite Fusion's open-source pickup gem (https://www.spritefusion.com/pixel-art-vfx/pickup), MIT License,
//! Copyright (c) 2026 Hugo Duprez.

use pixfx_core::particles::cycle;
use pixfx_core::sim::sparkle;
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{TAU, clamp, js_max, js_min, js_round};

use super::lib::{PALETTE, Pt, js_hypot, step};

pub struct IceCrystal;

pub static ICE_CRYSTAL: IceCrystal = IceCrystal;

const META: Meta = Meta {
    id: "ice/ice-crystal",
    name: "Ice crystal",
    desc: "A floating ice crystal turning slowly in its own cold light: its facets brighten and darken as they turn, glints flash on them, frost motes circle it.",
    tags: &["ice", "crystal", "gem", "pickup", "float", "magic", "loop"],
    size: (96, 100),
    origin: (48, 92),
    duration: D,
    looping: true,
    fps: 30,
    seed: 21,
    state: None,
    options: &[HEIGHT.spec()],
};

const HEIGHT: NumOpt = NumOpt::new("height", 34.0);
const D: f64 = 3.0;
/// How far it leans toward us, so the upper facets show.
const TIP: f64 = 0.28;

type V3 = [f64; 3];

fn norm([x, y, z]: V3) -> V3 {
    let l = js_hypot(&[x, y, z]);
    let l = if l == 0.0 { 1.0 } else { l };
    [x / l, y / l, z / l]
}

/// Toward the light: upper left, in front.
fn light() -> V3 {
    norm([-0.55, -0.75, -0.45])
}

impl Effect for IceCrystal {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&PALETTE)),
            height: opts.num(&HEIGHT),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    height: f64,
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

/// A visible face: projected corners, brightness, glint, centre.
struct Face {
    pts: Vec<Pt>,
    lit: f64,
    spec: f64,
    cx: f64,
    cy: f64,
}

/// The crystal: a hexagonal prism with a tall point above and a shorter one below, turned by th, tipped toward the
/// viewer. Returns the visible faces.
fn crystal_faces(th: f64, r: f64) -> Vec<Face> {
    let (ct, st) = (libm::cos(TIP), libm::sin(TIP));
    // tip toward the viewer (z toward us is negative)
    let pr = |x: f64, y: f64, z: f64| -> V3 { [x, y * ct - z * st, y * st + z * ct] };
    let ring = |y: f64| -> [V3; 6] {
        core::array::from_fn(|k| {
            let a = th + (k as f64 * TAU) / 6.0;
            pr(libm::cos(a) * r, y, libm::sin(a) * r)
        })
    };
    let (top, bot) = (ring(-r * 0.45), ring(r * 0.45));
    let (a, z) = (pr(0.0, -r * 2.2, 0.0), pr(0.0, r * 1.7, 0.0));
    let mut faces: Vec<Vec<V3>> = Vec::with_capacity(18);
    for k in 0..6 {
        let j = (k + 1) % 6;
        faces.push(vec![a, top[j], top[k]]);
        faces.push(vec![top[k], top[j], bot[j], bot[k]]);
        faces.push(vec![bot[k], bot[j], z]);
    }
    let l = light();
    let h = norm([l[0], l[1], l[2] - 1.0]);
    let mut out = Vec::new();
    for f in faces {
        let (a, b, c) = (f[0], f[1], f[2]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let mut n = norm([
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]);
        let len = f.len() as f64;
        let (mut cx, mut cy, mut cz) = (0.0, 0.0, 0.0);
        for q in &f {
            cx += q[0];
        }
        for q in &f {
            cy += q[1];
        }
        for q in &f {
            cz += q[2];
        }
        let (cx, cy, cz) = (cx / len, cy / len, cz / len);
        if n[0] * cx + n[1] * cy + n[2] * cz < 0.0 {
            n = [-n[0], -n[1], -n[2]]; // outward
        }
        if n[2] >= -0.02 {
            continue; // facing away
        }
        let lit = js_max(0.0, n[0] * l[0] + n[1] * l[1] + n[2] * l[2]);
        // a glint where the face mirrors the light toward the viewer (half way between the light and the eye)
        let spec = js_max(0.0, n[0] * h[0] + n[1] * h[1] + n[2] * h[2]);
        out.push(Face {
            pts: f.iter().map(|q| (q[0], q[1])).collect(),
            lit,
            spec,
            cx,
            cy,
        });
    }
    out
}

/// The convex hull of a set of points (monotone chain), for the crystal's silhouette.
fn hull_of(pts: &[Pt]) -> Vec<Pt> {
    let mut ps = pts.to_vec();
    ps.sort_by(|a, b| {
        let d = a.0 - b.0;
        let d = if d == 0.0 || d.is_nan() { a.1 - b.1 } else { d };
        d.partial_cmp(&0.0).unwrap_or(core::cmp::Ordering::Equal)
    });
    let cross = |o: Pt, a: Pt, b: Pt| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut lo: Vec<Pt> = Vec::new();
    for &q in &ps {
        while lo.len() >= 2 && cross(lo[lo.len() - 2], lo[lo.len() - 1], q) <= 0.0 {
            lo.pop();
        }
        lo.push(q);
    }
    let mut hi: Vec<Pt> = Vec::new();
    for &q in ps.iter().rev() {
        while hi.len() >= 2 && cross(hi[hi.len() - 2], hi[hi.len() - 1], q) <= 0.0 {
            hi.pop();
        }
        hi.push(q);
    }
    lo.pop();
    hi.pop();
    lo.extend(hi);
    lo
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let ph = t / D;
    let bob = libm::sin(TAU * ph) * 2.5;
    let y = -o.height + bob;
    // its shadow on the floor, smaller and darker as it dips, and a cold pool of light
    let s = 1.0 - bob / 12.0;
    let rx = 11.0 * s;
    let mut x = libm::floor(-rx);
    while x <= rx {
        let f = 1.0 - libm::pow(x / rx, 2.0);
        p.blend(x + 0.5, 0.0, c[0], 0.65 * f);
        if libm::fabs(x) < rx * 0.6 {
            p.blend(x + 0.5, 1.0, c[0], 0.4 * f);
        }
        x += 1.0;
    }
    p.glow(
        0.0,
        -1.0,
        24.0,
        c[4],
        0.2 + 0.05 * libm::sin(TAU * ph * 2.0),
    );
    motes(p, c, t, o.seed, y, false);
}

fn motes(p: &mut Painter, c: &[Rgba; 8], t: f64, seed: f64, y0: f64, front: bool) {
    cycle(
        t,
        D,
        16,
        seed,
        |g| g.range(1.6, 2.6),
        |age, life, r, i| {
            let th = r.range(0.0, TAU) + age * r.range(1.6, 2.6);
            let rad = r.range(14.0, 22.0);
            let y = y0 + r.range(-14.0, 12.0) - age * r.range(3.0, 7.0);
            let s = libm::sin(th);
            let k = age / life;
            if (s >= 0.0) != front {
                return;
            }
            let x = libm::cos(th) * rad;
            let yy = y + s * rad * 0.3;
            let a = js_min(1.0, age / 0.2) * if k > 0.75 { (1.0 - k) / 0.25 } else { 1.0 };
            if front {
                p.dot(x, yy, if i % 4 == 0 { WHITE } else { c[6] }, a);
                if i % 4 == 0 {
                    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                        p.dot(x + dx, yy + dy, c[5], a);
                    }
                }
                p.glow(x, yy, 4.0, c[5], 0.3 * a);
            } else {
                p.dot(x, yy, c[4], a * 0.9);
            }
        },
    );
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let ph = t / D;
    let bob = libm::sin(TAU * ph) * 2.5;
    let y0 = -o.height + bob;
    let r = 8.0;
    // its own cold light
    p.glow(0.0, y0, 34.0, c[3], 0.26);
    p.glow(0.0, y0, 14.0, c[5], 0.18 + 0.08 * libm::sin(TAU * ph * 3.0));
    // a sixth of a turn a loop: a hexagon looks the same after it, so the loop never seams
    let faces = crystal_faces((TAU / 6.0) * ph, r);
    // a dark rim round the silhouette, then the facets: deep in shadow, pale toward the light, white in a glint
    let all: Vec<Pt> = faces.iter().flat_map(|f| f.pts.iter().copied()).collect();
    let hull = hull_of(&all);
    let (mut mx, mut my) = (0.0, 0.0);
    for &(x, y) in &hull {
        mx += x;
        my += y;
    }
    mx /= hull.len() as f64;
    my /= hull.len() as f64;
    let rim: Vec<Pt> = hull
        .iter()
        .map(|&(x, y)| {
            let d = libm::hypot(x - mx, y - my);
            let d = if d == 0.0 { 1.0 } else { d };
            (x + ((x - mx) / d) * 1.2, y + y0 + ((y - my) / d) * 1.2)
        })
        .collect();
    p.poly(&rim, c[1], 1.0);
    for f in &faces {
        let col = if f.spec > 0.985 {
            WHITE
        } else {
            step(
                c,
                clamp(js_round(1.0 + libm::pow(f.lit, 1.4) * 6.6), 1.0, 7.0),
            )
        };
        let pts: Vec<Pt> = f.pts.iter().map(|&(x, y)| (x, y + y0)).collect();
        p.poly(&pts, col, 1.0);
    }
    // edges: pale where two faces meet, a dark rim round the outside, a white ridge on the lit side
    for f in &faces {
        for (i, &(x0, y1a)) in f.pts.iter().enumerate() {
            let Some(&(x1, y1)) = f.pts.get((i + 1) % f.pts.len()) else {
                continue;
            };
            let col = if f.lit > 0.6 {
                c[7]
            } else if f.lit > 0.3 {
                c[6]
            } else {
                c[4]
            };
            p.line(x0, y1a + y0, x1, y1 + y0, col, 1.0);
        }
    }
    // glints: a star flashes on a face as it turns into the light
    for f in &faces {
        if f.spec > 0.96 {
            sparkle(
                p,
                f.cx,
                f.cy + y0,
                js_round(2.0 + (f.spec - 0.96) * 150.0),
                c[6],
                c[4],
                0.5,
            );
        }
    }
    // a glint at the tip now and then
    let g = (ph * 2.0) % 1.0;
    if g < 0.12 {
        sparkle(
            p,
            0.0,
            y0 - r * 2.2 * libm::cos(TIP),
            js_round(5.0 * libm::sin((g / 0.12) * core::f64::consts::PI)),
            c[6],
            c[4],
            0.6,
        );
    }
    motes(p, c, t, o.seed, y0, true);
}
