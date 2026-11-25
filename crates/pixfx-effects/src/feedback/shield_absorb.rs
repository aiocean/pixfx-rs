//! A local dent rebounds while two light fronts run along the struck contour.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use libm::hypot;
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{
    Actor, Area, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette,
};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::js_round;
use pixfx_draw::ruler::{RULER, Ruler};

pub struct ShieldAbsorb;
pub static SHIELD_ABSORB: ShieldAbsorb = ShieldAbsorb;

const X: NumOpt = NumOpt::new("contactX", -12.0).within(-64.0, 64.0);
const Y: NumOpt = NumOpt::new("contactY", 0.0).within(-64.0, 64.0);
const STRENGTH: NumOpt = NumOpt::new("strength", 1.0).within(0.25, 2.0);
const TINT: ColorOpt = ColorOpt::new("color", Rgba::hex24(0x62e6ff));
const SECS: f64 = 0.62;
const META: Meta = Meta {
    id: "feedback/shield-absorb",
    name: "Shield Absorb",
    desc: "The sprite dents at the point of impact, rebounds, and sends light along its edge.",
    tags: &["feedback", "shield", "contact"],
    size: (160, 160),
    origin: (80, 80),
    duration: SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[
        FEEL.spec(),
        X.spec(),
        Y.spec(),
        STRENGTH.spec(),
        TINT.spec(),
    ],
};

impl Effect for ShieldAbsorb {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(
            0.0,
            (0.40 * opts.num(&STRENGTH)).min(0.69),
            opts.num(&X),
            opts.num(&Y),
            opts,
        )
    }

    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        Box::new(State {
            requested: (opts.num(&X), opts.num(&Y)),
            strength: opts.num(&STRENGTH),
            color: opts.color(&TINT),
            actor: None,
            contact: (0, 0),
            inward: (1.0, 0.0),
            edge: Vec::new(),
            run: 1.0,
        })
    }
}

struct State {
    requested: (f64, f64),
    strength: f64,
    color: Rgba,
    actor: Option<Arc<Actor>>,
    contact: (i32, i32),
    inward: (f64, f64),
    edge: Vec<((i32, i32), u32)>,
    run: f64,
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.edge.clear();
        self.run = 1.0;
        self.actor = actor;
        let Some(actor) = &self.actor else { return };
        let area = Area {
            x: actor.x(),
            y: actor.y(),
            w: actor.w(),
            h: actor.h(),
        };
        let shape = Silhouette::of(actor, area);
        let nearest = |(x, y): (i32, i32)| {
            hypot(
                f64::from(x) - self.requested.0,
                f64::from(y) - self.requested.1,
            )
        };
        let Some(contact) = shape
            .edge()
            .min_by(|&a, &b| nearest(a).total_cmp(&nearest(b)))
        else {
            return;
        };
        self.contact = contact;
        let Some(area) = shape.bounds() else { return };
        let dx = f64::from(area.x) + (f64::from(area.w) - 1.0) * 0.5 - f64::from(contact.0);
        let dy = f64::from(area.y) + (f64::from(area.h) - 1.0) * 0.5 - f64::from(contact.1);
        let length = hypot(dx, dy);
        self.inward = if length > 0.0 {
            (dx / length, dy / length)
        } else {
            (1.0, 0.0)
        };

        // Walk only adjacent edge cells, so a front cannot jump across a notch or an alpha gap.
        let mut remaining: HashSet<_> = shape.edge().collect();
        remaining.remove(&contact);
        let mut queue = VecDeque::from([(contact, 0)]);
        while let Some(((x, y), distance)) = queue.pop_front() {
            self.edge.push(((x, y), distance));
            self.run = self.run.max(f64::from(distance));
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let next = (x + dx, y + dy);
                    if remaining.remove(&next) {
                        queue.push_back((next, distance + 1));
                    }
                }
            }
        }
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let Some(actor) = &self.actor else { return };
        if layer != Layer::Front || self.edge.is_empty() {
            return;
        }
        // A dent spans roughly one blade length with a peak travel of a quarter blade.
        let radius = f64::from(Ruler::size(RULER.blade, 1.0));
        let depth = f64::from(Ruler::size(RULER.blade, 0.25));
        let release = (1.0 - t / 0.15).clamp(0.0, 1.0);
        let recoil = release * release;
        let displacement = |x: f64, y: f64| {
            if recoil == 0.0 {
                return (0.0, 0.0);
            }
            let distance = hypot(x - f64::from(self.contact.0), y - f64::from(self.contact.1));
            let amount = (1.0 - distance / radius).max(0.0) * recoil * depth * self.strength;
            (self.inward.0 * amount, self.inward.1 * amount)
        };

        // Inverse sample so the rebounding sprite keeps solid pixel coverage.
        for j in 0..actor.h() {
            for i in 0..actor.w() {
                let (x, y) = (
                    f64::from(actor.x()) + f64::from(i),
                    f64::from(actor.y()) + f64::from(j),
                );
                let (dx, dy) = displacement(x, y);
                let sx = js_round(f64::from(i) - dx) as i64;
                let sy = js_round(f64::from(j) - dy) as i64;
                if sx >= 0 && sy >= 0 {
                    let word = actor.at(sx as u32, sy as u32);
                    if word >> 24 != 0 {
                        p.paint_cell(x, y, Rgba::unpack(word), 1.0);
                    }
                }
            }
        }

        let width = f64::from(Ruler::size(RULER.blade, 0.4));
        let front = (self.run + width) * (t / 0.54);
        let fade = (1.0 - t / SECS).clamp(0.0, 1.0);
        for &((x, y), distance) in &self.edge {
            let k = (1.0 - (f64::from(distance) - front).abs() / width).max(0.0) * fade;
            if k > 0.0 {
                let (x, y) = (f64::from(x), f64::from(y));
                let (dx, dy) = displacement(x, y);
                let color = mix(self.color, WHITE, k * 0.82);
                p.glow(x + dx, y + dy, 2.5, color, k * 0.64);
            }
        }
        let (x, y) = (f64::from(self.contact.0), f64::from(self.contact.1));
        let (dx, dy) = displacement(x, y);
        p.glow(x + dx, y + dy, 5.0, self.color, recoil);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixfx_core::OptValue;

    #[test]
    fn light_follows_the_struck_edge_without_jumping_gaps() -> Result<(), String> {
        let mut opts = Options::default();
        opts.extra.insert("contactX".into(), OptValue::Num(0.0));
        let mut state = State {
            requested: (opts.num(&X), opts.num(&Y)),
            strength: 1.0,
            color: WHITE,
            actor: None,
            contact: (0, 0),
            inward: (1.0, 0.0),
            edge: Vec::new(),
            run: 1.0,
        };
        // A C-shaped component plus an isolated pixel across its opening.
        let pixels = [
            true, true, true, false, false, true, false, false, false, true, true, true, true,
            false, false,
        ]
        .map(|on| if on { u32::MAX } else { 0 });
        let actor = Actor::new(0, 0, 5, 3, pixels.to_vec()).map_err(|e| e.to_string())?;
        state.set_actor(Some(Arc::new(actor)));
        assert_eq!(state.edge.first(), Some(&((0, 0), 0)));
        assert!(state.edge.contains(&((2, 2), 3)));
        assert!(state.edge.iter().all(|&((x, _), _)| x < 3));
        state.set_actor(None);
        assert!(state.edge.is_empty());
        Ok(())
    }
}
