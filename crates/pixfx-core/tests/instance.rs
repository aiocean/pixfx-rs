//! Frame rendering against pixelfx `tools/lib/fxlib.mjs` (`renderFrames`, `edgeFade`).

use pixfx_core::color::Rgba;
use pixfx_core::effect::{Effect, EffectState, Layer, Meta, Options};
use pixfx_core::painter::Painter;
use pixfx_core::render::render_frames;

/// Light everywhere, to see the edge fade.
struct Glow {
    meta: Meta,
}

struct GlowState;

impl Effect for Glow {
    fn meta(&self) -> &Meta {
        &self.meta
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(GlowState)
    }
}

impl EffectState for GlowState {
    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Front {
            for y in -4..184 {
                for x in -4..324 {
                    p.add(f64::from(x), f64::from(y), Rgba(200, 100, 50, 255), 1.0);
                }
            }
        }
    }
}

const fn glow_meta(size: (u32, u32)) -> Meta {
    Meta {
        id: "test/glow",
        name: "Glow",
        desc: "Light over the whole canvas.",
        tags: &["test"],
        size,
        origin: (0, 0),
        duration: 1.0,
        looping: true,
        fps: 2,
        seed: 1,
    }
}

static GLOW: Glow = Glow {
    meta: glow_meta((24, 20)),
};
static GLOW_SCREEN: Glow = Glow {
    meta: glow_meta((320, 180)),
};

const FULL: u32 = 0xff3264c8;

#[test]
fn render_fades_light_at_the_edges() {
    let r = render_frames(&GLOW, &Options::default(), 0);
    let f = &r.frames[0];
    // (x, y, light) from pixelfx edgeFade (E = 8) on a 24 x 20 frame of light 0xff3264c8
    let expected = [
        (0, 0, 0xff010102),
        (3, 0, 0xff010102),
        (5, 4, 0xff1e3b77),
        (9, 9, 0xff3264c8),
        (10, 9, 0xff3264c8),
        (7, 9, 0xff3163c6),
        (8, 8, 0xff3264c8),
        (16, 10, 0xff3163c6),
        (15, 10, 0xff3264c8),
        (23, 19, 0xff010102),
        (12, 2, 0xff0c172e),
    ];
    for (x, y, l) in expected {
        assert_eq!(f.light[y * 24 + x], l, "light at ({x}, {y})");
    }
    // no fade with a margin or for a full-screen effect
    assert!(
        render_frames(&GLOW, &Options::default(), 1).frames[0]
            .light
            .iter()
            .all(|&l| l == FULL)
    );
    assert!(
        render_frames(&GLOW_SCREEN, &Options::default(), 0).frames[0]
            .light
            .iter()
            .all(|&l| l == FULL)
    );
}
