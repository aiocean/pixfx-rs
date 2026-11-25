//! Flame pillar: port of pixelfx `src/fx/fire/flame-pillar.js`.
//!
//! Beyond pixelfx: the eruption fires impact cues ([`pixfx_core::cue::impact`]) for the game to answer; the
//! `feel` option scales them, 0 for none.
//!
//! The flame-puff, smoke, spark, debris and floor-ring recipe is adapted from Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), and the column's levels, pulses and strands from its
//! holy light (https://www.spritefusion.com/pixel-art-vfx/holy-light), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{
    Bit, BitKind, Bits, Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, band,
};
use pixfx_core::{ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, rgb};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_max, noise, rand, smooth, to_uint32};

use super::lib::{Hot, LAYERS, Stone, flame_tongue, floor_light, hot_first, rock, sim_draw};

pub struct FlamePillar;

pub static FLAME_PILLAR: FlamePillar = FlamePillar;

const STONE: ColorOpt = ColorOpt::new("stone", Rgba::hex24(0x4a3f69));
// the column is drawn a row per pixel of its height: held to what draws in reasonable time
const HEIGHT: NumOpt = NumOpt::new("height", 140.0).within(0.0, 1000.0);
const META: Meta = Meta {
    id: "fire/flame-pillar",
    name: "Flame pillar",
    desc: "The floor cracks and glows, a column of fire erupts white-hot and roars, spewing embers and shedding flame, then collapses into rolling smoke.",
    tags: &["fire", "pillar", "eruption", "spell", "ground", "one-shot"],
    size: (184, 256),
    origin: (92, 232),
    duration: 5.0,
    looping: false,
    fps: 30,
    seed: 5,
    state: None,
    options: &[STONE.spec(), HEIGHT.spec(), FEEL.spec()],
};

const PALETTE: [Rgba; 9] = [
    Rgba::hex24(0x4a1712),
    Rgba::hex24(0x8a1f17),
    Rgba::hex24(0xc9281f),
    Rgba::hex24(0xf2561d),
    Rgba::hex24(0xff8a2a),
    Rgba::hex24(0xffc23d),
    Rgba::hex24(0xffe26b),
    Rgba::hex24(0xfff3a6),
    Rgba::hex24(0xffffff),
];

const DURATION: f64 = 5.0;
const ERUPT: f64 = 0.55;
const RISE: f64 = 0.12;
const FALL: f64 = 1.95;
const GONE: f64 = 2.45;
const SOOT: Rgba = Rgba::hex24(0x0d0a12);
const PI: f64 = core::f64::consts::PI;

impl Effect for FlamePillar {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the eruption asks the game for an impact's answer.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(ERUPT, 0.8, 0.0, -8.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let h = hot_first(&opts.palette_or(&PALETTE));
        let st = Stone::of(opts.color(&STONE));
        let mut r = Rng::new(to_uint32(seed as f64));
        let s = seed as f64 * 13.0;
        // the cracks: jagged lines racing out from the middle across the floor (squashed for perspective), with
        // branches
        let mut cracks: Vec<Crack> = Vec::new();
        let crack = |r: &mut Rng,
                     cracks: &mut Vec<Crack>,
                     mut x: f64,
                     mut y: f64,
                     mut a: f64,
                     len: f64,
                     t0: f64,
                     w: bool|
         -> Vec<(f64, f64)> {
            let mut pts = vec![(x, y)];
            let mut d = 0.0;
            while d < len {
                a += r.range(-0.55, 0.55);
                x += libm::cos(a) * 2.0;
                y += libm::sin(a) * 0.76;
                pts.push((x, y));
                d += 2.0;
            }
            let grow = r.range(0.22, 0.36);
            cracks.push(Crack {
                pts: pts.clone(),
                t0,
                grow,
                w,
            });
            pts
        };
        for i in 0..9 {
            let a = (f64::from(i) / 9.0) * TAU + r.range(-0.25, 0.25);
            let (x, y) = (r.range(-1.0, 1.0), r.range(-0.3, 0.3));
            let (len, t0) = (r.range(20.0, 40.0), r.range(0.0, 0.16));
            let pts = crack(&mut r, &mut cracks, x, y, a, len, t0, true);
            let j = 3.0 + r.int(0.0, pts.len() as f64 - 6.0);
            if pts.len() > 8 {
                let (px, py) = pts.get(j as usize).copied().unwrap_or((f64::NAN, f64::NAN));
                let a2 = a + r.sign() * r.range(0.5, 1.0);
                let len = r.range(6.0, 14.0);
                let t0 = 0.18 + r.range(0.0, 0.15);
                crack(&mut r, &mut cracks, px, py, a2, len, t0, false);
            }
        }
        // strands of hotter flame running up the column
        let strands = (0..7)
            .map(|i| {
                let i = f64::from(i);
                Strand {
                    o: (rand(&[i, s, 1.0]) - 0.5) * 1.4,
                    w: 0.05 + rand(&[i, s, 2.0]) * 0.06,
                    f: 1.0 + rand(&[i, s, 3.0]) * 2.0,
                }
            })
            .collect();
        let ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: vec![h[1], h[2], h[4], h[5]],
            ..ParticlesOpts::default()
        });
        Box::new(State {
            h,
            hmax: opts.num(&HEIGHT),
            st,
            r,
            s,
            ps,
            bits: Bits::new(0.0),
            lights: Lights::new(),
            cracks,
            strands,
            t: 0.0,
            erupted: false,
            fell: false,
        })
    }
}

struct Crack {
    pts: Vec<(f64, f64)>,
    t0: f64,
    grow: f64,
    w: bool,
}

struct Strand {
    o: f64,
    w: f64,
    f: f64,
}

struct State {
    h: Hot,
    hmax: f64,
    st: Stone,
    r: Rng,
    s: f64,
    ps: Particles,
    /// Each piece's data: whether it has kicked up its dust.
    bits: Bits<bool>,
    lights: Lights,
    cracks: Vec<Crack>,
    strands: Vec<Strand>,
    t: f64,
    erupted: bool,
    fell: bool,
}

impl State {
    fn glow(&self) -> Rgba {
        mix(mix(self.h[3], self.h[4], 0.5), WHITE, 0.25)
    }

    fn height(&self, tt: f64) -> f64 {
        let hm = self.hmax;
        if tt < ERUPT {
            0.0
        } else if tt < ERUPT + RISE {
            hm * ease::out_cubic((tt - ERUPT) / RISE)
        } else if tt < FALL {
            hm * (1.0 + 0.05 * noise(tt * 3.0, self.s))
        } else {
            hm * (1.0 - ease::in_quad(clamp((tt - FALL) / (GONE - FALL), 0.0, 1.0)))
        }
    }

    fn width(&self, tt: f64) -> f64 {
        if tt < ERUPT {
            0.0
        } else if tt < FALL {
            12.0 * (1.0 + 0.45 * libm::exp(-(tt - ERUPT) * 7.0))
                * (0.94 + 0.06 * noise(tt * 9.0, self.s + 1.0))
        } else {
            12.0 * (1.0 - smooth(FALL, GONE, tt)) + 1.5 * (1.0 - smooth(GONE - 0.1, GONE, tt))
        }
    }

    fn heat(tt: f64) -> f64 {
        smooth(0.0, ERUPT, tt) * 0.75 + 0.25 * smooth(ERUPT - 0.05, ERUPT + 0.1, tt)
            - smooth(GONE - 0.3, DURATION - 0.6, tt) * 1.05
    }

    fn column(&self, p: &mut Painter) {
        let (t, h, s) = (self.t, &self.h, self.s);
        let hh = self.height(t);
        let w0 = self.width(t);
        if hh < 1.0 || w0 < 0.3 {
            return;
        }
        let glow = self.glow();
        // a crown of tongues where the column breaks into the air, drawn first so the column flows up into them
        if t < FALL + 0.25 {
            let k = clamp(w0 / 12.0, 0.0, 1.0);
            let nz = |rate: f64, seed: f64, off: f64| noise(t * rate + off, seed);
            for i in 0..5 {
                let fi = f64::from(i);
                let n = (s + fi) * 17.0;
                let dx = (fi - 2.0) * w0 * 0.4;
                let th = (12.0 + 12.0 * (0.5 + 0.5 * noise(t * 3.3 + fi, n)))
                    * k
                    * if i == 2 {
                        1.2
                    } else if i % 4 == 0 {
                        0.6
                    } else {
                        0.9
                    };
                flame_tongue(
                    p,
                    dx + noise(t * 2.0, n + 2.0) * 1.5,
                    -hh + 12.0,
                    th,
                    3.4 * k * if i % 4 == 0 { 0.7 } else { 1.0 },
                    -dx * 0.3 + noise(t * 2.2, n + 1.0) * 3.0,
                    &nz,
                    n,
                    h,
                    &LAYERS,
                    0.5,
                    1.1,
                );
            }
        }
        let mut y = 0.0;
        while y >= -hh {
            let v = -y / hh;
            let cx = noise(-y * 0.04 + t * 1.6, s + 2.0) * 1.6 * v;
            // licks bulge out of the sides and run up them; a wider splash at the foot; the top thins into tongues
            let foot = 1.0 + 0.7 * libm::pow(js_max(0.0, 1.0 - -y / 12.0), 2.0);
            let top = 1.0 - smooth(0.82, 1.0, v) * 0.7;
            let wl = w0
                * foot
                * top
                * (1.0 + 0.65 * libm::pow(js_max(0.0, noise(y * 0.2 + t * 8.0, s + 3.0)), 1.5));
            let wr = w0
                * foot
                * top
                * (1.0 + 0.65 * libm::pow(js_max(0.0, noise(y * 0.2 + t * 8.0, s + 4.0)), 1.5));
            let pulse = if (-y * 0.032 + t * 3.6) % 1.0 < 0.09 {
                1.0
            } else {
                0.0
            };
            let mut x = libm::floor(cx - wl * 1.15);
            let x1 = libm::ceil(cx + wr * 1.15);
            while x <= x1 {
                let px = x;
                x += 1.0;
                let dx = px + 0.5 - cx;
                let q = libm::fabs(dx) / if dx < 0.0 { wl } else { wr };
                if q > 1.15 {
                    continue;
                }
                let mut lvl: f64 = if q < 0.12 {
                    0.0
                } else if q < 0.26 {
                    1.0
                } else if q < 0.42 {
                    2.0
                } else if q < 0.56 {
                    3.0
                } else if q < 0.7 {
                    4.0
                } else if q < 0.84 {
                    5.0
                } else if q < 0.96 {
                    6.0
                } else {
                    7.0
                };
                for st in &self.strands {
                    if libm::fabs(dx / w0 - st.o - 0.06 * libm::sin(t * st.f * 3.0 + st.o * 9.0))
                        < st.w
                    {
                        lvl = js_max(0.0, lvl - 1.0);
                    }
                }
                lvl = js_max(0.0, lvl - pulse);
                if lvl == 7.0 && rand(&[px, y, libm::floor(t * 20.0)]) < 0.5 {
                    continue;
                }
                p.paint_cell(
                    px,
                    y,
                    h.get(lvl as usize).copied().unwrap_or(Rgba::NONE),
                    1.0,
                );
            }
            if y % 12.0 == 0.0 {
                p.glow(
                    cx,
                    y,
                    w0 * 2.4 + 8.0,
                    glow,
                    0.28 * clamp(w0 / 12.0, 0.0, 1.0),
                );
            }
            y -= 1.0;
        }
        p.glow(
            0.0,
            -hh * 0.45,
            92.0,
            glow,
            0.3 * clamp(w0 / 12.0, 0.0, 1.0),
        );
    }

    fn cracks_draw(&self, p: &mut Painter) {
        let (t, h) = (self.t, &self.h);
        let k = Self::heat(t);
        if k <= -0.05 {
            return;
        }
        let glow = self.glow();
        for c in &self.cracks {
            let len = c.pts.len() as f64;
            let n = libm::floor(len * clamp((t - c.t0) / c.grow, 0.0, 1.0)) as usize;
            for i in 1..n {
                let (Some(&(x0, y0)), Some(&(x1, y1))) = (c.pts.get(i - 1), c.pts.get(i)) else {
                    continue;
                };
                let u = i as f64 / len;
                let hot = k * (1.0 - 0.45 * u);
                if hot < 0.0 {
                    // cooled: soot that fades away
                    let a = 1.0 + hot * 3.0;
                    if a > 0.0 {
                        p.line(x0, y0, x1, y1, SOOT, a);
                    }
                    continue;
                }
                let col = if hot > 0.85 {
                    h[1]
                } else if hot > 0.65 {
                    h[2]
                } else if hot > 0.45 {
                    h[4]
                } else if hot > 0.25 {
                    h[5]
                } else if hot > 0.1 {
                    h[6]
                } else {
                    h[7]
                };
                // the crack: a hot seam with a darker glowing lip, wider near the middle
                if c.w && (i as f64) < len * 0.5 && hot > 0.2 {
                    p.line(
                        x0,
                        y0 + 1.0,
                        x1,
                        y1 + 1.0,
                        if hot > 0.6 { h[5] } else { h[6] },
                        1.0,
                    );
                    p.line(x0, y0 - 1.0, x1, y1 - 1.0, h[7], 1.0);
                }
                p.line(x0, y0, x1, y1, col, 1.0);
                if i % 2 == 0 {
                    p.glow(x1, y1, 5.0 + 7.0 * hot, glow, 0.3 * hot);
                }
            }
        }
        // the floor glows from beneath while it's hot
        if k > 0.0 {
            floor_light(p, 0.0, 0.5, 44.0, 8.0, glow, 0.35 * k);
        }
    }

    fn ground_ring(&self, p: &mut Painter, back: bool) {
        let k = (self.t - ERUPT) / 0.45;
        if k > 0.0 && k < 1.0 {
            let st = &self.st;
            band(
                p,
                0.0,
                0.5,
                84.0 * ease::out_cubic(k),
                0.18,
                js_max(1.0, 4.0 * (1.0 - k)),
                1.15 - k,
                &[st.body, st.light, self.h[4], self.h[1]],
                |g| {
                    if back {
                        libm::sin(g) < 0.0
                    } else {
                        libm::sin(g) >= 0.0
                    }
                },
                !back && k < 0.15,
            );
        }
    }

    fn draw_back(&self, p: &mut Painter) {
        self.cracks_draw(p);
        sim_draw(p, &self.ps, &self.h, Layer::Back);
        self.ground_ring(p, true);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (t, h) = (self.t, &self.h);
        self.column(p);
        for b in self.bits.list() {
            rock(p, &b.bit, &self.st);
        }
        sim_draw(p, &self.ps, h, Layer::Front);
        self.ground_ring(p, false);
        // the eruption's white flash at the foot
        let f = (t - ERUPT) / 0.08;
        if f > 0.0 && f < 1.0 {
            p.ellipse(0.0, -2.0, 16.0 * (1.0 - f * 0.3), 6.0, h[2], 1.0);
            p.ellipse(
                0.0,
                -3.0,
                11.0 * (1.0 - f * 0.3),
                4.0,
                if f < 0.5 { WHITE } else { h[1] },
                1.0,
            );
        }
        self.lights.draw(p);
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let h = self.h;
        let k60 = dt * 60.0;
        // the charge: the crust in the middle glows, little flames lick out of it, heat leaks up the cracks
        if t < ERUPT && self.r.chance(0.6 * k60 * smooth(0.15, ERUPT, t)) {
            let r = &mut self.r;
            let x = r.range(-5.0, 5.0);
            let vx = r.range(-6.0, 6.0);
            let vy = -r.range(20.0, 45.0);
            let life = r.range(0.15, 0.3);
            let size = r.range(1.0, 2.2);
            self.ps.spawn(
                r,
                Spawn {
                    col: None,
                    size,
                    ..Spawn::new(Kind::Fire, x, -1.0, vx, vy, life)
                },
            );
        }
        if t < ERUPT && self.r.chance(0.5 * k60 * smooth(0.0, ERUPT, t)) {
            let r = &mut self.r;
            if let Some(c) = r.pick(&self.cracks) {
                let i = r.int(
                    0.0,
                    libm::floor(c.pts.len() as f64 * clamp((t - c.t0) / c.grow, 0.0, 1.0)) - 1.0,
                );
                if i >= 0.0 {
                    let (x, y) = c
                        .pts
                        .get(i as usize)
                        .copied()
                        .unwrap_or((f64::NAN, f64::NAN));
                    if r.chance(0.5) {
                        let vx = r.range(-5.0, 5.0);
                        let vy = -r.range(15.0, 35.0);
                        let life = r.range(0.15, 0.3);
                        let size = r.range(0.8, 1.6);
                        self.ps.spawn(
                            r,
                            Spawn {
                                col: None,
                                size,
                                ..Spawn::new(Kind::Fire, x, y - 1.0, vx, vy, life)
                            },
                        );
                    } else {
                        let vx = r.range(-15.0, 15.0);
                        let vy = -r.range(30.0, 70.0);
                        let life = r.range(0.5, 1.0);
                        self.ps.spawn(
                            r,
                            Spawn {
                                col: Some(h[3]),
                                ..Spawn::new(Kind::Ember, x, y - 1.0, vx, vy, life)
                            },
                        );
                    }
                }
            }
        }
        if !self.erupted && t >= ERUPT {
            self.erupted = true;
            self.erupt();
        }
        // the roar: flame shed off the sides, a billowing cap, embers spewed high
        let hh = self.height(t);
        let w = self.width(t);
        let r = &mut self.r;
        let ps = &mut self.ps;
        if t > ERUPT && t < GONE && hh > 4.0 {
            if r.chance(0.9 * k60) {
                let sg = r.sign();
                let y = -r.range(4.0, hh * 0.9);
                let x = sg * w * r.range(0.7, 1.1);
                let vx = sg * r.range(15.0, 40.0);
                let vy = -r.range(30.0, 70.0);
                let life = r.range(0.25, 0.5);
                let size = r.range(1.6, 3.2);
                let flags = if r.chance(0.35) {
                    Flags::SMOKES
                } else {
                    Flags::NONE
                };
                ps.spawn(
                    r,
                    Spawn {
                        col: None,
                        size,
                        flags,
                        ..Spawn::new(Kind::Fire, x, y, vx, vy, life)
                    },
                );
            }
            if r.chance(0.7 * k60) {
                let x = r.range(-w, w);
                let y = -hh + r.range(0.0, 6.0);
                let vx = r.range(-20.0, 20.0);
                let vy = -r.range(30.0, 80.0);
                let life = r.range(0.35, 0.7);
                let size = r.range(3.0, 5.5);
                let flags = if r.chance(0.6) {
                    Flags::SMOKES
                        | if t < FALL - 0.3 {
                            Flags::BIG
                        } else {
                            Flags::NONE
                        }
                } else {
                    Flags::NONE
                };
                ps.spawn(
                    r,
                    Spawn {
                        col: None,
                        size,
                        flags,
                        ..Spawn::new(Kind::Fire, x, y, vx, vy, life)
                    },
                );
            }
            if t < FALL && r.chance(0.5 * k60) {
                let x = r.range(-w, w) * 0.6;
                let vx = r.range(-60.0, 60.0);
                let vy = -r.range(50.0, 110.0);
                let life = r.range(0.7, 1.3);
                let size = if r.chance(0.2) { 2.0 } else { 1.0 };
                ps.spawn(
                    r,
                    Spawn {
                        col: Some(h[3]),
                        size,
                        ..Spawn::new(Kind::Ember, x, -hh + 4.0, vx, vy, life)
                    },
                );
            }
        }
        // the collapse: the falling top breaks into puffs that roll off as smoke
        if t >= FALL && !self.fell {
            self.fell = true;
        }
        if self.fell && t < GONE - 0.1 && r.chance(1.6 * k60) {
            let x = r.range(-w - 2.0, w + 2.0);
            let y = -hh - r.range(0.0, 8.0);
            let vx = r.range(-30.0, 30.0);
            let vy = -r.range(10.0, 50.0);
            let life = r.range(0.4, 0.75);
            let size = r.range(3.5, 6.5);
            ps.spawn(
                r,
                Spawn {
                    col: None,
                    size,
                    flags: Flags::SMOKES,
                    ..Spawn::new(Kind::Fire, x, y, vx, vy, life)
                },
            );
        }
        self.bits.update(dt);
        let dust = self.st.dust;
        for b in self.bits.list_mut() {
            if b.rest && !b.data {
                b.data = true;
                for _ in 0..3 {
                    let x = b.bit.x + r.range(-1.5, 1.5);
                    let vx = r.sign() * r.range(8.0, 35.0);
                    let vy = -r.range(2.0, 18.0);
                    let life = r.range(0.3, 0.6);
                    let size = r.range(1.3, 2.6);
                    ps.spawn(
                        r,
                        Spawn {
                            col: Some(dust),
                            size,
                            ..Spawn::new(Kind::Dust, x, -0.5, vx, vy, life)
                        },
                    );
                }
            }
        }
        ps.update(r, dt);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        match layer {
            Layer::Back => self.draw_back(p),
            Layer::Front => self.draw_front(p),
            Layer::Both => {
                self.draw_back(p);
                self.draw_front(p);
            }
        }
    }
}

impl State {
    fn erupt(&mut self) {
        let (r, ps, st) = (&mut self.r, &mut self.ps, &self.st);
        self.lights
            .add(0.0, -40.0, 90.0, rgb(255.0, 176.0, 96.0), 0.7, 0.6);
        self.lights.add(0.0, -8.0, 44.0, WHITE, 0.7, 0.1);
        for _ in 0..70 {
            let a = -PI / 2.0 + r.range(-1.1, 1.1);
            let sp = r.range(100.0, 260.0);
            let x = r.range(-4.0, 4.0);
            let life = r.range(0.25, 0.7);
            ps.spawn(
                r,
                Spawn::new(
                    Kind::Spark,
                    x,
                    -2.0,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    life,
                ),
            );
        }
        for _ in 0..18 {
            let b = Bit {
                kind: BitKind::Chunk,
                x: r.range(-6.0, 6.0),
                y: -2.0,
                vx: r.range(-110.0, 110.0),
                vy: -r.range(150.0, 330.0),
                g: 1.0,
                drag: 0.4,
                bounce: 0.3,
                life: r.range(1.6, 2.4),
                a: r.range(0.0, TAU),
                va: r.range(-16.0, 16.0),
                s: r.int(1.0, 3.0),
                c: WHITE,
                ramp: None,
            };
            self.bits.add(b);
        }
        for _ in 0..60 {
            let a = -PI / 2.0 + r.range(-0.9, 0.9);
            let sp = r.range(80.0, 260.0);
            let x = r.range(-5.0, 5.0);
            let life = r.range(1.0, 1.8);
            let col = r.pick(&[st.light, st.body, st.body, st.shade]).copied();
            let size = if r.chance(0.4) { 2.0 } else { 1.0 };
            ps.spawn(
                r,
                Spawn {
                    col,
                    size,
                    ..Spawn::new(
                        Kind::Grit,
                        x,
                        -1.0,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp,
                        life,
                    )
                },
            );
        }
        for _ in 0..14 {
            let sg = r.sign();
            let x = sg * r.range(2.0, 10.0);
            let y = -r.range(1.0, 4.0);
            let vx = sg * r.range(40.0, 110.0);
            let vy = -r.range(2.0, 10.0);
            let life = r.range(1.0, 1.8);
            let size = r.range(4.0, 7.0);
            let delay = r.range(0.0, 0.06);
            ps.spawn(
                r,
                Spawn {
                    kind: Kind::Smoke,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    col: Some(st.dust),
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
        }
    }
}
