//! Bagua mirror, the strike form: the bagua mirror spins up behind the caster's back as the shield form does, holds
//! there, and when the game orders a strike (`Scene::order`) it is thrown at the point: `arcane/bagua-mirror-flying`
//! spins across to it, shrinking to half, and bursts there as `arcane/bagua-mirror-burst`. The scene flies it to any
//! distance and stops drawing this effect once it is thrown. Made here from `pixfx-ref`'s
//! `examples/specs/shield/bagua-mirror.json` sheets; its golden hashes pin it.
//!
//! It wraps its actor: the mirror is sized from the sprite's body and stands behind its upper body, and a jade line
//! washes down the character's outline as it forms, then a pulse runs down it while it holds. It opens once (the
//! point of light, the spin up) and loops from `OPEN`: the trigrams light clockwise round once, the mirror sways and
//! bobs once and a ripple goes out, every `PERIOD`. The order is aimed from where the mirror stands on a stand-in body
//! (`order` does not see the sprite).

use std::sync::Arc;

use core::f64::consts::{PI, TAU};

use pixfx_core::volley::{Launch, Order, Shot, Volley};
use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::halo::Halo;
use pixfx_draw::math::{clamp, smooth};

use super::bagua::{self, Body, Look, Rim};
use super::bagua_mirror_burst::BAGUA_MIRROR_BURST;
use super::bagua_mirror_flying::{BAGUA_MIRROR_FLYING, FLIGHT_R};

pub struct BaguaMirrorStrike;

pub static BAGUA_MIRROR_STRIKE: BaguaMirrorStrike = BaguaMirrorStrike;

const META: Meta = Meta {
    id: "arcane/bagua-mirror-strike",
    name: "Bagua mirror strike",
    desc: "The bagua mirror spins up behind the back of the sprite it wraps, a jade line washing down its outline, and holds there lighting its trigrams until the game orders a strike: then it spins across to the point, shrinking to half, and bursts there, to any distance.",
    tags: &[
        "arcane",
        "mirror",
        "cultivation",
        "volley",
        "projectile",
        "character",
        "outline",
        "loop",
    ],
    size: (96, 96),
    origin: (48, 82),
    duration: (OPEN_FRAME + PERIOD_FRAMES) / FPS,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

/// The mirror's radius as a share of the body's height, its least and most, and where its centre stands (as the
/// shield form's).
const SIZE: f64 = 0.7;
const R_MIN: f64 = 14.0;
const R_MAX: f64 = 34.0;
const HIGH: f64 = 0.45;
const FOOT: f64 = 2.0;
/// Seconds: the point of light, the mirror fully grown. It loops from `OPEN`, every `PERIOD`, both on whole frames.
const FPS: f64 = 30.0;
const POINT: f64 = 0.1;
const GROWN: f64 = 0.55;
const OPEN_FRAME: f64 = 18.0;
const OPEN: f64 = OPEN_FRAME / FPS;
const PERIOD_FRAMES: f64 = 32.0;
const PERIOD: f64 = PERIOD_FRAMES / FPS;
/// The sway while it holds, either way.
const SWAY: f64 = 0.25;
/// Heats of the outline: resting, in the pulse.
const REST: f64 = 0.8;
const PULSED: f64 = 1.1;
/// The flight: pixels a second, the shortest and longest in seconds; how far it first rises out of the line to the
/// target (radians) before curving in.
const SPEED: f64 = 240.0;
const MIN_FLIGHT: f64 = 0.25;
const MAX_FLIGHT: f64 = 1.2;
const LIFT: f64 = 0.35;

/// The stand-in body's mirror, where an order is aimed from: centre and radius.
const AIM_X: f64 = 0.0;
const AIM_Y: f64 = -24.0;

/// The canvas the halos cover: its size and where the origin sits in it.
const W: usize = 96;
const H: usize = 96;
const OX: f64 = 48.0;
const OY: f64 = 82.0;

/// How the mirror flies at an order: one piece, rising a little out of the line and curving in, bursting as it lands.
static VOLLEY: Volley = Volley {
    piece: &BAGUA_MIRROR_FLYING,
    held: None,
    tail: None,
    hit: Some(&BAGUA_MIRROR_BURST),
    poise: 0.0,
    gap: 0.0,
    speed: SPEED,
    min_secs: MIN_FLIGHT,
    max_secs: MAX_FLIGHT,
    nose: FLIGHT_R,
    sink: FLIGHT_R,
    bend: 0.25,
    bend_min: 8.0,
    bend_max: 36.0,
    spread: (0.0, 0.0),
    ease: 1.3,
    beyond: 0.0,
    lands: true,
    orbit_turns: 0.0,
    orbit_radius: 0.0,
};

impl Effect for BaguaMirrorStrike {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    fn loop_from(&self) -> Option<f64> {
        Some(OPEN)
    }
    fn order(
        &self,
        opts: &Options,
        _t: f64,
        at: (f64, f64),
        flip: bool,
        order: &Order,
    ) -> Vec<Launch> {
        let seed = opts.seed.unwrap_or(META.seed) as f64;
        let sx = if flip { -1.0 } else { 1.0 };
        let from = (at.0 + sx * AIM_X, at.1 + AIM_Y);
        let way = libm::atan2(order.y - from.1, order.x - from.0);
        // rising out of the line first: up on whichever side the target lies
        let lift = if libm::cos(way) >= 0.0 { -LIFT } else { LIFT };
        let shot = Shot {
            at: from,
            heading: way + lift,
        };
        VOLLEY.aim(&[shot], order, seed)
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            body: Body::of(None),
        })
    }
}

struct State {
    body: Body,
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.body = Body::of(actor.as_deref());
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            let m = self.look(t);
            let mut halo = Halo::new(W, H, OX, OY);
            let px = bagua::mirror(&mut halo, &m);
            halo.spread(bagua::FALL);
            bagua::paint_halo(p, &halo);
            self.ripples(p, t);
            bagua::put(p, &px);
            let k = clamp(m.r / self.radius(), 0.0, 1.0) * libm::fabs(libm::cos(m.turn));
            if k > 0.0 {
                p.glow(m.x, m.y, m.r * 1.4, bagua::GLOW, 0.3 * k);
            }
            self.outline(p, t);
        }
        if layer != Layer::Back {
            self.point(p, t);
        }
    }
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

    /// The share of the loop at `t`, 0 to 1; 0 before the loop starts.
    fn cycle(t: f64) -> f64 {
        ((t - OPEN).max(0.0) % PERIOD) / PERIOD
    }

    /// The jade line round the body: washing down as the mirror forms, a pulse running down it each loop.
    fn outline(&self, p: &mut Painter, t: f64) {
        let rim = Rim {
            wash: smooth(GROWN - 0.15, OPEN, t),
            pulse: (t >= OPEN).then(|| Self::cycle(t)),
            flare: 0.0,
            fade: 1.0,
            rest: REST,
            pulsed: PULSED,
            flared: 0.0,
        };
        bagua::outline(p, &self.body, &rim, (W, H), (OX, OY));
    }

    /// The mirror at `t`: spun up as it grows, then swaying, bobbing and lighting its trigrams once a loop.
    fn look(&self, t: f64) -> Look {
        let grow = smooth(POINT, GROWN, t);
        let held = smooth(GROWN, OPEN, t);
        let c = Self::cycle(t);
        let sway = SWAY * libm::sin(TAU * c) * held;
        let spin_in = (1.0 - grow) * 3.0 * PI;
        let bob = libm::floor(libm::sin(TAU * c) + 0.5) * held;
        let (cx, cy) = self.centre();
        let mut m = Look::new(cx, cy + bob, self.radius() * grow);
        m.turn = sway + spin_in;
        m.swirl = TAU * c;
        if t >= OPEN {
            m.lit = (libm::floor(c * 8.0) as usize).min(7);
            m.lit_k = 1.0;
        }
        m
    }

    /// The point of light it grows from.
    fn point(&self, p: &mut Painter, t: f64) {
        let k = clamp(1.0 - (t - POINT) / 0.15, 0.0, 1.0) * clamp(t / 0.04, 0.0, 1.0);
        if k > 0.0 {
            let (cx, cy) = self.centre();
            bagua::star(p, cx, cy, 2, k);
            p.glow(cx, cy, 8.0, bagua::GLOW, 0.8 * k);
        }
    }

    /// The flat ripple rings on the ground under it: one as it forms, then one each loop.
    fn ripples(&self, p: &mut Painter, t: f64) {
        let (cx, _) = self.centre();
        let (y, max) = (self.body.bottom + 1.0, self.radius() * 1.3);
        bagua::ripple(p, cx, y, (t - 0.2) / 0.8, max);
        if t >= OPEN {
            bagua::ripple(p, cx, y, Self::cycle(t), max * 0.9);
        }
    }
}
