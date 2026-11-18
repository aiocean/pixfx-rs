//! `feedback/soaked`: water (or blood with `palette: "blood"`, acid with `palette: "poison"`) beads gather at the
//! wrapped sprite's extremities (hands, hem, hair and shoulders), swell and fall as teardrops that splat with ripples
//! into a pool at the feet, while runnels slide down the flanks.
//!
//! Adapted from Pixel VFX Studio's `status/soaked` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026 Hugo
//! Duprez). Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it maps
//! the drip sources and flank runnels onto the sprite's [`Silhouette`] edge once per pose; with no sprite it uses the
//! stand-in coordinates.

use std::sync::Arc;

use pixfx_core::{Actor, Area, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{Rng, fract, hash, js_round, js_sign, or_one, to_int32};

pub struct Soaked;

pub static SOAKED: Soaked = Soaked;

const FPS: u32 = 16;
const N: i32 = 16;
const POOL_CX: f64 = 0.0;
const POOL_CY: f64 = 1.0;
const POOL_RX: f64 = 15.0;
const POOL_RY: f64 = 3.8;
const GRAV: f64 = 0.55;
const HANG: f64 = 0.42;
const BOX: Area = Area {
    x: -22,
    y: -48,
    w: 44,
    h: 50,
};

const POOL_OPT: NumOpt = NumOpt::new("pool", 15.0).within(12.0, 18.0);
const DRIPS: NumOpt = NumOpt::new("drips", 8.0).within(2.0, 8.0);
const RUNNELS: NumOpt = NumOpt::new("runnels", 3.0).within(0.0, 3.0);
const SPLASH: NumOpt = NumOpt::new("splash", 1.0).within(0.0, 1.0);

const META: Meta = Meta {
    id: "feedback/soaked",
    name: "Soaked",
    desc: "Beads gather at the wrapped sprite's hands, hem, hair and shoulders, swell and fall as teardrops that splat with ripples into a pool at the feet while runnels slide down the flanks; use palette 'blood' for bleeding.",
    tags: &[
        "feedback",
        "water",
        "wet",
        "bleeding",
        "status",
        "character",
        "loop",
    ],
    size: (48, 56),
    origin: (24, 50),
    duration: 1.0,
    looping: true,
    fps: FPS,
    seed: 1_819_087_851,
    state: None,
    options: &[POOL_OPT.spec(), DRIPS.spec(), RUNNELS.spec(), SPLASH.spec()],
};

const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x0d1f45),
    Rgba::hex24(0x1a4c8f),
    Rgba::hex24(0x2a7bd0),
    Rgba::hex24(0x53b8f0),
    Rgba::hex24(0xb2ecff),
    Rgba::hex24(0xffffff),
];

#[derive(Clone, Copy)]
struct DripSrc {
    x: f64,
    y: f64,
    dy: f64,
    ph: f64,
    near: bool,
}

const STAND_SRC: [DripSrc; 8] = [
    DripSrc {
        x: -9.0,
        y: -21.0,
        dy: 1.5,
        ph: 0.0,
        near: true,
    },
    DripSrc {
        x: 10.0,
        y: -23.0,
        dy: 1.0,
        ph: 0.42,
        near: true,
    },
    DripSrc {
        x: -7.0,
        y: -35.0,
        dy: -1.0,
        ph: 0.7,
        near: false,
    },
    DripSrc {
        x: 11.0,
        y: -12.0,
        dy: 2.5,
        ph: 0.18,
        near: true,
    },
    DripSrc {
        x: -11.0,
        y: -10.0,
        dy: 0.5,
        ph: 0.85,
        near: false,
    },
    DripSrc {
        x: 6.0,
        y: -31.0,
        dy: -1.5,
        ph: 0.6,
        near: false,
    },
    DripSrc {
        x: 4.0,
        y: -39.0,
        dy: -0.5,
        ph: 0.3,
        near: false,
    },
    DripSrc {
        x: -3.0,
        y: -8.0,
        dy: 2.0,
        ph: 0.55,
        near: true,
    },
];

#[derive(Clone, Copy)]
struct RunnelSpec {
    x: f64,
    y0: f64,
    y1: f64,
    ph: f64,
}

const STAND_RUN: [RunnelSpec; 3] = [
    RunnelSpec {
        x: -12.0,
        y0: -30.0,
        y1: -14.0,
        ph: 0.1,
    },
    RunnelSpec {
        x: 12.0,
        y0: -28.0,
        y1: -16.0,
        ph: 0.55,
    },
    RunnelSpec {
        x: -10.0,
        y0: -16.0,
        y1: -4.0,
        ph: 0.75,
    },
];

impl Effect for Soaked {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(PALETTE[0]);
        Box::new(State {
            c: [c(0), c(1), c(2), c(3), c(4), c(5)],
            pool: opts.num(&POOL_OPT),
            drips: (js_round(opts.num(&DRIPS)) as usize).clamp(2, 8),
            runnels: (js_round(opts.num(&RUNNELS)) as usize).clamp(0, 3),
            splash: opts.num(&SPLASH) >= 0.5,
            seed: to_int32(seed as f64) as f64,
            src: STAND_SRC,
            run: STAND_RUN,
            beat_xy: (-4.0, -38.0),
        })
    }
}

struct Grid {
    x0: i32,
    y0: i32,
    w: usize,
    h: usize,
    g: Vec<i8>,
}

impl Grid {
    fn new(x0: i32, y0: i32, w: usize, h: usize) -> Self {
        Self {
            x0,
            y0,
            w,
            h,
            g: vec![-1; w * h],
        }
    }

    fn set(&mut self, x: f64, y: f64, c: i8) {
        if c < 0 {
            return;
        }
        let xi = libm::floor(x) as i32 - self.x0;
        let yi = libm::floor(y) as i32 - self.y0;
        if xi < 0 || yi < 0 || (xi as usize) >= self.w || (yi as usize) >= self.h {
            return;
        }
        if let Some(cell) = self.g.get_mut(yi as usize * self.w + xi as usize) {
            *cell = c;
        }
    }

    fn clean(&mut self) {
        let (w, h) = (self.w, self.h);
        let s = self.g.clone();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                let c = s[i];
                if c < 0 {
                    continue;
                }
                let n = [s[i - 1], s[i + 1], s[i - w], s[i + w]];
                if n.iter().any(|&v| v < 0 || v == c) {
                    continue;
                }
                let d = [s[i - w - 1], s[i - w + 1], s[i + w - 1], s[i + w + 1]];
                if d.iter().any(|&v| v < 0) {
                    continue;
                }
                let mut best = n[0];
                let mut bn = 0;
                for &v in &n {
                    let count = n.iter().filter(|&&u| u == v).count();
                    if count > bn || (count == bn && v < best) {
                        best = v;
                        bn = count;
                    }
                }
                self.g[i] = best;
            }
        }
    }

    fn blit(&self, p: &mut Painter, pal: &[Rgba; 6]) {
        for j in 0..self.h {
            for i in 0..self.w {
                let c = self.g[j * self.w + i];
                if let Some(&col) = (c >= 0).then(|| pal.get(c as usize)).flatten() {
                    p.px(
                        f64::from(self.x0 + i as i32) + 0.5,
                        f64::from(self.y0 + j as i32) + 0.5,
                        col,
                    );
                }
            }
        }
    }
}

fn draw_drop(g: &mut Grid, x: f64, y: f64, vx: f64, vy: f64, size: i32, tones: [i8; 3]) {
    let (fx, fy) = (libm::floor(x), libm::floor(y));
    let l = or_one(libm::hypot(vx, vy));
    let (bx, by) = (-vx / l, -vy / l);
    let sx = if libm::fabs(bx) > 0.38 {
        js_sign(bx)
    } else {
        0.0
    };
    let sy = if libm::fabs(by) > 0.38 {
        js_sign(by)
    } else {
        0.0
    };
    if size >= 3 {
        g.set(fx, fy, tones[1]);
        g.set(fx + 1.0, fy, tones[1]);
        g.set(fx, fy + 1.0, tones[0]);
        g.set(fx + 1.0, fy + 1.0, tones[0]);
        let ox = if sx > 0.0 { fx + 1.0 } else { fx };
        let oy = if sy > 0.0 { fy + 1.0 } else { fy };
        if sx != 0.0 || sy != 0.0 {
            g.set(ox + sx, oy + sy, tones[1]);
            g.set(ox + 2.0 * sx, oy + 2.0 * sy, tones[0]);
        }
        g.set(if sx > 0.0 { fx } else { fx + 1.0 }, fy, tones[2]);
    } else if size == 2 {
        g.set(fx, fy, tones[2]);
        if sx != 0.0 || sy != 0.0 {
            g.set(fx + sx, fy + sy, tones[0]);
        } else {
            g.set(fx, fy + 1.0, tones[0]);
        }
    } else {
        g.set(fx, fy, tones[1]);
    }
}

fn drip(g: &mut Grid, s: &DripSrc, k: i32) -> f64 {
    let u = fract(f64::from(k) / f64::from(N) + s.ph);
    let f = u * f64::from(N);
    let hang_f = HANG * f64::from(N);
    let floor = POOL_CY + s.dy;
    let h = floor - s.y;
    let fall_f = libm::sqrt((2.0 * h) / GRAV);
    let tones = if s.near { [1, 3, 4] } else { [0, 2, 3] };
    if f < hang_f {
        let sw = f / hang_f;
        g.set(s.x, s.y, tones[1]);
        g.set(s.x + 1.0, s.y, tones[0]);
        if sw >= 0.3 {
            g.set(s.x, s.y + 1.0, tones[2]);
            g.set(s.x + 1.0, s.y + 1.0, tones[1]);
            g.set(s.x, s.y + 2.0, tones[0]);
            g.set(s.x + 1.0, s.y + 2.0, tones[0]);
        }
        if sw >= 0.7 {
            g.set(s.x, s.y + 1.0, tones[1]);
            g.set(s.x, s.y + 2.0, tones[2]);
            g.set(s.x + 1.0, s.y + 2.0, tones[1]);
            g.set(s.x, s.y + 3.0, tones[0]);
            g.set(s.x + 1.0, s.y + 3.0, tones[0]);
        }
        return -1.0;
    }
    let ff = f - hang_f;
    if ff < fall_f {
        let y = s.y + 1.0 + 0.5 * GRAV * ff * ff;
        draw_drop(
            g,
            s.x,
            y,
            0.0,
            1.0 + ff * GRAV,
            if s.near { 3 } else { 2 },
            tones,
        );
        if ff > fall_f * 0.5 && s.near {
            g.set(s.x, y - 4.0, tones[0]);
        }
        return -1.0;
    }
    let age = ff - fall_f;
    if age < 1.0 {
        g.set(s.x - 1.0, floor, tones[1]);
        g.set(s.x + 1.0, floor, tones[1]);
        g.set(s.x, floor - 1.0, tones[2]);
        g.set(s.x - 2.0, floor - 2.0, tones[2]);
        g.set(s.x + 2.0, floor - 1.0, tones[1]);
    }
    if age < 4.0 { age } else { -1.0 }
}

fn ripple(g: &mut Grid, x: f64, y: f64, age: f64, near: bool) {
    let r = 1.5 + age * 1.5;
    let lim = libm::ceil(r) as i32;
    for s in -lim..=lim {
        let sf = f64::from(s);
        if libm::fabs(sf) < r * 0.45 {
            continue;
        }
        let yy = y + if libm::fabs(sf) < r * 0.8 {
            if near { 1.0 } else { -1.0 }
        } else {
            0.0
        };
        let c = if age < 2.0 {
            if near { 3 } else { 2 }
        } else {
            1
        };
        g.set(x + sf, yy, c);
    }
}

struct State {
    c: [Rgba; 6],
    pool: f64,
    drips: usize,
    runnels: usize,
    splash: bool,
    seed: f64,
    src: [DripSrc; 8],
    run: [RunnelSpec; 3],
    beat_xy: (f64, f64),
}

impl State {
    fn puddle(&self, g: &mut Grid, k: i32) {
        let rx = self.pool;
        let ry = POOL_RY * (self.pool / POOL_RX);
        let mut rg = Rng::new(hash(&[self.seed, 7.0, 0.0]));
        #[expect(clippy::approx_constant, reason = "source uses literal 6.28")]
        let ph = rg.range(0.0, 6.28);
        let ring_r = fract(f64::from(k) / f64::from(N)) * 1.25;
        let y0 = libm::floor(POOL_CY - ry - 2.0) as i32;
        let y1 = libm::ceil(POOL_CY + ry + 2.0) as i32;
        let x0 = libm::floor(POOL_CX - rx - 2.0) as i32;
        let x1 = libm::ceil(POOL_CX + rx + 2.0) as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                let (dx, dy) = ((px - POOL_CX) / rx, (py - POOL_CY) / ry);
                let a = libm::atan2(dy, dx);
                let edge = 1.0 + 0.09 * libm::sin(3.0 * a + ph) + 0.06 * libm::sin(5.0 * a - ph);
                let e = libm::hypot(dx, dy);
                if e > edge {
                    continue;
                }
                let near = dy > 0.0;
                let mut c = if near && e > edge - 0.14 {
                    0
                } else if near && e > edge - 0.34 {
                    2
                } else if !near && e > edge - 0.2 {
                    3
                } else {
                    let mut tone = 1;
                    if dy < -0.05
                        && dy > -0.62
                        && libm::fabs(dx + 0.15) < 0.5 - libm::fabs(dy + 0.33) * 0.6
                    {
                        tone = 2;
                    }
                    if dy > 0.25 && libm::fabs(dx) < 0.55 {
                        tone = 0;
                    }
                    let row_odd = (libm::floor(py) as i32) & 1;
                    let gx =
                        fract(f64::from(k) / f64::from(N) + 0.3 * f64::from(row_odd)) * 1.4 - 0.7;
                    if tone == 2 && libm::fabs(dx - gx) < 0.1 {
                        tone = 3;
                    }
                    if libm::fabs(e - ring_r) < 0.09 && ring_r > 0.25 && ring_r < 0.92 {
                        tone = if near { 3 } else { 2 };
                    }
                    tone
                };
                let _ = &mut c;
                g.set(f64::from(x), f64::from(y), c);
            }
        }
    }

    fn draw_runnels(&self, g: &mut Grid, k: i32) {
        for r in self.run.iter().take(self.runnels) {
            let u = fract((f64::from(k) / f64::from(N)) * 2.0 + r.ph);
            if u > 0.75 {
                continue;
            }
            let v = u / 0.75;
            let y = r.y0 + (r.y1 - r.y0) * v;
            let len = if v < 0.6 { 3 } else { 2 };
            for j in 0..len {
                g.set(r.x, y - f64::from(j), if j == 0 { 3 } else { 2 });
            }
            if v > 0.6 {
                g.set(r.x, y + 1.0, 4);
            }
        }
    }

    fn draw_beat(&self, g: &mut Grid, k: i32) -> Option<(f64, f64, f64)> {
        let (bx, by) = self.beat_xy;
        let b_land = 9;
        let b_floor = POOL_CY + 2.0;
        let fall_f = libm::sqrt((2.0 * (b_floor - by)) / GRAV);
        let start = f64::from(b_land) - fall_f;
        let kf = f64::from(k);
        let f = kf - start;
        let mut white = false;
        if kf >= start - 2.0 && kf < start {
            g.set(bx, by, 3);
            g.set(bx + 1.0, by, 2);
            if kf >= start - 1.0 {
                g.set(bx, by + 1.0, 3);
                g.set(bx + 1.0, by + 1.0, 1);
                g.set(bx, by + 2.0, 4);
            }
        } else if f >= 0.0 && k < b_land {
            let y = by + 1.0 + 0.5 * GRAV * f * f;
            draw_drop(g, bx, y, 0.0, 2.0 + f, 3, [1, 3, 4]);
            g.set(bx + 1.0, y - 1.0, 3);
            g.set(bx, y + 2.0, 2);
        }
        let age = k - b_land;
        if (0..=4).contains(&age) {
            white = age == 0;
            const CROWN: [[(i32, i32); 4]; 5] = [
                [(-4, -3), (3, -4), (-2, -6), (1, -6)],
                [(-7, -6), (6, -7), (-3, -10), (3, -11)],
                [(-9, -5), (8, -6), (-4, -11), (4, -12)],
                [(-10, -1), (9, -2), (-5, -9), (5, -10)],
                [(-11, 2), (10, 2), (-6, -5), (6, -6)],
            ];
            for (i, &(dx, dy)) in CROWN[age as usize].iter().enumerate() {
                let x = bx + js_round(f64::from(dx) * 1.3);
                let y = b_floor + f64::from(dy - 4);
                if age <= 1 {
                    g.set(x, y, 4);
                    g.set(x + 1.0, y, 3);
                    g.set(x, y + 1.0, 3);
                    g.set(x + 1.0, y + 1.0, 1);
                } else if age <= 3 {
                    g.set(x, y, if age == 2 { 3 } else { 2 });
                    g.set(x, y + 1.0, 1);
                } else if i % 2 == 0 {
                    g.set(x, y, 1);
                }
            }
            if age == 0 {
                for j in 1..=5 {
                    let jf = f64::from(j);
                    g.set(bx, b_floor - jf, if j < 4 { 5 } else { 4 });
                    g.set(bx + 1.0, b_floor - jf, if j < 3 { 4 } else { 3 });
                }
                for s in -3_i32..=3_i32 {
                    let sf = f64::from(s);
                    let dy = if s.abs() < 2 { 1.0 } else { 0.0 };
                    g.set(bx + sf, b_floor - dy, if s.abs() < 2 { 5 } else { 4 });
                }
            }
            ripple(g, bx, b_floor, f64::from(age) * 1.4, true);
        }
        if white {
            Some((bx, b_floor - 2.0, 0.45))
        } else if (0..=2).contains(&age) {
            Some((bx, b_floor - 2.0, 0.2))
        } else {
            None
        }
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        let Some(a) = actor else {
            self.src = STAND_SRC;
            self.run = STAND_RUN;
            self.beat_xy = (-4.0, -38.0);
            return;
        };
        let sil = Silhouette::of(&a, BOX);
        let Some(b) = sil.bounds() else {
            self.src = STAND_SRC;
            self.run = STAND_RUN;
            self.beat_xy = (-4.0, -38.0);
            return;
        };
        let edge_at = |want_y: f64, left: bool| -> (f64, f64) {
            let yi = (b.y + js_round((want_y / -40.0) * f64::from(b.h)) as i32)
                .clamp(b.y, b.y + b.h as i32 - 1);
            let mut best_x = if left { -9 } else { 9 };
            for dy in -2..=2 {
                let y = (yi + dy).clamp(b.y, b.y + b.h as i32 - 1);
                let xs: Vec<i32> = (b.x..b.x + b.w as i32)
                    .filter(|&x| sil.covers(x, y))
                    .collect();
                if let Some(&x) = if left { xs.first() } else { xs.last() } {
                    best_x = x;
                    return (f64::from(best_x), f64::from(y));
                }
            }
            (f64::from(best_x), f64::from(yi))
        };
        let mut src = STAND_SRC;
        for s in &mut src {
            let (ex, ey) = edge_at(s.y, s.x < 0.0);
            s.x = ex;
            s.y = ey;
        }
        let mut run = STAND_RUN;
        for r in &mut run {
            let (ex0, ey0) = edge_at(r.y0, r.x < 0.0);
            let (_, ey1) = edge_at(r.y1, r.x < 0.0);
            r.x = ex0 + if r.x < 0.0 { -1.0 } else { 1.0 };
            r.y0 = ey0;
            r.y1 = ey1.max(ey0 + 4.0);
        }
        self.src = src;
        self.run = run;
        self.beat_xy = (-4.0, f64::from(b.y + 2).min(-12.0));
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let k = (js_round(t * f64::from(FPS)) as i32).rem_euclid(N);
        let mut g = Grid::new(-24, -50, 48, 56);
        self.puddle(&mut g, k);
        let active_src = &self.src[..self.drips];
        let mut rip_far = Vec::new();
        for s in active_src.iter().filter(|s| !s.near) {
            let a = drip(&mut g, s, k);
            if a >= 0.0 {
                rip_far.push((s.x, POOL_CY + s.dy, a));
            }
        }
        for (rx, ry, a) in rip_far {
            ripple(&mut g, rx, ry, a, false);
        }
        self.draw_runnels(&mut g, k);
        let mut rip_near = Vec::new();
        for s in active_src.iter().filter(|s| s.near) {
            let a = drip(&mut g, s, k);
            if a >= 0.0 {
                rip_near.push((s.x, POOL_CY + s.dy, a));
            }
        }
        for (rx, ry, a) in rip_near {
            ripple(&mut g, rx, ry, a, true);
        }
        let splash = if self.splash {
            self.draw_beat(&mut g, k)
        } else {
            None
        };
        g.clean();
        p.glow(0.0, 1.0, self.pool + 1.0, self.c[2], 0.12);
        g.blit(p, &self.c);
        if let Some((sx, sy, sk)) = splash {
            p.glow(sx, sy, 8.0, self.c[4], sk);
        }
    }
}
