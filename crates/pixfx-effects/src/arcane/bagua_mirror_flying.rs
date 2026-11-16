//! Bagua mirror in flight: the bagua mirror spinning like a thrown coin, its face and edge in turn, smearing behind
//! it: its trailing edge, the jade rim, swept along the way it flew (`pixfx_draw::sweep`, the trace the scene hands
//! it), so the smear comes off the mirror's own glowing edge, curving with its flight, narrowing, cooling to deep teal
//! and breaking up toward the end. With no trace (its gif) it smears as if flying straight along +x. A projectile: the scene flies it and turns it to face where it goes
//! (`arcane/bagua-mirror-strike`'s volley, or a game moving it itself). It leaves at the full size it stood at behind
//! the caster, on the back layer for its first moment so it comes out from behind them, and shrinks to half
//! (`FLIGHT_R`). Made here from `pixfx-ref`'s
//! `examples/specs/shield/bagua-mirror.json` sheets; its golden hashes pin it.

use std::sync::Arc;

use core::f64::consts::TAU;

use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Trace, TracePoint};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::halo::Halo;
use pixfx_draw::math::smooth;
use pixfx_draw::sweep::{Brush, Look as SweepLook, sweep};

use super::bagua::{self, Look};

pub struct BaguaMirrorFlying;

pub static BAGUA_MIRROR_FLYING: BaguaMirrorFlying = BaguaMirrorFlying;

const META: Meta = Meta {
    id: "arcane/bagua-mirror-flying",
    name: "Bagua mirror in flight",
    desc: "The bagua mirror spinning like a thrown coin, smearing its own edge behind it along the way it flies, shrinking from full size to half as it leaves: a projectile to fly and turn.",
    tags: &["arcane", "projectile", "mirror", "cultivation", "loop"],
    size: (120, 76),
    origin: (76, 38),
    duration: 1.2,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

/// The radius it flies at, and the radius it leaves at (the strike form's mirror on a stand-in body).
pub(super) const FLIGHT_R: f64 = 15.0;
const START_R: f64 = 30.0;
/// Seconds it takes to shrink; turns a second it spins.
const SHRINK: f64 = 0.2;
const SPIN: f64 = 2.5;
/// Seconds it is drawn behind the caster, leaving from behind their back.
const BEHIND: f64 = 0.15;
/// The smear: seconds of its flight it reaches back (about 34 pixels at `SPEED`), and the speed its preview flies at
/// (the strike form's).
const SMEAR: f64 = 0.14;
const SPEED: f64 = 240.0;
/// How the smear's jade cools toward its end, white at the rim to deep teal.
const FADE: [Rgba; 6] = [
    bagua::WHITE,
    bagua::JADE_HI,
    bagua::JADE,
    bagua::RAMP[0],
    bagua::TEAL,
    bagua::BAND,
];

const W: usize = 120;
const H: usize = 76;
const OX: f64 = 76.0;
const OY: f64 = 38.0;

impl Effect for BaguaMirrorFlying {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(Flying { trace: None })
    }
    fn traces(&self, _opts: &Options) -> Option<f64> {
        Some(SMEAR)
    }
}

struct Flying {
    trace: Option<Arc<Trace>>,
}

/// The mirror's rim as painted (`bagua::paint_halo`): each lit halo pixel with its step's colour, the brighter steps
/// lit, for its back edge to be smeared (`Brush::edge`).
fn rim(halo: &Halo) -> impl Iterator<Item = (f64, f64, Rgba, f64)> + '_ {
    halo.each().filter_map(|(x, y, h, kind)| {
        let step = libm::floor(f64::from(h)) as usize;
        if kind != 0 || step == 0 {
            return None;
        }
        let color = *bagua::RAMP.get(step.min(bagua::RAMP.len() - 1))?;
        let light = if color == bagua::WHITE || color == bagua::JADE_HI {
            1.0
        } else {
            0.5
        };
        Some((x, y, color, light))
    })
}

/// With no trace handed over (its gif, sheet or page): the way of a mirror flying straight along +x at `SPEED`.
fn straight() -> Trace {
    Trace {
        points: (0..=20)
            .map(|i| {
                let age = f64::from(i) * SMEAR / 20.0;
                TracePoint {
                    x: -SPEED * age,
                    y: 0.0,
                    angle: 0.0,
                    age,
                }
            })
            .collect(),
    }
}

impl EffectState for Flying {
    fn set_trace(&mut self, trace: Option<Arc<Trace>>) {
        self.trace = trace;
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        // behind the caster as it leaves (it stood behind their back), in front once clear of them
        let side = if t < BEHIND {
            Layer::Back
        } else {
            Layer::Front
        };
        if layer != side {
            return;
        }
        let r = START_R + (FLIGHT_R - START_R) * smooth(0.0, SHRINK, t);
        let mut m = Look::new(0.0, 0.0, r);
        m.turn = TAU * SPIN * t;
        m.swirl = t * 4.0;
        m.lit = (libm::floor(t * 16.0) as usize) % 8;
        m.lit_k = 1.0;
        let mut halo = Halo::new(W, H, OX, OY);
        let px = bagua::mirror(&mut halo, &m);
        halo.spread(bagua::FALL);
        // the trail first, under the mirror: its trailing edge smeared along the way it flew
        let preview;
        let trace = match &self.trace {
            Some(t) => t.as_ref(),
            None => {
                preview = straight();
                &preview
            }
        };
        let frame = libm::floor(t * 30.0 + 1e-6);
        let look = SweepLook {
            secs: SMEAR,
            fade: 4.0,
            taper: 0.5,
            wave: 0.0,
            phase: 0.0,
            breakup: 0.35,
            shift: frame,
            light: 0.4,
            skip: 0.0,
        };
        sweep(
            p,
            &trace.poses(SMEAR),
            &Brush::edge(rim(&halo), 0),
            &FADE,
            &look,
        );
        bagua::paint_halo(p, &halo);
        bagua::put(p, &px);
        p.glow(0.0, 0.0, r * 1.3, bagua::GLOW, 0.35);
    }
}
