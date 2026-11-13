//! Coin spin: port of pixelfx `src/fx/pickups/coin-spin.js`.
//!
//! The hover, floor shadow and glints follow Sprite Fusion's open-source pickup
//! (https://www.spritefusion.com/pixel-art-vfx/pickup), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{TAU, js_round, rand};

use super::lib::{CoinLayer, blob_shadow, coin3d, glint};

pub struct CoinSpin;

pub static COIN_SPIN: CoinSpin = CoinSpin;

const META: Meta = Meta {
    id: "pickups/coin-spin",
    name: "Coin spin",
    desc: "A thick gold coin turning on its edge: an embossed star face, a reeded rim showing as it goes edge-on, a flash sweeping the face, bobbing over its shadow.",
    tags: &["pickup", "coin", "gold", "money", "loop"],
    size: (56, 56),
    origin: (28, 48),
    duration: 1.6,
    looping: true,
    fps: 30,
    seed: 3,
    state: None,
    options: &[],
};

// meta.options
const PALETTE: [Rgba; 8] = [
    Rgba::hex24(0x1e1206),
    Rgba::hex24(0x4a2c08),
    Rgba::hex24(0x86520c),
    Rgba::hex24(0xc48412),
    Rgba::hex24(0xf0b424),
    Rgba::hex24(0xffe066),
    Rgba::hex24(0xfff6c4),
    Rgba::hex24(0xffffff),
];

const R: f64 = 7.0;
const TH: f64 = 3.0;
const HOVER: f64 = 17.0;

impl Effect for CoinSpin {
    fn meta(&self) -> &Meta {
        &META
    }
    /// The coin draws its own outline (its ramp's second-darkest step), which the `outline` option recolours or
    /// drops.
    fn draws_outline(&self) -> bool {
        true
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        Box::new(State {
            edge: opts.outline_color(c[1]),
            c,
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    edge: Option<Rgba>,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            draw_back(p, t);
        }
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

fn draw_back(p: &mut Painter, t: f64) {
    let ph = t / META.duration;
    let th = ph * TAU;
    let y = -HOVER - 2.0 * libm::sin(ph * TAU);
    // the shadow narrows as the coin turns edge-on and thins as it rises
    blob_shadow(
        p,
        0.0,
        2.5 + 4.5 * libm::fabs(libm::cos(th)) + 1.5 * libm::fabs(libm::sin(th)),
        0.85 - (-y - HOVER + 2.0) * 0.06,
        0.0,
    );
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let c = &o.c;
    let d = META.duration;
    let ph = t / d;
    let th = ph * TAU;
    let y = -HOVER - 2.0 * libm::sin(ph * TAU);
    // warm light round it, swelling when the face catches the light
    let facing = libm::fabs(libm::cos(th));
    p.glow(0.0, y, 20.0, c[4], 0.3 + 0.12 * facing);
    let mut layer = CoinLayer::new(32, 32);
    let flash = coin3d(&mut layer, p, 0.0, y, R, TH, th, c, o.edge);
    p.glow(-1.0, y - 1.0, 12.0, c[6], 0.35 * flash);
    // a star glint where the flash leaves the rim
    if flash > 0.3 {
        glint(p, -4.0, y - 4.0, 1.0 + 3.2 * flash, c[6], 0.5 * flash);
    }
    // little twinkles round it, one after another
    for i in 0..5 {
        let i = f64::from(i);
        let life = 0.35;
        let age = (((t - (i / 5.0) * d - rand(&[i, o.seed, 3.0]) * 0.08) % d) + d) % d;
        if age > life {
            continue;
        }
        let a = rand(&[i, o.seed, 1.0]) * TAU + i * 2.1;
        let dist = 10.0 + rand(&[i, o.seed, 2.0]) * 5.0;
        let k = libm::sin((age / life) * core::f64::consts::PI);
        glint(
            p,
            libm::cos(a) * dist,
            y + libm::sin(a) * dist * 0.85 - age * 8.0,
            js_round(2.0 * k),
            if i % 2.0 == 1.0 { c[6] } else { WHITE },
            0.35 * k,
        );
    }
}
