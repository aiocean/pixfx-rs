//! Weakly compressible SPH in 2D, written from salva (`references/dimforge/salva`): its poly6 density kernel, spiky
//! pressure gradient and a symmetric pressure force, with XSPH velocity smoothing in place of a viscosity solve.
//! The rest density is measured on a square lattice of the particle spacing, so a block laid out at that spacing
//! starts at rest pressure. Neighbours are found on a fixed grid of cells one kernel radius wide over
//! [`SphOpts::bounds`]; a particle that leaves the bounds is dropped.

use core::f64::consts::PI;

use super::{Floor, substeps};

/// How an SPH liquid behaves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphOpts {
    /// Kernel radius in pixels: how far a particle feels its neighbours. About twice the spacing.
    pub h: f64,
    /// Spacing of the particles at rest, in pixels.
    pub spacing: f64,
    /// Speed of sound in px/s: how stiff the liquid is. About ten times the fastest the liquid moves.
    pub sound: f64,
    /// Surface tension, px/s² at its strongest (0 for none): neighbours a little apart pull together, so liquid
    /// keeps a thick puddle and breaks into round drops. Akinci et al. 2013's cohesion, as salva's
    /// `Akinci2013SurfaceTension` has it; its curvature term is left out, since in 2D it tears apart drops of a few
    /// particles (`tests/fluid.rs` holds one together).
    pub tension: f64,
    /// XSPH smoothing, 0..1: the share of its neighbours' velocity a particle takes on every 1/60 s, the same
    /// whatever the sub-step (thick liquid is higher).
    pub viscosity: f64,
    /// Gravity in px/s², down is positive.
    pub gravity: f64,
    /// The ground, if any.
    pub floor: Option<Floor>,
    /// `(x0, y0, x1, y1)`: the box particles live in; one that leaves it is dropped.
    pub bounds: (f64, f64, f64, f64),
    /// Longest sub-step, seconds.
    pub max_dt: f64,
}

impl Default for SphOpts {
    fn default() -> Self {
        Self {
            h: 4.0,
            spacing: 2.0,
            sound: 160.0,
            tension: 0.0,
            viscosity: 0.5,
            gravity: 320.0,
            floor: None,
            bounds: (-128.0, -128.0, 128.0, 128.0),
            max_dt: 1.0 / 600.0,
        }
    }
}

/// One particle of liquid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drop {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    /// Density over the rest density: 1 inside a body of liquid, toward 0 for a droplet alone.
    pub fill: f64,
    /// Seconds since it was added.
    pub age: f64,
    /// The caller's tag (a colour, a source); the solver never reads it.
    pub tag: u32,
}

/// An SPH liquid.
#[derive(Clone, Debug)]
pub struct Sph {
    opts: SphOpts,
    drops: Vec<Drop>,
    rho0: f64,
    stiffness: f64,
    density: Vec<f64>,
    pressure: Vec<f64>,
    ax: Vec<f64>,
    ay: Vec<f64>,
    sx: Vec<f64>,
    sy: Vec<f64>,
    /// Each particle's cell, then the cells' fill cursors: scratch for [`Sph::bin`].
    cell_of: Vec<usize>,
    cursor: Vec<usize>,
    /// Particle `i`'s neighbours within the kernel radius are `pairs[pair_at[i]..pair_at[i + 1]]`, found once a
    /// sub-step and read by every pass after.
    pairs: Vec<usize>,
    pair_at: Vec<usize>,
    cols: usize,
    rows: usize,
    start: Vec<usize>,
    order: Vec<usize>,
}

fn poly6(r2: f64, h: f64) -> f64 {
    let h2 = h * h;
    if r2 >= h2 {
        return 0.0;
    }
    let d = h2 - r2;
    4.0 / (PI * libm::pow(h, 8.0)) * d * d * d
}

/// Akinci's cohesion kernel scaled to 1 at its peak (`h / 2`): negative (a push) closer than that peak's
/// neighbourhood, a pull out to `h`.
fn cohesion(r: f64, h: f64) -> f64 {
    let h6 = libm::pow(h, 6.0);
    let c = if r <= h * 0.5 {
        let d = h - r;
        2.0 * d * d * d * r * r * r - h6 / 64.0
    } else if r < h {
        let d = h - r;
        d * d * d * r * r * r
    } else {
        0.0
    };
    c * 64.0 / h6
}

/// Magnitude of the spiky kernel's gradient at distance `r` (it points away from the neighbour).
fn spiky_grad(r: f64, h: f64) -> f64 {
    if r >= h {
        return 0.0;
    }
    let d = h - r;
    -30.0 / (PI * libm::pow(h, 5.0)) * d * d
}

impl Sph {
    pub fn new(opts: SphOpts) -> Self {
        let h = opts.h.max(0.5);
        let s = opts.spacing.max(0.25);
        // rest density: a particle inside a square lattice of the spacing (mass 1 each)
        let reach = libm::ceil(h / s) as i32;
        let mut rho0 = 0.0;
        for j in -reach..=reach {
            for i in -reach..=reach {
                let (x, y) = (f64::from(i) * s, f64::from(j) * s);
                rho0 += poly6(x * x + y * y, h);
            }
        }
        let (x0, y0, x1, y1) = opts.bounds;
        let cols = (libm::ceil((x1 - x0).max(0.0) / h) as usize).clamp(1, 4096);
        let rows = (libm::ceil((y1 - y0).max(0.0) / h) as usize).clamp(1, 4096);
        Self {
            opts: SphOpts {
                h,
                spacing: s,
                ..opts
            },
            drops: Vec::new(),
            rho0,
            // p = k (rho / rho0 - 1) gives a sound speed c with c^2 = k / rho0
            stiffness: opts.sound * opts.sound * rho0,
            density: Vec::new(),
            pressure: Vec::new(),
            ax: Vec::new(),
            ay: Vec::new(),
            sx: Vec::new(),
            sy: Vec::new(),
            cell_of: Vec::new(),
            cursor: Vec::new(),
            pairs: Vec::new(),
            pair_at: Vec::new(),
            cols,
            rows,
            start: vec![0; cols * rows + 1],
            order: Vec::new(),
        }
    }

    pub fn opts(&self) -> &SphOpts {
        &self.opts
    }

    /// Adds a particle at `(x, y)` moving at `(vx, vy)` px/s.
    pub fn add(&mut self, x: f64, y: f64, vx: f64, vy: f64, tag: u32) {
        self.drops.push(Drop {
            x,
            y,
            vx,
            vy,
            fill: 1.0,
            age: 0.0,
            tag,
        });
    }

    /// Adds a block of particles at the rest spacing filling the disc at `(cx, cy)` of radius `r`, all moving at
    /// `(vx, vy)`.
    pub fn add_disc(&mut self, cx: f64, cy: f64, r: f64, vx: f64, vy: f64, tag: u32) {
        let s = self.opts.spacing;
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

    pub fn drops(&self) -> &[Drop] {
        &self.drops
    }

    /// The particles, for a caller that removes or nudges some (a drop soaking into the ground).
    pub fn drops_mut(&mut self) -> &mut Vec<Drop> {
        &mut self.drops
    }

    /// Advances `dt` seconds in sub-steps of at most [`SphOpts::max_dt`].
    pub fn step(&mut self, dt: f64) {
        let Some((n, h)) = substeps(dt, self.opts.max_dt) else {
            return;
        };
        for _ in 0..n {
            self.substep(h);
        }
    }

    fn cell(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        let (x0, y0, _, _) = self.opts.bounds;
        let cx = libm::floor((x - x0) / self.opts.h);
        let cy = libm::floor((y - y0) / self.opts.h);
        if cx < 0.0 || cy < 0.0 || cx >= self.cols as f64 || cy >= self.rows as f64 {
            return None;
        }
        Some((cx as usize, cy as usize))
    }

    /// Counting sort of the particles by cell: `order[start[c]..start[c + 1]]` are the particles in cell `c`,
    /// in the order they were added, so every sum below runs in the same order on every target.
    fn bin(&mut self) {
        let (x0, y0, x1, y1) = self.opts.bounds;
        self.drops.retain(|d| {
            d.x >= x0 && d.x < x1 && d.y >= y0 && d.y < y1 && d.x.is_finite() && d.y.is_finite()
        });
        let cells = self.cols * self.rows;
        self.start.clear();
        self.start.resize(cells + 1, 0);
        self.cell_of.clear();
        for d in &self.drops {
            let c = self
                .cell(d.x, d.y)
                .map_or(cells, |(cx, cy)| cy * self.cols + cx);
            self.cell_of.push(c);
            if let Some(s) = self.start.get_mut(c) {
                *s += 1;
            }
        }
        let mut sum = 0;
        for s in &mut self.start {
            let k = *s;
            *s = sum;
            sum += k;
        }
        self.cursor.clear();
        self.cursor.extend_from_slice(&self.start);
        self.order.clear();
        self.order.resize(self.drops.len(), 0);
        for (i, &c) in self.cell_of.iter().enumerate() {
            if let Some(f) = self.cursor.get_mut(c) {
                if let Some(o) = self.order.get_mut(*f) {
                    *o = i;
                }
                *f += 1;
            }
        }
    }

    /// Lists every particle's neighbours closer than the kernel radius, itself included, in the 3 by 3 cells
    /// round its own, cell by cell and in each cell in the order the particles were added.
    fn neighbours(&mut self) {
        let h2 = self.opts.h * self.opts.h;
        self.pairs.clear();
        self.pair_at.clear();
        for d in &self.drops {
            self.pair_at.push(self.pairs.len());
            let Some((cx, cy)) = self.cell(d.x, d.y) else {
                continue;
            };
            for y in cy.saturating_sub(1)..=(cy + 1).min(self.rows - 1) {
                for x in cx.saturating_sub(1)..=(cx + 1).min(self.cols - 1) {
                    let c = y * self.cols + x;
                    let (Some(&a), Some(&b)) = (self.start.get(c), self.start.get(c + 1)) else {
                        continue;
                    };
                    for &j in self.order.get(a..b).unwrap_or(&[]) {
                        let Some(dj) = self.drops.get(j) else {
                            continue;
                        };
                        let (dx, dy) = (d.x - dj.x, d.y - dj.y);
                        if dx * dx + dy * dy < h2 {
                            self.pairs.push(j);
                        }
                    }
                }
            }
        }
        self.pair_at.push(self.pairs.len());
    }

    /// The neighbours of particle `i` found by [`Sph::neighbours`].
    fn near(&self, i: usize) -> &[usize] {
        match (self.pair_at.get(i), self.pair_at.get(i + 1)) {
            (Some(&a), Some(&b)) => self.pairs.get(a..b).unwrap_or(&[]),
            _ => &[],
        }
    }

    fn substep(&mut self, dt: f64) {
        self.bin();
        self.neighbours();
        let n = self.drops.len();
        let h = self.opts.h;
        let h2 = h * h;
        let floor = self.opts.floor;
        // density and pressure
        self.density.clear();
        self.pressure.clear();
        for i in 0..n {
            let Some(&di) = self.drops.get(i) else {
                continue;
            };
            let mut rho = 0.0;
            for &j in self.near(i) {
                let Some(dj) = self.drops.get(j) else {
                    continue;
                };
                let (dx, dy) = (di.x - dj.x, di.y - dj.y);
                rho += poly6(dx * dx + dy * dy, h);
                // the floor holds a mirror image of each neighbour (and of the particle itself), so liquid lying on
                // it is as dense as liquid inside a body and the floor pushes back by pressure; a mirror within the
                // kernel radius belongs to a neighbour that is itself within it
                if let Some(f) = floor {
                    let my = di.y - (2.0 * f.y - dj.y);
                    rho += poly6(dx * dx + my * my, h);
                }
            }
            let ratio = rho / self.rho0;
            let p = self.stiffness * (ratio - 1.0);
            self.density.push(rho.max(1e-9));
            // pressure only pushes; holding together is surface tension's job
            self.pressure.push(p.max(0.0));
        }
        let tension = self.opts.tension;
        // pressure force (symmetric, so momentum is kept), surface tension and XSPH smoothing
        self.ax.clear();
        self.ay.clear();
        self.sx.clear();
        self.sy.clear();
        for i in 0..n {
            let Some(&di) = self.drops.get(i) else {
                continue;
            };
            let (rho_i, p_i) = (self.density[i], self.pressure[i]);
            let (mut fx, mut fy, mut vx, mut vy) = (0.0, 0.0, 0.0, 0.0);
            for &j in self.near(i) {
                let Some(dj) = self.drops.get(j) else {
                    continue;
                };
                let (rho_j, p_j) = (self.density[j], self.pressure[j]);
                let pk = p_i / (rho_i * rho_i) + p_j / (rho_j * rho_j);
                if let Some(f) = floor {
                    let (dx, dy) = (di.x - dj.x, di.y - (2.0 * f.y - dj.y));
                    let r2 = dx * dx + dy * dy;
                    if r2 < h2 && r2 > 1e-18 {
                        let r = libm::sqrt(r2);
                        let k = -pk * spiky_grad(r, h);
                        fx += k * dx / r;
                        fy += k * dy / r;
                    }
                }
                if j == i {
                    continue;
                }
                let (dx, dy) = (di.x - dj.x, di.y - dj.y);
                let r2 = dx * dx + dy * dy;
                let r = libm::sqrt(r2);
                // two particles on the same spot push apart along x, the earlier one to the right
                let (ux, uy) = if r > 1e-9 {
                    (dx / r, dy / r)
                } else if i < j {
                    (1.0, 0.0)
                } else {
                    (-1.0, 0.0)
                };
                let k = -pk * spiky_grad(r, h);
                fx += k * ux;
                fy += k * uy;
                if tension != 0.0 {
                    // cohesion pulls at mid range and pushes when too close
                    let kij = 2.0 * self.rho0 / (rho_i + rho_j);
                    let c = kij * tension * cohesion(r, h);
                    fx -= c * ux;
                    fy -= c * uy;
                }
                let w = poly6(r2, h) / rho_j;
                vx += w * (dj.vx - di.vx);
                vy += w * (dj.vy - di.vy);
            }
            self.ax.push(fx);
            self.ay.push(fy + self.opts.gravity);
            self.sx.push(vx);
            self.sy.push(vy);
        }
        // XSPH: the share of the neighbours' velocity taken over dt, set per 1/60 s so it does not depend on the
        // sub-step
        let eps = 1.0 - libm::pow(1.0 - self.opts.viscosity.clamp(0.0, 1.0), dt * 60.0);
        for (i, d) in self.drops.iter_mut().enumerate() {
            d.vx += self.ax[i] * dt + eps * self.sx[i];
            d.vy += self.ay[i] * dt + eps * self.sy[i];
            d.x += d.vx * dt;
            d.y += d.vy * dt;
            d.fill = self.density[i] / self.rho0;
            d.age += dt;
            if let Some(f) = floor
                && d.y > f.y
            {
                d.y = f.y;
                if d.vy > 0.0 {
                    d.vy = -d.vy * f.restitution;
                }
                d.vx *= f.slide(dt);
            }
        }
    }
}
