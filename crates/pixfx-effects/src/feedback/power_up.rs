//! Power up: port of Sprite Fusion's power up (https://www.spritefusion.com/pixel-art-vfx/power-up), MIT License,
//! Copyright (c) 2026 Hugo Duprez. The source is vendored verbatim at `tools/spritefusion/feedback/power-up.ts`.
//!
//! The source works in world units of 2 art pixels (its `PX`) with the feet at world (0, 0); this port works in
//! art pixels around the feet, and halving is exact in floating point, so the numbers are the same. Each engine
//! sub-step is one call of the source's `update`, as the reference runs it.
//!
//! It wraps its actor ([`Effect::wraps_actor`]): the aura grips the outline of the character's sprite as it is on
//! each step, and the character flashes in the palette by each pixel's brightness, as the source does with its
//! `actor.silhouette()`. With no actor, or none inside the box round the feet, it grips the source's stand-in body
//! (head, trunk and legs).
//!
//! Beyond the source: the shake and flash it asks for at the burst are cues ([`Cue::Shake`], [`Cue::Flash`]);
//! the `feel` option scales them, 0 for none. The flash has no colour, as every pixfx flash.

use pixfx_core::cue::{Cue, FEEL, TimedCue};
use std::sync::Arc;

use pixfx_core::{
    Actor, Area, Canvas, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette,
};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, rgb, scale};
use pixfx_draw::math::{Rng, TAU, clamp, js_max, js_min, js_round, smooth, to_int32, to_uint32};

pub struct PowerUp;

pub static POWER_UP: PowerUp = PowerUp;

const META: Meta = Meta {
    id: "feedback/power-up",
    name: "Power up",
    desc: "A buff in its stat's colour: streaks drawn in, the character flashing, a shock racing along the floor, then an aura gripping the character's outline and licking upward, with speed streaks and motes.",
    tags: &[
        "feedback",
        "power-up",
        "buff",
        "aura",
        "speed",
        "pickup",
        "character",
        "one-shot",
    ],
    // `canvas` grows it with `size`; at size 1 and below this fits
    size: (160, 144),
    // multiples of 4, so the source's dither (by local pixel) lands on the same screen cells
    origin: (80, 104),
    duration: 3.0,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[SIZE.spec(), FEEL.spec()],
};

/// How big and strong the buff is (the source's `size`): the aura's reach, the glow, the shock and the number of
/// motes.
const SIZE: NumOpt = NumOpt::new("size", 1.0).within(0.25, 2.0);

const PALETTE: [Rgba; 7] = [
    Rgba::hex24(0xffffff),
    Rgba::hex24(0xe0fcff),
    Rgba::hex24(0x8ff0ff),
    Rgba::hex24(0x3fd0ff),
    Rgba::hex24(0x2290e6),
    Rgba::hex24(0x1f4fab),
    Rgba::hex24(0x172a68),
];
const DUSTC: Rgba = Rgba::hex24(0x6a5e82);

/// The source's beats, in seconds: the burst, the aura holding, fading, and the end.
const BURST: f64 = 0.32;
const HOLD: f64 = 1.75;
const FADE: f64 = 2.35;
const PULSE1: f64 = BURST + 0.55;
const PULSE2: f64 = BURST + 1.0;
/// The box the body and its aura are measured in, the feet at its bottom middle.
const BOX: Area = Area {
    x: -32,
    y: -70,
    w: 64,
    h: 76,
};
const MID_Y: f64 = -22.0;
/// The source's world unit in art pixels.
const PX: f64 = 2.0;

const BAYER: [f64; 16] = [
    0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
];

/// The source's `bayer(x, y)`, by local pixel.
fn bayer(x: f64, y: f64) -> f64 {
    let (x, y) = (to_int32(x), to_int32(y));
    let i = (((y & 3) << 2) | (x & 3)) as usize;
    (BAYER.get(i).copied().unwrap_or(0.0) + 0.5) / 16.0
}

/// The source's `hash(n, k)`: `((Math.imul(n + 1 + k * 7919, 2654435761) >>> 0) % 100000) / 100000`.
fn hash(n: f64, k: f64) -> f64 {
    let m = to_int32(n + 1.0 + k * 7919.0).wrapping_mul(to_int32(2_654_435_761.0)) as u32;
    f64::from(m % 100_000) / 100_000.0
}

/// The source's `noise2`: value noise in [-1, 1] over its own `hash`.
fn noise2(x: f64, y: f64, seed: f64) -> f64 {
    let (ix, iy) = (libm::floor(x), libm::floor(y));
    let (fx, fy) = (x - ix, y - iy);
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);
    let h = |a: f64, b: f64| hash(a * 73.0 + b * 9301.0, seed) * 2.0 - 1.0;
    let top = h(ix, iy) + (h(ix + 1.0, iy) - h(ix, iy)) * u;
    let bot = h(ix, iy + 1.0) + (h(ix + 1.0, iy + 1.0) - h(ix, iy + 1.0)) * u;
    top + (bot - top) * v
}

/// The source's `lum`: a colour's brightness, 0 to 1.
fn lum(c: Rgba) -> f64 {
    (f64::from(c.0) * 0.3 + f64::from(c.1) * 0.59 + f64::from(c.2) * 0.11) / 255.0
}

/// The source's `outExpo`.
fn out_expo(t: f64) -> f64 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - libm::pow(2.0, -10.0 * clamp(t, 0.0, 1.0))
    }
}

/// The source's `mix`: each channel `c1 * (1 - t) + c2 * t`, truncated.
fn mix(c1: Rgba, c2: Rgba, t: f64) -> Rgba {
    let u = 1.0 - t;
    let ch = |a: u8, b: u8| f64::from(a) * u + f64::from(b) * t;
    rgb(ch(c1.0, c2.0), ch(c1.1, c2.1), ch(c1.2, c2.2))
}

/// `exp(-((t - p) / 0.07)²)`: a pulse at `p`.
fn pulse(t: f64, p: f64, w: f64) -> f64 {
    libm::exp(-libm::pow((t - p) / w, 2.0))
}

/// The source's `dot`: a pixel at alpha `a`, faded by ordered dither.
fn dot(p: &mut Painter, x: f64, y: f64, c: Rgba, a: f64) {
    let (px, py) = (libm::floor(x), libm::floor(y));
    if a >= 1.0 || a > bayer(px, py) {
        p.px(px, py, c);
    }
}

/// The source's `disc`: a filled disc, one pixel when it is small.
fn disc(p: &mut Painter, cx: f64, cy: f64, r: f64, c: Rgba) {
    if r <= 0.5 {
        p.px(js_round(cx), js_round(cy), c);
        return;
    }
    let r2 = r * r;
    let mut y = libm::floor(cy - r);
    while y <= libm::ceil(cy + r) {
        let dy = y + 0.5 - cy;
        let mut x = libm::floor(cx - r);
        while x <= libm::ceil(cx + r) {
            let dx = x + 0.5 - cx;
            if dx * dx + dy * dy <= r2 {
                p.px(x, y, c);
            }
            x += 1.0;
        }
        y += 1.0;
    }
}

/// The source's `alphaDisc`: a translucent disc.
fn alpha_disc(p: &mut Painter, cx: f64, cy: f64, r: f64, c: Rgba, a: f64) {
    if a <= 0.01 {
        return;
    }
    let r2 = r * r;
    let mut y = libm::floor(cy - r);
    while y <= libm::ceil(cy + r) {
        let dy = y + 0.5 - cy;
        let mut x = libm::floor(cx - r);
        while x <= libm::ceil(cx + r) {
            let dx = x + 0.5 - cx;
            if dx * dx + dy * dy <= r2 {
                p.blend(x, y, c, a);
            }
            x += 1.0;
        }
        y += 1.0;
    }
}

/// The source's `line`: Bresenham between the rounded ends.
fn line(p: &mut Painter, x0: f64, y0: f64, x1: f64, y1: f64, c: Rgba) {
    let (mut x0, mut y0) = (js_round(x0), js_round(y0));
    let (x1, y1) = (js_round(x1), js_round(y1));
    let dx = libm::fabs(x1 - x0);
    let dy = -libm::fabs(y1 - y0);
    let sx = if x0 < x1 { 1.0 } else { -1.0 };
    let sy = if y0 < y1 { 1.0 } else { -1.0 };
    let mut err = dx + dy;
    for _ in 0..4096 {
        p.px(x0, y0, c);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2.0 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// The source's `glow`, which adds nothing at strength 0.01 or less.
fn glow(p: &mut Painter, x: f64, y: f64, r: f64, c: Rgba, k: f64) {
    if k > 0.01 {
        p.glow(x, y, r, c, k);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A streak drawn in towards the body before the burst.
    In,
    /// Dust kicked along the floor.
    Dust,
    /// A speed streak rising past the body.
    Streak,
    /// A four-point twinkle.
    Star,
    /// A wisp of aura licking upward.
    Wisp,
}

struct Mote {
    k: Kind,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    max: f64,
    c: Rgba,
    s: f64,
    front: bool,
}

impl Effect for PowerUp {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    /// Room for the aura, the motes and the shock of a buff of this `size`.
    fn canvas(&self, opts: &Options) -> Canvas {
        let k = js_max(1.0, opts.num(&SIZE));
        let w = 8 * libm::ceil(20.0 * k) as u32;
        let h = 8 * libm::ceil(18.0 * k) as u32;
        Canvas {
            size: (w, h),
            origin: ((w / 2) as i32, 4 * libm::ceil(26.0 * k) as i32),
        }
    }
    /// Beyond the source: the burst asks the game for the shake and flash the source asks its `shake` and `flash`
    /// for.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let k = opts.num(&FEEL) * opts.num(&SIZE);
        if k.is_nan() || k <= 0.0 {
            return Vec::new();
        }
        let at = |cue| TimedCue {
            t: BURST,
            cue,
            pan: None,
        };
        vec![
            at(Cue::Shake {
                trauma: clamp(0.32 * k, 0.0, 1.0),
            }),
            at(Cue::Flash {
                k: clamp(0.22 * k, 0.0, 1.0),
                dur: 0.1,
            }),
        ]
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let pal = opts.palette_or(&PALETTE);
        let c = |i: usize| pal.get(i).copied().unwrap_or(WHITE);
        Box::new(State {
            c: [c(0), c(1), c(2), c(3), c(4), c(5)],
            s: opts.num(&SIZE),
            r: Rng::new(to_uint32(seed as f64)),
            motes: Vec::new(),
            actor: None,
            body: body(None),
            t: 0.0,
            burst: false,
        })
    }
}

/// The source's `body()`: the cells of the box the actor's sprite covers, or, when it covers none, the stand-in
/// body (head, trunk, legs). The feet never move in the effect's space, so it changes only with the actor.
fn body(actor: Option<&Actor>) -> Silhouette {
    match actor.map(|a| Silhouette::of(a, BOX)) {
        Some(sprite) if !sprite.is_empty() => sprite,
        _ => Silhouette::from_fn(BOX, |px, py| {
            let head = libm::hypot(px, py + 38.0) < 4.5;
            let trunk = libm::fabs(px) < 2.2 && py > -34.0 && py < -14.0;
            let legs =
                (-14.0..0.0).contains(&py) && libm::fabs(libm::fabs(px) - (py + 14.0) * 0.28) < 1.4;
            head || trunk || legs
        }),
    }
}

struct State {
    /// The palette's first six steps, light to dark (the source's `C`).
    c: [Rgba; 6],
    /// The source's `S`.
    s: f64,
    r: Rng,
    motes: Vec<Mote>,
    /// The character's sprite as it is now, if the game gave one.
    actor: Option<Arc<Actor>>,
    body: Silhouette,
    t: f64,
    burst: bool,
}

impl State {
    fn top(&mut self) -> Rgba {
        let top = [self.c[0], self.c[1], self.c[2]];
        self.r.pick(&top).copied().unwrap_or(WHITE)
    }

    fn aura(&self) -> f64 {
        let t = self.t;
        smooth(BURST - 0.04, BURST + 0.12, t)
            * (1.0 - smooth(HOLD, FADE, t))
            * (0.85 + 0.15 * libm::sin(t * 13.0))
    }

    fn flare(&self) -> f64 {
        let t = self.t;
        js_max(pulse(t, PULSE1, 0.07), pulse(t, PULSE2, 0.07)) + pulse(t, BURST, 0.08)
    }

    /// A random point of the body (the source's `onBody`): its cell's centre, or the body's middle after 40
    /// misses.
    fn on_body(&mut self) -> (f64, f64) {
        match self.body.pick(&mut self.r, 40) {
            Some((x, y)) => (f64::from(x) + 0.5, f64::from(y) + 0.5),
            None => (f64::from(BOX.x) + f64::from(BOX.w / 2), MID_Y),
        }
    }

    /// One call of the source's `update(dt)`.
    fn step(&mut self, dt: f64) {
        self.t += dt;
        let (s, c) = (self.s, self.c);
        let (ccx, ccy) = (0.0, MID_Y);
        if self.t < BURST {
            let mut i = 0.0;
            while i < js_round(3.0 * s) {
                let a = self.r.range(0.0, TAU);
                let r = self.r.range(34.0, 64.0) * libm::sqrt(s);
                let sp = self.r.range(260.0, 420.0);
                let (ca, sa) = (libm::cos(a), libm::sin(a));
                let col = if self.r.chance(0.3) { c[1] } else { c[2] };
                self.motes.push(Mote {
                    k: Kind::In,
                    x: ccx + ca * r,
                    y: ccy + sa * r * 0.8,
                    vx: -ca * sp,
                    vy: -sa * sp * 0.8,
                    life: (r * PX) / sp,
                    max: (r * PX) / sp,
                    c: col,
                    s: 1.0,
                    front: true,
                });
                i += 1.0;
            }
        }
        if !self.burst && self.t >= BURST {
            self.burst = true;
            let dust = mix(DUSTC, c[4], 0.25);
            let mut i = 0.0;
            while i < 18.0 * s {
                let side = self.r.sign();
                let x = ccx + side * self.r.range(2.0, 10.0);
                let y = -self.r.range(0.0, 3.0);
                let vx = side * self.r.range(60.0, 170.0);
                let vy = -self.r.range(2.0, 16.0);
                let life = self.r.range(0.45, 0.8);
                let size = self.r.range(1.5, 3.0);
                self.motes.push(Mote {
                    k: Kind::Dust,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    max: 0.8,
                    c: dust,
                    s: size,
                    front: false,
                });
                i += 1.0;
            }
            let mut i = 0.0;
            while i < 22.0 * s {
                let (x, y) = self.on_body();
                let vx = self.r.range(-40.0, 40.0);
                let vy = -self.r.range(30.0, 110.0);
                let life = self.r.range(0.6, 1.3);
                let col = self.top();
                let size = if self.r.chance(0.35) {
                    2.0
                } else if self.r.chance(0.5) {
                    1.0
                } else {
                    0.0
                };
                let front = self.r.chance(0.6);
                self.motes.push(Mote {
                    k: Kind::Star,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    max: 1.3,
                    c: col,
                    s: size,
                    front,
                });
                i += 1.0;
            }
        }
        let a = self.aura();
        if self.burst && a > 0.05 {
            let streaks = (5.0 + 5.0 * self.flare()) * a * s * dt * 10.0;
            let mut n = 0.0;
            while n < streaks {
                let (x, _) = self.on_body();
                let side = self.r.sign();
                let x = x + side * self.r.range(5.0, 16.0) * libm::sqrt(s);
                let y = ccy + self.r.range(-16.0, 26.0);
                let vy = -self.r.range(220.0, 420.0);
                let life = self.r.range(0.14, 0.3);
                let col = if self.r.chance(0.5) { c[2] } else { c[1] };
                let len = self.r.range(6.0, 16.0);
                let front = self.r.chance(0.5);
                self.motes.push(Mote {
                    k: Kind::Streak,
                    x,
                    y,
                    vx: 0.0,
                    vy,
                    life,
                    max: 0.3,
                    c: col,
                    s: len,
                    front,
                });
                n += 1.0;
            }
            let mut n = 0.0;
            while n < 9.0 * a * s * dt {
                n += 1.0;
                let (x, y) = self.on_body();
                let up = clamp((MID_Y - y) / 16.0, 0.0, 1.0);
                if self.r.chance(1.0 - up * 0.8) {
                    continue;
                }
                let x = x + self.r.range(-4.0, 4.0);
                let y = y - self.r.range(6.0, 12.0) * s;
                let vx = self.r.range(-15.0, 15.0);
                let vy = -self.r.range(70.0, 130.0);
                let life = self.r.range(0.3, 0.55);
                let size = self.r.range(1.4, 2.4) * libm::sqrt(s);
                self.motes.push(Mote {
                    k: Kind::Wisp,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    max: 0.55,
                    c: c[2],
                    s: size,
                    front: false,
                });
            }
            if self.r.chance(0.5 * a) {
                let (x, y) = self.on_body();
                let x = x + self.r.range(-8.0, 8.0);
                let vx = self.r.range(-10.0, 10.0);
                let vy = -self.r.range(25.0, 70.0);
                let life = self.r.range(0.6, 1.1);
                let col = self.top();
                let size = if self.r.chance(0.25) { 1.0 } else { 0.0 };
                let front = self.r.chance(0.5);
                self.motes.push(Mote {
                    k: Kind::Star,
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    max: 1.1,
                    c: col,
                    s: size,
                    front,
                });
            }
        }
        self.motes.retain_mut(|m| {
            m.life -= dt;
            if m.life <= 0.0 {
                return false;
            }
            let drag = match m.k {
                Kind::Dust => 3.5,
                Kind::Star => 1.4,
                _ => 0.0,
            };
            m.vx -= m.vx * drag * dt;
            m.vy -= m.vy * drag * dt;
            if m.k == Kind::Star {
                m.vy -= 18.0 * dt;
            }
            m.x += (m.vx * dt) / PX;
            m.y += (m.vy * dt) / PX;
            true
        });
    }

    /// The aura: bands of the palette out from the body, reaching further where noise and height say, dithered
    /// at its rim.
    fn draw_aura(&self, p: &mut Painter, a: f64, fl: f64) {
        let (c, s, t) = (&self.c, self.s, self.t);
        for (x, y, d) in self.body.around(24.0) {
            let (px, py) = (f64::from(x), f64::from(y));
            let n = libm::pow(
                noise2(px * 0.5, (py + t * 34.0) * 0.26, 7.0) * 0.5 + 0.5,
                1.4,
            );
            let up = clamp((MID_Y - py) / 16.0, 0.0, 1.0);
            let reach = a * s * (3.4 + 14.0 * n * (0.3 + 1.15 * up)) * (1.0 + 0.3 * fl);
            if d > reach {
                continue;
            }
            let band = d / reach;
            if band < 0.22 {
                p.px(px, py, if fl > 0.35 && band < 0.14 { c[0] } else { c[1] });
            } else if band < 0.45 {
                p.px(px, py, c[2]);
            } else if band < 0.7 {
                p.px(px, py, c[3]);
            } else if bayer(px, py) < (1.0 - band) * 2.6 {
                p.px(px, py, if band < 0.86 { c[4] } else { c[5] });
            }
        }
    }

    /// The source's `shock`: a flash at the feet, a wave racing both ways along the floor with a crest of light,
    /// and sparks kicked up behind it. `t` is seconds since the burst, `z` its size.
    fn shock(&self, p: &mut Painter, t: f64, z: f64) {
        let l = &self.c;
        let (cx, fy) = (0.0, 0.0);
        let big_r = 72.0 * z;
        let q = t / 0.55;
        if !(0.0..1.8).contains(&q) {
            return;
        }
        if q < 0.5 {
            let mut dx = -js_round(14.0 * z);
            while dx <= 14.0 * z {
                let u = libm::fabs(dx) / (14.0 * z);
                let a = (1.0 - q / 0.5) * (1.0 - u) * 2.2;
                let c0 = if u < 0.3 {
                    WHITE
                } else if u < 0.6 {
                    l[1]
                } else {
                    l[2]
                };
                dot(p, cx + dx, fy, c0, a);
                dot(
                    p,
                    cx + dx,
                    fy - 1.0,
                    if u < 0.4 { l[1] } else { l[2] },
                    a * 0.7,
                );
                dot(p, cx + dx, fy + 1.0, l[2], a * 0.5);
                dx += 1.0;
            }
        }
        let reach = 4.0 + big_r * out_expo(js_min(1.0, q));
        let after = 1.0 - clamp((q - 0.25) / 1.4, 0.0, 1.0);
        let f = libm::floor(t * 20.0);
        let mut dx = -js_round(reach);
        while dx <= reach {
            let u = libm::fabs(dx) / reach;
            let g = hash(js_round(cx + dx) * 7.0 + f * 613.0, 0.0) < 0.04 * after;
            let c0 = if g {
                WHITE
            } else if u < 0.5 {
                l[2]
            } else {
                l[3]
            };
            dot(
                p,
                cx + dx,
                fy,
                c0,
                if g {
                    1.0
                } else {
                    after * 0.8 * (1.0 - u * 0.5)
                },
            );
            dot(p, cx + dx, fy - 1.0, l[3], after * 0.3 * (1.0 - u));
            dx += 1.0;
        }
        if q < 1.0 {
            let head = 4.0 + big_r * out_expo(q);
            let len = (6.0 + 22.0 * (1.0 - q)) * z;
            let crest = 6.0 * z * libm::pow(1.0 - q, 0.7);
            for sd in [-1.0, 1.0] {
                let mut j = 0.0;
                while j < len {
                    let x = cx + sd * (head - j);
                    let a = (1.0 - j / len) * (1.0 - q) * 2.2;
                    let lift = js_round(crest * js_max(0.0, 1.0 - j / (len * 0.5)));
                    let c0 = if j < 2.0 {
                        WHITE
                    } else if j < len * 0.35 {
                        l[1]
                    } else if j < len * 0.7 {
                        l[2]
                    } else {
                        l[3]
                    };
                    dot(p, x, fy, c0, a);
                    let mut r = 1.0;
                    while r <= lift {
                        let c1 = if j < 2.0 && r > lift - 2.0 {
                            WHITE
                        } else if r == lift {
                            l[1]
                        } else {
                            l[2]
                        };
                        dot(p, x, fy - r, c1, a * (1.0 - r / (lift + 2.0)) * 1.4);
                        r += 1.0;
                    }
                    j += 1.0;
                }
                let mut r = 0.0;
                while r < crest + 3.0 {
                    dot(
                        p,
                        cx + sd * head,
                        fy - r,
                        WHITE,
                        (1.0 - q) * 1.6 * (1.0 - r / (crest + 3.0)),
                    );
                    r += 1.0;
                }
                glow(p, cx + sd * head, fy - 3.0, 12.0 * z, l[2], 0.8 * (1.0 - q));
            }
            glow(p, cx, fy, reach, l[3], 0.25 * (1.0 - q));
        }
        let mut i = 0.0;
        while i < js_round(24.0 * z) {
            let t0 = hash(i, 1.0) * 0.32;
            let age = t - t0;
            let life = 0.3 + hash(i, 4.0) * 0.3;
            if age < 0.0 || age > life {
                i += 1.0;
                continue;
            }
            let sd = if i % 2.0 != 0.0 { 1.0 } else { -1.0 };
            let x = cx
                + sd * (4.0 + big_r * out_expo(t0 / 0.55))
                + sd * (15.0 + hash(i, 2.0) * 70.0) * age;
            let y = fy - 1.0 - ((60.0 + hash(i, 3.0) * 90.0) * age - 110.0 * age * age);
            let k = 1.0 - age / life;
            if y > fy - 1.0 {
                i += 1.0;
                continue;
            }
            dot(
                p,
                x,
                y,
                if age < life * 0.3 { WHITE } else { l[1] },
                1.4 * k,
            );
            if hash(i, 5.0) < 0.4 && k > 0.4 {
                for (ox, oy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    dot(p, x + ox, y + oy, l[2], k * 1.2);
                }
            }
            glow(p, x, y, 4.0, l[2], 0.45 * k);
            i += 1.0;
        }
    }

    /// The character flashing on each beat: every pixel of its sprite in the palette by its brightness, or the
    /// stand-in body without one (the source's `tint`).
    fn tint(&self, p: &mut Painter, e: f64) {
        let c = &self.c;
        let colour = |l: f64| {
            if l > 0.5 {
                if e > 0.75 { c[0] } else { c[1] }
            } else if l > 0.22 {
                c[2]
            } else {
                c[4]
            }
        };
        let (mut n, mut sx, mut top, mut bot) = (0.0, 0.0, f64::INFINITY, -1.0);
        let mut tint = |p: &mut Painter, x: f64, y: f64, l: f64| {
            n += 1.0;
            sx += x;
            top = js_min(top, y);
            bot = y;
            dot(p, x, y, colour(l), e * 1.5);
        };
        match &self.actor {
            Some(s) => {
                for (x, y, colour) in s.opaque() {
                    tint(p, f64::from(x), f64::from(y), lum(colour));
                }
            }
            None => {
                for (x, y) in self.body.cells() {
                    tint(p, f64::from(x), f64::from(y), 0.6);
                }
            }
        }
        if n > 0.0 {
            glow(p, sx / n, (top + bot) / 2.0, 26.0, self.c[3], 0.45 * e);
        }
    }
}

/// The source's `drawMote`.
fn draw_mote(p: &mut Painter, m: &Mote) {
    let k = m.life / m.max;
    match m.k {
        Kind::In => {
            let sp = libm::hypot(m.vx, m.vy);
            let sp = if sp == 0.0 || sp.is_nan() { 1.0 } else { sp };
            let (ux, uy) = (m.vx / sp, m.vy / sp);
            line(
                p,
                m.x - ux * 7.0,
                m.y - uy * 7.0,
                m.x - ux * 3.0,
                m.y - uy * 3.0,
                m.c,
            );
            line(p, m.x - ux * 2.5, m.y - uy * 2.5, m.x, m.y, WHITE);
        }
        Kind::Streak => {
            let l = m.s * (0.5 + 0.5 * k);
            let mut j = 0.0;
            while j <= l {
                let c = if j < 1.5 { WHITE } else { m.c };
                let a = if j < l * 0.5 {
                    1.0
                } else {
                    (1.0 - j / (l + 1.0)) * 1.6
                };
                dot(p, m.x, m.y + j, c, a);
                j += 1.0;
            }
            glow(p, m.x, m.y + l * 0.3, 4.0, m.c, 0.3 * k);
        }
        Kind::Wisp => {
            let r = m.s * (0.35 + 0.65 * k);
            disc(
                p,
                m.x,
                m.y,
                r + 0.7,
                if k > 0.5 { m.c } else { scale(m.c, 0.7) },
            );
            if k > 0.35 {
                disc(
                    p,
                    m.x - 0.3,
                    m.y - 0.4,
                    r * 0.55,
                    if k > 0.7 { WHITE } else { m.c },
                );
            }
            glow(p, m.x, m.y, 5.0 + r * 2.0, m.c, 0.3 * k);
        }
        Kind::Star => {
            let (px, py) = (libm::floor(m.x), libm::floor(m.y));
            let tw = 0.6 + 0.4 * libm::sin(m.life * 26.0 + m.x);
            if tw * k < 0.2 {
                return;
            }
            let arm = js_round(m.s * tw * js_min(1.0, k * 2.5));
            let mut d = 1.0;
            while d <= arm {
                let c = if d == arm && arm > 1.0 {
                    scale(m.c, 0.75)
                } else {
                    m.c
                };
                p.px(px + d, py, c);
                p.px(px - d, py, c);
                p.px(px, py + d, c);
                p.px(px, py - d, c);
                d += 1.0;
            }
            p.px(px, py, if tw > 0.75 { WHITE } else { m.c });
            glow(p, m.x, m.y, 3.0 + arm * 2.0, m.c, 0.45 * k * tw);
        }
        Kind::Dust => {}
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.step(dt);
    }

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.body = body(actor.as_deref());
        self.actor = actor;
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let (a, fl, s, t) = (self.aura(), self.flare(), self.s, self.t);
        let c = &self.c;
        if layer == Layer::Back {
            for m in self.motes.iter().filter(|m| m.k == Kind::Dust) {
                let k = m.life / m.max;
                alpha_disc(p, m.x, m.y, m.s * (1.6 - k * 0.6), m.c, k * 0.6);
            }
            if a > 0.02 {
                self.draw_aura(p, a, fl);
            }
            for m in self.motes.iter().filter(|m| !m.front) {
                draw_mote(p, m);
            }
            glow(
                p,
                0.0,
                MID_Y,
                (34.0 + 20.0 * fl) * s,
                c[3],
                (0.35 * a + 0.3 * fl) * js_min(1.4, s),
            );
            glow(p, 0.0, -2.0, 30.0 * s, c[2], 0.25 * a);
            return;
        }
        if self.burst {
            self.shock(p, t - BURST, 0.45 * s);
        }
        let e = if self.burst {
            [BURST, PULSE1, PULSE2]
                .iter()
                .map(|&q| pulse(t, q, 0.07))
                .fold(f64::NEG_INFINITY, js_max)
        } else {
            0.0
        };
        if e > 0.03 {
            self.tint(p, e);
        }
        for m in self.motes.iter().filter(|m| m.front) {
            draw_mote(p, m);
        }
    }
}
