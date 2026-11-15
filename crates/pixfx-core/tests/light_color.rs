//! `light_color`: the colour an effect lights its surroundings in.

use pixfx_core::effect::{Effect, EffectState, Layer, Meta, Options};
use pixfx_core::light_color;
use pixfx_draw::color::Rgba;
use pixfx_draw::painter::Painter;

/// A one-shot that lights orange for its first half and blue, twice as bright, for its second; it can paint
/// colour only instead.
struct Lamp {
    meta: Meta,
    lit: bool,
}

struct LampState {
    lit: bool,
}

impl Effect for Lamp {
    fn meta(&self) -> &Meta {
        &self.meta
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(LampState { lit: self.lit })
    }
}

impl EffectState for LampState {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        if !self.lit {
            p.put(2.0, 2.0, Rgba(255, 0, 0, 255), 1.0);
            return;
        }
        let (c, n) = if t < 0.5 {
            (Rgba(200, 100, 0, 255), 1)
        } else {
            (Rgba(0, 0, 100, 255), 4)
        };
        for x in 0..n {
            p.add(f64::from(4 + x), 4.0, c, 1.0);
        }
    }
}

const fn lamp_meta(id: &'static str) -> Meta {
    Meta {
        id,
        name: "Lamp",
        desc: "A light that changes colour halfway.",
        tags: &["test"],
        size: (12, 10),
        origin: (0, 0),
        duration: 1.0,
        looping: false,
        fps: 8,
        seed: 1,
        state: None,
        options: &[],
    }
}

static LAMP: Lamp = Lamp {
    meta: lamp_meta("test/lamp"),
    lit: true,
};
static PAINT: Lamp = Lamp {
    meta: lamp_meta("test/paint"),
    lit: false,
};

#[test]
fn light_color_is_the_brightness_weighted_mean_brightened() {
    // 4 samples of one orange pixel (200, 100, 0) and 4 of four blue ones (0, 0, 100):
    // sum (800, 400, 1600), brightest channel to 255
    assert_eq!(
        light_color(&LAMP, Options::default()),
        Some(Rgba(128, 64, 255, 255))
    );
}

#[test]
fn an_effect_that_casts_no_light_has_no_light_color() {
    assert_eq!(light_color(&PAINT, Options::default()), None);
}
