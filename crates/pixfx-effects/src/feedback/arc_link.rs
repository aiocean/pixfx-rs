//! A lightning arc grows from a supplied point to the sprite and flashes at the contact.

use libm::{ceil, hypot, sin};
use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Actor, Canvas, ColorOpt, Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::color::WHITE;
use pixfx_draw::{Painter, Rgba};
use std::sync::Arc;
const FROM_X: NumOpt = NumOpt::new("fromX", -46.0).within(-110.0, 110.0);
const FROM_Y: NumOpt = NumOpt::new("fromY", 0.0).within(-110.0, 110.0);
const CONTACT_X: NumOpt = NumOpt::new("contactX", 0.0).within(-48.0, 48.0);
const CONTACT_Y: NumOpt = NumOpt::new("contactY", -12.0).within(-48.0, 48.0);
const TINT: ColorOpt = ColorOpt::new("color", Rgba::hex24(0x69daff));
const SECS: f64 = 0.27;
pub struct ArcLink;
pub static ARC_LINK: ArcLink = ArcLink;
const META: Meta = Meta {
    id: "feedback/arc-link",
    name: "Arc Link",
    desc: "A lightning arc grows from a supplied point to the sprite and flashes at the contact.",
    tags: &["feedback", "contact"],
    size: (128, 128),
    origin: (64, 64),
    duration: SECS,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[
        FEEL.spec(),
        FROM_X.spec(),
        FROM_Y.spec(),
        CONTACT_X.spec(),
        CONTACT_Y.spec(),
        TINT.spec(),
    ],
};
impl Effect for ArcLink {
    fn meta(&self) -> &Meta {
        &META
    }
    fn canvas(&self, opts: &Options) -> Canvas {
        let room = |offset: f64| (ceil(offset.abs()) as u32 + 12).max(64);
        let x = room(opts.num(&FROM_X));
        let y = room(opts.num(&FROM_Y));
        Canvas {
            size: (x * 2, y * 2),
            origin: (x as i32, y as i32),
        }
    }

    fn wraps_actor(&self) -> bool {
        true
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(
            5.0 / 60.0,
            0.12,
            opts.num(&CONTACT_X),
            opts.num(&CONTACT_Y),
            opts,
        )
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            from: (opts.num(&FROM_X), opts.num(&FROM_Y)),
            requested: (opts.num(&CONTACT_X), opts.num(&CONTACT_Y)),
            target: None,
            tint: opts
                .palette
                .as_deref()
                .and_then(|ramp| ramp.get(ramp.len() / 2))
                .copied()
                .unwrap_or_else(|| opts.color(&TINT)),
            phase: (seed % 97) as f64,
        })
    }
}
struct State {
    from: (f64, f64),
    requested: (f64, f64),
    target: Option<(f64, f64)>,
    tint: Rgba,
    phase: f64,
}
impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.target = actor.and_then(|a| {
            let (x, y) = self.requested;
            a.opaque()
                .min_by(|a, b| {
                    let dist = |i, j| hypot(f64::from(i) - x, f64::from(j) - y);
                    dist(a.0, a.1).total_cmp(&dist(b.0, b.1))
                })
                .map(|(x, y, _)| (f64::from(x), f64::from(y)))
        });
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let Some((tx, ty)) = self.target else {
            return;
        };
        let (sx, sy) = self.from;
        let (dx, dy) = (tx - sx, ty - sy);
        let len = hypot(dx, dy).max(1.0);
        let progress = (t / 0.072).clamp(0.0, 1.0);
        let fade = 1.0 - t / SECS;
        let count = (ceil(len) as u32).max(1);
        for i in 0..=(f64::from(count) * progress) as u32 {
            let f = f64::from(i) / f64::from(count);
            let knot = f * 10.0;
            let low = libm::floor(knot);
            let u = knot - low;
            let wave = |n: f64| sin(n * 58.7 + self.phase) * sin(n * 0.3141592653589793) * 4.0;
            let shift = wave(low) * (1.0 - u) + wave(low + 1.0) * u;
            let (x, y) = (
                sx + dx * f - dy / len * shift,
                sy + dy * f + dx / len * shift,
            );
            p.dot(x, y, WHITE, fade);
            p.glow(x, y, 2.3, self.tint, fade * 0.37);
        }
        if progress >= 1.0 {
            p.glow(tx, ty, 6.0, self.tint, fade);
        }
    }
}
