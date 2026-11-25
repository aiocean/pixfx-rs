//! Fire ring: port of pixelfx `src/fx/fire/fire-ring.js`.
//!
//! pixelfx's `act` (the caster's poses, the slam, the scorched target) is gallery staging and is dropped. Beyond
//! pixelfx: the slam that sets the ring off fires impact cues ([`pixfx_core::cue::impact`]) for the game to answer,
//! weighted as the act's hit (a 0.1 s stop and a shake); the `feel` option scales them, 0 for none.
//!
//! The flame-puff, smoke and spark recipe and the floor ring are adapted from Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), and the tongue from its fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{Flags, Kind, Lights, Particles, ParticlesOpts, Spawn, band};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix, rgb};
use pixfx_draw::math::{
    Rng, TAU, clamp, ease, js_max, js_min, js_round, noise, rand, smooth, to_uint32,
};

use super::lib::{BACK_LAYERS, Hot, LAYERS, Layers, flame_tongue, hot_first, sim_draw};

pub struct FireRing;

pub static FIRE_RING: FireRing = FireRing;

// the rings of flames reach this far: held to what lays out in reasonable time
const REACH: NumOpt = NumOpt::new("reach", 92.0).within(0.0, 400.0);
const META: Meta = Meta {
    id: "fire/fire-ring",
    name: "Fire ring",
    desc: "A white flash at the feet, then flames race outward along the floor in a ring, flaring as the front passes and burning down behind it over a smouldering scorch.",
    tags: &["fire", "ring", "nova", "aoe", "spell", "ground", "one-shot"],
    size: (224, 128),
    origin: (112, 92),
    duration: 3.9,
    looping: false,
    fps: 30,
    seed: 6,
    state: None,
    options: &[REACH.spec(), FEEL.spec()],
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

const DURATION: f64 = 3.9;
const SY: f64 = 0.2;
const CHARGE: f64 = 0.32;
const SPREAD: f64 = 0.75;
const SOOT: Rgba = Rgba::hex24(0x0d0a12);
const SOOT2: Rgba = Rgba::hex24(0x17121d);
// the dying flames: deep orange and red, no white heart
const EMBER_LAYERS: [(f64, f64, usize); 3] = [(1.0, 1.0, 7), (0.7, 0.8, 6), (0.4, 0.55, 5)];

impl Effect for FireRing {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the slam at the feet asks the game for an impact's answer (pixelfx's act hit: a 0.1 s
    /// stop and a shake).
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(CHARGE, 0.6, 0.0, -2.0, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let h = hot_first(&opts.palette_or(&PALETTE));
        let reach = opts.num(&REACH);
        let mut r = Rng::new(to_uint32(seed as f64));
        // the flames: rings of tongues on the floor, each lit as the front reaches it, flaring then burning down
        let mut sites = Vec::new();
        let mut rr0 = 10.0;
        while rr0 <= reach {
            let per = TAU * libm::sqrt((rr0 * rr0 + libm::pow(rr0 * SY, 2.0)) / 2.0);
            let n = js_max(5.0, js_round(per / 8.5));
            let mut k = 0.0;
            while k < n {
                let a = ((k + r.range(0.15, 0.85)) / n) * TAU;
                let rr = rr0 + r.range(-3.0, 3.0);
                let q = js_min(1.0, rr / reach);
                // near the middle they flash and go out as the front passes; the rim, where it stops, burns on
                let t0 = reached_at(rr, reach) + r.range(0.0, 0.04);
                let life = (0.3 + 1.0 * q * q) * r.range(0.85, 1.2);
                let hh = r.range(9.0, 15.0) * (0.6 + 0.55 * q);
                let w = r.range(1.8, 2.7) * (0.75 + 0.3 * q);
                let n_ = r.int(0.0, 1e5);
                sites.push(Site {
                    x: libm::cos(a) * rr,
                    y: libm::sin(a) * rr * SY,
                    back: libm::sin(a) < 0.0,
                    t0,
                    life,
                    h: hh,
                    w,
                    n: n_,
                });
                k += 1.0;
            }
            rr0 += 9.0;
        }
        sites.sort_by(|a, b| a.y.total_cmp(&b.y));
        let ps = Particles::new(ParticlesOpts {
            ground: 0.0,
            sparks: vec![h[1], h[2], h[4], h[5]],
            ..ParticlesOpts::default()
        });
        Box::new(State {
            h,
            reach,
            s: seed as f64 * 7.0,
            r,
            ps,
            lights: Lights::new(),
            sites,
            t: 0.0,
            burst: false,
        })
    }
}

fn front(t: f64, reach: f64) -> f64 {
    reach * ease::out_cubic(clamp((t - CHARGE) / SPREAD, 0.0, 1.0))
}

fn reached_at(d: f64, reach: f64) -> f64 {
    CHARGE + SPREAD * (1.0 - libm::cbrt(1.0 - js_min(1.0, d / reach)))
}

struct Site {
    x: f64,
    y: f64,
    back: bool,
    t0: f64,
    life: f64,
    h: f64,
    w: f64,
    n: f64,
}

struct State {
    h: Hot,
    reach: f64,
    s: f64,
    r: Rng,
    ps: Particles,
    lights: Lights,
    sites: Vec<Site>,
    t: f64,
    burst: bool,
}

impl State {
    fn glow(&self) -> Rgba {
        mix(mix(self.h[3], self.h[4], 0.5), WHITE, 0.25)
    }

    fn flames(&self, p: &mut Painter, back: bool) {
        let (t, h) = (self.t, &self.h);
        let nz = |rate: f64, seed: f64, off: f64| noise(t * rate + off, seed);
        for s in &self.sites {
            if s.back != back {
                continue;
            }
            let age = t - s.t0;
            if age < 0.0 || age > s.life {
                continue;
            }
            let u = age / s.life;
            let rise = ease::out_back(clamp(age / 0.08, 0.0, 1.0), 2.0);
            let fade = 1.0 - libm::pow(js_max(0.0, (u - 0.1) / 0.9), 1.3);
            let hh = s.h
                * rise
                * fade
                * (0.85 + 0.15 * noise(t * 3.3, s.n))
                * (1.0 + 0.5 * js_max(0.0, noise(t * 2.6, s.n + 70.0) - 0.55));
            if hh < 1.0 {
                continue;
            }
            // fresh flames white-hot, then cooling as they burn down; the far ones a step dimmer
            let layers: &Layers = if u < 0.3 {
                if back { &BACK_LAYERS } else { &LAYERS }
            } else if u < 0.65 {
                &BACK_LAYERS
            } else {
                &EMBER_LAYERS
            };
            flame_tongue(
                p,
                s.x,
                s.y,
                hh,
                s.w * (0.6 + 0.4 * fade),
                nz(2.2, s.n + 40.0, 0.0) * 1.5 - s.x * 0.02,
                &nz,
                s.n,
                h,
                layers,
                0.5,
                1.1,
            );
        }
    }

    // the scorch: a translucent stain of soot where the fire has crossed, sparse embers cooling in it
    fn scorch(&self, p: &mut Painter) {
        let (t, h, reach) = (self.t, &self.h, self.reach);
        let fr = front(t, reach);
        let end = 1.0 - smooth(DURATION - 1.2, DURATION - 0.15, t);
        if fr < 1.0 || end <= 0.0 {
            return;
        }
        let mut y = libm::floor(-fr * SY);
        while y <= libm::ceil(fr * SY) {
            let mut x = libm::floor(-fr);
            while x <= libm::ceil(fr) {
                let (cx, cy) = (x, y);
                x += 1.0;
                let d = libm::hypot(cx + 0.5, (cy + 0.5) / SY);
                if d > fr {
                    continue;
                }
                let since = t - reached_at(d, reach);
                let hv = rand(&[cx, cy, self.s]);
                let hot = 1.0 - since / (1.2 + hv * 0.9);
                if hot > 0.0 && hv < 0.03 + 0.06 * hot {
                    p.paint_cell(
                        cx,
                        cy,
                        if hot > 0.7 {
                            h[3]
                        } else if hot > 0.45 {
                            h[4]
                        } else if hot > 0.2 {
                            h[5]
                        } else {
                            h[6]
                        },
                        1.0,
                    );
                    p.light(cx, cy, h[4], 0.3 * hot);
                    continue;
                }
                // the whole game cell, so a denser raster shows no gaps
                let (c, a) = (
                    if hv < 0.5 { SOOT } else { SOOT2 },
                    js_min(1.0, since * 3.0) * (0.55 - (d / reach) * 0.25) * end,
                );
                p.each_game_cell(cx, cy, |p, sx, sy| p.blend_px(sx, sy, c, a));
            }
            y += 1.0;
        }
    }

    fn ring(&self, p: &mut Painter, back: bool) {
        let t = self.t;
        let k = (t - CHARGE) / (SPREAD + 0.1);
        if !(0.0..1.0).contains(&k) {
            return;
        }
        let h = &self.h;
        band(
            p,
            0.0,
            0.0,
            front(t, self.reach) + 1.5,
            SY,
            js_max(1.5, 6.0 * (1.0 - k)),
            if k < 0.75 { 1.0 } else { (1.0 - k) / 0.25 },
            &[h[6], h[4], h[2], h[1]],
            |g| (libm::sin(g) < 0.0) == back,
            k < 0.12,
        );
    }

    fn draw_back(&self, p: &mut Painter) {
        let t = self.t;
        self.scorch(p);
        self.ring(p, true);
        self.flames(p, true);
        sim_draw(p, &self.ps, &self.h, Layer::Back);
        // light on the floor running with the front
        let fr = front(t, self.reach);
        let k = if t < CHARGE {
            0.0
        } else {
            1.0 - smooth(CHARGE + SPREAD, CHARGE + SPREAD + 1.4, t)
        };
        if k > 0.0 {
            let glow = self.glow();
            for i in 0..16 {
                let a = (f64::from(i) / 16.0) * TAU;
                p.glow(
                    libm::cos(a) * fr,
                    libm::sin(a) * fr * SY - 3.0,
                    16.0,
                    glow,
                    0.2 * k,
                );
            }
        }
    }

    fn draw_front(&self, p: &mut Painter) {
        let (t, h) = (self.t, &self.h);
        let glow = self.glow();
        self.ring(p, false);
        self.flames(p, false);
        sim_draw(p, &self.ps, h, Layer::Front);
        // the charge glowing at the feet, then the white flash as it goes
        if t < CHARGE {
            p.glow(
                0.0,
                -3.0,
                8.0 + 22.0 * (t / CHARGE),
                glow,
                0.6 * (t / CHARGE),
            );
        }
        if (CHARGE..CHARGE + 0.1).contains(&t) {
            let k = 1.0 - (t - CHARGE) / 0.1;
            p.ellipse(0.0, -1.0, 14.0 * (0.6 + 0.4 * k), 4.0, h[2], 1.0);
            p.ellipse(
                0.0,
                -1.0,
                9.0 * (0.6 + 0.4 * k),
                2.6,
                if k > 0.5 { WHITE } else { h[1] },
                1.0,
            );
        }
        let k = smooth(CHARGE - 0.05, CHARGE, t)
            * (1.0 - smooth(CHARGE + SPREAD * 0.5, CHARGE + SPREAD + 1.2, t));
        if k > 0.0 {
            p.glow(0.0, -6.0, 92.0, glow, 0.3 * k);
        }
        self.lights.draw(p);
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let h = self.h;
        let r = &mut self.r;
        let ps = &mut self.ps;
        // the charge: streaks of fire spiral in to the feet, heat building there
        if t < CHARGE - 0.05 && r.chance(0.9 * dt * 60.0) {
            let a = r.range(0.0, TAU);
            let d = r.range(26.0, 54.0);
            let tt = r.range(0.18, 0.26);
            let v = (d * 3.0) / (libm::exp(3.0 * tt) - 1.0);
            let x = libm::cos(a) * d;
            let y = js_min(-2.0, -6.0 + libm::sin(a) * d * 0.45);
            let l = pixfx_draw::math::or_one(libm::hypot(x, y + 4.0));
            let col = if r.chance(0.5) { h[2] } else { h[3] };
            ps.spawn(
                r,
                Spawn {
                    col: Some(col),
                    ..Spawn::new(Kind::Streak, x, y, (-x / l) * v, ((-4.0 - y) / l) * v, tt)
                },
            );
        }
        if t < CHARGE && r.chance(0.5 * dt * 60.0 * (t / CHARGE)) {
            let x = r.range(-4.0, 4.0);
            let vx = r.range(-8.0, 8.0);
            let vy = -r.range(20.0, 40.0);
            let life = r.range(0.12, 0.24);
            let size = r.range(1.0, 2.2);
            ps.spawn(
                r,
                Spawn {
                    col: None,
                    size,
                    ..Spawn::new(Kind::Fire, x, -1.0, vx, vy, life)
                },
            );
        }
        if !self.burst && t >= CHARGE {
            self.burst = true;
            self.lights
                .add(0.0, -18.0, 64.0, rgb(255.0, 176.0, 96.0), 0.85, 0.5);
            self.lights.add(0.0, -6.0, 30.0, WHITE, 0.8, 0.08);
            for _ in 0..40 {
                let a = r.range(0.0, TAU);
                let sp = r.range(80.0, 220.0);
                let vy = libm::sin(a) * sp * 0.4 - r.range(40.0, 120.0);
                let life = r.range(0.2, 0.5);
                ps.spawn(
                    r,
                    Spawn::new(
                        Kind::Spark,
                        libm::cos(a) * 3.0,
                        -2.0,
                        libm::cos(a) * sp,
                        vy,
                        life,
                    ),
                );
            }
        }
        // fire shed off the burning ring: puffs that rise and some cool into smoke, and embers
        if t > CHARGE && t < CHARGE + SPREAD + 1.2 {
            for _ in 0..3 {
                if !r.chance(0.75 * dt * 60.0) {
                    continue;
                }
                let Some(s) = r.pick(&self.sites) else {
                    continue;
                };
                let age = t - s.t0;
                if age < 0.0 || age > s.life * 0.7 {
                    continue;
                }
                let back = if s.back { Flags::BACK } else { Flags::NONE };
                let x = s.x + r.range(-1.0, 1.0);
                let vx = r.range(-5.0, 5.0);
                let vy = -r.range(20.0, 40.0);
                let life = r.range(0.2, 0.4);
                let size = r.range(1.2, 2.4);
                let flags = if r.chance(0.5) {
                    Flags::SMOKES | back
                } else {
                    back
                };
                ps.spawn(
                    r,
                    Spawn {
                        kind: Kind::Fire,
                        x,
                        y: s.y - s.h * 0.6,
                        vx,
                        vy,
                        life,
                        col: None,
                        size,
                        delay: 0.0,
                        flags,
                    },
                );
                if r.chance(0.12) {
                    let vx = r.range(-15.0, 15.0);
                    let vy = -r.range(30.0, 70.0);
                    let life = r.range(0.6, 1.3);
                    ps.spawn(
                        r,
                        Spawn {
                            kind: Kind::Ember,
                            x: s.x,
                            y: s.y - s.h * 0.5,
                            vx,
                            vy,
                            life,
                            col: Some(h[3]),
                            size: 1.0,
                            delay: 0.0,
                            flags: back,
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
