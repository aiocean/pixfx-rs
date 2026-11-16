//! MLS-MPM in 2D, written from sparkl and slosh (`references/dimforge/sparkl`, `references/dimforge/slosh`):
//! particles carry mass, velocity, an affine velocity field (APIC) and their volume ratio `J`; each sub-step scatters
//! them onto a grid with quadratic B-spline weights, applies gravity and the walls on the grid, and gathers the
//! velocities back. The liquid is an equation of state on `J` (bulk modulus) plus a Newtonian viscosity, so a thick
//! liquid like blood keeps its body, strings and pools. Particles never leave the grid: the walls hold them in.

use super::{Floor, substeps};

/// How an MPM liquid behaves and the grid it lives on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MpmOpts {
    /// Top-left corner of the grid in local pixels.
    pub origin: (f64, f64),
    /// Grid spacing in pixels; particles sit four to a cell at rest.
    pub cell: f64,
    /// Grid nodes across and down (at least 4 each).
    pub cols: usize,
    pub rows: usize,
    /// Bulk modulus in (px/s)²: how hard the liquid resists squeezing. Sound speed is its square root.
    pub bulk: f64,
    /// Dynamic viscosity in px²/s: 0 for water, higher for blood and mud.
    pub viscosity: f64,
    /// Gravity in px/s², down is positive.
    pub gravity: f64,
    /// The ground, if any (the grid's own walls hold the liquid too).
    pub floor: Option<Floor>,
    /// Longest sub-step, seconds.
    pub max_dt: f64,
}

impl Default for MpmOpts {
    fn default() -> Self {
        Self {
            origin: (-64.0, -64.0),
            cell: 2.0,
            cols: 64,
            rows: 64,
            bulk: 40_000.0,
            viscosity: 0.0,
            gravity: 320.0,
            floor: None,
            max_dt: 1.0 / 480.0,
        }
    }
}

/// One particle of liquid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MpmParticle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    /// Affine velocity (APIC `C`), row major.
    pub c: [f64; 4],
    /// Volume now over volume at rest.
    pub j: f64,
    /// Seconds since it was added.
    pub age: f64,
    /// The caller's tag; the solver never reads it.
    pub tag: u32,
}

/// An MPM liquid.
#[derive(Clone, Debug)]
pub struct Mpm {
    opts: MpmOpts,
    parts: Vec<MpmParticle>,
    /// Per node: mass, momentum x, momentum y (then velocity after the grid update).
    grid: Vec<[f64; 3]>,
}

/// Quadratic B-spline weights of the three nodes from `base` for a particle at `f` (0.5..1.5) past it.
fn weights(f: f64) -> [f64; 3] {
    let a = 1.5 - f;
    let b = f - 1.0;
    let c = f - 0.5;
    [0.5 * a * a, 0.75 - b * b, 0.5 * c * c]
}

impl Mpm {
    pub fn new(opts: MpmOpts) -> Self {
        let cols = opts.cols.clamp(4, 2048);
        let rows = opts.rows.clamp(4, 2048);
        let opts = MpmOpts {
            cols,
            rows,
            cell: opts.cell.max(0.25),
            ..opts
        };
        Self {
            opts,
            parts: Vec::new(),
            grid: vec![[0.0; 3]; cols * rows],
        }
    }

    pub fn opts(&self) -> &MpmOpts {
        &self.opts
    }

    /// Rest spacing of the particles (half a cell).
    pub fn spacing(&self) -> f64 {
        self.opts.cell * 0.5
    }

    /// Adds a particle at `(x, y)` moving at `(vx, vy)` px/s.
    pub fn add(&mut self, x: f64, y: f64, vx: f64, vy: f64, tag: u32) {
        let (x, y) = self.inside(x, y);
        self.parts.push(MpmParticle {
            x,
            y,
            vx,
            vy,
            c: [0.0; 4],
            j: 1.0,
            age: 0.0,
            tag,
        });
    }

    /// Adds particles at the rest spacing filling the disc at `(cx, cy)` of radius `r`, all moving at `(vx, vy)`.
    pub fn add_disc(&mut self, cx: f64, cy: f64, r: f64, vx: f64, vy: f64, tag: u32) {
        let s = self.spacing();
        let n = libm::ceil(r / s) as i32;
        for j in -n..=n {
            for i in -n..=n {
                let (x, y) = (f64::from(i) * s, f64::from(j) * s);
                if x * x + y * y <= r * r {
                    self.add(cx + x, cy + y, vx, vy, tag);
                }
            }
        }
    }

    pub fn particles(&self) -> &[MpmParticle] {
        &self.parts
    }

    /// The particles, for a caller that removes some (blood soaking into the ground).
    pub fn particles_mut(&mut self) -> &mut Vec<MpmParticle> {
        &mut self.parts
    }

    /// Advances `dt` seconds in sub-steps of at most [`MpmOpts::max_dt`].
    pub fn step(&mut self, dt: f64) {
        let Some((n, h)) = substeps(dt, self.opts.max_dt) else {
            return;
        };
        for _ in 0..n {
            self.substep(h);
        }
    }

    /// The point held to where every particle's three-node stencil is on the grid.
    fn inside(&self, x: f64, y: f64) -> (f64, f64) {
        let o = &self.opts;
        let lo = 1.0;
        let ux = ((x - o.origin.0) / o.cell).clamp(lo, o.cols as f64 - 2.0 - 1e-6);
        let uy = ((y - o.origin.1) / o.cell).clamp(lo, o.rows as f64 - 2.0 - 1e-6);
        let ux = if ux.is_finite() { ux } else { lo };
        let uy = if uy.is_finite() { uy } else { lo };
        (o.origin.0 + ux * o.cell, o.origin.1 + uy * o.cell)
    }

    /// Grid coordinates of a particle: the first of its three nodes and its offset past it, on each axis.
    fn stencil(&self, x: f64, y: f64) -> (usize, usize, f64, f64) {
        let o = &self.opts;
        // a particle a caller moved off the grid (`particles_mut`) is read where the grid holds it
        let (x, y) = self.inside(x, y);
        let ux = (x - o.origin.0) / o.cell;
        let uy = (y - o.origin.1) / o.cell;
        let bx = libm::floor(ux - 0.5).max(0.0);
        let by = libm::floor(uy - 0.5).max(0.0);
        (bx as usize, by as usize, ux - bx, uy - by)
    }

    fn substep(&mut self, dt: f64) {
        let o = self.opts;
        let dx = o.cell;
        let inv_d = 4.0 / (dx * dx);
        let vol = (dx * 0.5) * (dx * 0.5);
        let mass = vol; // density 1
        self.grid.fill([0.0; 3]);
        // particles to grid
        for p in &self.parts {
            let (bx, by, fx, fy) = self.stencil(p.x, p.y);
            let (wx, wy) = (weights(fx), weights(fy));
            let pressure = -dt * vol * inv_d * o.bulk * (p.j - 1.0);
            let visc = -dt * vol * inv_d * o.viscosity;
            let (c00, c01, c10, c11) = (p.c[0], p.c[1], p.c[2], p.c[3]);
            // affine = stress + mass * C, stress = pressure * I + viscosity * (C + C^T)
            let a00 = pressure + visc * 2.0 * c00 + mass * c00;
            let a01 = visc * (c01 + c10) + mass * c01;
            let a10 = visc * (c10 + c01) + mass * c10;
            let a11 = pressure + visc * 2.0 * c11 + mass * c11;
            for (j, &wyj) in wy.iter().enumerate() {
                for (i, &wxi) in wx.iter().enumerate() {
                    let w = wxi * wyj;
                    let px = (i as f64 - fx) * dx;
                    let py = (j as f64 - fy) * dx;
                    let k = (by + j) * o.cols + bx + i;
                    if let Some(g) = self.grid.get_mut(k) {
                        g[0] += w * mass;
                        g[1] += w * (mass * p.vx + a00 * px + a01 * py);
                        g[2] += w * (mass * p.vy + a10 * px + a11 * py);
                    }
                }
            }
        }
        // grid update: momentum to velocity, gravity, walls and floor
        let slide = o.floor.map_or(1.0, |f| f.slide(dt));
        for (k, g) in self.grid.iter_mut().enumerate() {
            if g[0] <= 0.0 {
                continue;
            }
            let (i, j) = (k % o.cols, k / o.cols);
            let mut vx = g[1] / g[0];
            let mut vy = g[2] / g[0] + dt * o.gravity;
            if (i < 2 && vx < 0.0) || (i + 3 > o.cols && vx > 0.0) {
                vx = 0.0;
            }
            if j < 2 && vy < 0.0 {
                vy = 0.0;
            }
            let ny = o.origin.1 + j as f64 * dx;
            let on_floor = o.floor.is_some_and(|f| ny >= f.y);
            if (j + 3 > o.rows || on_floor) && vy > 0.0 {
                vy = 0.0;
                vx *= slide;
            }
            g[1] = vx;
            g[2] = vy;
        }
        // grid to particles
        let floor = o.floor;
        for idx in 0..self.parts.len() {
            let Some(&p) = self.parts.get(idx) else {
                continue;
            };
            let (bx, by, fx, fy) = self.stencil(p.x, p.y);
            let (wx, wy) = (weights(fx), weights(fy));
            let (mut vx, mut vy) = (0.0, 0.0);
            let mut c = [0.0; 4];
            for (j, &wyj) in wy.iter().enumerate() {
                for (i, &wxi) in wx.iter().enumerate() {
                    let w = wxi * wyj;
                    let px = (i as f64 - fx) * dx;
                    let py = (j as f64 - fy) * dx;
                    let k = (by + j) * o.cols + bx + i;
                    let Some(g) = self.grid.get(k) else { continue };
                    vx += w * g[1];
                    vy += w * g[2];
                    c[0] += inv_d * w * g[1] * px;
                    c[1] += inv_d * w * g[1] * py;
                    c[2] += inv_d * w * g[2] * px;
                    c[3] += inv_d * w * g[2] * py;
                }
            }
            let mut x = p.x + dt * vx;
            let mut y = p.y + dt * vy;
            // the grid stops the liquid on the floor; this only catches a particle the grid's weights let sink
            // past it by more than a cell (pushing back every small overshoot squeezes the volume, which comes back
            // as a bounce)
            if let Some(f) = floor
                && y > f.y + dx
            {
                y = f.y + dx;
            }
            (x, y) = self.inside(x, y);
            let j = (p.j * (1.0 + dt * (c[0] + c[3]))).clamp(0.5, 1.5);
            if let Some(q) = self.parts.get_mut(idx) {
                *q = MpmParticle {
                    x,
                    y,
                    vx,
                    vy,
                    c,
                    j,
                    age: p.age + dt,
                    tag: p.tag,
                };
            }
        }
    }
}
