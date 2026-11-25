//! The contact lights up, then particles carry the sprite's outline outward.

use super::wave::{CONTACT_X, CONTACT_Y, REACH, center, nearest, tint};
use libm::{hypot, sin};
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, Area, ColorOpt, Effect, EffectState, Layer, Meta, Options, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use std::sync::Arc;
const TINT: ColorOpt = ColorOpt::new("color", Rgba::hex24(0xffca74));
const RING_AT: f64 = 0.10;
const SECS: f64 = 0.55;
pub struct Resonance;
pub static RESONANCE: Resonance = Resonance;
const META: Meta = Meta {
    id: "impacts/resonance",
    name: "Resonance",
    desc: "The contact lights up, then particles carry the sprite's outline outward.",
    tags: &["impacts", "pulse"],
    size: (152, 152),
    origin: (76, 76),
    duration: SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[
        FEEL.spec(),
        CONTACT_X.spec(),
        CONTACT_Y.spec(),
        REACH.spec(),
        TINT.spec(),
    ],
};
impl Effect for Resonance {
    fn meta(&self) -> &Meta {
        &META
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(RING_AT, 0.61, 0.0, 0.0, opts)
    }
    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        Box::new(State {
            contact: (opts.num(&CONTACT_X), opts.num(&CONTACT_Y)),
            origin: None,
            hit: None,
            reach: opts.num(&REACH),
            tint: tint(opts, &TINT),
            pixels: Vec::new(),
            outline: Vec::new(),
            extent: 1.0,
        })
    }
}
struct State {
    contact: (f64, f64),
    origin: Option<(f64, f64)>,
    hit: Option<(f64, f64)>,
    reach: f64,
    tint: Rgba,
    pixels: Vec<(f64, f64, Rgba)>,
    outline: Vec<(f64, f64)>,
    extent: f64,
}
impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.origin = actor.as_deref().and_then(center);
        self.hit = actor
            .as_deref()
            .and_then(|actor| nearest(actor, self.contact));
        self.pixels.clear();
        self.outline.clear();
        self.extent = 1.0;
        if let (Some(actor), Some((cx, cy))) = (actor, self.origin) {
            let shape = Silhouette::of(
                &actor,
                Area {
                    x: actor.x(),
                    y: actor.y(),
                    w: actor.w(),
                    h: actor.h(),
                },
            );
            self.outline
                .extend(shape.edge().map(|(x, y)| (f64::from(x), f64::from(y))));
            for &(x, y) in &self.outline {
                self.extent = self.extent.max(hypot(x - cx, y - cy));
            }
            self.pixels.extend(
                actor
                    .opaque()
                    .map(|(x, y, c)| (f64::from(x), f64::from(y), c)),
            );
        }
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let (Some((cx, cy)), Some((hx, hy))) = (self.origin, self.hit) else {
            return;
        };
        let strike = (1.0 - t / 0.14).max(0.0);
        let ringing = if t >= RING_AT {
            (1.0 - (t - RING_AT) / 0.34).max(0.0)
        } else {
            0.0
        };
        let sway = sin((t - RING_AT) * 58.0) * ringing * 1.4;
        for &(x, y, color) in &self.pixels {
            let hot = (1.0 - hypot(x - hx, y - hy) / 9.0).max(0.0) * strike;
            if hot > 0.0 || ringing > 0.0 {
                p.paint_cell(
                    x + sway,
                    y,
                    mix(color, WHITE, 0.65 * hot),
                    (hot + 0.42 * ringing).min(1.0),
                );
                p.light(x + sway, y, self.tint, 0.7 * hot);
            }
        }
        for (delay, strength) in [(RING_AT, 1.0), (RING_AT + 0.09, 0.48)] {
            let age = (t - delay) / (SECS - delay);
            if !(0.0..1.0).contains(&age) {
                continue;
            }
            let travel = self.reach * age * (2.0 - age) / self.extent;
            let fade = (1.0 - age) * strength;
            for (i, &(x, y)) in self.outline.iter().enumerate() {
                let drift = 1.0 + (i % 5) as f64 * 0.018;
                let px = x + (x - cx) * travel * drift;
                let py = y + (y - cy) * travel * drift;
                p.dot(px, py, mix(self.tint, WHITE, fade * 0.42), fade);
                if i % 4 == 0 {
                    p.glow(px, py, 2.5, self.tint, fade * 0.46);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixfx_core::render_frames_with_actor;

    #[test]
    fn expanded_outline_depends_on_sprite_shape_at_the_same_center()
    -> Result<(), pixfx_core::ActorError> {
        let outline = |cross: bool| -> Result<Vec<u32>, pixfx_core::ActorError> {
            let pixels = (-6i32..=6)
                .flat_map(|y| {
                    (-6i32..=6).map(move |x| {
                        if !cross || x.abs() <= 2 || y.abs() <= 2 {
                            0xff775533
                        } else {
                            0
                        }
                    })
                })
                .collect();
            let actor = Arc::new(Actor::new(-6, -6, 13, 13, pixels)?);
            let rendered = render_frames_with_actor(&RESONANCE, &Options::default(), 0, |_| {
                Some(Arc::clone(&actor))
            });
            let (cx, cy) = rendered.origin;
            let limit = 8 * rendered.frames[15].density().factor() as i32;
            Ok(rendered.frames[15]
                .color
                .iter()
                .enumerate()
                .filter_map(|(i, &pixel)| {
                    let x = (i % rendered.w as usize) as i32 - cx;
                    let y = (i / rendered.w as usize) as i32 - cy;
                    (x.abs() > limit || y.abs() > limit).then_some(pixel)
                })
                .collect())
        };
        let square = outline(false)?;
        let cross = outline(true)?;
        assert!(square.iter().any(|&p| p != 0) && cross.iter().any(|&p| p != 0));
        assert_ne!(square, cross);
        Ok(())
    }
}
