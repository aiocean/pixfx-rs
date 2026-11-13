//! Frame rendering against pixelfx `tools/lib/fxlib.mjs` (`renderFrames`, `edgeFade`).

use std::collections::BTreeMap;

use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::effect::{Canvas, Effect, EffectState, Layer, Meta, NumOpt, OptValue, Options};
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
        state: None,
        options: &[],
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

/// A test effect that sizes its canvas from its `width` option (8 when unset), 4 tall, origin mid-width on
/// row 3; it lights its origin.
struct Wide;

struct WideState;

static WIDE_META: Meta = Meta {
    id: "test/wide",
    name: "Wide",
    desc: "A canvas as wide as its width option.",
    tags: &["test"],
    size: (8, 4),
    origin: (4, 3),
    duration: 1.0,
    looping: true,
    fps: 30,
    seed: 5,
    state: None,
    options: &[],
};

impl Effect for Wide {
    fn meta(&self) -> &Meta {
        &WIDE_META
    }
    fn canvas(&self, opts: &Options) -> Canvas {
        let w = opts.num(&NumOpt::new("width", 8.0)) as u32;
        Canvas {
            size: (w, 4),
            origin: ((w / 2) as i32, 3),
        }
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(WideState)
    }
}

impl EffectState for WideState {
    fn draw(&self, p: &mut Painter, _t: f64, _layer: Layer) {
        p.px(0.0, 0.0, WHITE);
    }
}

#[test]
fn render_sizes_frames_from_the_instances_canvas() {
    let opts = Options {
        extra: BTreeMap::from([("width".to_string(), OptValue::Num(20.0))]),
        ..Options::default()
    };
    let r = render_frames(&Wide, &opts, 2);
    assert_eq!(
        (r.w, r.h),
        (24, 8),
        "the canvas its options give, plus the margin"
    );
    // its origin (10, 3) sits 2 px in from the frame's corner
    let lit = r
        .frames
        .first()
        .and_then(|f| (0..f.w).find(|&x| f.color[(5 * f.w + x) as usize] != 0));
    assert_eq!(lit, Some(12));
    assert_eq!(
        r.origin,
        (10, 3),
        "the origin on the canvas its options give"
    );
}
