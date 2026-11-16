//! Bagua mirror, the shield form: a large octagonal bronze mirror ringed with the eight trigrams stands behind the
//! character's back and throws a blow back as a beam of light (the cultivation-fantasy protective treasure). Made here
//! from `pixfx-ref`'s `examples/specs/shield/bagua-mirror.json` sheets; it has no source, so no parity check; its
//! golden hashes pin it. The mirror itself is [`super::bagua`], shared with `arcane/bagua-mirror-strike`.
//!
//! It wraps its actor: the mirror is sized from the sprite's body (`SIZE` of its height) and stands behind its upper
//! body, its foot near the ground, so the character stays in front of it; once it has formed a jade line washes down
//! the character's outline. It spins up out of a point of jade light behind the chest to full size; while it holds it
//! sways a little, lights its trigrams one after another clockwise and sends a ripple ring out under it. A blow lands
//! anywhere (`hitX`, `hitY`, by default the mirror's right edge): the face flashes white, the outline flares round
//! the whole body, the mirror is knocked away from it, trembles and squashes, springing back as it settles, and the blow goes back the way it came as a straight jade-white beam. At the end
//! it spins faster, shrinks to a point and winks out in shards, the outline fading with it. With no sprite it stands
//! behind a stand-in body. Everything is drawn from `t` and the seed.

use std::sync::Arc;

use core::f64::consts::{PI, TAU};

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::halo::Halo;
use pixfx_draw::math::{clamp, smooth};

use super::bagua::{self, Body, Look, Rim};

pub struct BaguaMirror;

pub static BAGUA_MIRROR: BaguaMirror = BaguaMirror;

const META: Meta = Meta {
    id: "arcane/bagua-mirror",
    name: "Bagua mirror",
    desc: "A large octagonal bronze mirror ringed with the eight trigrams spins up behind the back of the sprite it wraps and sways, lighting its trigrams in turn, while a jade line washes down the character's outline; a blow anywhere (hitX, hitY) flashes its jade face, flares the outline and is thrown back as a beam; then it spins down to a point and winks out.",
    tags: &[
        "arcane",
        "shield",
        "barrier",
        "mirror",
        "reflect",
        "cultivation",
        "block",
        "character",
        "outline",
        "one-shot",
    ],
    size: (96, 96),
    origin: (48, 82),
    duration: 3.6,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[FEEL.spec(), HIT_X.spec(), HIT_Y.spec()],
};

/// The mirror's radius across the flats, as a share of the body's height, and its least and most; where its centre
/// stands, as a share of the body's height down from its top, its foot kept `FOOT` pixels under the feet at most.
const SIZE: f64 = 0.7;
const R_MIN: f64 = 14.0;
const R_MAX: f64 = 34.0;
const HIGH: f64 = 0.45;
const FOOT: f64 = 2.0;
/// Seconds: the point of light, the mirror fully grown, the blow, the end, the shrink's length.
const POINT: f64 = 0.1;
const GROWN: f64 = 0.55;
const HIT: f64 = 1.6;
/// The blow: pixels it is knocked back and trembles, how much it squashes, and seconds until it is still again.
const KNOCK: f64 = 5.0;
const TREMOR: f64 = 2.0;
const SQUASH: f64 = 0.15;
const KNOCK_T: f64 = 0.6;
const BREAK: f64 = 2.9;
const SHRINK: f64 = 0.45;
/// The sway while it holds: its turn either way and its period; a trigram lit each `LIGHT` seconds; a ripple each
/// `RIPPLE` seconds.
const SWAY: f64 = 0.3;
const SWAY_T: f64 = 2.6;
const LIGHT: f64 = 0.12;
const RIPPLE: f64 = 1.1;
/// The beam: its speed and length.
const BEAM_SPEED: f64 = 320.0;
const BEAM_LEN: f64 = 30.0;
/// Heats of the outline: resting, under the blow.
const REST: f64 = 0.8;
const FLARE: f64 = 1.7;
/// Where the blow lands, in local pixels (the feet at the origin): by default the right edge of the mirror on the
/// stand-in body.
const HIT_X: NumOpt = NumOpt::new("hitX", 30.0).within(-48.0, 48.0);
const HIT_Y: NumOpt = NumOpt::new("hitY", -28.0).within(-80.0, 14.0);

/// The canvas the halos cover: its size and where the origin sits in it.
const W: usize = 96;
const H: usize = 96;
const OX: f64 = 48.0;
const OY: f64 = 82.0;

impl Effect for BaguaMirror {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    /// A hard knock where the blow lands, a light one as the mirror winks out.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let mut out = impact(HIT, 0.45, opts.num(&HIT_X), opts.num(&HIT_Y), opts);
        out.extend(impact(BREAK + SHRINK, 0.12, 0.0, -28.0, opts));
        out
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            seed: seed as f64,
            hx: opts.num(&HIT_X),
            hy: opts.num(&HIT_Y),
            body: Body::of(None),
        })
    }
}

struct State {
    seed: f64,
    /// Where the blow lands.
    hx: f64,
    hy: f64,
    body: Body,
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.body = Body::of(actor.as_deref());
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let m = self.look(t);
        let dir = self.dir();
        let since = t - HIT;
        let (bx, by) = (
            m.x + dir * m.r * libm::fabs(libm::cos(m.turn)),
            self.hit_row(),
        );
        if layer != Layer::Front {
            // the mirror behind the back, the outline over it
            let mut halo = Halo::new(W, H, OX, OY);
            let px = bagua::mirror(&mut halo, &m);
            halo.spread(bagua::FALL);
            bagua::paint_halo(p, &halo);
            self.ripples(p, t);
            bagua::put(p, &px);
            self.light(p, &m);
            self.outline(p, t);
        }
        if layer != Layer::Back {
            // what leaves it comes out in front: the beam, the sparks, the point it grows from and ends in
            let mut halo = Halo::new(W, H, OX, OY);
            let px = bagua::beam(&mut halo, bx, by, dir, since - 0.04, BEAM_SPEED, BEAM_LEN);
            halo.spread(bagua::FALL);
            bagua::paint_halo(p, &halo);
            bagua::put(p, &px);
            if (0.0..0.3).contains(&since) {
                p.glow(bx, by, 14.0, bagua::WHITE, 0.9 * (1.0 - since / 0.3));
            }
            self.point(p, t);
            bagua::shards(p, bx, by, since, 10, self.seed, 0.45, dir);
            let (cx, cy) = self.centre();
            bagua::shards(
                p,
                cx,
                cy,
                t - BREAK - SHRINK * 0.6,
                16,
                self.seed + 7.0,
                0.7,
                0.0,
            );
        }
    }
}

/// How the mirror takes a blow `since` seconds ago from side `dir`: knocked away on the frame it lands, then
/// springing back past its place and settling; trembling up and down, a pixel or two each frame, dying out; squashed
/// on contact and swelling back a little past its size. (x, y) in whole pixels and a scale for the radius.
fn knock(since: f64, dir: f64) -> (f64, f64, f64) {
    if !(0.0..KNOCK_T).contains(&since) {
        return (0.0, 0.0, 1.0);
    }
    let fade = |rate: f64| libm::exp(-rate * since);
    let x = libm::floor(-dir * KNOCK * fade(8.0) * libm::cos(22.0 * since) + 0.5);
    let frame = libm::floor(since * 30.0);
    let shiver = libm::floor(TREMOR * fade(9.0) + 0.5);
    let y = if frame.rem_euclid(2.0) == 0.0 {
        shiver
    } else {
        -shiver
    };
    let scale = 1.0 - SQUASH * fade(14.0) * libm::cos(25.0 * since);
    (x, y, scale)
}

impl State {
    /// The mirror's radius on this body.
    fn radius(&self) -> f64 {
        clamp(libm::floor(self.body.height() * SIZE + 0.5), R_MIN, R_MAX)
    }

    /// The mirror's centre at rest: behind the body's middle, `HIGH` of the way down, its foot near the ground.
    fn centre(&self) -> (f64, f64) {
        let b = &self.body;
        let y = (b.top + b.height() * HIGH).min(b.bottom + FOOT - self.radius());
        (b.chest.0, libm::floor(y))
    }

    /// The row the blow lands on, kept on the mirror's face.
    fn hit_row(&self) -> f64 {
        let (_, cy) = self.centre();
        let r = self.radius();
        clamp(self.hy, cy - r + 3.0, cy + r - 3.0)
    }

    /// The side the blow comes from: 1 right, -1 left.
    fn dir(&self) -> f64 {
        if self.hx >= self.centre().0 {
            1.0
        } else {
            -1.0
        }
    }

    /// The jade line round the body: washing down once the mirror has formed, flaring under the blow, fading as the
    /// mirror winks out.
    fn outline(&self, p: &mut Painter, t: f64) {
        let since = t - HIT;
        let flare = if since >= 0.0 {
            clamp(1.0 - (since - 0.06).max(0.0) / 0.3, 0.0, 1.0)
        } else {
            0.0
        };
        let rim = Rim {
            wash: smooth(GROWN - 0.1, GROWN + 0.3, t),
            pulse: None,
            flare,
            fade: 1.0 - smooth(0.0, SHRINK, t - BREAK),
            rest: REST,
            pulsed: 0.0,
            flared: FLARE,
        };
        bagua::outline(p, &self.body, &rim, (W, H), (OX, OY));
    }

    /// The mirror at `t`: grown, turned, lit, flashing, jolted.
    fn look(&self, t: f64) -> Look {
        let grow = smooth(POINT, GROWN, t);
        let shrink = 1.0 - smooth(0.0, SHRINK, t - BREAK);
        let s = grow * shrink;
        // spun in as it grows, faster and faster as it shrinks
        let held = smooth(GROWN, GROWN + 0.3, t);
        let sway = SWAY * libm::sin(TAU * (t - GROWN) / SWAY_T) * held;
        let spin_in = (1.0 - grow) * 3.0 * PI;
        let b = (t - BREAK).max(0.0);
        let spin_out = b * b * 40.0;
        let since = t - HIT;
        let (mut jolt, mut flash) = (0.0, 0.0);
        let hit = knock(since, self.dir());
        if since >= 0.0 {
            jolt = -self.dir() * 0.6 * libm::sin(25.0 * since) * libm::exp(-7.0 * since);
            flash = clamp(1.0 - since / 0.25, 0.0, 1.0);
        }
        let bob = libm::floor(libm::sin(TAU * t / 1.6) + 0.5) * held;
        let (cx, cy) = self.centre();
        let mut m = Look::new(cx + hit.0, cy + bob + hit.1, self.radius() * s * hit.2);
        m.turn = sway + spin_in + spin_out + jolt;
        m.flash = flash;
        m.swirl = t * 2.0;
        if t > GROWN && t < BREAK {
            m.lit = (libm::floor((t - GROWN) / LIGHT) as i64).rem_euclid(8) as usize;
            m.lit_k = 1.0;
        }
        m
    }

    /// The point of light it grows from and winks out to.
    fn point(&self, p: &mut Painter, t: f64) {
        let start = clamp(1.0 - (t - POINT) / 0.15, 0.0, 1.0) * clamp(t / 0.04, 0.0, 1.0);
        let end = t - BREAK - SHRINK;
        let wink = if (-0.1..0.12).contains(&end) {
            1.0 - libm::fabs(end) / 0.12
        } else {
            0.0
        };
        let k = start.max(wink);
        if k > 0.0 {
            let (cx, cy) = self.centre();
            bagua::star(p, cx, cy, 2, k);
            p.glow(cx, cy, 8.0, bagua::GLOW, 0.8 * k);
        }
    }

    /// The flat ripple rings on the ground under it: one as it forms, then one each `RIPPLE` seconds while it holds.
    fn ripples(&self, p: &mut Painter, t: f64) {
        let (cx, _) = self.centre();
        let (y, max) = (self.body.bottom + 1.0, self.radius() * 1.3);
        if t < BREAK {
            bagua::ripple(p, cx, y, (t - 0.2) / 0.8, max);
        }
        if t > GROWN && t < BREAK + 0.3 {
            bagua::ripple(p, cx, y, ((t - GROWN) % RIPPLE) / RIPPLE, max * 0.9);
        }
    }

    /// The faint additive bloom over the jade face, brighter as it flashes.
    fn light(&self, p: &mut Painter, m: &Look) {
        let k = clamp(m.r / self.radius(), 0.0, 1.0) * libm::fabs(libm::cos(m.turn));
        if k > 0.0 {
            p.glow(m.x, m.y, m.r * 1.4, bagua::GLOW, (0.25 + 0.8 * m.flash) * k);
        }
    }
}
