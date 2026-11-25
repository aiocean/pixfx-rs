//! Meteor: port of pixelfx `src/fx/fire/meteor.js`.
//!
//! Beyond pixelfx: the strike fires impact cues ([`pixfx_core::cue::impact`]) for the game to answer; the `feel`
//! option scales them, 0 for none.
//!
//! The impact's particle recipe (fire puffs that cool into smoke, sparks, embers, burning brands that land and
//! keep burning, rocks, grit, dust, scorch) is adapted from Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{
    Bit, BitKind, Bits, Blazes, Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, band, flare,
};
use pixfx_core::{ColorOpt, Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, ramp, rgb};
use pixfx_draw::math::{
    Rng, TAU, clamp, ease, js_max, js_min, js_round, noise, rand, smooth, to_int32, to_uint32,
};

use super::lib::{
    Hot, LAYERS, OUTLINE, Stone, flame_blade, floor_light, hot_first, rock, sim_draw,
};

pub struct Meteor;

pub static METEOR: Meteor = Meteor;

const STONE: ColorOpt = ColorOpt::new("stone", Rgba::hex24(0x4a3f69));
const ROCK: RampOpt = RampOpt::new(
    "rock",
    &[
        Rgba::hex24(0x120e16),
        Rgba::hex24(0x2a2230),
        Rgba::hex24(0x463a4a),
        Rgba::hex24(0x6a5a66),
    ],
);
const META: Meta = Meta {
    id: "fire/meteor",
    name: "Meteor",
    desc: "A burning rock streaks in from the upper left in a comet of fire and smashes into the floor: a white flash, a fireball, rocks and brands thrown, a glowing crater that cools.",
    tags: &[
        "fire",
        "meteor",
        "impact",
        "explosion",
        "spell",
        "debris",
        "one-shot",
    ],
    size: (272, 232),
    origin: (150, 208),
    duration: 5.0,
    looping: false,
    fps: 30,
    seed: 9,
    state: None,
    options: &[STONE.spec(), ROCK.spec(), FEEL.spec()],
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
const HIT: f64 = 0.62;
const FROM: (f64, f64) = (-112.0, -142.0);
const SOOT: Rgba = Rgba::hex24(0x0d0a12);
const SOOT2: Rgba = Rgba::hex24(0x17121d);
const PI: f64 = core::f64::consts::PI;

impl Effect for Meteor {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the strike asks the game for a blast's answer.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(HIT, 1.0, 0.0, -6.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let h = hot_first(&opts.palette_or(&PALETTE));
        let ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: vec![h[1], h[2], h[4], h[5]],
            ..ParticlesOpts::default()
        });
        let len = libm::hypot(FROM.0, FROM.1);
        Box::new(State {
            h,
            rk: ramp(&opts.ramp(&ROCK)),
            st: Stone::of(opts.color(&STONE)),
            r: Rng::new(to_uint32(seed as f64)),
            s: seed as f64 * 11.0,
            ps,
            blazes: Blazes::new(),
            bits: Bits::new(0.0),
            lights: Lights::new(),
            ux: -FROM.0 / len,
            uy: -FROM.1 / len,
            t: 0.0,
            hit: false,
            last: pos(0.0),
        })
    }
}

// the flight: speeding up a touch as it falls
fn pos(tt: f64) -> (f64, f64) {
    let k = ease::in_quad(clamp(tt / HIT, 0.0, 1.0)) * 0.35 + clamp(tt / HIT, 0.0, 1.0) * 0.65;
    (FROM.0 * (1.0 - k), FROM.1 * (1.0 - k))
}

struct State {
    h: Hot,
    rk: [Rgba; 4],
    st: Stone,
    r: Rng,
    s: f64,
    ps: Particles,
    blazes: Blazes,
    /// Each piece's data: whether it has kicked up its dust.
    bits: Bits<bool>,
    lights: Lights,
    ux: f64,
    uy: f64,
    t: f64,
    hit: bool,
    last: (f64, f64),
}

impl State {
    fn glow(&self) -> Rgba {
        mix(mix(self.h[3], self.h[4], 0.5), WHITE, 0.25)
    }

    // the meteor: a lumpy rock, white-hot on its leading face, cracks glowing over the dark back, an outline
    fn meteor(&self, p: &mut Painter, x: f64, y: f64, r0: f64, cool: f64, alpha: f64) {
        let (h, rk, s, ux, uy) = (&self.h, &self.rk, self.s, self.ux, self.uy);
        p.shade(
            x - r0 - 2.0,
            y - r0 - 2.0,
            x + r0 + 2.0,
            y + r0 + 2.0,
            |px, py, out| {
                out.a = alpha;
                let (dx, dy) = (px - x, py - y);
                let d = libm::hypot(dx, dy);
                let a = libm::atan2(dy, dx);
                let rr =
                    r0 * (1.0 + 0.18 * noise(a * 2.2 + 3.0, s) + 0.1 * noise(a * 5.0, s + 1.0));
                if d > rr + 1.0 {
                    return None;
                }
                if d > rr {
                    return Some(OUTLINE);
                }
                let face = (dx * ux + dy * uy) / js_max(1.0, d);
                let heat = (1.0 - cool) * (face * 0.6 + 0.5 - (d / rr) * 0.25);
                if heat > 0.62 {
                    return Some(if heat > 0.85 {
                        h[0]
                    } else if heat > 0.74 {
                        h[1]
                    } else {
                        h[2]
                    });
                }
                if heat > 0.42 {
                    return Some(h[4]);
                }
                let hv = rand(&[libm::floor(px - x), libm::floor(py - y), s + 3.0]);
                if hv < 0.22 * (1.0 - cool * 0.7) {
                    return Some(if (1.0 - cool) > 0.5 { h[4] } else { h[6] });
                }
                Some(if dx * -0.6 + dy * -0.8 > 0.0 {
                    rk[2]
                } else if d / rr > 0.6 {
                    rk[0]
                } else {
                    rk[1]
                })
            },
        );
    }

    // the crater: hot rock pooled in the dent cooling to soot, a lip of broken floor
    fn crater(&self, p: &mut Painter) {
        let (t, h, s, st) = (self.t, &self.h, self.s, &self.st);
        if t < HIT {
            return;
        }
        let a = t - HIT;
        let end = 1.0 - smooth(DURATION - 1.0, DURATION - 0.15, t);
        let rx = 36.0 * js_min(1.0, a / 0.08);
        let ry = 6.0;
        for yi in -1..=3 {
            let y = f64::from(yi);
            let mut x = libm::floor(-rx);
            while x <= libm::ceil(rx) {
                let cx = x;
                x += 1.0;
                let (dx, dy) = ((cx + 0.5) / rx, (y + 0.5) / ry);
                let d = libm::sqrt(dx * dx + dy * dy);
                if d > 1.0 {
                    continue;
                }
                let k = rand(&[cx * 31.0 + y, s + 5.0]);
                let tv = libm::pow(clamp(1.0 - a / 2.6, 0.0, 1.0), 1.3) * (1.25 - d) - k * 0.35;
                if tv > 0.06 {
                    let i = js_min(8.0, libm::floor((1.0 - js_min(1.0, tv)) * 8.0));
                    let c = h.get(i as usize).copied().unwrap_or(Rgba::NONE);
                    p.paint_cell(cx, y, c, end);
                    if to_int32(cx) & 3 == 0 {
                        p.glow(cx + 0.5, y, 7.0, c, 0.2 * tv * end);
                    }
                } else {
                    p.paint_cell(
                        cx,
                        y,
                        if d < 0.55 { SOOT } else { SOOT2 },
                        (if d < 0.95 - k * 0.3 { 1.0 } else { 1.0 - d }) * end,
                    );
                }
            }
        }
        // the lip: broken floor heaped round the edge, lit by the glow inside
        for xi in -40..=40 {
            let x = f64::from(xi);
            let e = libm::fabs(x) / 40.0;
            let hh = if e < 0.7 {
                0.0
            } else {
                js_round(
                    2.4 * libm::sin(((e - 0.7) / 0.3) * PI) * (0.6 + 0.4 * rand(&[x, s + 7.0])),
                )
            };
            let mut j = 0.0;
            while j < hh {
                p.paint_cell(
                    x,
                    -1.0 - j,
                    if j == hh - 1.0 {
                        mix(st.light, h[4], 0.5 * clamp(1.0 - a / 2.0, 0.0, 1.0))
                    } else {
                        st.shade
                    },
                    end,
                );
                j += 1.0;
            }
            if hh != 0.0 {
                p.paint_cell(x, 0.0, OUTLINE, end);
            }
        }
    }

    fn ground_ring(&self, p: &mut Painter, back: bool) {
        let k = (self.t - HIT) / 0.5;
        if k > 0.0 && k < 1.0 {
            let st = &self.st;
            band(
                p,
                0.0,
                0.5,
                110.0 * ease::out_cubic(k),
                0.16,
                js_max(1.0, 5.0 * (1.0 - k)),
                1.15 - k,
                &[st.body, st.light, self.h[4], self.h[1]],
                |g| {
                    if back {
                        libm::sin(g) < 0.0
                    } else {
                        libm::sin(g) >= 0.0
                    }
                },
                !back && k < 0.12,
            );
        }
    }

    fn draw_back(&self, p: &mut Painter) {
        self.crater(p);
        sim_draw(p, &self.ps, &self.h, Layer::Back);
        self.blazes.draw_back(p);
        self.ground_ring(p, true);
    }

    fn draw_front(&self, p: &mut Painter) {
        let (t, h, s) = (self.t, &self.h, self.s);
        let glow = self.glow();
        self.ground_ring(p, false);
        for b in self.bits.list() {
            rock(p, &b.bit, &self.st);
        }
        if t < HIT {
            // the comet: tongues of flame streaming back off the rock, the rock, its light; the floor lights up
            let (x, y) = pos(t);
            let (ux, uy) = (self.ux, self.uy);
            let nz = |rate: f64, seed: f64, off: f64| noise(t * rate + off, seed);
            floor_light(
                p,
                0.0,
                0.5,
                30.0 + 40.0 * (t / HIT),
                7.0,
                glow,
                0.5 * libm::pow(t / HIT, 2.0),
            );
            let tails = [
                (36.0, 7.0, 0.0),
                (26.0, 7.5, -2.5),
                (26.0, 7.5, 2.5),
                (16.0, 8.0, 0.0),
            ];
            for (i, &(l, w, off)) in tails.iter().enumerate() {
                let n = (s + i as f64) * 13.0;
                flame_blade(
                    p,
                    x - uy * off,
                    y + ux * off,
                    -ux,
                    -uy,
                    l * (0.85 + 0.25 * nz(5.0, n, 0.0)),
                    w,
                    nz(3.0, n + 3.0, 0.0) * 3.0,
                    &nz,
                    n,
                    h,
                    &LAYERS,
                    0.5,
                    1.6,
                );
            }
            self.meteor(p, x, y, 6.5, 0.0, 1.0);
            p.glow(x, y, 46.0, glow, 0.55);
            p.glow(x, y, 12.0, h[1], 0.4);
        } else {
            // the core of the meteor sits half-buried in the crater, cooling
            let a = t - HIT;
            let cool = clamp(a / 2.8, 0.0, 1.0);
            let end = 1.0 - smooth(DURATION - 1.1, DURATION - 0.2, t);
            if end > 0.0 {
                self.meteor(p, 0.0, 0.0, 5.5, cool, end);
                p.glow(0.0, -2.0, 22.0, glow, 0.4 * (1.0 - cool) * end);
            }
        }
        sim_draw(p, &self.ps, h, Layer::Front);
        // the hero frame: a white-hot flash and a flare of rays
        let f = (t - HIT) / 0.08;
        if f > 0.0 && f < 1.0 {
            p.circle(0.0, -6.0, 11.0 * (1.0 - f * 0.3) + 1.5, h[3], 1.0);
            p.circle(
                0.0,
                -6.0,
                11.0 * (1.0 - f * 0.3),
                if f < 0.5 { h[1] } else { h[2] },
                1.0,
            );
            p.circle(
                0.0,
                -6.0,
                7.0 * (1.0 - f * 0.3),
                if f < 0.5 { WHITE } else { h[1] },
                1.0,
            );
        }
        let fr = (t - HIT) / 0.24;
        if fr > 0.0 && fr < 1.0 {
            flare(
                p,
                0.0,
                -6.0,
                44.0 * libm::pow(1.0 - fr, 1.4),
                1.0 - fr,
                &[h[6], h[4], h[2], h[1]],
            );
        }
        self.lights.draw(p);
    }

    fn fly(&mut self, dt: f64) {
        let (t, h, ux, uy) = (self.t, self.h, self.ux, self.uy);
        let (r, ps) = (&mut self.r, &mut self.ps);
        // the flight: a comet of fire shed behind, smoke, sparks, embers
        let (x, y) = pos(t);
        let (vx, vy) = ((x - self.last.0) / dt, (y - self.last.1) / dt);
        self.last = (x, y);
        for _ in 0..3 {
            if !r.chance(0.8 * dt * 60.0) {
                continue;
            }
            let fx = x - ux * r.range(2.0, 8.0) + r.range(-3.0, 3.0);
            let fy = y - uy * r.range(2.0, 8.0) + r.range(-3.0, 3.0);
            let fvx = vx * 0.15 + r.range(-20.0, 20.0);
            let fvy = vy * 0.15 + r.range(-30.0, 10.0);
            let life = r.range(0.3, 0.6);
            let size = r.range(3.0, 6.0);
            let flags = if r.chance(0.65) {
                Flags::SMOKES | Flags::BIG
            } else {
                Flags::NONE
            };
            ps.spawn(
                r,
                Spawn {
                    col: None,
                    size,
                    flags,
                    ..Spawn::new(Kind::Fire, fx, fy, fvx, fvy, life)
                },
            );
        }
        if r.chance(0.6 * dt * 60.0) {
            let svx = vx * 0.35 + r.range(-80.0, 80.0);
            let svy = vy * 0.35 + r.range(-80.0, 40.0);
            let life = r.range(0.2, 0.5);
            ps.spawn(r, Spawn::new(Kind::Spark, x, y, svx, svy, life));
        }
        if r.chance(0.3 * dt * 60.0) {
            let evx = vx * 0.1 + r.range(-30.0, 30.0);
            let evy = vy * 0.1 + r.range(-40.0, 0.0);
            let life = r.range(0.8, 1.6);
            let size = if r.chance(0.2) { 2.0 } else { 1.0 };
            ps.spawn(
                r,
                Spawn {
                    col: Some(h[3]),
                    size,
                    ..Spawn::new(Kind::Ember, x, y, evx, evy, life)
                },
            );
        }
    }

    fn strike(&mut self) {
        let h = self.h;
        let (r, ps, st) = (&mut self.r, &mut self.ps, &self.st);
        // the impact: flash, fireball thrown up and forward with the rock's momentum, a column, smoke, sparks
        self.lights
            .add(0.0, -40.0, 100.0, rgb(255.0, 176.0, 96.0), 0.95, 0.9);
        self.lights.add(0.0, -20.0, 46.0, WHITE, 0.9, 0.12);
        let fire = |r: &mut Rng, ps: &mut Particles, x, y, vx, vy, life, size, delay, flags| {
            ps.spawn(
                r,
                Spawn {
                    kind: Kind::Fire,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    col: None,
                    size,
                    delay,
                    flags,
                },
            );
        };
        let smokes_big = |r: &mut Rng, p: f64| {
            if r.chance(p) {
                Flags::SMOKES | Flags::BIG
            } else {
                Flags::NONE
            }
        };
        for _ in 0..34 {
            let a = -PI / 2.0 + r.range(-1.3, 1.3) + 0.25;
            let sp = r.range(50.0, 210.0);
            let x = r.range(-6.0, 6.0);
            let y = -r.range(2.0, 10.0);
            let life = r.range(0.5, 1.1);
            let size = r.range(5.0, 9.0);
            let flags = smokes_big(r, 0.5);
            let (vx, vy) = (libm::cos(a) * sp, libm::sin(a) * sp * 0.75 - 30.0);
            fire(r, ps, x, y, vx, vy, life, size, 0.0, flags);
        }
        for _ in 0..9 {
            let x = r.range(-6.0, 6.0);
            let vx = r.range(-14.0, 14.0);
            let vy = -r.range(130.0, 220.0);
            let life = r.range(0.8, 1.2);
            let size = r.range(6.0, 8.5);
            let delay = r.range(0.0, 0.12);
            fire(
                r,
                ps,
                x,
                -6.0,
                vx,
                vy,
                life,
                size,
                delay,
                Flags::SMOKES | Flags::BIG,
            );
        }
        for _ in 0..20 {
            let a = r.range(-PI, 0.0);
            let d = r.range(4.0, 18.0);
            let y = -r.range(6.0, 24.0);
            let vx = libm::cos(a) * r.range(10.0, 40.0);
            let vy = -r.range(30.0, 110.0);
            let life = r.range(1.8, 2.8);
            let size = r.range(8.0, 12.0);
            let delay = r.range(0.15, 0.5);
            ps.spawn(
                r,
                Spawn {
                    kind: Kind::Smoke,
                    x: libm::cos(a) * d,
                    y,
                    vx,
                    vy,
                    life,
                    col: None,
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
        }
        for _ in 0..16 {
            let sg = r.sign();
            let x = sg * r.range(4.0, 18.0);
            let y = -r.range(1.0, 5.0);
            let vx = sg * r.range(60.0, 130.0) + 10.0;
            let vy = -r.range(2.0, 12.0);
            let life = r.range(1.2, 2.0);
            let size = r.range(5.0, 8.0);
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
        for _ in 0..90 {
            let a = -PI / 2.0 + r.range(-1.3, 1.3) + 0.15;
            let sp = r.range(100.0, 300.0);
            let life = r.range(0.2, 0.7);
            ps.spawn(
                r,
                Spawn::new(
                    Kind::Spark,
                    0.0,
                    -4.0,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    life,
                ),
            );
        }
        for _ in 0..36 {
            let a = r.range(0.0, TAU);
            let sp = r.range(40.0, 150.0);
            let life = r.range(1.2, 2.6);
            ps.spawn(
                r,
                Spawn {
                    col: Some(h[3]),
                    ..Spawn::new(
                        Kind::Ember,
                        libm::cos(a) * 8.0,
                        -8.0 + libm::sin(a) * 6.0,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp - 70.0,
                        life,
                    )
                },
            );
        }
        for _ in 0..10 {
            let a = -PI / 2.0 + r.range(-1.0, 1.05);
            let sp = r.range(130.0, 240.0);
            let life = r.range(0.7, 1.3);
            ps.spawn(
                r,
                Spawn::new(
                    Kind::Brand,
                    0.0,
                    -6.0,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    life,
                ),
            );
        }
        let chunk = vec![
            OUTLINE,
            st.shade,
            st.body,
            st.light,
            mix(st.light, WHITE, 0.3),
        ];
        for _ in 0..28 {
            let b = Bit {
                kind: BitKind::Chunk,
                x: r.range(-8.0, 8.0),
                y: -3.0,
                vx: r.range(-80.0, 110.0),
                vy: -r.range(140.0, 340.0),
                g: 1.0,
                drag: 0.4,
                bounce: 0.3,
                life: r.range(1.8, 2.8),
                a: r.range(0.0, TAU),
                va: r.range(-16.0, 16.0),
                s: r.int(1.0, 3.0),
                ramp: Some(chunk.clone()),
                c: WHITE,
            };
            self.bits.add(b);
        }
        for _ in 0..80 {
            let u = r.range(-1.0, 1.0);
            let a = -PI / 2.0 + u * 0.7 + 0.15;
            let sp = r.range(90.0, 270.0) * (1.0 - 0.4 * libm::fabs(u));
            let life = r.range(1.2, 2.2);
            let col = r.pick(&[st.light, st.body, st.body, st.shade]).copied();
            let size = if r.chance(0.45) { 2.0 } else { 1.0 };
            ps.spawn(
                r,
                Spawn {
                    col,
                    size,
                    ..Spawn::new(
                        Kind::Grit,
                        u * 6.0,
                        -1.0,
                        libm::cos(a) * sp,
                        libm::sin(a) * sp,
                        life,
                    )
                },
            );
        }
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        if self.t < HIT {
            self.fly(dt);
        } else if !self.hit {
            self.hit = true;
            self.strike();
        }
        self.bits.update(dt);
        let (r, ps) = (&mut self.r, &mut self.ps);
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
        self.blazes.update(ps, r, dt);
        let blazes = &mut self.blazes;
        // a brand that lands starts a little fire where it lands
        ps.update_with(r, dt, |r, x, y| {
            let life = r.range(0.9, 1.6);
            let power = r.range(0.5, 0.85);
            blazes.add(r, x, y - 0.25, life, power);
        });
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
