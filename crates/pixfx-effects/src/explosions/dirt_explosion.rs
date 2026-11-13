//! Dirt explosion: port of pixelfx `src/fx/explosions/dirt-explosion.js`.
//!
//! Beyond pixelfx: it fires impact cues ([`pixfx_core::cue::impact`]) for the game to answer; the `feel` option
//! scales them, 0 for none.
//!
//! The earth recipe (grit thrown in a fan, rocks that bounce and kick up dust, a dust column and dust rolling out
//! along the floor) is adapted from Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.

use core::f64::consts::FRAC_PI_2;

use pixfx_core::color::{Rgba, WHITE, mix, ramp, rgb};
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::math::{Rng, TAU, ease, js_max, js_min, js_round, smooth, to_uint32};
use pixfx_core::sim::{
    Bit, BitKind, Bits, Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, band,
};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Painter, RampOpt};

use super::lib::{Stone, crater, cull, dome, draw_dirt, fade_edges, pool, rock};

pub struct DirtExplosion;

pub static DIRT_EXPLOSION: DirtExplosion = DirtExplosion;

const META: Meta = Meta {
    id: "explosions/dirt-explosion",
    name: "Dirt explosion",
    desc: "A buried charge goes off: a flash under the floor, a V of dirt clods and grit thrown high, a brown dust plume that billows and settles, and a crater left behind.",
    tags: &[
        "explosion",
        "dirt",
        "earth",
        "dust",
        "debris",
        "ground",
        "one-shot",
    ],
    size: (260, 240),
    origin: (130, 206),
    duration: 3.6,
    looping: false,
    fps: 30,
    seed: 3,
    state: None,
    options: &[FIRE.spec(), FEEL.spec()],
};

// meta.options
const PALETTE: [Rgba; 7] = [
    Rgba::hex24(0x1c1610),
    Rgba::hex24(0x33281c),
    Rgba::hex24(0x4f3f2c),
    Rgba::hex24(0x6e5a40),
    Rgba::hex24(0x927a58),
    Rgba::hex24(0xb89f78),
    Rgba::hex24(0xdcc8a2),
];
const FIRE: RampOpt = RampOpt::new(
    "fire",
    &[
        Rgba::hex24(0x4a1712),
        Rgba::hex24(0xc9281f),
        Rgba::hex24(0xf2561d),
        Rgba::hex24(0xff8a2a),
        Rgba::hex24(0xffc23d),
        Rgba::hex24(0xffe26b),
        Rgba::hex24(0xfff3a6),
        Rgba::hex24(0xffffff),
    ],
);

const D: f64 = META.duration;
// the canvas box in local px, 3 px in from each edge
const X0: f64 = -(META.origin.0 as f64) + 3.0;
const X1: f64 = META.size.0 as f64 - META.origin.0 as f64 - 3.0;
const Y0: f64 = -(META.origin.1 as f64) + 3.0;
const Y1: f64 = META.size.1 as f64 - META.origin.1 as f64 - 3.0;

/// What the effect keeps on each clod: its vy before the step, and whether its landing already thudded.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Clod {
    pvy: f64,
    thud: bool,
}

impl Effect for DirtExplosion {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the charge going off asks the game for an impact's answer (shake, hit-stop, ring, flash by weight).
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(0.0, 0.8, 0.0, -8.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let e: [Rgba; 7] = ramp(&opts.palette_or(&PALETTE));
        let f: [Rgba; 8] = ramp(&opts.ramp(&FIRE));
        let mut r = Rng::new(to_uint32(seed as f64));
        let clod = Stone {
            light: e[5],
            body: e[3],
            shade: e[2],
        };
        let dust = mix(e[4], rgb(150.0, 140.0, 150.0), 0.2);
        let soil = mix(e[3], rgb(90.0, 80.0, 90.0), 0.2);
        let dark = mix(e[2], rgb(60.0, 52.0, 60.0), 0.3);
        let mut ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: vec![f[6], f[5], f[3], f[2]],
            ..ParticlesOpts::default()
        });
        let mut lights = Lights::new();
        let mut clods: Bits<Clod> = Bits::new(0.0);

        lights.add(0.0, -10.0, 56.0, f[4], 0.7, 0.22);
        lights.add(0.0, -4.0, 26.0, WHITE, 0.6, 0.06);
        // the charge: a few sparks out of the hole
        for _ in 0..14 {
            let a = -FRAC_PI_2 + r.range(-0.7, 0.7);
            let sp = r.range(160.0, 320.0);
            let life = r.range(0.15, 0.35);
            ps.spawn(
                &mut r,
                Spawn::new(
                    Kind::Spark,
                    0.0,
                    -2.0,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    life,
                ),
            );
        }
        // the fountain: grit in a V, clods thrown high, a few pebbles
        for _ in 0..300 {
            let side = r.sign();
            let a = -FRAC_PI_2 + side * r.range(0.05, 0.62);
            let sp = r.range(150.0, 470.0) * (1.0 - 0.3 * libm::fabs(a + FRAC_PI_2));
            let x = r.range(-4.0, 4.0);
            let life = r.range(1.2, 2.2);
            let col = r.pick(&[e[1], e[2], e[2], e[3], e[3], e[4], e[5]]).copied();
            let size = if r.chance(0.45) { 2.0 } else { 1.0 };
            let delay = r.range(0.0, 0.06);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Grit,
                    x,
                    y: -1.0,
                    vx: libm::cos(a) * sp,
                    vy: libm::sin(a) * sp,
                    life,
                    col,
                    size,
                    delay,
                    flags: Flags::NONE,
                },
            );
        }
        for i in 0..30 {
            let side = r.sign();
            let a = -FRAC_PI_2 + side * r.range(0.1, 0.6);
            let sp = r.range(200.0, 430.0);
            let x = r.range(-5.0, 5.0);
            let life = r.range(2.2, 3.0);
            let ang = r.range(0.0, TAU);
            let va = r.range(-12.0, 12.0);
            let s = if i < 12 {
                r.range(2.6, 3.8)
            } else {
                r.range(1.4, 2.4)
            };
            clods.add(Bit {
                vx: libm::cos(a) * sp,
                vy: libm::sin(a) * sp,
                g: 1.0,
                drag: 0.35,
                bounce: 0.25,
                life,
                a: ang,
                va,
                s,
                ..Bit::new(BitKind::Chunk, x, -2.0)
            });
        }
        // the dust: a plume punched straight up, two lobes riding the V, and a skirt rolling out along the floor
        for _ in 0..22 {
            let x = r.range(-7.0, 7.0);
            let y = -r.range(2.0, 12.0);
            let vx = r.range(-25.0, 25.0);
            let vy = -r.range(70.0, 180.0);
            let life = r.range(1.8, 2.6);
            let col = if r.chance(0.6) { soil } else { dust };
            let size = r.range(9.0, 14.0);
            let delay = r.range(0.0, 0.14);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Smoke,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    col: Some(col),
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
        }
        for _ in 0..14 {
            let side = r.sign();
            let a = -FRAC_PI_2 + side * r.range(0.35, 0.6);
            let sp = r.range(110.0, 210.0);
            let x = side * r.range(2.0, 8.0);
            let y = -r.range(2.0, 8.0);
            let life = r.range(1.6, 2.4);
            let col = if r.chance(0.5) { soil } else { dust };
            let size = r.range(8.0, 12.0);
            let delay = r.range(0.02, 0.1);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Smoke,
                    x,
                    y,
                    vx: libm::cos(a) * sp,
                    vy: libm::sin(a) * sp,
                    life,
                    col: Some(col),
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
        }
        for _ in 0..16 {
            let s = r.sign();
            let x = s * r.range(4.0, 16.0);
            let y = -r.range(1.0, 5.0);
            let vx = s * r.range(70.0, 200.0);
            let vy = -r.range(2.0, 12.0);
            let life = r.range(1.4, 2.2);
            let size = r.range(6.0, 9.0);
            let delay = r.range(0.02, 0.1);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Smoke,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    col: Some(dust),
                    size,
                    delay,
                    flags: Flags::BIG,
                },
            );
        }
        // dark soil thrown up with it, heavier and lower
        for _ in 0..8 {
            let x = r.range(-8.0, 8.0);
            let y = -r.range(2.0, 8.0);
            let vx = r.range(-40.0, 40.0);
            let vy = -r.range(40.0, 110.0);
            let life = r.range(1.2, 1.8);
            let size = r.range(6.0, 9.0);
            let delay = r.range(0.0, 0.05);
            ps.spawn(
                &mut r,
                Spawn {
                    kind: Kind::Smoke,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    col: Some(dark),
                    size,
                    delay,
                    flags: Flags::NONE,
                },
            );
        }
        Box::new(State {
            e,
            f,
            clod,
            dust,
            r,
            ps,
            lights,
            clods,
            t: 0.0,
        })
    }
}

struct State {
    e: [Rgba; 7],
    f: [Rgba; 8],
    clod: Stone,
    dust: Rgba,
    r: Rng,
    ps: Particles,
    lights: Lights,
    clods: Bits<Clod>,
    t: f64,
}

impl State {
    /// The shock band rolling out along the floor: its far half (`far`) behind, its near half in front.
    fn shock(&self, p: &mut Painter, far: bool) {
        let e = &self.e;
        let k = self.t / 0.45;
        if k < 1.0 {
            band(
                p,
                0.0,
                0.5,
                90.0 * ease::out_cubic(k),
                0.16,
                js_max(1.0, 3.0 * (1.0 - k)),
                1.1 - k,
                &[e[2], e[4], e[5], e[6]],
                |g| (libm::sin(g) < 0.0) == far,
                false,
            );
        }
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        for b in self.clods.list_mut() {
            b.data.pvy = b.bit.vy;
        }
        self.clods.update(dt);
        // clods thud down: a puff of dust and a spray of crumbs where they land, the big ones break up
        let (r, ps, e, dust) = (&mut self.r, &mut self.ps, &self.e, self.dust);
        let list = self.clods.list_mut();
        for i in (0..list.len()).rev() {
            let Some(b) = list.get_mut(i) else {
                continue;
            };
            if b.bit.x < X0 + 4.0 || b.bit.x > X1 - 4.0 {
                list.remove(i);
                continue;
            }
            let landed = (b.data.pvy > 0.0 && b.bit.vy <= 0.0) || (b.rest && !b.data.thud);
            if !landed || b.data.pvy < 80.0 {
                if b.rest {
                    b.data.thud = true;
                }
                continue;
            }
            b.data.thud = b.rest;
            let mut j = 0.0;
            while j < 2.0 + js_round(b.bit.s) {
                let x = b.bit.x + r.range(-2.0, 2.0);
                let vx = r.sign() * r.range(15.0, 55.0);
                let vy = -r.range(4.0, 22.0);
                let life = r.range(0.35, 0.7);
                let size = r.range(1.6, 1.2 + b.bit.s);
                ps.spawn(
                    r,
                    Spawn {
                        col: Some(dust),
                        size,
                        ..Spawn::new(Kind::Dust, x, -0.5, vx, vy, life)
                    },
                );
                j += 1.0;
            }
            let mut j = 0.0;
            while j < js_round(b.bit.s * 3.0) {
                let vx = r.range(-70.0, 70.0);
                let vy = -r.range(40.0, 140.0);
                let life = r.range(0.8, 1.4);
                let col = r.pick(&[e[2], e[3], e[4]]).copied();
                ps.spawn(
                    r,
                    Spawn {
                        col,
                        ..Spawn::new(Kind::Grit, b.bit.x, -1.0, vx, vy, life)
                    },
                );
                j += 1.0;
            }
            if b.bit.s > 2.2 && b.data.pvy > 200.0 {
                b.bit.s *= 0.6;
            }
        }
        ps.update(r, dt);
        cull(ps, X0, Y0, X1, Y1);
        fade_edges(ps, X0, X1, 24.0, Y1);
        self.lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let t = self.t;
        if layer == Layer::Back {
            crater(
                p,
                &self.e,
                0.0,
                28.0,
                ease::out_cubic(js_min(1.0, t / 0.12)),
                1.0 - smooth(D - 0.7, D - 0.1, t),
            );
            self.shock(p, true);
            draw_dirt(p, &self.ps, Layer::Back);
            return;
        }
        self.shock(p, false);
        // the flash out of the ground: a squat dome of white-hot air for a frame or two
        if t < 0.09 {
            dome(p, &self.f, 0.0, 15.0 * (1.0 - t * 3.0), 1.0 - t / 0.09);
        }
        for b in self.clods.list() {
            rock(p, &b.bit, &self.clod, &self.f, 0.0);
        }
        draw_dirt(p, &self.ps, Layer::Front);
        pool(
            p,
            0.0,
            1.0,
            60.0,
            10.0,
            self.f[3],
            0.5 * js_max(0.0, 1.0 - t / 0.3),
        );
        self.lights.draw(p);
    }
}
