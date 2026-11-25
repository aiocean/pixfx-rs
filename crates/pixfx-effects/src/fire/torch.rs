//! Torch: port of pixelfx `src/fx/fire/torch.js`.
//!
//! The tongue, lick, flame-puff, smoke and ember recipe is adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, mix, ramp};
use pixfx_draw::math::{clamp, js_round, rand};

use super::lib::{Burn, Hot, burn, hot_first, nl};

pub struct Torch;

pub static TORCH: Torch = Torch;

const META: Meta = Meta {
    id: "fire/torch",
    name: "Torch",
    desc: "A wall torch on an iron bracket: a licking flame leaping off a pitch-soaked wrap, licks and puffs torn off its tips, smoke, embers, a flickering light on the wall.",
    tags: &["fire", "torch", "prop", "light", "wall", "dungeon", "loop"],
    size: (72, 120),
    origin: (36, 94),
    duration: 2.0,
    looping: true,
    fps: 30,
    seed: 4,
    state: None,
    options: &[WOOD.spec(), IRON.spec()],
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
const WOOD: RampOpt = RampOpt::new(
    "wood",
    &[
        Rgba::hex24(0x0e080a),
        Rgba::hex24(0x24130f),
        Rgba::hex24(0x3b2216),
        Rgba::hex24(0x5a3520),
        Rgba::hex24(0x8a5a34),
        Rgba::hex24(0xb9884f),
    ],
);
const IRON: RampOpt = RampOpt::new(
    "iron",
    &[
        Rgba::hex24(0x0d0b12),
        Rgba::hex24(0x1e1b26),
        Rgba::hex24(0x363242),
        Rgba::hex24(0x5b5566),
        Rgba::hex24(0x8a8396),
    ],
);

const D: f64 = 2.0;
const BASE: f64 = -16.0;
// the flame: (x, y from the wrap's top, height share, half-width, back row?)
const TONGUES: [(f64, f64, f64, f64, bool); 5] = [
    (-1.9, -1.0, 0.74, 2.0, true),
    (2.1, -1.0, 0.7, 2.0, true),
    (-3.4, 0.5, 0.42, 1.6, false),
    (3.6, 0.5, 0.46, 1.6, false),
    (0.0, 0.0, 1.0, 2.9, false),
];

impl Effect for Torch {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            h: hot_first(&opts.palette_or(&PALETTE)),
            w: ramp(&opts.ramp(&WOOD)),
            i: ramp(&opts.ramp(&IRON)),
            seed: seed as f64,
        })
    }
}

struct State {
    h: Hot,
    w: [Rgba; 6],
    i: [Rgba; 5],
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

// the bracket: a bevelled iron plate on the wall with two rivets, and a ring round the handle
fn plate(p: &mut Painter, i: &[Rgba; 5], h: &Hot, fl: f64) {
    p.shade(-5.5, -5.5, 5.5, 11.5, |x, y, _| {
        if x < -5.0 || x > 5.0 || y < -5.0 || y > 11.0 {
            return Some(i[0]);
        }
        let rv = libm::fabs(x) < 0.6 && (libm::fabs(y + 3.5) < 0.6 || libm::fabs(y - 9.5) < 0.6);
        Some(if rv {
            mix(i[4], h[3], 0.35 * fl)
        } else if y < -4.0 || x < -4.0 {
            mix(i[3], h[4], 0.3 * fl)
        } else if y > 10.0 || x > 4.0 || y >= 2.0 {
            i[1]
        } else {
            i[2]
        })
    });
}

fn handle(p: &mut Painter, w: &[Rgba; 6]) {
    p.shade(-3.0, -12.0, 3.0, 15.0, |x, y, _| {
        let half = 1.3 + clamp((14.0 - y) / 26.0, 0.0, 1.0) * 0.9;
        if libm::fabs(x) > half + 0.5 || y > 14.5 {
            return None;
        }
        if libm::fabs(x) > half - 0.5 || y > 13.5 {
            return Some(w[0]);
        }
        let g = rand(&[libm::floor(y / 3.0), js_round(x), 9.0]);
        Some(if x < -0.3 {
            if g < 0.3 { w[2] } else { w[4] }
        } else if g < 0.4 {
            w[1]
        } else {
            w[2]
        })
    });
}

fn ring(p: &mut Painter, i: &[Rgba; 5], h: &Hot, fl: f64) {
    p.rect(-3.0, -2.0, 7.0, 4.0, i[0], 1.0);
    p.rect(-2.0, -1.0, 5.0, 1.0, mix(i[4], h[3], 0.3 * fl), 1.0);
    p.rect(-2.0, 0.0, 5.0, 1.0, i[2], 1.0);
}

// the head: rags bound with cord and soaked in pitch, flaring into a cup at the top, charred where it burns
// with embers breathing in the folds
fn wrap(p: &mut Painter, w: &[Rgba; 6], i: &[Rgba; 5], h: &Hot, t: f64) {
    p.shade(-6.0, BASE - 2.0, 6.0, -9.0, |x, y, out| {
        let v = (y - (BASE - 1.5)) / 7.5;
        let half = 4.6 - v * 1.9;
        if libm::fabs(x) > half + 0.5 || y < BASE - 1.5 || y > -9.5 {
            return None;
        }
        if libm::fabs(x) > half - 0.5 || y > -10.5 {
            return Some(w[0]);
        }
        let cord = libm::floor(y - x * 0.6 + 40.0) % 3.0 == 0.0;
        let hv = rand(&[libm::floor(x), libm::floor(y), 31.0]);
        if v < 0.45 {
            if hv < 0.42 {
                let g = 0.5 + 0.5 * libm::sin((t / D) * core::f64::consts::PI * 4.0 + hv * 40.0);
                out.k = 0.3 * g;
                return Some(if g > 0.66 {
                    h[2]
                } else if g > 0.33 {
                    h[4]
                } else {
                    h[5]
                });
            }
            return Some(if hv < 0.75 { w[0] } else { w[1] });
        }
        if cord {
            return Some(i[2]);
        }
        Some(if x < -1.0 {
            w[3]
        } else if x < 1.5 {
            w[2]
        } else {
            w[1]
        })
    });
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, w, i, s) = (&o.h, &o.w, &o.i, o.seed);
    let fl0 = 0.74 + 0.16 * nl(D, t, 7.3, s + 1.0, 0.0) + 0.1 * nl(D, t, 19.0, s + 2.0, 0.0);
    plate(p, i, h, fl0);
    handle(p, w);
    ring(p, i, h, fl0);
    burn(
        p,
        t,
        &TONGUES,
        &Burn {
            h,
            d: D,
            s,
            x: 0.0,
            y: BASE,
            height: 20.0,
            lean: 0.12,
            spread: 4.0,
            puffs: 10.0,
            licks: 5.0,
            smoke: 7.0,
            smoke_size: 2.3,
            smoke_life: (1.3, 2.1),
            embers: 7.0,
            glow_r: 48.0,
            glow: 1.25,
            wind: 2.0,
        },
        |p| wrap(p, w, i, h, t),
    );
}
