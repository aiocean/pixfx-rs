//! Shared by the ice effects (pixelfx `src/fx/ice/_lib.js`; the leading _ keeps the tools from listing it as an
//! effect): panes of ice that tumble and skid, polygon helpers, a character turned to ice, cracks that race
//! through a shape, ice blades and snowflakes.
//! The crack walk and the faceted crystal shading are adapted from Sprite Fusion's open-source ice
//! (https://www.spritefusion.com/pixel-art-vfx/ice), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::sim::{BitKind, Bits, GRAVITY, Particles};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, luma};
use pixfx_draw::math::{Rng, TAU, clamp, js_max, js_min, js_round, to_int32};

/// The ice ramp every ice effect defaults to, dark to bright (pixelfx `options.palette`).
pub const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x151a3d),
    Rgba::hex24(0x212f6b),
    Rgba::hex24(0x2c58aa),
    Rgba::hex24(0x3a8bd8),
    Rgba::hex24(0x62c8f4),
    Rgba::hex24(0xa6ecff),
    Rgba::hex24(0xdcfbff),
    Rgba::hex24(0xffffff),
];

/// A point of a polygon.
pub type Pt = (f64, f64);

/// Step `i` of an 8-step ramp, clamped to its ends (every index the ice effects compute is in range).
pub fn step(c: &[Rgba; 8], i: f64) -> Rgba {
    c[clamp(i, 0.0, 7.0) as usize]
}

/// Spawn a particle with its arguments evaluated in pixelfx's order (left to right, then the seed):
/// `emit!(ps, r, Kind::Flake, x, y, vx, vy, life, col, size, delay)`; col, size and delay may be left off.
#[allow(unused_macros, reason = "not every ice effect spawns particles")]
macro_rules! emit {
    ($ps:expr, $r:expr, $k:expr, $x:expr, $y:expr, $vx:expr, $vy:expr, $life:expr) => {
        emit!(
            $ps,
            $r,
            $k,
            $x,
            $y,
            $vx,
            $vy,
            $life,
            ::pixfx_draw::color::WHITE,
            1.0,
            0.0
        )
    };
    ($ps:expr, $r:expr, $k:expr, $x:expr, $y:expr, $vx:expr, $vy:expr, $life:expr, $col:expr) => {
        emit!($ps, $r, $k, $x, $y, $vx, $vy, $life, $col, 1.0, 0.0)
    };
    ($ps:expr, $r:expr, $k:expr, $x:expr, $y:expr, $vx:expr, $vy:expr, $life:expr, $col:expr, $size:expr) => {
        emit!($ps, $r, $k, $x, $y, $vx, $vy, $life, $col, $size, 0.0)
    };
    ($ps:expr, $r:expr, $k:expr, $x:expr, $y:expr, $vx:expr, $vy:expr, $life:expr, $col:expr, $size:expr, $delay:expr) => {{
        let kind = $k;
        let x = $x;
        let y = $y;
        let vx = $vx;
        let vy = $vy;
        let life = $life;
        let col = Some($col);
        let size = $size;
        let delay = $delay;
        $ps.spawn(
            $r,
            ::pixfx_core::sim::Spawn {
                kind,
                x,
                y,
                vx,
                vy,
                life,
                col,
                size,
                delay,
                flags: ::pixfx_core::sim::Flags::NONE,
            },
        );
    }};
}
#[allow(unused_imports, reason = "not every ice effect spawns particles")]
pub(crate) use emit;

/// Add a piece of debris with its fields set in the JS object literal's order (so the rng draws keep theirs):
/// `bit!(bits, BitKind::Shard; x = .., y = .., vx = ..)`; fields left out keep pixelfx's defaults.
#[allow(unused_macros, reason = "not every ice effect throws debris")]
macro_rules! bit {
    ($bits:expr, $kind:expr; $($f:ident = $v:expr),* $(,)?) => {{
        let mut b = ::pixfx_core::sim::Bit::new($kind, 0.0, 0.0);
        $( b.$f = $v; )*
        $bits.add(b);
    }};
}
#[allow(unused_imports, reason = "not every ice effect throws debris")]
pub(crate) use bit;

/// Is (x, y) inside the polygon? (even-odd rule)
pub fn in_poly(pts: &[Pt], x: f64, y: f64) -> bool {
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

/// A polygon's centroid and area (pixelfx `centroid`).
pub fn centroid(pts: &[Pt]) -> Centroid {
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
        let (mut sx, mut sy) = (0.0, 0.0);
        for &(x, y) in pts {
            sx += x;
            sy += y;
        }
        return Centroid {
            x: sx / n,
            y: sy / n,
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
pub fn split_poly(pts: &[Pt], x: f64, y: f64, a: f64) -> Vec<Vec<Pt>> {
    let (nx, ny) = (-libm::sin(a), libm::cos(a));
    let side = |q: Pt| (q.0 - x) * nx + (q.1 - y) * ny;
    let (mut l, mut rt) = (Vec::new(), Vec::new());
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

/// A colour turned to ice: its brightness picks a step of the 8-step ice ramp (dark to bright), never the darkest.
pub fn ice_of(c: Rgba) -> usize {
    let (n, l) = (8.0, luma(c));
    let k = if l < 0.12 { 2.0 } else { 2.0 + l * 5.2 };
    clamp(js_round((k * (n - 1.0)) / 7.0), 1.0, n - 1.0) as usize
}

/// Fill a polygon with translucent colour (true blending, no dither): glass, ice, light.
pub fn glass_poly(p: &mut Painter, pts: &[Pt], c: Rgba, a: f64) {
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

/// One pane of ice (the object pixelfx `Panes.add` takes, with its defaults): corners round (0, 0), place,
/// turn `a`, tumble `f`, how much light its face catches, gravity scale, drag, bounce, life. `brittle` breaks it
/// on that many hard landings (0: never), `crumble` the seconds it lies still before it crumbles; `solid` draws
/// it opaque: one `tone` 0..1, or shaded like a blade split along `ridge`.
#[derive(Clone, Debug, PartialEq)]
pub struct Pane {
    pub pts: Vec<Pt>,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub a: f64,
    pub va: f64,
    pub f: f64,
    pub vf: f64,
    pub lit: f64,
    pub g: f64,
    pub drag: f64,
    pub bounce: f64,
    pub life: f64,
    pub brittle: u32,
    pub crumble: Option<f64>,
    pub solid: bool,
    pub tone: Option<f64>,
    pub ridge: Option<f64>,
    max: f64,
    rest: bool,
    rest_t: f64,
    hits: u32,
}

impl Pane {
    /// A still pane with pixelfx's defaults.
    pub fn new(pts: Vec<Pt>, x: f64, y: f64) -> Self {
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
            brittle: 0,
            crumble: None,
            solid: false,
            tone: None,
            ridge: None,
            max: 1.0,
            rest: false,
            rest_t: 0.0,
            hits: 0,
        }
    }

    /// The pane's corners as drawn: tumbled (squashed across its own width), turned and placed.
    fn corners(&self) -> Vec<Pt> {
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
        let mut low = f64::NEG_INFINITY;
        for c in self.corners() {
            low = js_max(low, c.1);
        }
        low
    }
}

/// What a pane did in an update step (pixelfx `onLand`, `onCrumble`, `onSkid`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaneEvent {
    /// It hit the floor hard at (x, y) at `speed`, moving `vx`.
    Land { vx: f64, x: f64, y: f64, speed: f64 },
    /// It lay still long enough and crumbled away at x.
    Crumble { x: f64, vx: f64 },
    /// It slides along the floor at (x, y).
    Skid { x: f64, y: f64 },
}

/// Panes of ice (pixelfx `Panes`): flat polygons that fly, spin and tumble end over end (they squash as they turn
/// edge-on and flash as they square up to the light), bounce on the floor, skid, settle and fade. Drawn as glass:
/// a translucent face, edges lit where they face the light, a glint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Panes {
    list: Vec<Pane>,
    ground: f64,
}

impl Panes {
    /// Panes over a floor at y = `ground`.
    pub fn new(ground: f64) -> Self {
        Self {
            list: Vec::new(),
            ground,
        }
    }

    pub fn add(&mut self, mut q: Pane) {
        q.max = q.life;
        self.list.push(q);
    }

    pub fn count(&self) -> usize {
        self.list.len()
    }

    /// Advance every pane by `dt`, telling `on` what happens as it happens (pixelfx's callbacks).
    pub fn update(&mut self, dt: f64, mut on: impl FnMut(PaneEvent)) {
        for i in (0..self.list.len()).rev() {
            let Some(q) = self.list.get_mut(i) else {
                continue;
            };
            q.life -= dt;
            if q.life <= 0.0 {
                self.list.remove(i);
                continue;
            }
            let floor = self.ground - 0.5;
            if q.rest
                && let Some(crumble) = q.crumble
            {
                q.rest_t += dt;
                if q.rest_t > crumble {
                    on(PaneEvent::Crumble { x: q.x, vx: q.vx });
                    self.list.remove(i);
                    continue;
                }
            }
            if q.rest {
                // skidding: friction slows the slide and the spin, the pane stays seated on the floor
                if libm::fabs(q.vx) > 8.0 {
                    on(PaneEvent::Skid {
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
                    vx: q.vx,
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

    /// Draw every pane in the ice ramp `c` (dark to bright).
    pub fn draw(&self, p: &mut Painter, c: &[Rgba; 8]) {
        let n = 8.0;
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
                if let (true, Some(tone)) = (q.solid, q.tone) {
                    // a chunk of one face: its tone, brighter as it squares up to the light
                    let col = if flash {
                        c[7]
                    } else {
                        step(
                            c,
                            clamp(
                                js_round(1.0 + tone * 5.0 * (0.6 + 0.4 * face)),
                                1.0,
                                n - 2.0,
                            ),
                        )
                    };
                    p.poly(&pts, col, a);
                } else if q.solid {
                    // shaded like the blade it broke from: a lit side, a pale ridge, a deep side
                    let rr = q.ridge.unwrap_or(-core::f64::consts::FRAC_PI_2);
                    let (cr, sr, ca, sa) =
                        (libm::cos(rr), libm::sin(rr), libm::cos(q.a), libm::sin(q.a));
                    let (rx, ry) = (cr * ca - sr * k * sa, cr * sa + sr * k * ca);
                    let (mut nx, mut ny) = (-ry, rx);
                    if nx * -0.55 + ny * -0.84 < 0.0 {
                        nx = -nx;
                        ny = -ny;
                    }
                    let lit = if flash {
                        c[7]
                    } else {
                        step(c, clamp(js_round(3.0 + lv * 3.0), 3.0, n - 2.0))
                    };
                    let deep = step(c, clamp(js_round(1.0 + lv * 2.0), 1.0, 3.0));
                    let (qx, qy, ridge) = (q.x, q.y, c[6]);
                    p.poly(
                        &pts,
                        |x: f64, y: f64| {
                            let d = (x - qx) * nx + (y - qy) * ny;
                            if d > 0.6 {
                                lit
                            } else if d > -0.6 {
                                ridge
                            } else {
                                deep
                            }
                        },
                        a,
                    );
                } else {
                    let col = if flash {
                        c[7]
                    } else {
                        step(c, clamp(js_round(3.0 + lv * 3.0), 3.0, n - 2.0))
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
                let l = libm::hypot(nx, ny);
                let l = if l == 0.0 { 1.0 } else { l };
                let lit = (-nx * 0.55 - ny * 0.84) / l;
                let col = if face < 0.25 || lit > 0.35 {
                    c[7]
                } else if lit > -0.25 {
                    c[6]
                } else if q.solid {
                    c[1]
                } else {
                    c[4]
                };
                p.line(x0, y0, x1, y1, col, a);
                if let Some(&(_, ty)) = pts.get(top)
                    && y0 < ty
                {
                    top = i;
                }
            }
            if face > 0.7
                && let Some(&(tx, ty)) = pts.get(top)
            {
                p.dot(tx, ty, WHITE, a);
            }
            p.glow(cx, cy, 5.0, c[5], (if flash { 0.5 } else { 0.14 }) * a);
        }
    }
}

/// Splinters thrown off where a piece of ice hits the floor at (x, y) moving `vx` (icicle fall's and ice spikes'
/// `splinter`).
pub fn splinter(bits: &mut Bits, r: &mut Rng, c: &[Rgba; 8], x: f64, y: f64, vx: f64, n: u32) {
    for _ in 0..n {
        bit!(bits, BitKind::Shard;
            x = x + r.range(-2.0, 2.0),
            y = y - 1.0,
            vx = vx * 0.5 + r.range(-45.0, 45.0),
            vy = -r.range(30.0, 100.0),
            g = 0.8,
            drag = 0.6,
            bounce = 0.35,
            life = r.range(0.4, 0.75),
            a = r.range(0.0, TAU),
            va = r.range(-18.0, 18.0),
            s = r.range(1.2, 2.4),
            c = step(c, r.int(4.0, 7.0)),
        );
    }
}

/// Nothing outlives the effect: trims the life of every particle, bit and pane that would last past `d`.
pub fn end_by(
    t: f64,
    d: f64,
    ps: &mut Particles,
    bits: Option<&mut Bits>,
    panes: Option<&mut Panes>,
) {
    let left = js_max(0.001, d - 0.05 - t);
    for q in ps.list_mut() {
        if q.life > left {
            q.life = left;
        }
    }
    if let Some(bits) = bits {
        for q in bits.list_mut() {
            if q.bit.life > left {
                q.bit.life = left;
            }
        }
    }
    if let Some(panes) = panes {
        for q in &mut panes.list {
            if q.life > left {
                q.life = left;
            }
        }
    }
}

/// Options of [`blade`]: alpha `a`, `white` flashes it white, `glint` a highlight across it at that height (0..1,
/// or -1), `jag` where the notch starts (0..1, or -1), `dim` steps darker, for back rows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BladeOpts {
    pub a: f64,
    pub white: bool,
    pub glint: f64,
    pub jag: f64,
    pub dim: f64,
}

impl Default for BladeOpts {
    fn default() -> Self {
        Self {
            a: 1.0,
            white: false,
            glint: -1.0,
            jag: -1.0,
            dim: 0.0,
        }
    }
}

/// A jagged ice blade standing on its base (bx, by), tip at (tx, ty), w wide at the base. Shaded in the ice ramp:
/// a lit facet on the left with a pale edge, a bright ridge, a deep facet on the right, a white tip, a dark rim,
/// and a notch cut into one side so it reads as a shard, not a cone.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn blade(
    p: &mut Painter,
    bx: f64,
    by: f64,
    tx: f64,
    ty: f64,
    w: f64,
    c: &[Rgba; 8],
    o: BladeOpts,
) {
    let (dx, dy) = (tx - bx, ty - by);
    let l_len = libm::hypot(dx, dy);
    if l_len < 1.5 || w < 0.5 {
        return;
    }
    let (ux, uy) = (dx / l_len, dy / l_len);
    let (nx, ny) = (-uy, ux);
    let (a, dim, jag, glint) = (o.a, o.dim, o.jag, o.glint);
    let k = |i: f64| step(c, clamp(i - dim, 0.0, 7.0));
    let hw = w / 2.0;
    // half widths on the left (lit) and right (shadow) sides at height u; the notch steps the right side in
    let left = |u: f64| hw * (1.0 - u) + 0.35;
    let right = |u: f64| hw * (1.0 - u) * (if jag >= 0.0 && u > jag { 0.55 } else { 1.0 }) + 0.35;
    let ridge = |u: f64| -w * 0.12 * (1.0 - u);
    let pad = hw + 2.0;
    p.shade(
        js_min(bx, tx) - pad,
        js_min(by, ty) - pad,
        js_max(bx, tx) + pad,
        js_max(by, ty),
        |x, y, out| {
            let (rx, ry) = (x - bx, y - by);
            let u = (rx * ux + ry * uy) / l_len;
            let s = rx * nx + ry * ny;
            if u < 0.0 {
                return None;
            }
            let l = left(js_min(u, 1.0));
            let r = right(js_min(u, 1.0));
            if u > 1.0 || s < -l || s > r {
                // a dark rim just outside the blade
                let over = if u > 1.0 { (u - 1.0) * l_len } else { 0.0 };
                let out_by = js_max(over, if s < -l { -l - s } else { s - r });
                if out_by < 1.0 && u < 1.0 + 1.0 / l_len {
                    out.c = k(0.0);
                    out.a = a;
                }
                return None;
            }
            out.a = a;
            if o.white {
                return Some(WHITE);
            }
            let rd = ridge(u);
            let mut col = if u > 0.88 {
                WHITE
            } else if libm::fabs(s - rd) < 0.55 {
                k(6.0)
            } else if s < rd {
                if s + l < 1.0 {
                    k(6.0)
                } else if (s + l) / js_max(1.0, rd + l) > 0.55 {
                    k(5.0)
                } else {
                    k(4.0)
                }
            } else if r - s < 1.1 {
                k(1.0)
            } else if (s - rd) / js_max(1.0, r - rd) > 0.5 {
                k(2.0)
            } else {
                k(3.0)
            };
            // the notch's ledge catches light; a little glitter in the lit facet
            if jag >= 0.0 && libm::fabs(u - jag) * l_len < 0.8 && s > rd {
                col = k(6.0);
            } else if s < rd && col != k(6.0) && glitter(x, y) {
                col = k(7.0);
            }
            if glint >= 0.0 && s <= rd + 0.6 {
                let g = libm::fabs(u - glint) * l_len;
                if g < 0.8 {
                    col = WHITE;
                } else if g < 1.8 {
                    col = k(6.0);
                }
            }
            Some(col)
        },
    );
}

/// JS `((Math.floor(x) * 73856093) ^ (Math.floor(y) * 19349663)) % 997 < 30`.
fn glitter(x: f64, y: f64) -> bool {
    let h = to_int32(libm::floor(x) * 73_856_093.0) ^ to_int32(libm::floor(y) * 19_349_663.0);
    h % 997 < 30
}

/// A snowflake design, seeded: pairs of side branches along each arm (where 0..1, length 0..1 of the arm), maybe
/// a hexagonal plate round the heart, maybe forked tips.
#[derive(Clone, Debug, PartialEq)]
pub struct FlakeDesign {
    pub branches: Vec<(f64, f64)>,
    pub plate: bool,
    pub fork: bool,
}

/// A seeded snowflake design (pixelfx `flakeDesign`).
pub fn flake_design(r: &mut Rng) -> FlakeDesign {
    let n = r.int(1.0, 3.0);
    let mut branches = Vec::new();
    let mut i = 0.0;
    while i < n {
        let u = 0.38 + (0.4 * (i + r.range(0.2, 0.8))) / n;
        let l = r.range(0.24, 0.36) * (1.0 - i * 0.15);
        branches.push((u, l));
        i += 1.0;
    }
    let plate = r.chance(0.45);
    let fork = r.chance(0.45);
    FlakeDesign {
        branches,
        plate,
        fork,
    }
}

/// A six-armed snowflake of radius r turned by rot, crisp 1 px lines: pale arms, side branches at 60 degrees, a
/// plate or forked tips by design, a white heart, light. `hot` (0..1) makes it twinkle white; `a` is its alpha.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn snowflake(
    p: &mut Painter,
    x: f64,
    y: f64,
    r: f64,
    rot: f64,
    d: &FlakeDesign,
    c: &[Rgba; 8],
    a: f64,
    hot: f64,
) {
    use core::f64::consts::PI;
    // turn in 10 degree ticks, like sprite frames
    let rot = js_round(rot / (PI / 18.0)) * (PI / 18.0);
    if r < 1.2 {
        p.dot(x, y, WHITE, a);
        return;
    }
    if r < 2.6 {
        // too small for branches: a six-point star that flips between two poses as it turns
        let pose = libm::floor(((rot % (PI / 3.0)) + PI / 3.0) / (PI / 6.0)) % 2.0;
        let pts: [(f64, f64); 4] = if pose != 0.0 {
            [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]
        } else {
            [(1.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0)]
        };
        for (dx, dy) in pts {
            p.dot(x + dx, y + dy, if hot > 0.5 { WHITE } else { c[5] }, a);
        }
        p.dot(x, y, WHITE, a);
        p.glow(x, y, 4.0, c[5], 0.2 * a);
        return;
    }
    // detail by size: a bare star when small, one pair of branches when middling, the full design when big
    let arm = if hot > 0.5 { WHITE } else { c[6] };
    let br = if hot > 0.5 { c[6] } else { c[5] };
    let big = r >= 7.0;
    let middling = [(0.58, 0.36)];
    let branches: &[(f64, f64)] = if r < 4.0 {
        &[]
    } else if big {
        d.branches.get(..2).unwrap_or(&d.branches)
    } else {
        &middling
    };
    for k in 0..6 {
        let g = rot + (f64::from(k) * TAU) / 6.0;
        let (ca, sa) = (libm::cos(g), libm::sin(g));
        p.line(x, y, x + ca * r, y + sa * r, arm, a);
        for &(u, l) in branches {
            let (bx, by, bl) = (x + ca * r * u, y + sa * r * u, js_max(1.0, r * l));
            for s in [-1.0, 1.0] {
                let ang = g + (s * PI) / 3.0;
                p.line(
                    bx,
                    by,
                    bx + libm::cos(ang) * bl,
                    by + libm::sin(ang) * bl,
                    br,
                    a,
                );
            }
        }
        if d.fork && big && !d.plate {
            for s in [-1.0, 1.0] {
                p.dot(
                    x + ca * r + libm::cos(g + s * 0.9) * 1.2,
                    y + sa * r + libm::sin(g + s * 0.9) * 1.2,
                    br,
                    a,
                );
            }
        }
        p.dot(x + ca * r, y + sa * r, WHITE, a);
    }
    if d.plate && big {
        let q = r * 0.34;
        for k in 0..6 {
            let g0 = rot + PI / 6.0 + (f64::from(k) * TAU) / 6.0;
            let g1 = g0 + TAU / 6.0;
            p.line(
                x + libm::cos(g0) * q,
                y + libm::sin(g0) * q,
                x + libm::cos(g1) * q,
                y + libm::sin(g1) * q,
                c[5],
                a,
            );
        }
    }
    p.px(x, y, WHITE);
    p.glow(x, y, 3.0 + r * 1.2, c[4], (0.14 + 0.35 * hot) * a);
    if hot > 0.3 {
        p.glow(x, y, 2.0 + r * 0.5, WHITE, 0.3 * hot * a);
    }
}

/// A crack: points (x, y, distance from the start along it).
pub type Crack = Vec<(f64, f64, f64)>;

/// Options of [`fractures`]: step length range, steps per crack, fork chance, first angle (`None`: random),
/// the spread of the cracks round it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractureOpts {
    pub step: (f64, f64),
    pub len: f64,
    pub fork: f64,
    pub a0: Option<f64>,
    pub spread: f64,
}

impl Default for FractureOpts {
    fn default() -> Self {
        Self {
            step: (1.6, 2.6),
            len: 18.0,
            fork: 0.16,
            a0: None,
            spread: TAU,
        }
    }
}

/// Cracks racing out from (x, y) through a shape: n jagged lines that wander, fork now and then and stop at the
/// shape's edge (`inside(x, y)` says what's in it). Each point carries its distance from the start along the
/// crack, so [`draw_fractures`] can grow them at a speed.
pub fn fractures(
    r: &mut Rng,
    inside: &impl Fn(f64, f64) -> bool,
    x: f64,
    y: f64,
    n: u32,
    o: FractureOpts,
) -> Vec<Crack> {
    struct Walk<'a, F> {
        r: &'a mut Rng,
        inside: &'a F,
        o: FractureOpts,
        out: Vec<Crack>,
    }
    impl<F: Fn(f64, f64) -> bool> Walk<'_, F> {
        fn walk(&mut self, mut x: f64, mut y: f64, mut d: f64, mut a: f64, len: f64, depth: u32) {
            let mut pts = vec![(x, y, d)];
            let mut s = 0.0;
            while s < len {
                s += 1.0;
                a += self.r.range(-0.55, 0.55);
                let st = self.r.range(self.o.step.0, self.o.step.1);
                let (nx, ny) = (x + libm::cos(a) * st, y + libm::sin(a) * st);
                if !(self.inside)(nx, ny) {
                    break;
                }
                x = nx;
                y = ny;
                d += st;
                pts.push((x, y, d));
                if depth < 2 && pts.len() > 2 && self.r.chance(self.o.fork) {
                    let turn = self.r.sign() * self.r.range(0.6, 1.2);
                    self.walk(x, y, d, a + turn, libm::ceil(len * 0.5), depth + 1);
                }
            }
            if pts.len() > 1 {
                self.out.push(pts);
            }
        }
    }
    let a0 = match o.a0 {
        Some(a) => a,
        None => r.range(0.0, TAU),
    };
    let spread = o.spread;
    let mut w = Walk {
        r,
        inside,
        o,
        out: Vec::new(),
    };
    for i in 0..n {
        let a = a0
            + (f64::from(i) / f64::from(n)) * spread
            + w.r.range(-0.3, 0.3) * js_min(1.0, spread / TAU);
        w.walk(x, y, 0.0, a, o.len, 0);
    }
    w.out
}

/// Draw cracks grown out to distance `reach` from their start: a bright line over a dark one a pixel lower (an
/// engraved fracture), a white-hot tip while they run (`hot`), light leaking out along them. ox shifts them.
pub fn draw_fractures(p: &mut Painter, cracks: &[Crack], reach: f64, c: &[Rgba; 8], ox: f64) {
    let (oy, hot, a) = (0.0, true, 1.0);
    let seg = |pts: &Crack, i: usize| -> Option<(f64, f64, f64, f64, bool)> {
        let (&(x0, y0, d0), &(x1, y1, d1)) = (pts.get(i - 1)?, pts.get(i)?);
        if d0 >= reach {
            return None;
        }
        let u = js_min(1.0, (reach - d0) / js_max(0.01, d1 - d0));
        Some((
            x0 + ox,
            y0 + oy,
            x0 + ox + (x1 - x0) * u,
            y0 + oy + (y1 - y0) * u,
            reach - d1 < 3.0,
        ))
    };
    for pts in cracks {
        for i in 1..pts.len() {
            let Some(s) = seg(pts, i) else {
                break;
            };
            p.line(s.0, s.1 + 1.0, s.2, s.3 + 1.0, c[2], a);
        }
    }
    for pts in cracks {
        for i in 1..pts.len() {
            let Some(s) = seg(pts, i) else {
                break;
            };
            let tip = hot && s.4;
            p.line(s.0, s.1, s.2, s.3, if tip { WHITE } else { c[6] }, a);
            if tip {
                p.glow(s.2, s.3, 5.0, c[5], 0.5 * a);
            } else if i % 2 == 1 {
                p.glow(s.2, s.3, 3.0, c[5], 0.18 * a);
            }
        }
    }
}

/// JS `Math.hypot(...)` of any number of values as V8 computes it (scaled by the largest value, Kahan-summed);
/// for two values the ice effects use `libm::hypot`, as the core does.
pub fn js_hypot(xs: &[f64]) -> f64 {
    let mut max = 0.0_f64;
    for &x in xs {
        let a = libm::fabs(x);
        if a.is_infinite() {
            return f64::INFINITY;
        }
        if a.is_nan() || a > max {
            max = if a.is_nan() { f64::NAN } else { a };
        }
    }
    if max.is_nan() {
        return f64::NAN;
    }
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut comp) = (0.0, 0.0);
    for &x in xs {
        let n = libm::fabs(x) / max;
        let summand = n * n - comp;
        let pre = sum + summand;
        comp = (pre - sum) - summand;
        sum = pre;
    }
    libm::sqrt(sum) * max
}
