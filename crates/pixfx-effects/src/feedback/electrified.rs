//! `feedback/electrified`: jagged 2 px-cored arcs crawl and flicker around the wrapped sprite's outline, one forked
//! arc snaps across the body per loop leaving a dashed afterimage, and sparks spit off and skid along the floor.
//!
//! Adapted from Pixel VFX Studio's `status/electrified` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026
//! Hugo Duprez). Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it
//! builds the outline loop and cross-body snap anchors from [`Silhouette`] once per pose; with no sprite it uses the
//! stand-in outline.

use std::sync::Arc;

use pixfx_core::{Actor, Area, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{
    Rng, TAU, clamp, hash, js_min, js_round, js_sign, lerp, or_one, to_int32, wrap,
};
use pixfx_draw::shapes::{flash_star, tempo, wedge};

pub struct Electrified;

pub static ELECTRIFIED: Electrified = Electrified;

const FPS: u32 = 16;
const N: i32 = 14;
const MS: f64 = 0.065;
const BEAT: i32 = 8;
const K_PATH: f64 = 21.0;
const K_SPARK: f64 = 22.0;
const K_DASH: f64 = 23.0;
const K_FORK: f64 = 24.0;
const K_SPECK: f64 = 25.0;
const BOX: Area = Area {
    x: -22,
    y: -45,
    w: 44,
    h: 48,
};

const JAG: NumOpt = NumOpt::new("jag", 1.0).within(0.4, 1.8);
const LENGTH: NumOpt = NumOpt::new("length", 1.0).within(0.6, 1.5);
const SPARKS: NumOpt = NumOpt::new("sparks", 1.0).within(0.0, 2.0);
const BEAT_OPT: NumOpt = NumOpt::new("beat", 1.0).within(0.0, 1.0);

const META: Meta = Meta {
    id: "feedback/electrified",
    name: "Electrified",
    desc: "Jagged 2 px-cored lightning arcs crawl and flicker around the wrapped sprite's outline, one forked arc snaps across the body per loop leaving a short afterimage, and sparks spit off and skid on the floor.",
    tags: &[
        "feedback",
        "lightning",
        "electric",
        "status",
        "shock",
        "character",
        "loop",
    ],
    size: (48, 56),
    origin: (24, 47),
    duration: 0.875,
    looping: true,
    fps: FPS,
    seed: 3_158_554_053,
    state: None,
    options: &[JAG.spec(), LENGTH.spec(), SPARKS.spec(), BEAT_OPT.spec()],
};

const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x1c0c3d),
    Rgba::hex24(0x4b2391),
    Rgba::hex24(0x2f5cff),
    Rgba::hex24(0x3fd6ff),
    Rgba::hex24(0xc9f7ff),
    Rgba::hex24(0xffffff),
];

/// Stand-in outline points around a character footprint (`Ruler::size(RULER.width, 0.8)` x `Ruler::size(RULER.height, 0.55)`).
const STAND_OUTLINE: [(f64, f64); 21] = [
    (-7.0, 1.0),
    (-9.0, -6.0),
    (-10.0, -12.0),
    (-12.0, -16.0),
    (-13.0, -20.0),
    (-11.0, -23.5),
    (-7.0, -24.5),
    (-6.5, -28.0),
    (-5.0, -32.5),
    (-2.0, -34.5),
    (2.0, -34.5),
    (5.0, -32.5),
    (6.5, -28.0),
    (7.0, -24.5),
    (11.0, -23.5),
    (13.0, -20.0),
    (12.0, -16.0),
    (10.0, -12.0),
    (9.0, -6.0),
    (7.0, 1.0),
    (0.0, 1.5),
];

const STAND_SNAP_A: (f64, f64) = (-12.0, -24.0);
const STAND_SNAP_B: (f64, f64) = (10.0, -5.0);

struct CrawlerSpec {
    d0: f64,
    laps: f64,
    len: f64,
    s: f64,
    on: &'static [u8; 14],
}

const CRAWLERS: [CrawlerSpec; 3] = [
    CrawlerSpec {
        d0: 96.0,
        laps: 1.0,
        len: 26.0,
        s: 1.0,
        on: b"11001110001101",
    },
    CrawlerSpec {
        d0: 100.0,
        laps: -1.0,
        len: 21.0,
        s: 0.75,
        on: b"01110011001110",
    },
    CrawlerSpec {
        d0: 44.0,
        laps: 1.0,
        len: 19.0,
        s: 0.9,
        on: b"10111101000011",
    },
];

impl Effect for Electrified {
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
            r: [c(0), c(1), c(2), c(3), c(4), c(5)],
            jag: opts.num(&JAG),
            length: opts.num(&LENGTH),
            sparks: opts.num(&SPARKS),
            beat: opts.num(&BEAT_OPT) >= 0.5,
            seed: to_int32(seed as f64) as f64,
            outline: OutlineLoop::from_pts(STAND_OUTLINE.to_vec(), STAND_SNAP_A, STAND_SNAP_B),
        })
    }
}

struct OutlineLoop {
    pts: Vec<(f64, f64)>,
    cum: Vec<f64>,
    perim: f64,
    snap_a: (f64, f64),
    snap_b: (f64, f64),
}

impl OutlineLoop {
    fn from_pts(pts: Vec<(f64, f64)>, snap_a: (f64, f64), snap_b: (f64, f64)) -> Self {
        let mut cum = Vec::with_capacity(pts.len() + 1);
        cum.push(0.0);
        for i in 1..=pts.len() {
            let a = pts[i - 1];
            let b = pts[i % pts.len()];
            let prev = cum.last().copied().unwrap_or(0.0);
            cum.push(prev + libm::hypot(b.0 - a.0, b.1 - a.1));
        }
        let perim = cum.last().copied().unwrap_or(1.0).max(1.0);
        Self {
            pts,
            cum,
            perim,
            snap_a,
            snap_b,
        }
    }

    fn on_outline(&self, d: f64) -> (f64, f64, f64, f64) {
        let d = wrap(d, self.perim);
        let mut i = 1;
        while i + 1 < self.cum.len() && self.cum[i] < d {
            i += 1;
        }
        let a = self.pts[i - 1];
        let b = self.pts[i % self.pts.len()];
        let seg_len = or_one(self.cum[i] - self.cum[i - 1]);
        let u = (d - self.cum[i - 1]) / seg_len;
        let (x, y) = (lerp(a.0, b.0, u), lerp(a.1, b.1, u));
        let (nx, ny) = (x, (y + 17.0) * 0.7);
        let l = or_one(libm::hypot(nx, ny));
        (x, y, nx / l, ny / l)
    }

    fn crawl_path(&self, seed: u32, d0: f64, len: f64, amp: f64, bow: f64) -> Vec<(f64, f64)> {
        let mut r = Rng::new(seed);
        let start = self.on_outline(d0);
        let mut out = vec![(start.0, start.1)];
        let mut d = 0.0;
        let mut side = if r.next() < 0.5 { -1.0 } else { 1.0 };
        loop {
            d += r.range(3.5, 6.0);
            if d > len - 2.0 {
                break;
            }
            let u = d / or_one(len);
            let (x, y, nx, ny) = self.on_outline(d0 + d);
            let off = bow * libm::sin(core::f64::consts::PI * u) + side * amp * r.range(0.6, 1.2);
            let slip = side * r.range(0.3, 1.2);
            out.push((x + nx * off - ny * slip, y + ny * off + nx * slip));
            if r.next() < 0.85 {
                side = -side;
            }
        }
        let end = self.on_outline(d0 + len);
        out.push((end.0, end.1));
        out
    }
}

struct State {
    r: [Rgba; 6],
    jag: f64,
    length: f64,
    sparks: f64,
    beat: bool,
    seed: f64,
    outline: OutlineLoop,
}

fn snap_path(seed: u32, ax: f64, ay: f64, bx: f64, by: f64, amp: f64, bow: f64) -> Vec<(f64, f64)> {
    let mut r = Rng::new(seed);
    let (dx, dy) = (bx - ax, by - ay);
    let l = or_one(libm::hypot(dx, dy));
    let (nx, ny) = (-dy / l, dx / l);
    let mut out = vec![(ax, ay)];
    let mut u = 0.0;
    let mut side = 1.0;
    loop {
        u += r.range(3.0, 5.5) / l;
        if u > 1.0 - 3.0 / l {
            break;
        }
        let off = bow * libm::sin(core::f64::consts::PI * u) + side * amp * r.range(0.5, 1.1);
        out.push((lerp(ax, bx, u) + nx * off, lerp(ay, by, u) + ny * off));
        if r.next() < 0.85 {
            side = -side;
        }
    }
    out.push((bx, by));
    out
}

fn lengths(pts: &[(f64, f64)]) -> Vec<f64> {
    let mut d = Vec::with_capacity(pts.len());
    d.push(0.0);
    for pair in pts.windows(2) {
        let prev = d.last().copied().unwrap_or(0.0);
        d.push(prev + libm::hypot(pair[1].0 - pair[0].0, pair[1].1 - pair[0].1));
    }
    d
}

fn sub_path(pts: &[(f64, f64)], d_list: &[f64], d0: f64, d1: f64) -> Vec<(f64, f64)> {
    let at = |d: f64| -> (f64, f64) {
        for i in 1..pts.len() {
            if d <= d_list[i] || i == pts.len() - 1 {
                let span = or_one(d_list[i] - d_list[i - 1]);
                let u = clamp((d - d_list[i - 1]) / span, 0.0, 1.0);
                return (
                    lerp(pts[i - 1].0, pts[i].0, u),
                    lerp(pts[i - 1].1, pts[i].1, u),
                );
            }
        }
        pts.last().copied().unwrap_or((0.0, 0.0))
    };
    let mut out = vec![at(d0)];
    for i in 1..pts.len().saturating_sub(1) {
        if d_list[i] > d0 && d_list[i] < d1 {
            out.push(pts[i]);
        }
    }
    out.push(at(d1));
    out
}

fn strand(p: &mut Painter, pts: &[(f64, f64)], w: f64, w_end: f64, c: Rgba) {
    let d_list = lengths(pts);
    let tot = or_one(d_list.last().copied().unwrap_or(1.0));
    let prof = |d: f64| {
        let u = d / tot;
        let e = js_min(u, 1.0 - u) * 2.0;
        lerp(w_end, w, js_min(1.0, e * 1.6))
    };
    for i in 1..pts.len() {
        let (x0, y0) = pts[i - 1];
        let (x1, y1) = pts[i];
        let (w0, w1) = (prof(d_list[i - 1]), prof(d_list[i]));
        wedge(p, x0, y0, x1, y1, w0, w1, c, 1.0);
        if w1 >= 2.0 && i + 1 < pts.len() {
            p.circle(x1, y1, w1 * 0.5 - 0.25, c, 1.0);
        }
    }
}

struct RunInfo {
    age: i32,
    len: i32,
}

fn run_of(on: &[u8; 14], k: i32) -> Option<RunInfo> {
    let ku = k.rem_euclid(N) as usize;
    if on[ku] != b'1' {
        return None;
    }
    let mut a = 0;
    while a < N && on[(k - a - 1).rem_euclid(N) as usize] == b'1' {
        a += 1;
    }
    let mut b = 0;
    while b < N && on[(k + b + 1).rem_euclid(N) as usize] == b'1' {
        b += 1;
    }
    Some(RunInfo {
        age: a,
        len: a + b + 1,
    })
}

impl State {
    fn draw_sparks(&self, p: &mut Painter, k: i32) {
        let r_pal = &self.r;
        let perim = self.outline.perim;
        struct Birth {
            f: i32,
            x: f64,
            y: f64,
            nx: f64,
            n: i32,
            key: i32,
            s: f64,
        }
        let mut births = Vec::new();
        for (i, c) in CRAWLERS.iter().enumerate() {
            for f in 0..N {
                let fu = f as usize;
                let prev = (f - 1).rem_euclid(N) as usize;
                if c.on[fu] != b'1' || c.on[prev] == b'1' {
                    continue;
                }
                let head = c.d0 + (c.laps * perim * f64::from(f)) / f64::from(N);
                let end = self.outline.on_outline(head + c.len * self.length);
                births.push(Birth {
                    f,
                    x: end.0,
                    y: end.1,
                    nx: end.2,
                    n: 2,
                    key: (i as i32) * 100 + f,
                    s: c.s,
                });
            }
        }
        if self.beat {
            let (sa, sb) = (self.outline.snap_a, self.outline.snap_b);
            births.push(Birth {
                f: BEAT,
                x: sb.0,
                y: sb.1,
                nx: 1.0,
                n: 4,
                key: 999,
                s: 1.0,
            });
            births.push(Birth {
                f: BEAT,
                x: sa.0,
                y: sa.1,
                nx: -1.0,
                n: 3,
                key: 998,
                s: 1.0,
            });
        }
        let ramp = [r_pal[4], r_pal[3], r_pal[2], r_pal[1], r_pal[0]];
        for b in &births {
            let n = js_round(f64::from(b.n) * self.sparks) as i32;
            for j in 0..n {
                let mut rg = Rng::new(hash(&[self.seed, K_SPARK, f64::from(b.key), f64::from(j)]));
                let life = rg.int(3.0, 5.0) as i32;
                let age = (k - b.f).rem_euclid(N);
                if age >= life {
                    continue;
                }
                let sign_nx = js_sign(b.nx);
                let dir = if sign_nx == 0.0 || sign_nx.is_nan() {
                    if j % 2 != 0 { 1.0 } else { -1.0 }
                } else {
                    sign_nx
                };
                let vx = dir * rg.range(25.0, 70.0);
                let vy = -rg.range(30.0, 90.0);
                let tt = (f64::from(age) + 0.5) * MS;
                let x = b.x + vx * tt;
                let mut y = b.y + vy * tt + 420.0 * tt * tt;
                if y > 2.0 {
                    y = 2.0;
                }
                let col = tempo(&ramp, (f64::from(age) + 0.5) / f64::from(life));
                if age == 0 {
                    let (dx, dy) = (vx, vy + 840.0 * tt);
                    let l = or_one(libm::hypot(dx, dy));
                    p.line(x, y, x - (dx / l) * 2.5, y - (dy / l) * 2.5, r_pal[3], 1.0);
                    p.light(x, y, r_pal[3], 0.4 * b.s);
                } else if age == 1 {
                    p.rect(
                        libm::floor(x),
                        libm::floor(y),
                        2.0,
                        f64::from(1 + (j % 2)),
                        col,
                        1.0,
                    );
                } else {
                    p.px(x, y, col);
                }
            }
        }
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.outline = match actor {
            Some(a) => {
                let sil = Silhouette::of(&a, BOX);
                if let (Some(b), Some((cx, cy))) = (sil.bounds(), sil.centroid()) {
                    let mut ring: Vec<(f64, f64, f64)> = sil
                        .ring(1, false)
                        .map(|(x, y)| {
                            let (px, py) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                            let ang = libm::atan2(py - cy, px - cx);
                            (ang, px, py)
                        })
                        .collect();
                    ring.sort_by(|u, v| u.0.total_cmp(&v.0));
                    let pts: Vec<(f64, f64)> = ring
                        .iter()
                        .step_by((ring.len() / 20).max(1))
                        .map(|&(_, px, py)| (px, py))
                        .collect();
                    if pts.len() >= 6 {
                        let snap_a = (f64::from(b.x), f64::from(b.y) + f64::from(b.h) * 0.3);
                        let snap_b = (
                            f64::from(b.x + b.w as i32),
                            f64::from(b.y) + f64::from(b.h) * 0.82,
                        );
                        OutlineLoop::from_pts(pts, snap_a, snap_b)
                    } else {
                        OutlineLoop::from_pts(STAND_OUTLINE.to_vec(), STAND_SNAP_A, STAND_SNAP_B)
                    }
                } else {
                    OutlineLoop::from_pts(STAND_OUTLINE.to_vec(), STAND_SNAP_A, STAND_SNAP_B)
                }
            }
            None => OutlineLoop::from_pts(STAND_OUTLINE.to_vec(), STAND_SNAP_A, STAND_SNAP_B),
        };
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let k = (js_round(t * f64::from(FPS)) as i32).rem_euclid(N);
        let r = &self.r;
        let perim = self.outline.perim;
        let (sa, sb) = (self.outline.snap_a, self.outline.snap_b);
        let mut ends: Vec<(f64, f64)> = Vec::new();

        self.draw_sparks(p, k);

        let beat_path = snap_path(
            hash(&[self.seed, K_PATH, 99.0]),
            sa.0,
            sa.1,
            sb.0,
            sb.1,
            3.6,
            -4.0,
        );
        if self.beat && k == BEAT + 1 {
            let d_list = lengths(&beat_path);
            let tot = d_list.last().copied().unwrap_or(1.0);
            let mut rg = Rng::new(hash(&[self.seed, K_DASH, 99.0]));
            let mut d = rg.range(0.0, 2.0);
            while d < tot {
                let on = rg.range(5.0, 9.0);
                let pc = sub_path(&beat_path, &d_list, d, js_min(tot, d + on));
                strand(p, &pc, 4.4, 1.4, r[1]);
                strand(p, &pc, 2.4, 1.0, r[2]);
                if on > 6.5 {
                    strand(p, &pc, 1.2, 1.0, r[3]);
                }
                d += on + rg.range(2.0, 4.0);
            }
            p.glow_line(sa.0, sa.1, sb.0, sb.1, 5.0, r[2], 0.18);
        } else if self.beat && k == BEAT + 2 {
            let d_list = lengths(&beat_path);
            let tot = d_list.last().copied().unwrap_or(1.0);
            let mut rg = Rng::new(hash(&[self.seed, K_SPECK, 99.0]));
            let mut d = rg.range(1.0, 4.0);
            while d < tot {
                let pt = sub_path(&beat_path, &d_list, d, d + 1.0)[0];
                if rg.next() < 0.5 {
                    p.rect(
                        libm::floor(pt.0),
                        libm::floor(pt.1 - 1.0),
                        2.0,
                        1.0,
                        r[1],
                        1.0,
                    );
                } else {
                    p.rect(
                        libm::floor(pt.0),
                        libm::floor(pt.1 - 1.0),
                        1.0,
                        2.0,
                        r[0],
                        1.0,
                    );
                }
                d += rg.range(5.0, 8.0);
            }
        }

        // Static violet ticks on the outline every frame
        {
            let mut rg = Rng::new(hash(&[self.seed, K_SPECK, 7.0, f64::from(k)]));
            for j in 0..3 {
                let d = (f64::from(j) / 3.0 + rg.range(0.0, 0.3)) * perim;
                let tick = self.outline.crawl_path(
                    hash(&[self.seed, K_PATH, f64::from(50 + j), f64::from(k)]),
                    d,
                    rg.range(4.0, 6.0),
                    1.4,
                    0.6,
                );
                strand(p, &tick, 2.0, 1.0, if j == 1 { r[0] } else { r[1] });
            }
        }

        // Crawler arcs
        for (i, c) in CRAWLERS.iter().enumerate() {
            let Some(run) = run_of(c.on, k) else {
                continue;
            };
            let head = c.d0 + (c.laps * perim * f64::from(k)) / f64::from(N);
            let s = c.s;
            let path = self.outline.crawl_path(
                hash(&[self.seed, K_PATH, f64::from(i as i32), f64::from(k)]),
                head,
                c.len * self.length * lerp(0.85, 1.1, f64::from(run.age) / 2.0),
                2.8 * s * self.jag,
                1.5 + s,
            );
            let last = run.age == run.len - 1 && run.len > 1;
            let far = s < 0.8;
            if run.age == 0 {
                strand(
                    p,
                    &path,
                    if far { 4.0 } else { 4.8 },
                    1.6,
                    if far { r[0] } else { r[1] },
                );
                strand(p, &path, 2.2, 1.0, if far { r[2] } else { r[3] });
                if !far {
                    let d_list = lengths(&path);
                    let tot = d_list.last().copied().unwrap_or(1.0);
                    strand(
                        p,
                        &sub_path(&path, &d_list, tot * 0.3, tot * 0.7),
                        1.0,
                        1.0,
                        r[4],
                    );
                }
                if let (Some(&first), Some(&last_pt)) = (path.first(), path.last()) {
                    ends.push(first);
                    ends.push(last_pt);
                }
                for pair in path.windows(2) {
                    p.glow_line(
                        pair[0].0,
                        pair[0].1,
                        pair[1].0,
                        pair[1].1,
                        4.0 * s,
                        r[3],
                        if far { 0.12 } else { 0.2 },
                    );
                }
            } else if !last {
                strand(
                    p,
                    &path,
                    if far { 3.4 } else { 3.8 },
                    1.2,
                    if far { r[0] } else { r[1] },
                );
                strand(p, &path, 1.8, 1.0, r[2]);
                let d_list = lengths(&path);
                let tot = d_list.last().copied().unwrap_or(1.0);
                if !far {
                    strand(
                        p,
                        &sub_path(&path, &d_list, tot * 0.4, tot * 0.62),
                        1.2,
                        1.0,
                        r[3],
                    );
                }
                if let (Some(&first), Some(&last_pt)) = (path.first(), path.last()) {
                    p.glow_line(first.0, first.1, last_pt.0, last_pt.1, 4.0 * s, r[2], 0.12);
                }
            } else {
                let d_list = lengths(&path);
                let tot = d_list.last().copied().unwrap_or(1.0);
                let mut rg = Rng::new(hash(&[
                    self.seed,
                    K_DASH,
                    f64::from(i as i32),
                    f64::from(k),
                ]));
                let mut d = rg.range(0.0, 3.0);
                while d < tot {
                    let on = rg.range(4.0, 7.0);
                    let pc = sub_path(&path, &d_list, d, js_min(tot, d + on));
                    strand(p, &pc, 3.2, 1.0, if far { r[0] } else { r[1] });
                    strand(p, &pc, 1.6, 1.0, if far { r[1] } else { r[2] });
                    d += on + rg.range(2.0, 3.5);
                }
            }
        }

        // The beat: charge and snap
        if self.beat && k == BEAT - 1 {
            for (x, y, dir) in [(sa.0, sa.1, 1.0), (sb.0, sb.1, -1.0)] {
                let tick = snap_path(
                    hash(&[self.seed, K_FORK, 98.0, dir]),
                    x,
                    y,
                    x + dir * 7.0,
                    y - dir * 1.0 + dir * 4.0,
                    1.6,
                    0.0,
                );
                strand(p, &tick, 3.0, 1.0, r[1]);
                strand(p, &tick, 1.6, 1.0, r[2]);
                flash_star(p, x, y, 4.0, r[1], 4, 0.28, 0.3, 1.0);
                flash_star(p, x, y, 2.6, r[3], 4, 0.4, 0.3, 1.0);
                p.glow(x, y, 7.0, r[3], 0.4);
            }
        } else if self.beat && k == BEAT {
            let mut rg = Rng::new(hash(&[self.seed, K_FORK, 99.0]));
            let mut forks = Vec::with_capacity(4);
            let inner_len = beat_path.len().saturating_sub(2).max(1);
            for j in 0..4 {
                let at = beat_path[1 + ((j * 2 + 1) % inner_len)];
                let a = (if j % 2 != 0 { -0.9 } else { 2.2 }) + rg.range(-0.5, 0.5);
                let l = rg.range(6.0, 10.0);
                forks.push(snap_path(
                    hash(&[self.seed, K_FORK, 99.0, f64::from(j as i32)]),
                    at.0,
                    at.1,
                    at.0 + libm::cos(a) * l,
                    at.1 + libm::sin(a) * l,
                    1.6,
                    0.0,
                ));
            }
            for f in &forks {
                strand(p, f, 3.4, 1.0, r[1]);
            }
            strand(p, &beat_path, 7.0, 2.0, r[1]);
            for f in &forks {
                strand(p, f, 1.8, 1.0, r[3]);
            }
            strand(p, &beat_path, 4.4, 1.4, r[3]);
            let d_list = lengths(&beat_path);
            let tot = d_list.last().copied().unwrap_or(1.0);
            strand(
                p,
                &sub_path(&beat_path, &d_list, tot * 0.1, tot * 0.9),
                2.0,
                1.0,
                r[5],
            );
            for (x, y) in [sa, sb] {
                flash_star(p, x, y, 5.0, r[3], 4, 0.28, 0.3, 1.0);
                flash_star(p, x, y, 3.0, r[4], 4, 0.4, 0.3, 1.0);
                p.rect(libm::floor(x), libm::floor(y), 1.0, 1.0, r[5], 1.0);
            }
            for pair in beat_path.windows(2) {
                p.glow_line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, 8.0, r[3], 0.3);
            }
            p.glow(0.0, -16.0, 22.0, r[2], 0.25);
        }

        p.glow(0.0, -16.0, 18.0, r[1], 0.12);
        for &(x, y) in &ends {
            p.glow(x, y, 4.0, r[3], 0.3);
        }
        p.clean_flecks(2, true);
        let _ = TAU;
    }
}
