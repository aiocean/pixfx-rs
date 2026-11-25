//! A fading copy of a supplied sprite pose at a fixed point along its motion.

use std::sync::Arc;

use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, Options};
use pixfx_core::{ColorOpt, NumOpt};
use pixfx_draw::color::mix;
use pixfx_draw::{Painter, color::Rgba};

const TINT: ColorOpt = ColorOpt::new("tint", Rgba::hex24(0x8cb8ff));
const OPACITY: NumOpt = NumOpt::new("opacity", 0.75).within(0.0, 1.0);

pub struct Afterimage;
pub static AFTERIMAGE: Afterimage = Afterimage;

const DURATION: f64 = 0.28;

const META: Meta = Meta {
    id: "feedback/afterimage",
    name: "Afterimage",
    desc: "A fading copy of a supplied sprite pose at a fixed point along its motion.",
    tags: &["feedback", "character"],
    size: (128, 128),
    origin: (64, 64),
    duration: DURATION,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[TINT.spec(), OPACITY.spec()],
};

impl Effect for Afterimage {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            tint: opts.color(&TINT),
            opacity: opts.num(&OPACITY),
            pixels: Vec::new(),
        })
    }
}

struct State {
    tint: Rgba,
    opacity: f64,
    pixels: Vec<(i32, i32, Rgba)>,
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.pixels.clear();
        if let Some(actor) = actor {
            self.pixels.extend(
                actor
                    .opaque()
                    .map(|(x, y, color)| (x, y, mix(color, self.tint, 0.72))),
            );
        }
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            return;
        }
        let t = t / DURATION;
        let alpha = self.opacity * (1.0 - t) * (1.0 - t);
        for &(x, y, color) in &self.pixels {
            p.paint_cell(f64::from(x), f64::from(y), color, alpha);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixfx_draw::{Frame, PixelDensity, color::WHITE};

    #[test]
    fn a_solid_actor_cell_covers_four_texels_at_double_density()
    -> Result<(), pixfx_core::ActorError> {
        let mut state = State {
            tint: WHITE,
            opacity: 1.0,
            pixels: Vec::new(),
        };
        state.set_actor(Some(Arc::new(Actor::new(0, 0, 1, 1, vec![WHITE.pack()])?)));
        let mut frame = Frame::with_density(1, 1, 0, 0, PixelDensity::Two);
        state.draw(&mut Painter::new(&mut frame), 0.0, Layer::Back);
        assert_eq!(frame.color, [WHITE.pack(); 4]);
        Ok(())
    }
}
