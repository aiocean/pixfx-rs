//! Restriction: a seal of three rune rings round the character at knee, waist and head height (the
//! cultivation-fantasy restriction barrier). Made here from the parts sheet of `pixfx-ref`'s
//! `examples/specs/shield/restriction.json`. Its golden hashes pin it.
//!
//! The runes are written one character at a time, ring by ring from the ground up, each flashing as it appears, the
//! ribbon under them drawn behind the pen. Every ring spins on its own, the middle one against the other two, and a
//! pulse runs along each ring's runes while it holds. A blow lands anywhere (`hitX`, `hitY`, by default the right
//! side of the waist ring): the nearest ring flares white from where it lands round, tilts away from it and throws
//! rune sparks out from it; at the end the rings unravel from the top, rune after rune
//! peeling off and fading. The far half of each ring (its runes mirrored) and the pillars go behind the character,
//! the near half in front. Everything is drawn from `t` and the seed.
//!
//! The glow is painted, as on the reference sheet: every stroke (a rune, a ribbon edge, a pillar) has a heat, the heat
//! spreads a step a pixel, and each step is one colour of the ramp, the last one dithered. A flash, the pulse and the
//! blow only raise a stroke's heat, so its halo grows. Additive light is a faint layer on top.

use core::f64::consts::{PI, TAU};

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::halo::Halo;
use pixfx_draw::math::{clamp, rand};

pub struct Restriction;

pub static RESTRICTION: Restriction = Restriction;

const META: Meta = Meta {
    id: "arcane/restriction",
    name: "Restriction",
    desc: "Three rune rings write themselves round the character one character at a time, from the ground up, and spin against each other; a blow anywhere (hitX, hitY) flares the nearest ring from where it lands and jolts it; then the rings unravel from the top, rune by rune.",
    tags: &[
        "arcane",
        "shield",
        "barrier",
        "runes",
        "seal",
        "cultivation",
        "block",
        "character",
        "one-shot",
    ],
    size: (96, 96),
    origin: (48, 82),
    duration: 3.8,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[FEEL.spec(), HIT_X.spec(), HIT_Y.spec()],
};

/// The glow's steps, from its faint dithered edge to the white core (from the parts sheet).
const RAMP: [Rgba; 5] = [
    Rgba::hex24(0x3a2496),
    Rgba::hex24(0x6c48e0),
    Rgba::hex24(0xa888ff),
    Rgba::hex24(0xdcd0ff),
    Rgba::hex24(0xfdfcfe),
];
/// The specks scattered through the ribbons.
const SPECKS: [Rgba; 2] = [Rgba::hex24(0x8fd8ff), Rgba::hex24(0xff9ef0)];
const WHITE: Rgba = Rgba::hex24(0xffffff);
const GLOW: Rgba = Rgba::hex24(0x8c5cff);

/// The rings' heights (knee, waist, head), their half width and half depth, the ribbon's half height, runes a ring.
const RINGS: [f64; 3] = [-9.0, -29.0, -49.0];
const RX: f64 = 22.0;
const RY: f64 = 9.0;
const RIBBON: f64 = 2.0;
const SLOTS: usize = 16;
/// Turns a second of each ring: the middle one against the others.
const SPIN: [f64; 3] = [1.1, -1.4, 1.1];
/// Seconds: writing starts, one rune each `STROKE`; the blow; the unravelling, a ring each `UNDO_RING`, a rune each
/// `UNDO_RUNE`, each rune fading over `PEEL`.
const WRITE: f64 = 0.1;
const STROKE: f64 = 0.019;
const HIT: f64 = 1.8;
const BREAK: f64 = 2.8;
const UNDO_RING: f64 = 0.18;
const UNDO_RUNE: f64 = 0.015;
const PEEL: f64 = 0.4;
const SPARKS: usize = 14;
/// Where the blow lands, in local pixels (the feet at the origin): by default the right side of the waist ring.
const HIT_X: NumOpt = NumOpt::new("hitX", RX).within(-48.0, 48.0);
const HIT_Y: NumOpt = NumOpt::new("hitY", -29.0).within(-80.0, 14.0);

/// Heats: a rune, a ribbon edge, the ribbon between its edges, a pillar. Each step of the ramp is one heat. A rune or
/// a pillar is drawn in the ramp's core colour whatever its heat: the heat only says how far its halo reaches.
const RUNE: f32 = 2.9;
const EDGE: f32 = 2.3;
const FILL: f32 = 1.9;
const PILLAR: f32 = 2.9;
/// Heat lost a pixel out from a stroke: a halo of about one pale step, one violet step and a dither.
const FALL: f32 = 1.4;

/// The canvas the halos cover: its size and where the origin sits in it.
const W: usize = 96;
const H: usize = 96;
const OX: f64 = 48.0;
const OY: f64 = 82.0;

/// The runes, 3 by 5, a row each as bits from the left.
const GLYPHS: [[u8; 5]; 5] = [
    [0b010, 0b111, 0b010, 0b110, 0b010],
    [0b010, 0b101, 0b111, 0b101, 0b010],
    [0b110, 0b011, 0b100, 0b011, 0b101],
    [0b101, 0b010, 0b111, 0b010, 0b101],
    [0b111, 0b100, 0b110, 0b001, 0b110],
];

impl Effect for Restriction {
    fn meta(&self) -> &Meta {
        &META
    }
    /// A sharp knock where the blow lands, a light one when the seal comes undone.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let b = Blow::new(opts.num(&HIT_X), opts.num(&HIT_Y));
        let mut out = impact(HIT, 0.4, b.x, b.y, opts);
        out.extend(impact(BREAK, 0.15, 0.0, RINGS[2], opts));
        out
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            ramp: opts.palette_or(&RAMP),
            seed: seed as f64,
            blow: Blow::new(opts.num(&HIT_X), opts.num(&HIT_Y)),
        })
    }
}

struct State {
    ramp: Vec<Rgba>,
    seed: f64,
    blow: Blow,
}

/// Where the blow lands on the seal: the ring nearest it, the angle round that ring nearest the point it was aimed
/// at (fixed in the world, not turning with the ring), and that point on the ring.
#[derive(Clone, Copy)]
struct Blow {
    ring: usize,
    angle: f64,
    x: f64,
    y: f64,
}

impl Blow {
    fn new(hx: f64, hy: f64) -> Self {
        let mut ring = 0;
        for (r, &cy) in RINGS.iter().enumerate() {
            if libm::fabs(cy - hy) < libm::fabs(RINGS[ring] - hy) {
                ring = r;
            }
        }
        let cy = RINGS[ring];
        let (u, v) = (hx / RX, (hy - cy) / RY);
        // straight at the middle of the seal: the near side takes it
        let angle = if libm::fabs(u) + libm::fabs(v) < 1e-6 {
            PI / 2.0
        } else {
            libm::atan2(v, u)
        };
        Blow {
            ring,
            angle,
            x: RX * libm::cos(angle),
            y: cy + RY * libm::sin(angle),
        }
    }
}

/// When rune `k` of ring `r` is written.
fn written(r: usize, k: usize) -> f64 {
    WRITE + (r * SLOTS + k) as f64 * STROKE
}

/// When rune `k` of ring `r` peels off: the top ring first, each from where its writing began.
fn peeled(r: usize, k: usize) -> f64 {
    BREAK + (RINGS.len() - 1 - r) as f64 * UNDO_RING + k as f64 * UNDO_RUNE
}

/// The jolt of the blow: the struck ring tilts, the others a little, dying out.
fn jolt(b: &Blow, r: usize, t: f64) -> f64 {
    let since = t - HIT;
    if since < 0.0 {
        return 0.0;
    }
    let k = if r == b.ring { 2.5 } else { 0.8 };
    -k * libm::sin(30.0 * since) * libm::exp(-6.0 * since)
}

/// A point of ring `r` at angle `a` round it (`PI / 2` nearest the viewer), and how near it is (-1 far, 1 near).
/// Under the blow the ring tilts about the axis across the blow: the side struck moves most.
fn at(b: &Blow, r: usize, a: f64, t: f64) -> (f64, f64, f64) {
    let x = RX * libm::cos(a);
    let tilt = jolt(b, r, t) * libm::cos(a - b.angle);
    let y = RINGS[r] + RY * libm::sin(a) + libm::floor(tilt + 0.5);
    (x, y, libm::sin(a))
}

/// How far round a ring two angles are, 0 to `PI`.
fn apart(a: f64, b: f64) -> f64 {
    let d = libm::fmod(libm::fabs(a - b), TAU);
    d.min(TAU - d)
}

/// Ring `r`'s turn at `t`: its slot 0 starts nearest the viewer.
fn turn(r: usize, t: f64) -> f64 {
    PI / 2.0 + SPIN[r] * t
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let depth = |drawn: bool| {
            if drawn {
                Halo::new(W, H, OX, OY)
            } else {
                Halo::none()
            }
        };
        let mut back = depth(layer != Layer::Front);
        let mut front = depth(layer != Layer::Back);
        if layer != Layer::Front {
            self.pillars(&mut back, t);
        }
        for r in 0..RINGS.len() {
            self.ribbon(&mut back, &mut front, r, t);
            for k in 0..SLOTS {
                self.rune(&mut back, &mut front, r, k, t);
            }
        }
        if layer != Layer::Front {
            back.spread(FALL);
            self.paint(p, &back, t);
        }
        if layer != Layer::Back {
            front.spread(FALL);
            self.paint(p, &front, t);
            self.light(p, t);
            self.sparks(p, t);
        }
    }
}

impl State {
    fn c(&self, i: usize) -> Rgba {
        self.ramp
            .get(i.min(self.ramp.len().saturating_sub(1)))
            .copied()
            .unwrap_or(Rgba::NONE)
    }

    /// Each pixel's heat as a step of the ramp: under 1 the dithered edge, then a colour a step, white at the top.
    fn paint(&self, p: &mut Painter, halo: &Halo, t: f64) {
        let flicker = libm::floor(t * 10.0);
        for (x, y, h, kind) in halo.each() {
            // the canvas pixel, for the dither and the specks
            let (gx, gy) = (x + OX, y + OY);
            let step = libm::floor(f64::from(h)) as usize;
            if step == 0 {
                // the faint edge, on every other pixel
                if (gx + gy) % 2.0 == 0.0 {
                    p.dot(x, y, self.c(0), 0.85);
                }
                continue;
            }
            let mut c = match kind {
                2 => WHITE,
                3 => self.c(4),
                _ => self.c(step),
            };
            if kind == 1 && (2..4).contains(&step) {
                // cyan and pink specks through the ribbons, changing ten times a second
                let r = rand(&[gx, gy, flicker, self.seed]);
                if r < 0.06 {
                    c = SPECKS[usize::from(r < 0.03)];
                }
            }
            p.dot(x, y, c, 1.0);
            if step >= 2 || kind >= 2 {
                // the hot strokes bloom a little
                p.light(x, y, GLOW, 0.14);
            }
        }
    }

    /// Ring `r`'s ribbon: two edges with a fill between, drawn behind the pen as it writes, taken away behind the
    /// peeling.
    fn ribbon(&self, back: &mut Halo, front: &mut Halo, r: usize, t: f64) {
        let from = clamp((t - peeled(r, 0)) / (SLOTS as f64 * UNDO_RUNE), 0.0, 1.0);
        let to = clamp((t - written(r, 0)) / (SLOTS as f64 * STROKE), 0.0, 1.0);
        if to <= from {
            return;
        }
        let steps = 200;
        for i in 0..steps {
            // in the ring's own turn, from slot 0 round, half a slot back so the ribbon runs under the first rune
            let u = i as f64 / steps as f64;
            if u < from || u > to {
                continue;
            }
            let a = turn(r, t) + (u - 0.5 / SLOTS as f64) * TAU;
            let (x, y, near) = at(&self.blow, r, a, t);
            let g = if near >= 0.0 { &mut *front } else { &mut *back };
            let dim = if near >= 0.0 { 0.0 } else { 0.5 };
            g.stamp(x, y - RIBBON, EDGE - dim, 1);
            g.stamp(x, y + RIBBON, EDGE - dim, 1);
            for dy in 1..RIBBON as i32 * 2 {
                g.stamp(x, y - RIBBON + f64::from(dy), FILL - dim, 1);
            }
        }
    }

    /// Rune `k` of ring `r`: written with a flash, pulsing while it holds, flaring under the blow, peeling off at
    /// the end.
    fn rune(&self, back: &mut Halo, front: &mut Halo, r: usize, k: usize, t: f64) {
        let born = t - written(r, k);
        if born < 0.0 {
            return;
        }
        let gone = t - peeled(r, k);
        if gone > PEEL {
            return;
        }
        let a = turn(r, t) + k as f64 / SLOTS as f64 * TAU;
        let (mut x, mut y, near) = at(&self.blow, r, a, t);
        let mut fade = 1.0;
        if gone > 0.0 {
            // peeling: drifts out and up, cooling
            let g = gone / PEEL;
            x += libm::cos(a) * 10.0 * g;
            y -= 14.0 * g + 4.0 * rand(&[r as f64, k as f64, self.seed]) * g;
            fade = 1.0 - g;
        }
        // a flash as it is written
        let flash = clamp(1.0 - born / 0.15, 0.0, 1.0);
        // a pulse running along the ring while it holds
        let head = (t * 9.0 + r as f64 * 4.0) % SLOTS as f64;
        let d = libm::fabs(k as f64 - head);
        let pulse = clamp(1.0 - d.min(SLOTS as f64 - d) / 1.5, 0.0, 1.0)
            * clamp((t - written(2, SLOTS - 1)) / 0.2, 0.0, 1.0);
        // the blow flares the struck ring from where it lands round
        let since = t - HIT;
        let flare = if r == self.blow.ring && since >= 0.0 {
            let delay = apart(a, self.blow.angle) / PI * 0.12;
            if since >= delay {
                clamp(1.0 - (since - delay) / 0.35, 0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        let hot = flash.max(flare);
        let far = near < 0.0;
        let heat = ((f64::from(RUNE) + 1.0 * pulse + 2.0 * hot) * fade
            - if far { 0.5 } else { 0.0 }) as f32;
        let kind = if hot > 0.3 { 2 } else { 3 };
        let glyph = GLYPHS[(rand(&[r as f64, k as f64, self.seed, 7.0]) * GLYPHS.len() as f64)
            as usize
            % GLYPHS.len()];
        let side = libm::fabs(libm::cos(a)) > 0.85;
        let (gx, gy) = (libm::floor(x), libm::floor(y) - 2.0);
        let g = if far { back } else { front };
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..3 {
                // seen from behind the rune is mirrored; seen edge-on only its middle stroke shows
                let bit = if far { col } else { 2 - col };
                if side && col != 1 {
                    continue;
                }
                if bits >> bit & 1 == 0 {
                    continue;
                }
                g.stamp(gx + col as f64 - 1.0, gy + row as f64, heat, kind);
            }
        }
    }

    /// The vertical strokes joining the rings once they are written, behind the character.
    fn pillars(&self, back: &mut Halo, t: f64) {
        let up = clamp((t - written(2, SLOTS - 1)) / 0.3, 0.0, 1.0);
        let down = clamp(1.0 - (t - BREAK) / 0.3, 0.0, 1.0);
        let k = up * down;
        if k <= 0.0 {
            return;
        }
        // drawn from the bottom ring up as they appear, from the top down as they go
        let top = RINGS[2] + (RINGS[0] - RINGS[2]) * (1.0 - up);
        let bottom = RINGS[0] - (RINGS[0] - RINGS[2]) * (1.0 - down);
        for x in [-RX + 1.0, 0.0, RX - 1.0] {
            let mut y = top;
            while y <= bottom {
                back.stamp(x, y, PILLAR, 3);
                back.stamp(x + 1.0, y, PILLAR - 0.5, 1);
                y += 1.0;
            }
        }
    }

    /// The faint additive layer: a soft bloom over the seal while it stands.
    fn light(&self, p: &mut Painter, t: f64) {
        let up = clamp((t - written(2, SLOTS - 1)) / 0.3, 0.0, 1.0);
        let down = clamp(1.0 - (t - BREAK) / 0.4, 0.0, 1.0);
        let k = up * down;
        if k > 0.0 {
            p.glow(0.0, RINGS[1], 40.0, GLOW, 0.4 * k);
            for cy in RINGS {
                p.glow(0.0, cy, 28.0, GLOW, 0.1 * k);
            }
        }
    }

    /// Rune sparks thrown out where the blow lands.
    fn sparks(&self, p: &mut Painter, t: f64) {
        let since = t - HIT;
        if !(0.0..0.5).contains(&since) {
            return;
        }
        let (x0, y0) = (self.blow.x, self.blow.y);
        // out from the side struck; a blow on the near face sprays all round
        let side = libm::cos(self.blow.angle);
        let base = if side >= 0.0 { 0.0 } else { PI };
        let spread = 2.4 + 3.9 * (1.0 - libm::fabs(side));
        p.glow(x0, y0, 10.0, WHITE, 1.2 * (1.0 - since * 2.0));
        p.glow(x0, y0, 22.0, GLOW, 0.6 * (1.0 - since * 2.0));
        for i in 0..SPARKS {
            let fi = i as f64;
            let life = 0.3 + 0.2 * rand(&[fi, self.seed, 31.0]);
            if since > life {
                continue;
            }
            let ang = base + (rand(&[fi, self.seed, 33.0]) - 0.5) * spread;
            let speed = 40.0 + 50.0 * rand(&[fi, self.seed, 35.0]);
            let x = x0 + libm::cos(ang) * speed * since;
            let y = y0 + libm::sin(ang) * speed * since + 40.0 * since * since;
            let k = 1.0 - since / life;
            let c = if i % 3 == 0 { WHITE } else { self.c(3) };
            p.dot(libm::floor(x), libm::floor(y), c, k);
            p.light(x, y, GLOW, 0.9 * k);
        }
    }
}
