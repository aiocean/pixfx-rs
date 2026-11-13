//! Void orb: port of pixelfx `src/fx/shadow/void-orb.js`.

use pixfx_core::color::{Rgba, WHITE, ramp};
use pixfx_core::math::{Rng, TAU, clamp, ease, fbm, js_max, js_min, smooth};
use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter};

use super::lib::{VOID, halo, ring_fast, ring_light, unlight};

pub struct VoidOrb;

pub static VOID_ORB: VoidOrb = VoidOrb;

const META: Meta = Meta {
    id: "shadow/void-orb",
    name: "Void orb",
    desc: "A black orb hangs in the air, a pale rim crawling round it; a dark vortex winds into it, motes and streaks are pulled in, and light bends round its edge.",
    tags: &["shadow", "void", "orb", "vortex", "dark", "magic", "loop"],
    size: (168, 160),
    origin: (84, 150),
    duration: 3.2,
    looping: true,
    fps: 30,
    seed: 5,
};

// the orb's centre and radius, the vortex's reach, the radius light bends to (Einstein ring), the loop
// matter can't orbit inside GAP: there the dark is empty but for light bent round the orb
const Y: f64 = -62.0;
const R: f64 = 12.0;
const VR: f64 = 48.0;
const GAP: f64 = R + 9.0;
const E: f64 = R + 4.5;
const D: f64 = 3.2;
const ARMS: f64 = 3.0;
const K: f64 = 7.0;
const WIND: f64 = (TAU * 2.0) / D;
// two gulps a loop: the rim flares, the vortex brightens, a ring is drawn in
const GULPS: [f64; 2] = [0.2, 0.68];

fn wrap(x: f64) -> f64 {
    x - libm::floor(x)
}

fn gulp(t: f64) -> f64 {
    GULPS.iter().fold(0.0, |s, &g| {
        let v = ((wrap(t / D - g + 0.5) - 0.5) * D) / 0.07;
        s + libm::exp(-(v * v))
    })
}

/// JS `Math.sign` for a value that is never NaN.
fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// A speck of light behind the orb on a looping path.
struct Source {
    cx: f64,
    cy: f64,
    rho: f64,
    n: f64,
    ph: f64,
    s: f64,
    sy: f64,
}

// specks of light behind the orb on looping paths; we see them through the lens: far off, a speck; passing
// behind, smeared into an arc hugging the rim, with a faint second image on the far side
const SOURCES: [Source; 7] = [
    Source {
        cx: 0.0,
        cy: 0.0,
        rho: 6.0,
        n: 1.0,
        ph: 0.0,
        s: 2.6,
        sy: 0.75,
    },
    Source {
        cx: 1.0,
        cy: -1.0,
        rho: 8.0,
        n: -1.0,
        ph: 2.2,
        s: 2.0,
        sy: 0.6,
    },
    Source {
        cx: 18.0,
        cy: -6.0,
        rho: 24.0,
        n: 1.0,
        ph: 0.4,
        s: 2.4,
        sy: 0.7,
    },
    Source {
        cx: -26.0,
        cy: 10.0,
        rho: 30.0,
        n: 1.0,
        ph: 2.6,
        s: 2.0,
        sy: 0.55,
    },
    Source {
        cx: 8.0,
        cy: 22.0,
        rho: 20.0,
        n: 2.0,
        ph: 4.1,
        s: 1.6,
        sy: 0.8,
    },
    Source {
        cx: -12.0,
        cy: -26.0,
        rho: 34.0,
        n: 1.0,
        ph: 5.3,
        s: 1.8,
        sy: 0.6,
    },
    Source {
        cx: 34.0,
        cy: 18.0,
        rho: 16.0,
        n: 2.0,
        ph: 1.2,
        s: 1.4,
        sy: 0.9,
    },
];

/// One cell of the table round the orb (stored as `f32`, as pixelfx's `Float32Array`s do): radius, angle, log
/// radius, and lumpy turbulence for the vortex.
#[derive(Clone, Copy)]
struct Cell {
    r: f32,
    th: f32,
    ln: f32,
    nz: f32,
}

/// The `n x n` cells round the orb, built once per instance (pixelfx builds them once per page).
fn tables(n: usize) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            let x = i as f64 - VR + 0.5;
            let y = j as f64 - VR + 0.5;
            let a = libm::atan2(y, x);
            let d = libm::hypot(x, y);
            let l = libm::log(js_max(1.0, d));
            cells.push(Cell {
                r: d as f32,
                th: a as f32,
                ln: l as f32,
                nz: fbm(
                    libm::cos(a) * 2.5 + l * 2.4,
                    libm::sin(a) * 2.5 + l * 2.4,
                    17.0,
                    3,
                ) as f32,
            });
        }
    }
    cells
}

impl Effect for VoidOrb {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&VOID)),
            seed: seed as f64,
            cells: tables((VR * 2.0) as usize),
        })
    }
}

struct State {
    c: [Rgba; 8],
    seed: f64,
    cells: Vec<Cell>,
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

/// The vortex: three arms of dark dust winding inward, a dithered veil threaded with thin bright filaments that
/// stream in along them faster than the arms turn, a lit leading edge, all heating to pale violet near the rim.
fn vortex(p: &mut Painter, c: &[Rgba; 8], cells: &[Cell], t: f64, g: f64) {
    let n = (VR * 2.0) as usize;
    let wf = (TAU * 12.0) / D;
    for j in 0..n {
        for i in 0..n {
            let Some(cell) = cells.get(j * n + i) else {
                continue;
            };
            let (r, th, ln, nz) = (
                f64::from(cell.r),
                f64::from(cell.th),
                f64::from(cell.ln),
                f64::from(cell.nz),
            );
            if r < GAP || r > VR {
                continue;
            }
            let env = smooth(VR, VR * 0.42, r) * smooth(GAP, GAP + 4.0, r);
            let ph = ARMS * th + K * ln + WIND * t + nz * 2.6;
            let cs = libm::cos(ph);
            let a = (0.5 + 0.5 * cs) * env * (1.0 + 0.4 * g);
            if a < 0.16 {
                continue;
            }
            let x = i as f64 - VR + 0.5;
            let y = Y + j as f64 - VR + 0.5;
            let hot = 1.0 - smooth(GAP, GAP + 16.0, r);
            let fil = libm::cos(12.0 * th + 28.0 * ln + wf * t + nz * 1.6);
            // streams break into dashes that run inward with them
            let dash = libm::cos(9.0 * ln * 2.2 + 2.2 * th + wf * 0.75 * t);
            if fil > 0.86 && a > 0.3 && dash > -0.55 {
                let col = if hot > 0.6 {
                    c[7]
                } else if (hot > 0.25 || fil > 0.97) && dash > 0.0 {
                    c[6]
                } else {
                    c[5]
                };
                p.px(x, y, col);
                p.light(x, y, col, 0.1 + 0.2 * hot);
                continue;
            }
            // the arm's leading edge, where it turns into the light
            if a > 0.36 && a < 0.46 && libm::sin(ph) > 0.0 {
                p.px(x, y, if hot > 0.4 { c[6] } else { c[4] });
                continue;
            }
            // the veil: translucent dark dust, denser in the arm's heart
            p.blend(
                x,
                y,
                if a > 0.6 { c[3] } else { c[2] },
                js_min(0.75, (a - 0.16) * 1.4),
            );
        }
    }
}

/// What the orb's gravity does to light from behind it.
fn lensing(p: &mut Painter, c: &[Rgba; 8], t: f64, g: f64) {
    // the Einstein ring itself, faint, brightening as it gulps
    let st = 0.7 / E;
    let mut a = 0.0;
    while a < TAU {
        let f = 0.5 + 0.5 * libm::cos(5.0 * a - (TAU * 2.0 * t) / D) + 0.5 * g;
        p.px(
            libm::cos(a) * E,
            Y + libm::sin(a) * E,
            if f > 0.85 {
                c[5]
            } else if f > 0.45 {
                c[4]
            } else {
                c[3]
            },
        );
        a += st;
    }
    for s in &SOURCES {
        let q = (TAU * s.n * t) / D + s.ph;
        let sx = s.cx + s.rho * libm::cos(q);
        let sy = s.cy + s.rho * libm::sin(q) * s.sy;
        let b = js_max(0.3, libm::hypot(sx, sy));
        let al = libm::atan2(sy, sx);
        let root = libm::sqrt(b * b + 4.0 * E * E);
        let images = [
            ((b + root) / 2.0, al, 1.0),
            ((root - b) / 2.0, al + core::f64::consts::PI, 0.55),
        ];
        for (r, a0, w) in images {
            if r < R + 1.5 {
                continue;
            }
            let mag = (r / b) * w;
            let hw = js_min(core::f64::consts::PI, (s.s * 1.6) / (2.0 * b));
            let step = 0.6 / r;
            let mut a = a0 - hw;
            while a <= a0 + hw + 1e-6 {
                let u = if hw > 0.0 {
                    libm::fabs(a - a0) / hw
                } else {
                    0.0
                };
                let x = libm::cos(a) * r;
                let y = Y + libm::sin(a) * r;
                let col = if u < 0.3 && mag > 0.8 {
                    WHITE
                } else if u < 0.65 {
                    c[7]
                } else {
                    c[5]
                };
                p.dot(x, y, col, js_min(1.0, mag * 1.4) * (1.15 - u));
                if mag > 1.2 && u < 0.6 {
                    p.dot(x + libm::cos(a), y + libm::sin(a), c[6], 0.8);
                }
                a += step;
            }
            p.glow(
                libm::cos(a0) * r,
                Y + libm::sin(a0) * r,
                4.0 + js_min(10.0, hw * r),
                c[6],
                js_min(0.5, 0.18 * mag),
            );
        }
    }
}

/// The orb: void black, a dull glint and a crescent of reflected light inside, and a pale rim crawling round.
fn orb(p: &mut Painter, c: &[Rgba; 8], t: f64, g: f64) {
    p.shade(-R - 1.0, Y - R - 1.0, R + 1.0, Y + R + 1.0, |x, y, _| {
        let dx = x;
        let dy = y - Y;
        let d = libm::sqrt(dx * dx + dy * dy);
        if d > R + 0.5 {
            return None;
        }
        let a = libm::atan2(dy, dx);
        if d > R - 0.6 {
            let f = 0.5 * libm::cos(3.0 * a - (TAU * 2.0 * t) / D)
                + 0.3 * libm::cos(7.0 * a + (TAU * 3.0 * t) / D + 1.0)
                + 0.25 * libm::cos(13.0 * a - (TAU * 5.0 * t) / D + 2.0)
                + g;
            return Some(if f > 0.5 {
                WHITE
            } else if f > 0.05 {
                c[6]
            } else if f > -0.4 {
                c[5]
            } else {
                c[4]
            });
        }
        if R - d < 1.4 && libm::cos(a - 0.8) > 0.6 {
            return Some(c[2]);
        }
        let (gx, gy) = (dx + R * 0.42, dy + R * 0.45);
        if gx * gx + gy * gy < 1.6 {
            return Some(c[3]);
        }
        Some(c[0])
    });
}

fn draw_back(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let g = gulp(t);
    p.glow(0.0, -1.0, 52.0, c[4], 0.22 + 0.1 * g);
    vortex(p, c, &o.cells, t, g);
    lensing(p, c, t, g);
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let g = gulp(t);
    let s = o.seed;
    // dust drawn up off the floor, curving into the orb
    cycle(
        t,
        D,
        14,
        s + 2.0,
        |r: &mut Rng| r.range(1.6, 2.4),
        |age, l, r, _| {
            let x0 = r.range(-44.0, 44.0);
            let u = libm::pow(age / l, 1.7);
            let side = if sign(x0) == 0.0 { 1.0 } else { sign(x0) };
            let ea = core::f64::consts::FRAC_PI_2 + side * r.range(0.3, 1.0);
            let ex = libm::cos(ea) * R;
            let ey = Y + libm::sin(ea) * R;
            let mx = x0 * 0.2;
            let my = -18.0 - r.range(0.0, 12.0);
            let x = (1.0 - u) * (1.0 - u) * x0 + 2.0 * (1.0 - u) * u * mx + u * u * ex;
            let y = -((1.0 - u) * (1.0 - u)) + 2.0 * (1.0 - u) * u * my + u * u * ey;
            if age < 0.5 {
                p.disc(x0, -1.5, 2.0 + age * 6.0, c[3], 0.4 * (1.0 - age / 0.5));
            }
            p.dot(
                x,
                y,
                if u > 0.6 {
                    c[7]
                } else if u > 0.25 {
                    c[6]
                } else {
                    c[5]
                },
                js_min(1.0, age * 4.0),
            );
            if u > 0.3 {
                p.glow(x, y, 3.0, c[5], 0.25);
            }
        },
    );
    // motes swirl in along the arms, faster as they fall, and spark on the rim where they're swallowed
    cycle(
        t,
        D,
        30,
        s,
        |r: &mut Rng| r.range(1.1, 1.8),
        |age, l, r, _| {
            let mut th0 = r.range(0.0, TAU);
            if libm::sin(th0) > 0.75 {
                th0 = -th0;
            }
            let r0 = r.range(30.0, 64.0);
            let sw = r.range(1.4, 2.6);
            let col = r
                .pick(&[WHITE, c[7], c[6], c[6], c[5]])
                .copied()
                .unwrap_or(WHITE);
            let pos = |aa: f64| {
                let u = clamp(aa / l, 0.0, 1.0);
                let k = libm::pow(u, 1.8);
                let rr = R + 1.0 + (r0 - R - 1.0) * (1.0 - k);
                let th = th0 - sw * (k + 0.5 * u);
                (libm::cos(th) * rr, Y + libm::sin(th) * rr, rr)
            };
            let (x, y, rr) = pos(age);
            let (tx, ty, _) = pos(age - 0.05);
            let a = js_min(1.0, age * 5.0);
            if rr < R + 3.0 {
                p.px(x, y, WHITE);
                p.glow(x, y, 5.0, c[6], 0.5);
                return;
            }
            p.line(tx, ty, (tx + x) / 2.0, (ty + y) / 2.0, c[4], a * 0.8);
            p.line((tx + x) / 2.0, (ty + y) / 2.0, x, y, col, a);
            p.glow(x, y, 3.0, col, 0.3 * a);
        },
    );
    // streaks: long dotted lines shot in from further out, white at the head (never from under the floor)
    cycle(
        t,
        D,
        9,
        s + 1.0,
        |_| 0.42,
        |age, l, r, _| {
            let mut th = r.range(0.0, TAU);
            if libm::sin(th) > 0.45 {
                th = -th;
            }
            let r0 = r.range(56.0, 74.0);
            let u = age / l;
            let k = (libm::exp(3.0 * u) - 1.0) / (libm::exp(3.0) - 1.0);
            let rr = r0 - (r0 - R - 1.0) * k;
            let len = 2.0 + 10.0 * k;
            let ca = libm::cos(th);
            let sa = libm::sin(th);
            let mut d = 0.0;
            while d <= len {
                p.dot(
                    ca * (rr + d),
                    Y + sa * (rr + d),
                    if d < 1.0 {
                        WHITE
                    } else if d < len * 0.5 {
                        c[6]
                    } else {
                        c[4]
                    },
                    if d < len * 0.5 { 1.0 } else { 1.2 - d / len },
                );
                d += 0.5;
            }
            p.glow(ca * rr, Y + sa * rr, 4.0, c[6], 0.4);
        },
    );
    // a gulp draws a faint ring in after it
    for gp in GULPS {
        let u = wrap(t / D - gp) / (0.4 / D);
        if u >= 1.0 {
            continue;
        }
        let rr = R + 2.0 + (VR - R) * (1.0 - ease::in_quad(u));
        ring_fast(p, 0.0, Y, rr, 1.0, c[4], 0.35 * (1.0 - u));
        ring_light(p, 0.0, Y, rr, 4.0, c[5], 0.3 * (1.0 - u));
    }
    orb(p, c, t, g);
    halo(p, 0.0, Y, R - 0.5, R + 7.0, c[6], 0.6 + 0.3 * g);
    halo(p, 0.0, Y, R - 0.5, R + 46.0, c[4], 0.38 + 0.15 * g);
    unlight(p, -R, Y - R, R, Y + R, 0.0, |x, y| {
        x * x + (y - Y) * (y - Y) < (R - 0.6) * (R - 0.6)
    });
}
