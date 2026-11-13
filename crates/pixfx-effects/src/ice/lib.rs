//! Shared by the ice effects: panes of ice that tumble and skid, polygon helpers. Not an effect (the leading _
//! keeps the tools away).
//! The crack walk and the faceted crystal shading are adapted from Sprite Fusion's open-source ice
//! (https://www.spritefusion.com/pixel-art-vfx/ice), MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/ice/_lib.js`: the functions the ported ice effects use (`Panes`, `inPoly`, `glassPoly`,
//! `centroid`, `splitPoly`, `endBy`).

use core::f64::consts::FRAC_PI_2;

use pixfx_core::Painter;
use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::math::{clamp, js_max, js_min, js_round};
use pixfx_core::sim::{Bits, GRAVITY, Particles};

/// A polygon's corners `[(x, y), ...]`.
pub type Poly = Vec<(f64, f64)>;

/// Is (x, y) inside the polygon? (even-odd rule)
pub fn in_poly(pts: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut inside = false;
    let mut j = pts.len().wrapping_sub(1);
    for (i, &(xi, yi)) in pts.iter().enumerate() {
        if let Some(&(xj, yj)) = pts.get(j)
            && (yi > y) != (yj > y)
            && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// A polygon's centroid and area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Centroid {
    pub x: f64,
    pub y: f64,
    pub area: f64,
}

pub fn centroid(pts: &[(f64, f64)]) -> Centroid {
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    let mut j = pts.len().wrapping_sub(1);
    for (i, &(x1, y1)) in pts.iter().enumerate() {
        if let Some(&(x0, y0)) = pts.get(j) {
            let k = x0 * y1 - x1 * y0;
            a += k;
            cx += (x0 + x1) * k;
            cy += (y0 + y1) * k;
        }
        j = i;
    }
    if libm::fabs(a) < 1e-6 {
        let n = pts.len() as f64;
        return Centroid {
            x: pts.iter().fold(0.0, |s, q| s + q.0) / n,
            y: pts.iter().fold(0.0, |s, q| s + q.1) / n,
            area: 0.0,
        };
    }
    Centroid {
        x: cx / (3.0 * a),
        y: cy / (3.0 * a),
        area: libm::fabs(a) / 2.0,
    }
}

/// Cut a polygon along the line through (x, y) at angle a: the pieces with three corners or more.
pub fn split_poly(pts: &[(f64, f64)], x: f64, y: f64, a: f64) -> Vec<Poly> {
    let (nx, ny) = (-libm::sin(a), libm::cos(a));
    let side = |q: (f64, f64)| (q.0 - x) * nx + (q.1 - y) * ny;
    let (mut l, mut rt): (Poly, Poly) = (Vec::new(), Vec::new());
    for (i, &p) in pts.iter().enumerate() {
        let Some(&q) = pts.get((i + 1) % pts.len()) else {
            continue;
        };
        let (sp, sq) = (side(p), side(q));
        if sp >= 0.0 {
            l.push(p);
        } else {
            rt.push(p);
        }
        if (sp >= 0.0) != (sq >= 0.0) {
            let u = sp / (sp - sq);
            let m = (p.0 + (q.0 - p.0) * u, p.1 + (q.1 - p.1) * u);
            l.push(m);
            rt.push(m);
        }
    }
    [l, rt].into_iter().filter(|s| s.len() >= 3).collect()
}

/// Fill a polygon with translucent colour (true blending, no dither): glass, ice, light.
pub fn glass_poly(p: &mut Painter, pts: &[(f64, f64)], c: Rgba, a: f64) {
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in pts {
        x0 = js_min(x0, x);
        x1 = js_max(x1, x);
        y0 = js_min(y0, y);
        y1 = js_max(y1, y);
    }
    let mut y = libm::floor(y0);
    while y <= libm::ceil(y1) {
        let mut x = libm::floor(x0);
        while x <= libm::ceil(x1) {
            if in_poly(pts, x + 0.5, y + 0.5) {
                p.blend(x, y, c, a);
            }
            x += 1.0;
        }
        y += 1.0;
    }
}

/// What happened to a pane during [`Panes::update`] (pixelfx `onLand`, `onCrumble`, `onSkid`).
#[derive(Clone, Copy, Debug)]
pub enum PaneEvent<'a> {
    /// It hit the floor hard, at (x, y), at this speed (`onLand`).
    Land {
        pane: &'a Pane,
        x: f64,
        y: f64,
        speed: f64,
    },
    /// It lay still and crumbled away (`onCrumble`).
    Crumble(&'a Pane),
    /// It slides along the floor at (x, y) (`onSkid`).
    Skid { pane: &'a Pane, x: f64, y: f64 },
}

/// One pane of ice: the entry pixelfx `Panes.add` takes. Build it with [`Pane::new`] and set what differs.
#[derive(Clone, Debug, PartialEq)]
pub struct Pane {
    /// Corners around (0, 0).
    pub pts: Poly,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    /// Turn.
    pub a: f64,
    pub va: f64,
    /// Tumble.
    pub f: f64,
    pub vf: f64,
    /// 0..1: how much light its face catches.
    pub lit: f64,
    pub g: f64,
    pub drag: f64,
    pub bounce: f64,
    pub life: f64,
    pub rest: bool,
    /// Breaks on that many hard landings (0: never).
    pub brittle: u32,
    /// Seconds it lies still before it crumbles.
    pub crumble: Option<f64>,
    /// Opaque: one `tone` 0..1, or shaded like a blade split along `ridge`, an angle.
    pub solid: bool,
    pub tone: Option<f64>,
    pub ridge: f64,
    hits: u32,
    rest_t: f64,
}

impl Pane {
    /// A still pane at (x, y) with pixelfx's defaults.
    pub fn new(pts: Poly, x: f64, y: f64) -> Self {
        Self {
            pts,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            a: 0.0,
            va: 0.0,
            f: 0.0,
            vf: 0.0,
            lit: 0.6,
            g: 1.0,
            drag: 0.3,
            bounce: 0.35,
            life: 1.0,
            rest: false,
            brittle: 0,
            crumble: None,
            solid: false,
            tone: None,
            ridge: -FRAC_PI_2,
            hits: 0,
            rest_t: 0.0,
        }
    }

    /// The pane's corners as drawn: tumbled (squashed across its own width), turned and placed.
    pub fn corners(&self) -> Poly {
        let (ca, sa, k) = (libm::cos(self.a), libm::sin(self.a), libm::cos(self.f));
        self.pts
            .iter()
            .map(|&(x, y)| {
                let yy = y * k;
                (self.x + x * ca - yy * sa, self.y + x * sa + yy * ca)
            })
            .collect()
    }

    fn seat(&self) -> f64 {
        self.corners()
            .iter()
            .fold(f64::NEG_INFINITY, |low, c| js_max(low, c.1))
    }
}

/// Panes of ice: flat polygons that fly, spin and tumble end over end (they squash as they turn edge-on and flash
/// as they square up to the light), bounce on the floor, skid, settle and fade. Drawn as glass: a translucent
/// face, edges lit where they face the light, a glint. Port of pixelfx `Panes`.
#[derive(Clone, Debug, PartialEq)]
pub struct Panes {
    list: Vec<Pane>,
    /// The floor y in local px (pixelfx `ground`, a function of x there).
    ground: f64,
}

impl Panes {
    pub fn new(ground: f64) -> Self {
        Self {
            list: Vec::new(),
            ground,
        }
    }

    pub fn add(&mut self, q: Pane) {
        self.list.push(q);
    }

    /// Advance every pane by `dt` seconds; `on` hears landings, crumbles and skids as they happen.
    pub fn update(&mut self, dt: f64, mut on: impl FnMut(PaneEvent<'_>)) {
        let ground = self.ground;
        for i in (0..self.list.len()).rev() {
            let Some(q) = self.list.get_mut(i) else {
                continue;
            };
            q.life -= dt;
            if q.life <= 0.0 {
                self.list.remove(i);
                continue;
            }
            let floor = ground - 0.5;
            if q.rest
                && let Some(crumble) = q.crumble
            {
                q.rest_t += dt;
                if q.rest_t > crumble {
                    on(PaneEvent::Crumble(q));
                    self.list.remove(i);
                    continue;
                }
            }
            if q.rest {
                // skidding: friction slows the slide and the spin, the pane stays seated on the floor
                if libm::fabs(q.vx) > 8.0 {
                    on(PaneEvent::Skid {
                        pane: q,
                        x: q.x,
                        y: floor + 0.5,
                    });
                }
                q.x += q.vx * dt;
                q.vx -= q.vx * 7.0 * dt;
                q.a += q.va * dt;
                q.va -= q.va * 9.0 * dt;
                q.vf -= q.vf * 9.0 * dt;
                q.f += q.vf * dt;
                q.y -= q.seat() - floor;
                continue;
            }
            q.vy += GRAVITY * q.g * dt;
            q.vx -= q.vx * q.drag * dt;
            q.vy -= q.vy * q.drag * dt;
            q.x += q.vx * dt;
            q.y += q.vy * dt;
            q.a += q.va * dt;
            q.f += q.vf * dt;
            let low = q.seat();
            if q.g <= 0.0 || low < floor || q.vy <= 0.0 {
                continue;
            }
            q.y -= low - floor;
            if q.vy > 70.0 {
                q.hits += 1;
                on(PaneEvent::Land {
                    pane: q,
                    x: q.x,
                    y: floor + 0.5,
                    speed: q.vy,
                });
            }
            if q.brittle > 0 && q.vy > 70.0 && q.hits >= q.brittle {
                self.list.remove(i);
                continue;
            }
            if q.vy > 70.0 && q.bounce > 0.0 {
                q.vy = -q.vy * q.bounce;
                q.vx *= 0.75;
                q.va *= 0.6;
                q.vf *= 0.5;
            } else {
                q.rest = true;
                q.vy = 0.0;
            }
        }
    }

    /// Draw with an ice ramp `c`, dark to bright.
    pub fn draw(&self, p: &mut Painter, c: &[Rgba]) {
        let n = c.len();
        let nf = n as f64;
        let step = |i: f64| {
            if i < 0.0 {
                Rgba::NONE
            } else {
                c.get(i as usize).copied().unwrap_or(Rgba::NONE)
            }
        };
        for q in &self.list {
            let a = js_min(1.0, q.life / 0.35);
            let pts = q.corners();
            let k = libm::cos(q.f);
            let face = libm::fabs(k);
            // the face: glassy, brighter the more it faces us and the more light it catches; it flashes as it
            // squares up
            let lv = q.lit * (0.35 + 0.65 * face);
            let flash = face > 0.97 && !q.rest;
            if face > 0.25 {
                if q.solid
                    && let Some(tone) = q.tone
                {
                    // a chunk of one face: its tone, brighter as it squares up to the light
                    let col = if flash {
                        step(nf - 1.0)
                    } else {
                        step(clamp(
                            js_round(1.0 + tone * 5.0 * (0.6 + 0.4 * face)),
                            1.0,
                            nf - 2.0,
                        ))
                    };
                    p.poly(&pts, col, a);
                } else if q.solid {
                    // shaded like the blade it broke from: a lit side, a pale ridge, a deep side
                    let (rr, ca, sa) = (q.ridge, libm::cos(q.a), libm::sin(q.a));
                    let (cr, sr) = (libm::cos(rr), libm::sin(rr));
                    let rx = cr * ca - sr * k * sa;
                    let ry = cr * sa + sr * k * ca;
                    let (mut nx, mut ny) = (-ry, rx);
                    if nx * -0.55 + ny * -0.84 < 0.0 {
                        nx = -nx;
                        ny = -ny;
                    }
                    let lit = if flash {
                        step(nf - 1.0)
                    } else {
                        step(clamp(js_round(3.0 + lv * 3.0), 3.0, nf - 2.0))
                    };
                    let deep = step(clamp(js_round(1.0 + lv * 2.0), 1.0, 3.0));
                    let pale = step(nf - 2.0);
                    p.poly(
                        &pts,
                        |x: f64, y: f64| {
                            let d = (x - q.x) * nx + (y - q.y) * ny;
                            if d > 0.6 {
                                lit
                            } else if d > -0.6 {
                                pale
                            } else {
                                deep
                            }
                        },
                        a,
                    );
                } else {
                    let col = if flash {
                        step(nf - 1.0)
                    } else {
                        step(clamp(js_round(3.0 + lv * 3.0), 3.0, nf - 2.0))
                    };
                    glass_poly(
                        p,
                        &pts,
                        col,
                        (0.3 + 0.35 * lv + if flash { 0.3 } else { 0.0 }) * a,
                    );
                }
            }
            let (mut cx, mut cy, mut top) = (0.0, 0.0, 0);
            for &(x, y) in &pts {
                cx += x;
                cy += y;
            }
            cx /= pts.len() as f64;
            cy /= pts.len() as f64;
            // edges: lit where they face the light (up and left), cool where they face away
            for (i, &(x0, y0)) in pts.iter().enumerate() {
                let Some(&(x1, y1)) = pts.get((i + 1) % pts.len()) else {
                    continue;
                };
                let (mut nx, mut ny) = (y1 - y0, x0 - x1);
                if nx * ((x0 + x1) / 2.0 - cx) + ny * ((y0 + y1) / 2.0 - cy) < 0.0 {
                    nx = -nx;
                    ny = -ny;
                }
                let mut l = libm::hypot(nx, ny);
                if l == 0.0 {
                    l = 1.0;
                }
                let lit = (-nx * 0.55 - ny * 0.84) / l;
                let col = if face < 0.25 || lit > 0.35 {
                    step(nf - 1.0)
                } else if lit > -0.25 {
                    step(nf - 2.0)
                } else if q.solid {
                    step(1.0)
                } else {
                    step(js_max(2.0, nf - 4.0))
                };
                p.line(x0, y0, x1, y1, col, a);
                if pts.get(i).map(|c| c.1) < pts.get(top).map(|c| c.1) {
                    top = i;
                }
            }
            if face > 0.7
                && let Some(&(tx, ty)) = pts.get(top)
            {
                p.dot(tx, ty, WHITE, a);
            }
            p.glow(
                cx,
                cy,
                5.0,
                step(nf - 3.0),
                (if flash { 0.5 } else { 0.14 }) * a,
            );
        }
    }
}

/// A system whose things have a `life` that [`end_by`] can trim.
pub trait Lifetimes {
    fn for_each_life(&mut self, f: &mut dyn FnMut(&mut f64));
}

impl Lifetimes for Particles {
    fn for_each_life(&mut self, f: &mut dyn FnMut(&mut f64)) {
        for q in self.list_mut() {
            f(&mut q.life);
        }
    }
}

impl<T> Lifetimes for Bits<T> {
    fn for_each_life(&mut self, f: &mut dyn FnMut(&mut f64)) {
        for q in self.list_mut() {
            f(&mut q.bit.life);
        }
    }
}

impl Lifetimes for Panes {
    fn for_each_life(&mut self, f: &mut dyn FnMut(&mut f64)) {
        for q in &mut self.list {
            f(&mut q.life);
        }
    }
}

/// Nothing outlives the effect: trims the life of every particle, bit and pane that would last past `d`.
pub fn end_by(t: f64, d: f64, systems: &mut [&mut dyn Lifetimes]) {
    let left = js_max(0.001, d - 0.05 - t);
    for sys in systems {
        sys.for_each_life(&mut |life| {
            if *life > left {
                *life = left;
            }
        });
    }
}
