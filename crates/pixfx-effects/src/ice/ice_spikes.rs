//! Ice spikes: port of pixelfx `src/fx/ice/ice-spikes.js`.
//!
//! The eruption recipe (a frost line racing ahead, each spike punching up white with a spray of shards, a puff
//! and a glint, the floor frosting behind) is adapted from Sprite Fusion's open-source ice
//! (https://www.spritefusion.com/pixel-art-vfx/ice), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::sim::{BitKind, Bits, Kind, Lights, Particles, ParticlesOpts, sparkle};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{
    Rng, TAU, clamp, ease, js_max, js_min, js_round, rand, seg, smooth, to_uint32,
};

use super::lib::{
    BladeOpts, PALETTE, Pane, PaneEvent, Panes, Pt, bit, blade, emit, end_by, splinter, split_poly,
    step,
};

pub struct IceSpikes;

pub static ICE_SPIKES: IceSpikes = IceSpikes;

const META: Meta = Meta {
    id: "ice/ice-spikes",
    name: "Ice spikes",
    desc: "A frost line races along the floor and clusters of jagged ice blades burst up behind it in a rising wave, glint, flash white together and shatter into toppling shards.",
    tags: &[
        "ice",
        "spikes",
        "ground",
        "wave",
        "directional",
        "magic",
        "one-shot",
    ],
    size: (264, 112),
    origin: (24, 96),
    duration: D,
    looping: false,
    fps: 30,
    seed: 3,
    state: None,
    options: &[REACH.spec(), HEIGHT.spec()],
};

const REACH: NumOpt = NumOpt::new("reach", 172.0);
const HEIGHT: NumOpt = NumOpt::new("height", 56.0);
const D: f64 = 3.0;
/// The frost line runs out.
const START: f64 = 0.04;
const RUN: f64 = 0.44;
/// A glint runs down the line.
const SHIMMER: f64 = 0.74;
/// They shiver, flash white together and break.
const TREMBLE: f64 = 1.2;
const FLASH: f64 = 1.36;
const SHATTER: f64 = 1.42;
/// The frost goes, oldest first.
const MELT: f64 = 1.75;

/// One blade of a cluster: offset, height, base width, lean at the tip, in the back row, how late it bursts,
/// where its notch starts (-1 none), the cluster's main blade.
struct BladeSpec {
    dx: f64,
    h: f64,
    w: f64,
    lean: f64,
    back: bool,
    delay: f64,
    jag: f64,
    main: bool,
}

struct Cluster {
    x: f64,
    q: f64,
    h: f64,
    blades: Vec<BladeSpec>,
    t0: f64,
    up: bool,
}

impl Cluster {
    fn main_lean(&self) -> f64 {
        self.blades.get(1).map_or(0.0, |b| b.lean)
    }
}

/// A blade's base, tip and look right now.
struct Pose {
    g: f64,
    x: f64,
    tx: f64,
    ty: f64,
    white: bool,
}

fn pose(t: f64, c: &Cluster, b: &BladeSpec) -> Pose {
    let g = ease::out_back(clamp((t - c.t0 - b.delay) / 0.09, 0.0, 1.0), 2.2);
    let x = c.x + b.dx;
    Pose {
        g,
        x,
        tx: x + b.lean * g,
        ty: -b.h * g,
        white: t - c.t0 - b.delay < 0.03 || (FLASH..SHATTER).contains(&t),
    }
}

impl Effect for IceSpikes {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        let (reach, height) = (opts.num(&REACH), opts.num(&HEIGHT));
        let mut r = Rng::new(to_uint32(seed as f64));
        // clusters along the line: a main blade leaning into the wave and two to four splayed round it, heights
        // swelling to a crest past the middle and falling off at the end
        let env = |q: f64| 0.28 + 0.72 * libm::exp(-libm::pow((q - 0.64) / 0.3, 2.0));
        let mut clusters = Vec::new();
        let mut x = 10.0;
        while x < reach - 2.0 {
            let q = x / reach;
            let hh = height * env(q) * r.range(0.88, 1.06);
            let w = clamp(hh * 0.36, 5.0, 14.0);
            let lean = hh * r.range(0.2, 0.34);
            // a big blade leaning into the wave, a smaller one behind it leaning further, a stub in front of its foot
            let back = BladeSpec {
                dx: w * r.range(0.35, 0.6),
                h: hh * r.range(0.5, 0.7),
                w: w * 0.7,
                lean: hh * r.range(0.4, 0.6),
                back: true,
                delay: r.range(0.02, 0.05),
                jag: if r.chance(0.5) {
                    r.range(0.35, 0.6)
                } else {
                    -1.0
                },
                main: false,
            };
            let main = BladeSpec {
                dx: 0.0,
                h: hh,
                w,
                lean,
                back: false,
                delay: 0.0,
                jag: if r.chance(0.7) {
                    r.range(0.32, 0.58)
                } else {
                    -1.0
                },
                main: true,
            };
            let mut blades = vec![back, main];
            if hh > 14.0 {
                blades.push(BladeSpec {
                    dx: -w * r.range(0.35, 0.55),
                    h: hh * r.range(0.24, 0.4),
                    w: w * 0.55,
                    lean: -hh * r.range(0.0, 0.12),
                    back: false,
                    delay: r.range(0.03, 0.07),
                    jag: -1.0,
                    main: false,
                });
            }
            clusters.push(Cluster {
                x,
                q,
                h: hh,
                blades,
                t0: reached_at(x, reach) + 0.012,
                up: false,
            });
            x += r.range(20.0, 26.0);
        }
        let mut crest = 0;
        for (i, cl) in clusters.iter().enumerate() {
            if clusters.get(crest).is_some_and(|a| cl.h > a.h) {
                crest = i;
            }
        }
        Box::new(State {
            c,
            reach,
            r,
            ps: Particles::new(ParticlesOpts {
                ground: 0.0,
                sparks: vec![c[7], c[6], c[5], c[4]],
                haze: Some(c[5]),
            }),
            bits: Bits::new(0.0),
            lights: Lights::new(),
            panes: Panes::new(0.0),
            clusters,
            crest,
            t: 0.0,
            broke: false,
            trembled: false,
        })
    }
}

/// When the frost line reaches x.
fn reached_at(x: f64, reach: f64) -> f64 {
    START + RUN * (1.0 - libm::pow(1.0 - clamp(x / reach, 0.0, 1.0), 1.0 / 1.7))
}

struct State {
    c: [Rgba; 8],
    reach: f64,
    r: Rng,
    ps: Particles,
    bits: Bits,
    lights: Lights,
    panes: Panes,
    clusters: Vec<Cluster>,
    crest: usize,
    t: f64,
    broke: bool,
    trembled: bool,
}

impl State {
    /// How far the frost line has run.
    fn front(&self, t: f64) -> f64 {
        self.reach * (1.0 - libm::pow(1.0 - seg(t, START, START + RUN), 1.7))
    }

    fn draw_clusters(&self, p: &mut Painter) {
        let (c, t) = (&self.c, self.t);
        if t >= SHATTER {
            return;
        }
        for pass in 0..3 {
            for cl in &self.clusters {
                if t < cl.t0 {
                    continue;
                }
                let jx = if (TREMBLE..FLASH).contains(&t) {
                    if (libm::floor(t * 40.0) + js_round(cl.x)) % 2.0 != 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                for b in &cl.blades {
                    if (pass == 0) != b.back || (pass == 1) != b.main {
                        continue;
                    }
                    let s = pose(t, cl, b);
                    if s.g <= 0.0 {
                        continue;
                    }
                    let gl = (t - SHIMMER - cl.q * 0.34) / 0.2;
                    blade(
                        p,
                        s.x + jx,
                        0.0,
                        s.tx + jx,
                        s.ty,
                        b.w,
                        c,
                        BladeOpts {
                            a: 1.0,
                            white: s.white,
                            jag: b.jag,
                            dim: if b.back { 1.0 } else { 0.0 },
                            glint: if (0.0..=1.0).contains(&gl) { gl } else { -1.0 },
                        },
                    );
                }
                if pass == 2 {
                    // light: a pop at the base as it bursts, a cold glow up its length while it stands
                    let pop = clamp(1.0 - (t - cl.t0) / 0.15, 0.0, 1.0);
                    let wh = t >= FLASH;
                    p.glow(
                        cl.x + cl.main_lean() * 0.4,
                        -cl.h * 0.45,
                        cl.h * 0.9 + 10.0,
                        c[4],
                        (0.16 + 0.42 * pop + if wh { 0.4 } else { 0.0 })
                            * (0.85 + 0.15 * libm::sin(t * 3.0 + cl.x)),
                    );
                    if pop > 0.0 {
                        p.glow(cl.x, -2.0, 14.0, WHITE, 0.55 * pop);
                    }
                }
            }
        }
    }

    /// The frosted floor: a crust with rime tufts, a bloom under the surface, a white-hot head while it runs.
    fn draw_frost(&self, p: &mut Painter) {
        let (c, t, reach) = (&self.c, self.t, self.reach);
        let fr = self.front(t);
        let mut x = -3.0;
        while x <= js_min(fr, reach + 6.0) {
            let xx = x;
            x += 1.0;
            let q = clamp(xx / reach, 0.0, 1.0);
            let age = t - reached_at(js_max(0.0, xx), reach);
            let a = 1.0 - smooth(MELT + q * 0.35, MELT + q * 0.35 + 0.6, t);
            if a <= 0.0 || age < 0.0 {
                continue;
            }
            let h = rand(&[xx, 17.0]);
            let grow = smooth(0.0, 0.3, age);
            let head = fr - xx < 3.0 && fr < reach;
            p.dot(
                xx,
                0.0,
                if head || age < 0.06 || h < 0.3 {
                    WHITE
                } else {
                    c[6]
                },
                a,
            );
            let l = if h < 0.45 {
                js_round(grow * (1.0 + rand(&[xx, 5.0]) * 2.6))
            } else {
                0.0
            };
            let mut j = 1.0;
            while j <= l {
                p.dot(xx, -j, if j == l { WHITE } else { c[5] }, a);
                j += 1.0;
            }
            if l > 1.0 && rand(&[xx, 9.0]) < 0.5 {
                p.dot(xx + 1.0, -l + 1.0, c[6], a);
            }
            let d = grow * (2.0 + rand(&[xx, 11.0]) * 4.0) * (1.0 - q * 0.3);
            let mut j = 1.0;
            while j <= d {
                let col = if rand(&[xx * 31.0 + j, 7.0]) < 0.06 {
                    WHITE
                } else if j < 2.0 {
                    c[5]
                } else if j < 4.0 {
                    c[4]
                } else {
                    c[3]
                };
                p.dot(xx, j, col, a * (1.1 - j / (d + 1.0)));
                j += 1.0;
            }
            if head {
                p.dot(xx, -1.0, WHITE, 1.0);
            }
        }
        if fr < reach && t > START {
            p.glow(fr, -2.0, 16.0, c[6], 0.6);
            p.glow(fr, -1.0, 7.0, WHITE, 0.6);
        }
    }
}

/// Break a blade into toppling pieces, splinters and ice dust.
fn break_blade(
    t: f64,
    cl: &Cluster,
    b: &BladeSpec,
    c: &[Rgba; 8],
    r: &mut Rng,
    panes: &mut Panes,
    bits: &mut Bits,
) {
    use core::f64::consts::FRAC_PI_2;
    let s = pose(t, cl, b);
    let (ax, ay) = (s.tx - s.x, s.ty);
    let l = libm::hypot(ax, ay);
    let l = if l == 0.0 { 1.0 } else { l };
    let (nx, ny, hw) = (-ay / l, ax / l, b.w / 2.0);
    let tri: Vec<Pt> = vec![
        (s.x - nx * hw, -ny * hw),
        (s.tx, s.ty),
        (s.x + nx * hw, ny * hw),
    ];
    let ang = libm::atan2(ay, ax) + FRAC_PI_2;
    let mut parts = vec![tri];
    if b.h > 9.0 {
        parts = parts
            .iter()
            .flat_map(|poly| split_poly(poly, s.x + ax * 0.4, ay * 0.4, ang))
            .collect();
    }
    if b.h > 22.0 {
        parts = parts
            .into_iter()
            .flat_map(|poly| {
                if poly.iter().any(|&(_, y)| y < ay * 0.5) {
                    split_poly(&poly, s.x + ax * 0.72, ay * 0.72, ang)
                } else {
                    vec![poly]
                }
            })
            .collect();
    }
    for part in parts {
        let (mut mx, mut my) = (0.0, 0.0);
        for &(x, y) in &part {
            mx += x;
            my += y;
        }
        mx /= part.len() as f64;
        my /= part.len() as f64;
        let up = clamp(-my / 40.0, 0.0, 1.0);
        let pts = part.iter().map(|&(x, y)| (x - mx, y - my)).collect();
        let mut q = Pane::new(pts, mx, my);
        q.vx = r.range(-25.0, 60.0);
        q.vy = -r.range(40.0, 130.0) * (0.5 + up);
        q.a = 0.0;
        q.va = r.range(-8.0, 8.0);
        q.f = r.range(-0.7, 0.7);
        q.vf = r.range(-6.0, 6.0);
        q.lit = if b.back { 0.45 } else { 0.75 };
        q.g = 0.9;
        q.drag = 0.6;
        q.bounce = 0.3;
        q.solid = true;
        q.ridge = Some(libm::atan2(ay, ax));
        q.brittle = if b.h > 30.0 { 2 } else { 1 };
        q.crumble = Some(r.range(0.12, 0.4));
        q.life = r.range(1.0, 1.4);
        panes.add(q);
    }
    let n = 2.0 + b.h / 7.0;
    let mut i = 0.0;
    while i < n {
        i += 1.0;
        let u = r.next();
        let kind = if r.chance(0.15) {
            BitKind::Chunk
        } else {
            BitKind::Shard
        };
        bit!(bits, kind;
            x = s.x + ax * u + r.range(-1.5, 1.5),
            y = ay * u,
            vx = r.range(-60.0, 90.0),
            vy = -r.range(60.0, 230.0),
            g = 0.9,
            drag = 0.6,
            bounce = 0.3,
            life = r.range(0.7, 1.3),
            a = r.range(0.0, TAU),
            va = r.range(-20.0, 20.0),
            s = r.range(1.2, 3.2),
            c = step(c, r.int(4.0, 7.0)),
            ramp = Some(vec![c[1], c[3], c[4], c[6], c[7]]),
        );
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let fr = self.front(t);
        let reach = self.reach;
        let State {
            c,
            r,
            ps,
            bits,
            lights,
            panes,
            clusters,
            crest,
            broke,
            trembled,
            ..
        } = self;
        let c = *c;
        // the cast: a puff and a spray where the line starts
        if t - dt <= START && t > START {
            for _ in 0..4 {
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    r.range(0.0, 6.0),
                    -r.range(0.0, 3.0),
                    r.range(10.0, 60.0),
                    -r.range(2.0, 12.0),
                    r.range(0.4, 0.7),
                    c[6],
                    r.range(2.0, 3.2)
                );
            }
            lights.add(4.0, -4.0, 26.0, c[5], 0.6, 0.25);
        }
        // the running front throws frost: streaks along the floor, motes, vapour
        if t > START && fr < reach - 1.0 {
            if r.chance(0.5) {
                emit!(
                    ps,
                    r,
                    Kind::Streak,
                    fr - r.range(4.0, 12.0),
                    -r.range(1.0, 3.0),
                    r.range(40.0, 80.0),
                    -r.range(0.0, 6.0),
                    r.range(0.08, 0.14),
                    if r.chance(0.5) { c[6] } else { WHITE }
                );
            }
            if r.chance(0.3) {
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    fr,
                    -1.0,
                    r.range(10.0, 50.0),
                    -r.range(20.0, 70.0),
                    r.range(0.5, 0.9),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(0.5, 1.2)
                );
            }
            if r.chance(0.12) {
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    fr,
                    -r.range(0.0, 3.0),
                    r.range(20.0, 60.0),
                    -r.range(2.0, 10.0),
                    r.range(0.4, 0.7),
                    c[6],
                    r.range(1.6, 2.8)
                );
            }
        }
        for (ci, cl) in clusters.iter_mut().enumerate() {
            if cl.up || t < cl.t0 {
                continue;
            }
            cl.up = true;
            // each cluster bursts up: shards sprayed forward, a puff, motes, a glint at the tip once it stands
            let big = ci == *crest;
            let n = 3.0 + js_round(cl.h / 8.0);
            let mut i = 0.0;
            while i < n {
                i += 1.0;
                let a = -core::f64::consts::FRAC_PI_2 + r.range(-1.1, 1.0);
                let sp = r.range(60.0, 190.0) * (0.45 + cl.h / 44.0);
                bit!(bits, BitKind::Shard;
                    x = cl.x + r.range(-3.0, 3.0),
                    y = -1.0,
                    vx = libm::cos(a) * sp + 25.0,
                    vy = libm::sin(a) * sp,
                    g = 0.8,
                    drag = 0.6,
                    bounce = 0.3,
                    life = r.range(0.6, 1.1),
                    a = r.range(0.0, TAU),
                    va = r.range(-18.0, 18.0),
                    s = r.range(1.2, 3.0),
                    c = step(&c, r.int(5.0, 7.0)),
                );
            }
            emit!(
                ps,
                r,
                Kind::Vapour,
                cl.x,
                -1.0,
                r.range(-10.0, 40.0),
                -r.range(2.0, 10.0),
                r.range(0.5, 0.9),
                c[6],
                r.range(2.2, 3.4) * (0.6 + cl.h / 60.0)
            );
            for _ in 0..2 {
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    cl.x + r.range(-4.0, 4.0),
                    -r.range(2.0, cl.h * 0.5),
                    r.range(-20.0, 40.0),
                    -r.range(20.0, 60.0),
                    r.range(0.6, 1.1),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(0.5, 1.3)
                );
            }
            if cl.h > 16.0 {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    cl.x + cl.main_lean(),
                    -cl.h * 0.96,
                    0.0,
                    -4.0,
                    r.range(0.3, 0.5),
                    WHITE,
                    r.range(1.4, 2.6),
                    0.08
                );
            }
            lights.add(cl.x, -cl.h * 0.4, 16.0 + cl.h * 0.6, c[5], 0.5, 0.18);
            if big {
                lights.add(cl.x, -cl.h * 0.5, 54.0, WHITE, 0.5, 0.14);
                for _ in 0..4 {
                    emit!(
                        ps,
                        r,
                        Kind::Vapour,
                        cl.x + r.range(-6.0, 6.0),
                        -r.range(0.0, 4.0),
                        r.range(-40.0, 60.0),
                        -r.range(4.0, 16.0),
                        r.range(0.6, 1.0),
                        c[6],
                        r.range(3.0, 4.5)
                    );
                }
            }
        }
        // while they stand: glints wink on the blades
        if t > START + RUN && t < TREMBLE && r.chance(0.09) {
            let ci = libm::floor(r.next() * clusters.len() as f64) as usize;
            if let Some(cl) = clusters.get(ci) {
                let bi = libm::floor(r.next() * cl.blades.len() as f64) as usize;
                if let Some(b) = cl.blades.get(bi) {
                    let s = pose(t, cl, b);
                    let u = r.range(0.3, 0.95);
                    emit!(
                        ps,
                        r,
                        Kind::Star,
                        s.x + (s.tx - s.x) * u,
                        s.ty * u,
                        0.0,
                        -3.0,
                        r.range(0.25, 0.5),
                        if r.chance(0.6) { WHITE } else { c[6] },
                        r.range(0.6, 2.0)
                    );
                }
            }
        }
        // they shiver: chips fall
        if !*trembled && t >= TREMBLE {
            *trembled = true;
            for cl in clusters.iter() {
                for _ in 0..2 {
                    bit!(bits, BitKind::Shard;
                        x = cl.x + r.range(-4.0, 4.0),
                        y = -r.range(4.0, cl.h * 0.8),
                        vx = r.range(-20.0, 20.0),
                        vy = -r.range(0.0, 30.0),
                        g = 0.8,
                        drag = 0.5,
                        bounce = 0.3,
                        life = r.range(0.3, 0.6),
                        a = r.range(0.0, TAU),
                        va = r.range(-18.0, 18.0),
                        s = r.range(1.0, 1.8),
                        c = step(&c, r.int(5.0, 7.0)),
                    );
                }
            }
        }
        if !*broke && t >= SHATTER {
            *broke = true;
            // all at once: every blade breaks into toppling pieces, splinters and ice dust, light along the line
            for cl in clusters.iter() {
                for b in &cl.blades {
                    break_blade(t, cl, b, &c, r, panes, bits);
                }
                for _ in 0..2 {
                    emit!(
                        ps,
                        r,
                        Kind::Vapour,
                        cl.x + r.range(-6.0, 6.0),
                        -r.range(1.0, 8.0),
                        r.range(-30.0, 50.0),
                        -r.range(4.0, 18.0),
                        r.range(0.6, 1.1),
                        c[6],
                        r.range(2.4, 4.0) * (0.7 + cl.h / 60.0)
                    );
                }
                if r.chance(0.7) {
                    emit!(
                        ps,
                        r,
                        Kind::Star,
                        cl.x + r.range(-6.0, 6.0),
                        -r.range(4.0, cl.h),
                        r.range(-10.0, 10.0),
                        -r.range(6.0, 20.0),
                        r.range(0.4, 0.8),
                        if r.chance(0.5) { WHITE } else { c[6] },
                        r.range(1.0, 2.2)
                    );
                }
                for _ in 0..2 {
                    emit!(
                        ps,
                        r,
                        Kind::Flake,
                        cl.x + r.range(-6.0, 6.0),
                        -r.range(4.0, cl.h),
                        r.range(-30.0, 60.0),
                        -r.range(30.0, 90.0),
                        r.range(0.9, 1.5),
                        if r.chance(0.6) { WHITE } else { c[6] },
                        r.range(0.5, 1.5)
                    );
                }
                lights.add(cl.x, -cl.h * 0.5, 22.0 + cl.h, WHITE, 0.45, 0.12);
            }
            for i in 0..5 {
                lights.add(
                    (reach * (f64::from(i) + 0.5)) / 5.0,
                    -18.0,
                    40.0,
                    c[4],
                    0.4,
                    0.5,
                );
            }
        }
        // the shatter's dust settles; glints on the debris
        if *broke && t < SHATTER + 0.8 && r.chance(0.1) && bits.count() > 0 {
            let i = libm::floor(r.next() * bits.count() as f64) as usize;
            if let Some(b) = bits.list().get(i).map(|q| (q.bit.x, q.bit.y)) {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    b.0,
                    b.1 - 1.0,
                    0.0,
                    -3.0,
                    r.range(0.2, 0.4),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(0.6, 1.5)
                );
            }
        }
        ps.update(r, dt);
        bits.update(dt);
        panes.update(dt, |e| match e {
            PaneEvent::Crumble { x, .. } => {
                splinter(bits, r, &c, x, 0.0, 0.0, 3);
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    x,
                    -1.0,
                    r.range(-12.0, 12.0),
                    -r.range(2.0, 8.0),
                    r.range(0.4, 0.7),
                    c[6],
                    r.range(1.4, 2.2)
                );
            }
            PaneEvent::Land { vx, x, y, speed } => {
                if speed < 110.0 {
                    return;
                }
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    x,
                    y - 1.0,
                    r.range(-20.0, 20.0),
                    -r.range(2.0, 8.0),
                    r.range(0.4, 0.7),
                    c[6],
                    r.range(1.4, 2.4)
                );
                splinter(bits, r, &c, x, y, vx, 3);
                if r.chance(0.3) {
                    emit!(
                        ps,
                        r,
                        Kind::Star,
                        x,
                        y - 2.0,
                        0.0,
                        -5.0,
                        r.range(0.2, 0.4),
                        WHITE,
                        r.range(0.8, 1.6)
                    );
                }
            }
            PaneEvent::Skid { .. } => {}
        });
        lights.update(dt);
        end_by(t, D, ps, Some(bits), Some(panes));
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer != Layer::Front {
            self.draw_frost(p);
            self.ps.draw(p, Layer::Back);
        }
        if layer == Layer::Back {
            return;
        }
        let (c, t) = (&self.c, self.t);
        self.draw_clusters(p);
        // the cold light the frosted floor gives off
        let k = (1.0 - smooth(MELT, MELT + 0.9, t)) * smooth(START, START + 0.3, t);
        let fr = self.front(t);
        for i in 0..6 {
            p.glow(
                (fr * (f64::from(i) + 0.5)) / 6.0,
                -3.0,
                26.0,
                c[4],
                0.15 * k,
            );
        }
        if (FLASH..SHATTER + 0.1).contains(&t) {
            for cl in &self.clusters {
                sparkle(
                    p,
                    cl.x + cl.main_lean(),
                    -cl.h,
                    6.0 * (1.0 - clamp((t - FLASH) / 0.16, 0.0, 1.0)),
                    c[6],
                    c[4],
                    0.4,
                );
            }
        }
        self.panes.draw(p, c);
        self.bits.draw(p);
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}
