//! `feedback/burning`: the wrapped sprite is on fire. Small flames lick from four hot spots on its outline (the hair,
//! a shoulder, the two flanks) and lean into the wind, embers stream up and away, thin smoke drifts behind it; twice
//! a loop a tick flares every flame, the body flashes white then orange, and two scorch patches char the clothes.
//!
//! Adapted from Pixel VFX Studio's `status/burning` (MIT License, Copyright (c) 2026 Patch, Copyright (c) 2026 Hugo
//! Duprez). The flame tongues, embers and thin smoke are adapted from Sprite Fusion's open-source fire
//! (https://www.spritefusion.com/pixel-art-vfx/fire, MIT License, Copyright (c) 2026 Hugo Duprez).
//! Wraps its actor ([`Effect::wraps_actor`]): when handed a sprite via [`EffectState::set_actor`], it reads the
//! [`Body`] once per pose to place the hot spots and the scorch patches; with no sprite it uses the stand-in figure.

use std::sync::Arc;

use pixfx_core::particles::{cycle, fly};
use pixfx_core::sim::SMOKE_RAMP;
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, at, mix};
use pixfx_draw::math::{Rng, js_max, js_min, js_round, rand, tile_noise, to_int32, to_uint32};

use super::status::{BOX, Body, ORIGIN, SIZE, floor_light};

pub struct Burning;

pub static BURNING: Burning = Burning;

const FPS: u32 = 16;
const N: i32 = 16;
/// Seconds per loop.
const D: f64 = 1.0;
/// Frames between ticks: two ticks a loop.
const BEAT: i32 = 8;
/// The wind the flames, embers and smoke lean into, px/s to -x.
const WIND: f64 = -14.0;
const K_BURST: f64 = 31.0;
const K_SCORCH: f64 = 32.0;

const SPOTS: NumOpt = NumOpt::new("spots", 4.0).within(2.0, 4.0);
const SPARKS: NumOpt = NumOpt::new("sparks", 1.0).within(0.0, 2.0);
const SMOKE: NumOpt = NumOpt::new("smoke", 1.0).within(0.0, 1.5);
const BEAT_OPT: NumOpt = NumOpt::new("beat", 1.0).within(0.0, 1.0);

const META: Meta = Meta {
    id: "feedback/burning",
    name: "Burning",
    desc: "Small flames lick from four hot spots on the wrapped sprite's outline and lean into the wind, embers stream up and away and thin smoke drifts behind it; twice a second a tick flares the flames, flashes the body white then orange and chars two scorch patches on the clothes.",
    tags: &[
        "feedback",
        "fire",
        "burn",
        "status",
        "dot",
        "character",
        "loop",
    ],
    size: SIZE,
    origin: ORIGIN,
    duration: D,
    looping: true,
    fps: FPS,
    seed: 2_406_117_833,
    state: None,
    options: &[SPOTS.spec(), SPARKS.spec(), SMOKE.spec(), BEAT_OPT.spec()],
};

/// Dark to light, as the source lists it; the effect reads it bright end first (`C[0]` is white).
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

/// The char a scorch patch leaves: its heart, its body, its rim.
const CHAR: [Rgba; 3] = [
    Rgba::hex24(0x120c0a),
    Rgba::hex24(0x2a1a14),
    Rgba::hex24(0x4a2c1c),
];

/// The smoke's own tint, lit a little by the fire.
const SMOKE_COLOR: Rgba = Rgba::hex24(0x5b5566);

/// A tongue's nested layers: half-width, height and ramp step (0 the brightest): a deep rim, orange, yellow, a
/// white heart.
const LAYERS: [(f64, f64, usize); 4] = [
    (1.0, 1.0, 6),
    (0.76, 0.86, 4),
    (0.54, 0.68, 2),
    (0.3, 0.42, 0),
];

impl Effect for Burning {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(8 - i).copied().unwrap_or(PALETTE[8 - i]);
        let seed = f64::from(to_int32(seed as f64));
        let mut state = State {
            c: [c(0), c(1), c(2), c(3), c(4), c(5), c(6), c(7), c(8)],
            spots: js_round(opts.num(&SPOTS)) as usize,
            sparks: opts.num(&SPARKS),
            smoke: opts.num(&SMOKE),
            beat: opts.num(&BEAT_OPT) >= 0.5,
            seed,
            list: Vec::new(),
            scorch: [[Scorch::NONE; 2]; 2],
            body: Vec::new(),
            bounds: (0, 0),
        };
        state.read(None);
        Box::new(state)
    }
}

#[derive(Clone, Copy)]
struct Tongue {
    dx: f64,
    h: f64,
    /// Its noise seed.
    n: f64,
}

/// A hot spot on the outline: where its tongues stand (`x`, `y`, 2 rows inside the body so a flame looks rooted).
struct Spot {
    x: f64,
    y: f64,
    tongues: Vec<Tongue>,
}

/// A scorch patch: its centre (a cell centre on the clothes) and its full radius.
#[derive(Clone, Copy)]
struct Scorch {
    x: f64,
    y: f64,
    r: f64,
}

impl Scorch {
    const NONE: Scorch = Scorch {
        x: 0.0,
        y: 0.0,
        r: 0.0,
    };
}

/// A body cell as the tick flash, the blend and the scorch read it: its cell and its place in the body's box
/// (the source's mask space, which seeds the scorch jitter).
#[derive(Clone, Copy)]
struct BodyCell {
    x: i32,
    y: i32,
    mx: f64,
    my: f64,
}

struct State {
    /// The palette bright end first: `c[0]` white, `c[4]` orange, `c[8]` the darkest red.
    c: [Rgba; 9],
    spots: usize,
    sparks: f64,
    smoke: f64,
    beat: bool,
    seed: f64,
    /// The hot spots of this pose, the first `spots` of hair, shoulder, left flank, right flank.
    list: Vec<Spot>,
    /// Two patches per tick, two ticks.
    scorch: [[Scorch; 2]; 2],
    /// Every body cell of this pose.
    body: Vec<BodyCell>,
    /// The body box's top-left, for the scorch jitter.
    bounds: (i32, i32),
}

/// pixelfx `tnoise`: noise of time `t` that repeats every `d` seconds, about `f` wiggles a second; `off` slides
/// along it.
fn tnoise(t: f64, d: f64, f: f64, seed: f64, off: f64) -> f64 {
    let p = js_max(2.0, js_round(f * d));
    tile_noise((t / d) * p + off, 0.0, p, 0.0, seed)
}

/// pixelfx `travel`: how far something has moved after `age` seconds, launched at `v` under acceleration `g` with
/// linear drag `d` ([`fly`]'s closed form along one axis).
fn travel(v: f64, g: f64, d: f64, age: f64) -> f64 {
    fly(0.0, 0.0, 0.0, v, age, g, d).1
}

/// A flame tongue standing on (`x`, `y`): `h` tall, `w` half-wide at its root, its tip swept `sx` px sideways,
/// filled in nested layers from `c` (bright to dark). Rows ripple with noise that loops every `D` seconds (seed `n`).
#[expect(clippy::too_many_arguments, reason = "the source's signature")]
fn tongue(p: &mut Painter, x: f64, y: f64, h: f64, w: f64, sx: f64, c: &[Rgba; 9], t: f64, n: f64) {
    if h < 1.0 {
        return;
    }
    for &(lw, lh, ri) in &LAYERS {
        if ri == 0 && h < 4.0 {
            continue;
        }
        let hh = h * lh;
        let col = c[ri];
        let mut r = 0.0;
        while r <= hh {
            let u = r / js_max(1.0, hh);
            let half = w * lw * libm::pow(1.0 - u, 0.85) * js_min(1.0, (r + 1.5) / 3.5)
                + tnoise(t, D, 9.0, n, r * 0.45) * 0.5;
            if half >= 0.3 {
                let cx = x + sx * u * u * lh + tnoise(t, D, 4.0, n + 7.0, r * 0.2) * u * 1.1;
                let row = js_round(y - r);
                let mut px = js_round(cx - half);
                let end = js_round(cx + half);
                while px <= end {
                    p.px(px, row, col);
                    px += 1.0;
                }
            }
            r += 1.0;
        }
    }
}

/// A puff of smoke at age `u` (0..1): three soft discs (a body, a denser core sitting low, a lit cap while young),
/// swelling and thinning, tinted `col` and walking down the smoke ramp.
fn smoke(p: &mut Painter, x: f64, y: f64, size: f64, u: f64, col: Rgba, k: f64) {
    let r = size * (0.65 + js_min(1.0, u * 2.5) * 0.45 + u * 0.4);
    let a = js_min(1.0, u * 12.0) * js_min(1.0, (1.0 - u) * 2.4) * 0.58 * k;
    if a <= 0.01 {
        return;
    }
    let c = mix(col, at(&SMOKE_RAMP, u), u);
    p.disc(x, y, r, c, a * 0.45);
    p.disc(x + r * 0.05, y + r * 0.06, r * 0.74, c, a * 0.62);
    if u < 0.5 {
        p.disc(
            x - r * 0.22,
            y - r * 0.3,
            r * 0.5,
            mix(c, WHITE, 0.12),
            a * 0.4 * (1.0 - u * 2.0),
        );
    }
}

/// An ember at age `u`: hot and flickering, cooling to a dim red that blinks out. `e` is hot, warm, cool.
fn ember(p: &mut Painter, x: f64, y: f64, u: f64, e: [Rgba; 3], ph: f64) {
    let left = 1.0 - u;
    if left < 0.22 && libm::sin(ph * 40.0) < 0.0 {
        return;
    }
    let c = if left > 0.5 {
        if libm::sin(ph * 22.0) > -0.2 {
            e[0]
        } else {
            e[1]
        }
    } else if left > 0.22 {
        e[1]
    } else {
        e[2]
    };
    p.px(x, y, c);
    p.glow(x, y, 3.0, c, 0.32 * left);
}

impl State {
    /// The source's `spots(a, seed)`: the hot spots on the outline and the scorch patches near them, from the body
    /// of this pose (or the stand-in).
    fn read(&mut self, actor: Option<&Actor>) {
        let body = Body::read(actor, BOX);
        let b = body.bounds;
        let (bw, bh) = (f64::from(b.w), f64::from(b.h));
        let mut rng = Rng::new(to_uint32(self.seed * 13.0 + 5.0));
        // the row at `q` of the height, its left or right end
        let at = |q: f64, right: bool| -> (i32, i32) {
            let my = js_round(bh * q) as i32;
            let (l, r) = body
                .rows
                .get(my.clamp(0, b.h.saturating_sub(1) as i32) as usize)
                .copied()
                .unwrap_or((0, -1));
            (if right { r } else { l }, b.y + my)
        };
        let up: Vec<(i32, i32)> = body.sil.open(0, -1).collect();
        let hair = up
            .iter()
            .copied()
            .filter(|&(x, _)| f64::from(x - b.x) < bw * 0.42)
            .min_by_key(|&(x, y)| (y, x))
            .unwrap_or_else(|| at(0.0, false));
        let shoulder = up
            .iter()
            .copied()
            .filter(|&(x, y)| {
                let (mx, my) = (f64::from(x - b.x), f64::from(y - b.y));
                mx > bw / 2.0 && my > bh * 0.4 && my < bh * 0.62
            })
            .min_by_key(|&(_, y)| y)
            .unwrap_or_else(|| at(0.5, true));
        // hair `size(head, 0.65)` = 5 tall at rest, the others `size(head, 0.5)` = 4
        let spots = [
            (hair, 3, 5.0),
            (shoulder, 2, 4.0),
            (at(0.62, false), 2, 4.0),
            (at(0.6, true), 2, 4.0),
        ];
        let mut list: Vec<Spot> = spots
            .iter()
            .enumerate()
            .map(|(i, &((x, y), n, h))| Spot {
                x: f64::from(x) + 0.5,
                y: f64::from(y) + 1.5,
                tongues: (0..n)
                    .map(|j| Tongue {
                        dx: (f64::from(j) - f64::from(n - 1) / 2.0) * 1.6 + rng.range(-0.4, 0.4),
                        h: h * rng.range(0.8, 1.2),
                        n: f64::from(i as i32 * 17 + j * 5) + self.seed,
                    })
                    .collect(),
            })
            .collect();
        list.truncate(self.spots.max(1));
        // scorch marks: each tick chars two patches on the clothes (mid-tones, where char shows) near a hot spot,
        // within `size(head, 1.3)` = 9 px of it
        let mut scorch = [[Scorch::NONE; 2]; 2];
        for (tick, set) in scorch.iter_mut().enumerate() {
            for (k, patch) in set.iter_mut().enumerate() {
                let sp = &list[(tick * 2 + k + 1) % list.len()];
                let near = |c: &super::status::Cell| {
                    libm::hypot(f64::from(c.x) + 0.5 - sp.x, f64::from(c.y) + 0.5 - sp.y) < 9.0
                        && f64::from(c.y - b.y) > bh * 0.42
                        && c.l > 0.3
                        && c.l < 0.6
                };
                let mut cand: Vec<(i32, i32)> = body
                    .cells
                    .iter()
                    .filter(|c| near(c))
                    .map(|c| (c.x, c.y))
                    .collect();
                if cand.is_empty() {
                    cand = body
                        .cells
                        .iter()
                        .filter(|c| f64::from(c.y - b.y) > bh * 0.45)
                        .map(|c| (c.x, c.y))
                        .collect();
                }
                let pick = libm::floor(rng.next() * cand.len() as f64) as usize;
                let (x, y) = cand.get(pick).copied().unwrap_or((b.x, b.y));
                // radius about `size(hand, 2.0)` = 4
                *patch = Scorch {
                    x: f64::from(x) + 0.5,
                    y: f64::from(y) + 0.5,
                    r: rng.range(3.2, 4.4),
                };
            }
        }
        self.body = body
            .cells
            .iter()
            .map(|c| BodyCell {
                x: c.x,
                y: c.y,
                mx: f64::from(c.x - b.x),
                my: f64::from(c.y - b.y),
            })
            .collect();
        self.bounds = (b.x, b.y);
        self.list = list;
        self.scorch = scorch;
    }

    /// Seconds since the last tick (`k` the frame in the loop) and the flare it leaves, 1 on the tick.
    fn beat_at(&self, k: i32) -> (f64, f64) {
        let ph = f64::from(k.rem_euclid(BEAT)) / f64::from(FPS);
        let fl = if self.beat {
            libm::exp(-ph / 0.12)
        } else {
            0.0
        };
        (ph, fl)
    }

    fn draw_back(&self, p: &mut Painter, k: i32) {
        let t = f64::from(k) / f64::from(FPS);
        let c = &self.c;
        let (_, fl) = self.beat_at(k);
        let flick = 0.8 + 0.2 * tnoise(t, D, 7.5, 3.0, 0.0);
        // warm flicker on the floor, `size(width, 1.3)` = 26 half-wide
        floor_light(
            p,
            26.0,
            4.5,
            mix(c[3], c[4], 0.5),
            (0.16 + 0.22 * fl) * flick,
        );
        // thin smoke drifting up and away from each spot, a puff `size(eye.1, 0.75)` = 2 across
        let smoke_c = mix(SMOKE_COLOR, c[6], 0.2);
        let count = js_round(16.0 * self.smoke) as usize;
        if self.list.is_empty() || count == 0 {
            return;
        }
        cycle(
            t,
            D,
            count,
            self.seed + 4.0,
            |r| r.range(0.8, 1.2),
            |age, life, r, i| {
                let sp = &self.list[i % self.list.len()];
                let x = sp.x
                    + travel(WIND * 0.6, WIND * 0.5, 0.8, age)
                    + libm::sin(age * 4.0 + i as f64) * 1.2;
                let y = sp.y - 3.0 - travel(18.0, 6.0, 1.0, age);
                smoke(p, x, y, r.range(1.1, 1.9), age / life, smoke_c, 0.9);
            },
        );
    }

    fn draw_front(&self, p: &mut Painter, k: i32) {
        let t = f64::from(k) / f64::from(FPS);
        let c = &self.c;
        let kb = k.rem_euclid(BEAT);
        let tick = (k / BEAT).rem_euclid(2) as usize;
        let (ph, fl) = self.beat_at(k);
        // the tick: the body's own pixels flash white for a frame, then orange, fading
        if self.beat && kb == 0 {
            for cell in &self.body {
                p.paint_cell(f64::from(cell.x), f64::from(cell.y), WHITE, 1.0);
            }
        } else if self.beat && ph < 0.22 {
            let kk = 1.0 - (ph - 0.034) / 0.186;
            for cell in &self.body {
                p.each_game_cell(f64::from(cell.x), f64::from(cell.y), |p, x, y| {
                    p.blend_px(x, y, c[4], 0.55 * kk);
                });
            }
        }
        // scorch marks spreading over the clothes from this tick, ember-edged, cooling and gone by the next:
        // small on the tick, full by frame 2, fading over frames 4..7
        let grow = js_min(1.0, f64::from(kb) / 2.0);
        let fade = if kb <= 3 {
            1.0
        } else {
            1.0 - f64::from(kb - 3) / 5.0
        };
        let flicker = libm::floor(t * 20.0);
        for sc in &self.scorch[tick] {
            let r = sc.r * (0.4 + 0.6 * grow);
            if fade <= 0.0 || r <= 0.0 {
                continue;
            }
            for cell in &self.body {
                let (x, y) = (f64::from(cell.x) + 0.5, f64::from(cell.y) + 0.5);
                let d = libm::hypot(x - sc.x, (y - sc.y) * 1.2)
                    + rand(&[self.seed, K_SCORCH, cell.mx, cell.my, 7.0]) * 0.9;
                if d > r {
                    continue;
                }
                let (cx, cy) = (f64::from(cell.x), f64::from(cell.y));
                if d > r - 1.0 {
                    let hot = rand(&[self.seed, K_SCORCH, cell.mx, cell.my, flicker]) < 0.5;
                    p.dot(cx, cy, if hot { c[3] } else { c[4] }, fade * 1.2);
                } else {
                    p.dot(
                        cx,
                        cy,
                        if d < r * 0.5 { CHAR[0] } else { CHAR[1] },
                        fade * 1.1,
                    );
                }
            }
            p.glow(sc.x, sc.y, 6.0, c[4], 0.3 * fade);
        }
        // the hot spots: little tongues licking up, flaring on the tick; the root `size(limb, 0.4)` = 1 half-wide
        for sp in &self.list {
            for tg in &sp.tongues {
                let h = tg.h * (0.85 + 0.25 * tnoise(t, D, 5.0, tg.n, 0.0)) * (1.0 + 1.6 * fl);
                let sx = WIND * 0.08 * h / 4.0 + tnoise(t, D, 3.0, tg.n + 9.0, 0.0) * 0.8;
                tongue(p, sp.x + tg.dx, sp.y, h, 1.1 + 0.3 * fl, sx, c, t, tg.n);
            }
            p.glow(sp.x, sp.y - 2.0, 9.0 + 6.0 * fl, c[4], 0.22 + 0.35 * fl);
        }
        if self.list.is_empty() {
            return;
        }
        // embers streaming up and away, `size(eye.0, 1.0)` = 1 px each
        let count = js_round(20.0 * self.sparks) as usize;
        cycle(
            t,
            D,
            count,
            self.seed + 6.0,
            |r| r.range(0.5, 0.9),
            |age, life, r, i| {
                let sp = &self.list[i % self.list.len()];
                let x = sp.x
                    + r.range(-1.5, 1.5)
                    + travel(WIND + r.range(-10.0, 6.0), WIND * 0.6, 1.2, age)
                    + libm::sin(age * 9.0 + i as f64) * 1.2 * age;
                let y = sp.y - 2.0 + travel(-r.range(25.0, 55.0), -18.0, 1.4, age);
                ember(p, x, y, age / life, [c[1], c[3], c[5]], age + i as f64);
            },
        );
        // the tick throws a burst of sparks off every spot
        if self.beat && ph < 0.3 {
            for (si, sp) in self.list.iter().enumerate() {
                for j in 0..4 {
                    let (si, j, tk) = (si as f64, f64::from(j), tick as f64);
                    let an = -core::f64::consts::FRAC_PI_2
                        + (rand(&[self.seed, K_BURST, si, j, tk]) - 0.5) * 2.2;
                    let v = 50.0 + 50.0 * rand(&[self.seed, K_BURST, si, j, 2.0]);
                    let d = travel(v, 0.0, 3.0, ph);
                    let x = sp.x + libm::cos(an) * d;
                    let y = sp.y - 2.0 + libm::sin(an) * d + 60.0 * ph * ph;
                    p.px(x, y, if ph < 0.12 { c[1] } else { c[3] });
                    p.light(x, y, c[3], 0.4 * (1.0 - ph / 0.3));
                }
            }
        }
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.read(actor.as_deref());
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let k = (js_round(t * f64::from(FPS)) as i32).rem_euclid(N);
        match layer {
            Layer::Back => self.draw_back(p, k),
            Layer::Front => self.draw_front(p, k),
            Layer::Both => {
                self.draw_back(p, k);
                self.draw_front(p, k);
            }
        }
    }
}
