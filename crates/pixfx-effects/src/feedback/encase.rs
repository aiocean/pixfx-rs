//! `feedback/encase`: collars of stone (or ice with `palette: "ice"`) close round the wrapped sprite from the feet up
//! in seamed plates, set with a white flash of cracks, snapping chips and a rolling dust skirt at the feet, hold, then
//! crumble from the top down into falling chunks and grit.
//!
//! Adapted from Pixel VFX Studio's `status/petrified` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026 Hugo
//! Duprez). Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it sizes
//! each collar's width and height to the sprite's [`Silhouette`] at that elevation; with no sprite it uses the
//! stand-in collar widths.

use std::collections::BTreeSet;
use std::sync::Arc;

use pixfx_core::cue::{Cue, FEEL, TimedCue};
use pixfx_core::{Actor, Area, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{Rng, clamp, hash, js_max, js_min, js_round, to_int32};
use pixfx_draw::shapes::{Lobe, LobesOpt, lobes};

pub struct Encase;

pub static ENCASE: Encase = Encase;

const FPS: u32 = 16;
const PEAK: i32 = 6;
const N: i32 = 17;
const SAG: f64 = 1.6;
const BOX: Area = Area {
    x: -24,
    y: -44,
    w: 48,
    h: 46,
};

const CRACKS_OPT: NumOpt = NumOpt::new("cracks", 2.0).within(1.0, 3.0);
const DUST: NumOpt = NumOpt::new("dust", 1.0).within(0.7, 1.25);

const META: Meta = Meta {
    id: "feedback/encase",
    name: "Encase",
    desc: "Stone or ice collars close round the wrapped sprite from the feet up in seamed plates, set with a white flash of cracks, snapping chips and a dust skirt at the feet, hold, then crumble from the top down into falling chunks and grit.",
    tags: &[
        "feedback",
        "stone",
        "petrify",
        "freeze",
        "status",
        "character",
        "one-shot",
    ],
    size: (72, 54),
    origin: (36, 48),
    duration: 1.0,
    looping: false,
    fps: FPS,
    seed: 4_260_010_098,
    state: None,
    options: &[CRACKS_OPT.spec(), DUST.spec(), FEEL.spec()],
};

const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x211d27),
    Rgba::hex24(0x47424e),
    Rgba::hex24(0x6f6a70),
    Rgba::hex24(0x9d968d),
    Rgba::hex24(0xd2cab8),
    Rgba::hex24(0xffffff),
];

#[derive(Clone, Copy)]
struct Collar {
    yb: i32,
    w: f64,
    h: i32,
}

/// Stand-in collars from ankles to neck (`Ruler::size(RULER.width, 0.5)` half-width).
const STAND_COLLARS: [Collar; 6] = [
    Collar {
        yb: 0,
        w: 8.0,
        h: 4,
    },
    Collar {
        yb: -6,
        w: 6.5,
        h: 3,
    },
    Collar {
        yb: -11,
        w: 8.5,
        h: 5,
    },
    Collar {
        yb: -18,
        w: 7.5,
        h: 4,
    },
    Collar {
        yb: -23,
        w: 9.5,
        h: 5,
    },
    Collar {
        yb: -31,
        w: 5.5,
        h: 3,
    },
];

impl Effect for Encase {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let k = opts.num(&FEEL);
        if k.is_nan() || k <= 0.0 {
            return Vec::new();
        }
        let t = f64::from(PEAK) / f64::from(FPS);
        vec![
            TimedCue {
                t,
                cue: Cue::Shake {
                    trauma: clamp(0.28 * k, 0.0, 1.0),
                },
                pan: None,
            },
            TimedCue {
                t,
                cue: Cue::Flash {
                    k: clamp(0.18 * k, 0.0, 1.0),
                    dur: 0.08,
                },
                pan: None,
            },
        ]
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(PALETTE[0]);
        Box::new(State {
            r: [c(0), c(1), c(2), c(3), c(4), c(5)],
            cracks: (js_round(opts.num(&CRACKS_OPT)) as i32).clamp(1, 3),
            dust: opts.num(&DUST),
            seed: to_int32(seed as f64) as f64,
            collars: STAND_COLLARS,
        })
    }
}

struct State {
    r: [Rgba; 6],
    cracks: i32,
    dust: f64,
    seed: f64,
    collars: [Collar; 6],
}

#[derive(Clone, Copy)]
struct CasingCell {
    x: i32,
    y: i32,
    c: usize,
    j: usize,
    is_lip: bool,
}

fn seams(seed: f64, j: usize, w: f64) -> Vec<i32> {
    let mut rg = Rng::new(hash(&[seed, f64::from(0x5ea), f64::from(j as i32)]));
    let mut out = Vec::new();
    let mut x = -w + 2.0 + rg.int(1.0, 3.0) + f64::from((j % 2) as i32 * 2);
    while x < w - 2.0 {
        out.push(js_round(x) as i32);
        x += rg.int(4.0, 6.0);
    }
    out
}

fn crack_path(seed: f64, i: i32, n: i32) -> Vec<(i32, i32, i32)> {
    let mut rg = Rng::new(hash(&[seed, f64::from(0xc4a), f64::from(i)]));
    let span = 10.0 / f64::from(n.max(1));
    let mut x = js_round(-5.0 + span * (f64::from(i) + 0.5) + rg.range(-1.0, 1.0)) as i32;
    let mut rows = Vec::with_capacity(35);
    for y in -33..=1 {
        let r = rg.next();
        let x0 = x;
        if r < 0.3 {
            x += if rg.next() < 0.5 { -1 } else { 1 };
        }
        rows.push((y, x0, x));
    }
    rows
}

struct Dust {
    far: Vec<Lobe>,
    near: Vec<Lobe>,
    hol: f64,
    r_far: [Rgba; 3],
    r_near: [Rgba; 3],
}

impl State {
    fn casing(&self, k: i32) -> Vec<CasingCell> {
        let mut cells: Vec<CasingCell> = Vec::new();
        let upto = k.clamp(0, PEAK - 1) as usize;
        for (j, cl) in self.collars.iter().enumerate() {
            if j > upto {
                continue;
            }
            let forming = k < PEAK && j == k as usize;
            let sm = seams(self.seed, j, cl.w);
            let mut tilt_rg = Rng::new(hash(&[self.seed, f64::from(0x7117), f64::from(j as i32)]));
            let tilt = tilt_rg.range(-0.14, 0.14);
            let w_lim = libm::floor(cl.w) as i32;
            for x in -w_lim..=w_lim {
                let xf = f64::from(x);
                let xn = xf / cl.w;
                let ell = libm::sqrt(js_max(0.0, 1.0 - xn * xn));
                let bottom = js_round(f64::from(cl.yb) - SAG * (1.0 - ell) + tilt * xf) as i32;
                let mut top = bottom - cl.h + 1;
                if hash(&[
                    self.seed,
                    f64::from(0x11b),
                    f64::from(j as i32),
                    f64::from(x >> 1),
                ])
                .is_multiple_of(4)
                {
                    top += 1;
                }
                if forming {
                    let h_off = (hash(&[
                        self.seed,
                        f64::from(0xf20),
                        f64::from(j as i32),
                        f64::from(x),
                    ]) % 2) as i32;
                    top = top.max(bottom - 1 - h_off);
                }
                let shadow = xn > 0.6;
                let light_side = xn < -0.35;
                let end = x.abs() == w_lim;
                if end && !forming {
                    top += 1;
                }
                for y in top..=bottom {
                    let r = y - top;
                    let c = if forming {
                        if r == 0 { 4 } else { 3 }
                    } else if r == 0 {
                        if shadow { 3 } else { 4 }
                    } else if y == bottom {
                        if shadow { 0 } else { 1 }
                    } else if sm.contains(&x) && !end {
                        0
                    } else if sm.contains(&(x - 1)) && !end && !shadow {
                        4
                    } else if shadow {
                        1
                    } else if end {
                        if x < 0 { 2 } else { 1 }
                    } else if light_side || r == 1 {
                        3
                    } else {
                        2
                    };
                    let entry = CasingCell {
                        x,
                        y,
                        c,
                        j,
                        is_lip: r == 0,
                    };
                    if let Some(existing) = cells.iter_mut().find(|e| e.x == x && e.y == y) {
                        *existing = entry;
                    } else {
                        cells.push(entry);
                    }
                }
            }
        }
        cells
    }

    fn dust_lobes(&self, k: i32) -> Option<Dust> {
        if !(PEAK..=PEAK + 8).contains(&k) {
            return None;
        }
        let r_pal = &self.r;
        let age = (k - PEAK) as usize;
        let f_age = f64::from(age as i32);
        let s = self.dust;
        let mut far = Vec::new();
        let mut near = Vec::new();
        for side in [-1, 1] {
            for j in 0..4 {
                let mut rg = Rng::new(hash(&[self.seed, 71.0, f64::from(side), f64::from(j)]));
                let is_near = (j + if side > 0 { 1 } else { 0 }) % 2 == 0;
                let spread = if age == 0 {
                    2.2
                } else {
                    3.6 + libm::sqrt(f_age) * 1.5
                };
                let reach = (4.0 + f64::from(j) * spread + rg.range(-0.8, 0.8)) * s;
                let r = (5.4 - f64::from(j) * 0.8 + rg.range(-0.6, 0.6))
                    * s
                    * (if age == 0 { 0.75 } else { 1.0 })
                    * (if is_near { 1.0 } else { 0.85 });
                let rise = f_age * rg.range(0.45, 0.95) * (1.0 - f64::from(j) * 0.15);
                let lobe = Lobe {
                    x: f64::from(side) * reach,
                    y: (if is_near { 0.5 } else { -1.5 }) - r * 0.75 - rise,
                    r,
                };
                if is_near {
                    near.push(lobe);
                } else {
                    far.push(lobe);
                }
            }
        }
        let hol = [0.0, 0.0, 0.0, 0.74, 0.85, 0.92, 0.96, 1.0, 1.0][age];
        let r_far = if age < 2 {
            [r_pal[1], r_pal[2], r_pal[3]]
        } else if age < 5 {
            [r_pal[1], r_pal[2], r_pal[2]]
        } else {
            [r_pal[1], r_pal[1], r_pal[2]]
        };
        let r_near = if age < 2 {
            [r_pal[2], r_pal[3], r_pal[4]]
        } else if age < 5 {
            [r_pal[2], r_pal[3], r_pal[3]]
        } else {
            [r_pal[1], r_pal[2], r_pal[3]]
        };
        Some(Dust {
            far,
            near,
            hol,
            r_far,
            r_near,
        })
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        let Some(a) = actor else {
            self.collars = STAND_COLLARS;
            return;
        };
        let sil = Silhouette::of(&a, BOX);
        let Some(b) = sil.bounds() else {
            self.collars = STAND_COLLARS;
            return;
        };
        let mut collars = STAND_COLLARS;
        for c in &mut collars {
            let mut max_hw = 0;
            for dy in -c.h..=0 {
                let y = (c.yb + dy).clamp(b.y, b.y + b.h as i32 - 1);
                for x in b.x..b.x + b.w as i32 {
                    if sil.covers(x, y) {
                        max_hw = max_hw.max(x.abs());
                    }
                }
            }
            if max_hw >= 3 {
                c.w = f64::from(max_hw + 1).clamp(4.5, 14.0);
            }
        }
        self.collars = collars;
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let k = js_round(t * f64::from(FPS)) as i32;
        if !(0..N - 1).contains(&k) {
            return;
        }
        let r = &self.r;
        let dust = self.dust_lobes(k);
        if let Some(d) = &dust {
            lobes(
                p,
                &d.far,
                &d.r_far,
                LobesOpt {
                    lx: -0.3,
                    ly: -1.0,
                    hollow: d.hol,
                    shadow: 0.35,
                    seed: 3.0,
                },
            );
        }

        let cells = self.casing(k);
        let mut crack_at: BTreeSet<(i32, i32)> = BTreeSet::new();
        if k >= PEAK {
            for i in 0..self.cracks {
                for (y, x0, x1) in crack_path(self.seed, i, self.cracks) {
                    crack_at.insert((x0, y));
                    if x1 != x0 {
                        crack_at.insert((x1, y));
                    }
                }
            }
        }
        let crack_c = if k == PEAK {
            5
        } else if k == PEAK + 1 {
            4
        } else {
            0
        };
        for cell in &cells {
            let (x, y, c0, j, is_lip) = (cell.x, cell.y, cell.c, cell.j, cell.is_lip);
            let mut c = c0;
            if k == PEAK && is_lip && c0 >= 2 {
                c = 4;
            }
            if crack_at.contains(&(x, y)) {
                c = crack_c;
            }
            let kf = 11 + ((self.collars.len() - 1 - j) as i32) / 2;
            let a = k - kf;
            if a >= 0 {
                if crack_at.contains(&(x, y)) || c0 == 0 {
                    continue;
                }
                let cl = self.collars[j];
                let h_off = (hash(&[self.seed, f64::from(0x7c), f64::from(j as i32)]) % 3) as f64;
                let chunk = libm::floor((f64::from(x) + cl.w + h_off) / 5.0);
                let mut rg = Rng::new(hash(&[
                    self.seed,
                    f64::from(0xc0b),
                    f64::from(j as i32),
                    chunk,
                ]));
                let side = if x < 0 { -1.0 } else { 1.0 };
                let f_a = f64::from(a);
                let dy = if j == 0 {
                    0
                } else {
                    (-cl.yb).min(
                        js_round((1.0 + 2.2 * f_a * f_a + 2.0 * f_a) * rg.range(0.7, 1.2)) as i32,
                    )
                };
                let dx = if j == 0 {
                    js_round(side * f_a * 0.6) as i32
                } else {
                    js_round(side * (f_a + 0.5) * rg.range(0.4, 1.2)) as i32
                };
                let under = c0 == 1
                    && hash(&[
                        self.seed,
                        f64::from(0x401),
                        f64::from(x),
                        f64::from(j as i32),
                    ])
                    .is_multiple_of(2);
                if a == 1 && !is_lip && !under {
                    continue;
                }
                if a == 2
                    && (!hash(&[self.seed, f64::from(0x402), f64::from(x >> 1), f64::from(y)])
                        .is_multiple_of(3)
                        || !is_lip)
                {
                    continue;
                }
                if a == 3
                    && (!hash(&[self.seed, f64::from(0x403), f64::from(x >> 1)]).is_multiple_of(4)
                        || !is_lip)
                {
                    continue;
                }
                if a >= 4 {
                    continue;
                }
                let cc = if a == 0 {
                    c
                } else if a == 1 {
                    c.min(2)
                } else {
                    1
                };
                p.px(f64::from(x + dx), f64::from((y + dy).min(0)), r[cc]);
                continue;
            }
            p.px(f64::from(x), f64::from(y), r[c]);
        }

        if k == PEAK {
            for &(cx, cy) in &crack_at {
                if cells.iter().any(|c| c.x == cx && c.y == cy) {
                    p.light(f64::from(cx), f64::from(cy), r[4], 0.6);
                }
            }
            p.glow(0.0, -16.0, 18.0, r[3], 0.25);
        } else if k < PEAK {
            p.glow(
                0.0,
                f64::from(self.collars[k as usize].yb - 2),
                9.0,
                r[3],
                0.18,
            );
        }

        // Grit flicked off the forming collar
        if (1..PEAK).contains(&k) {
            let cl = self.collars[k as usize];
            for i in 0..3 {
                let mut rg = Rng::new(hash(&[
                    self.seed,
                    f64::from(0x9e1),
                    f64::from(k),
                    f64::from(i),
                ]));
                let x = rg.range(-cl.w, cl.w);
                let y = f64::from(cl.yb - 3) - rg.range(0.0, 3.0);
                p.px(x, y, if i != 0 { r[3] } else { r[2] });
            }
        }

        // Chips snapping off at the set
        if k >= PEAK {
            let age = k - PEAK;
            let f_age = f64::from(age);
            for i in 0..3 {
                let mut rg = Rng::new(hash(&[self.seed, 61.0, f64::from(i)]));
                let side = if i % 2 != 0 { 1.0 } else { -1.0 };
                let x0 = side * rg.range(6.0, 9.0);
                let y0 = -rg.range(14.0, 28.0);
                let vx = side * rg.range(4.0, 7.0);
                let vy = -rg.range(3.0, 6.0);
                let land = 2 + rg.int(0.0, 2.0) as i32;
                let f_land = f64::from(land);
                let mut x = x0 + vx * js_min(f_age, f_land) * 1.1;
                let mut y = y0 + vy * f_age + 4.5 * f_age * f_age;
                if y > -1.0 || age > land + 1 {
                    y = -1.0;
                }
                if age > land + 1 {
                    x = x0 + vx * f_land * 1.1 + side;
                }
                let life = 8 + rg.int(0.0, 2.0) as i32;
                if age >= life {
                    continue;
                }
                let c = if age < 3 {
                    r[3]
                } else if age < 6 {
                    r[2]
                } else {
                    r[1]
                };
                let big = i != 2 && age < life - 2;
                p.px(x, y, c);
                if big {
                    p.px(x + 1.0, y, r[1]);
                    p.px(x, y - 1.0, if age < 3 { r[4] } else { r[2] });
                    p.px(x + 1.0, y - 1.0, c);
                }
            }
        }

        // Dust skirt near lobes
        if let Some(d) = &dust {
            lobes(
                p,
                &d.near,
                &d.r_near,
                LobesOpt {
                    lx: -0.3,
                    ly: -1.0,
                    hollow: d.hol,
                    shadow: 0.35,
                    seed: 9.0,
                },
            );
            if k == PEAK {
                p.glow(0.0, -3.0, 14.0 * self.dust, r[3], 0.2);
            }
        }
        p.clean_flecks(1, false);

        // Settling grit
        if (PEAK + 5..=N - 2).contains(&k) {
            for i in 0..9 {
                let mut rg = Rng::new(hash(&[self.seed, 83.0, f64::from(i)]));
                let born = PEAK + 5 + rg.int(0.0, 3.0) as i32;
                let life = 2 + rg.int(0.0, 3.0) as i32;
                let a = k - born;
                if a < 0 || a >= life {
                    continue;
                }
                let side = if i % 2 != 0 { 1.0 } else { -1.0 };
                let f_a = f64::from(a);
                let gx = side * rg.range(4.0, 20.0) * self.dust + side * f_a;
                let gy = js_min(0.0, -rg.range(0.0, 6.0) + f_a);
                if p.get(gx, gy) == 0 {
                    p.px(gx, gy, if a < 1 { r[2] } else { r[1] });
                }
            }
        }
    }
}
