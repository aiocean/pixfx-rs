//! The critters' kit (pixelfx `src/fx/critters/_lib.js`; the leading _ keeps the tools from listing it as an
//! effect): loop-safe noise, the toolkit's particle looks for particles whose state is a function of time,
//! flame tongues, fire puffs and soft shadows. The flame tongue and fire puff recipes are adapted from Sprite
//! Fusion's open-source fire (https://www.spritefusion.com/pixel-art-vfx/fire), MIT License, Copyright (c) 2026
//! Hugo Duprez.
//!
//! This file has the functions wisp uses: `shift`, `lnoise`, `Swarm`, `tongue`, `fire_puff`, `shadow`.

use pixfx_core::Layer;
use pixfx_core::sim::{Flags, Kind, Particle, Particles, ParticlesOpts};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{js_max, js_min, js_round, tile_noise, wrap};

/// The loop time for this critter: seeds other than 1 start it at a different point in its loop, so a crowd of
/// the same critter doesn't move in lockstep.
pub fn shift(t: f64, d: f64, seed: f64) -> f64 {
    wrap(t + (seed - 1.0) * 0.618 * d, d)
}

/// Smooth noise in -1..1 that repeats every `d` seconds: `f` bumps a loop (a whole number), `off` shifts it
/// along, so `lnoise(t, d, f, r * 0.4, seed)` wobbles along a flame and still loops.
pub fn lnoise(t: f64, d: f64, f: f64, off: f64, seed: f64) -> f64 {
    tile_noise((t / d) * f + off, 0.5, f, 0.0, seed)
}

/// The toolkit's particle looks for particles whose state is a function of time (pure loops): `new(t, sparks)`,
/// then `add(..)` for each one alive now, then `draw(p)`. `sparks` is the SPARK/EMBER colours, head to tail.
/// (pixelfx `Swarm`; its own `Rng` is never drawn from, as `add` gives every particle its seed.)
pub struct Swarm {
    ps: Particles,
}

impl Swarm {
    pub fn new(t: f64, sparks: Vec<Rgba>) -> Self {
        let mut ps = Particles::new(ParticlesOpts {
            sparks,
            ..ParticlesOpts::default()
        });
        ps.set_time(t);
        Self { ps }
    }

    /// One particle `age` seconds into a `life`-second life; nothing when it isn't alive. JS defaults:
    /// `col = WHITE`, `size = 1`, `flag = 0`, `seed = 0`.
    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    pub fn add(
        &mut self,
        kind: Kind,
        x: f64,
        y: f64,
        vx: f64,
        vy: f64,
        age: f64,
        life: f64,
        col: Rgba,
        size: f64,
        flags: Flags,
        seed: f64,
    ) {
        if age < 0.0 || age >= life {
            return;
        }
        self.ps.list_mut().push(Particle {
            kind,
            x,
            y,
            vx,
            vy,
            life: life - age,
            max: life,
            col,
            size,
            seed,
            delay: 0.0,
            flags,
        });
    }

    pub fn draw(&self, p: &mut Painter, layer: Layer) {
        self.ps.draw(p, layer);
    }
}

// Hugo's flame layers: [width, height, ramp step] from the widest dark outer flame to the white-hot heart
const LAYERS: [(f64, f64, usize); 4] = [
    (1.0, 1.0, 6),
    (0.76, 0.86, 4),
    (0.54, 0.68, 2),
    (0.3, 0.42, 0),
];

fn step(heat: &[Rgba], i: usize) -> Rgba {
    heat.get(i).copied().unwrap_or(Rgba::NONE)
}

/// A tongue of flame standing on (x, y), h tall, w half-wide at its root, its tip swaying sx px; four nested
/// layers down a 9-step fire ramp given bright-first (heat[0] white). n seeds its flicker; t, d keep it looping.
/// only: draw just that layer (0 outer .. 3 heart), so a bunch of tongues can be laid down layer by layer and
/// merge into one flame with one dark rim. JS default `only = -1` is `None`.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn tongue(
    p: &mut Painter,
    x: f64,
    y: f64,
    h: f64,
    w: f64,
    sx: f64,
    t: f64,
    d: f64,
    n: f64,
    heat: &[Rgba],
    only: Option<usize>,
) {
    if h < 1.0 {
        return;
    }
    let f1 = js_max(1.0, js_round(9.0 * d));
    let f2 = js_max(1.0, js_round(4.0 * d));
    for (li, &(lw, lh, ri)) in LAYERS.iter().enumerate() {
        if only.is_some_and(|o| o != li) {
            continue;
        }
        let hh = h * lh;
        let col = step(heat, ri);
        let mut r = 0.0;
        while r <= hh {
            let u = r / js_max(1.0, hh);
            let half = w * lw * libm::pow(1.0 - u, 0.85) * js_min(1.0, (r + 1.5) / 3.5)
                + lnoise(t, d, f1, r * 0.45, n) * 0.5;
            let cx = x + sx * u * u * lh + lnoise(t, d, f2, r * 0.2, n + 7.0) * u * 1.1;
            if half >= 0.3 {
                let mut px = js_round(cx - half);
                while px <= js_round(cx + half) {
                    p.px(px, js_round(y - r), col);
                    px += 1.0;
                }
            }
            r += 1.0;
        }
    }
}

/// A puff of flame k of its life left (1 new .. 0 gone), size px, cooling down heat (bright-first, 9 steps):
/// a darker rim, a lit core, a tongue on top, light.
pub fn fire_puff(p: &mut Painter, x: f64, y: f64, k: f64, size: f64, seed: f64, heat: &[Rgba]) {
    let age = 1.0 - k;
    let n = heat.len();
    let nf = n as f64;
    let hot = js_min(0.999, libm::pow(age, 0.62) + (seed % 1.0) * 0.1);
    let idx = js_min(nf - 2.0, libm::floor(hot * (nf - 1.0))) as usize;
    let r = size * (0.55 + 0.45 * js_min(1.0, age * 7.0)) * (1.0 - age * 0.55);
    let c = step(heat, idx);
    if idx + 2 >= n {
        return p.disc(x, y, r, c, js_min(1.0, k * 3.0) * 0.85);
    }
    p.circle(x, y, r, step(heat, (idx + 2).min(n - 1)), 1.0);
    p.circle(x - r * 0.16, y - r * 0.22, r * 0.78, c, 1.0);
    if r > 1.6 {
        p.circle(x, y - r * 0.9, r * 0.42, c, 1.0);
    }
    if idx <= 3 && r > 1.8 {
        p.circle(
            x - r * 0.28,
            y - r * 0.34,
            r * 0.36,
            step(heat, idx.saturating_sub(1)),
            1.0,
        );
    }
    if idx < 6 {
        p.glow(x, y, r * 2.4 + 2.0, c, 0.18 * k);
    }
}

/// A soft shadow: an ellipse blended over what's there, a alpha at the center fading to nothing at the rim.
pub fn shadow(p: &mut Painter, cx: f64, cy: f64, rx: f64, ry: f64, c: Rgba, a: f64) {
    if rx <= 0.0 || ry <= 0.0 || a <= 0.0 {
        return;
    }
    let mut y = libm::floor(cy - ry);
    while y <= libm::ceil(cy + ry) {
        let mut x = libm::floor(cx - rx);
        while x <= libm::ceil(cx + rx) {
            let dx = (x + 0.5 - cx) / rx;
            let dy = (y + 0.5 - cy) / ry;
            let d = dx * dx + dy * dy;
            if d < 1.0 {
                p.blend(x, y, c, a * libm::pow(1.0 - d, 0.7));
            }
            x += 1.0;
        }
        y += 1.0;
    }
}
