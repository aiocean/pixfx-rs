//! The violet portal rim from the supplied reference, with painted radial sparks.

use std::borrow::Cow;

use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RenderKind};
use pixfx_draw::color::{Rgba, blend, luma};
use pixfx_draw::math::js_round;
use pixfx_draw::{Painter, RULER, Ruler};

pub struct VioletPortal;

pub static VIOLET_PORTAL: VioletPortal = VioletPortal;

const FPS: u32 = 14;
const POSES: usize = 7;
const META: Meta = Meta {
    id: "feedback/violet-portal",
    name: "Violet Portal",
    desc: "Violet bands sweep around an open portal as sparks break outward from its rim.",
    tags: &["magic", "portal", "loop"],
    size: (72, 72),
    origin: (36, 36),
    duration: POSES as f64 / FPS as f64,
    looping: true,
    fps: FPS,
    seed: 1,
    state: None,
    options: &[],
};

// Reference pixels at game scale: 41 rows is about two thirds of the current fighter height.
const WIDTH: u32 = 34;
const HEIGHT: u32 = 41;
const FRAME_BYTES: usize = WIDTH as usize * HEIGHT as usize * 4;
const RIM: &[u8; POSES * FRAME_BYTES] = include_bytes!("art/violet_portal.rgba");

const VIOLET: Rgba = Rgba::hex24(0x580aff);
const LAVENDER: Rgba = Rgba::hex24(0xc8a4ff);
// The darkest and brightest opaque swatches in the rim strip.
const SHADOW: Rgba = Rgba::hex24(0x3012b4);
const HIGHLIGHT: Rgba = Rgba::hex24(0xddd8f3);
const SPARKS: [(f64, f64, usize); 8] = [
    (0.02, -1.0, 0),
    (0.73, -0.77, 1),
    (1.0, 0.02, 0),
    (0.83, 0.72, 3),
    (0.02, 1.0, 4),
    (-0.83, 0.78, 1),
    (-1.0, -0.02, 0),
    (-0.67, -0.74, 2),
];

impl Effect for VioletPortal {
    fn meta(&self) -> &Meta {
        &META
    }

    fn render_kind(&self) -> RenderKind {
        RenderKind::Hybrid
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let palette = opts.palette.as_deref().unwrap_or_default();
        let mut rim: Cow<'static, [u8]> = Cow::Borrowed(RIM);
        if !palette.is_empty() {
            for [r, g, b, a] in rim.to_mut().as_chunks_mut::<4>().0 {
                if *a != 0 {
                    let Rgba(red, green, blue, _) = tint(Rgba(*r, *g, *b, *a), palette);
                    (*r, *g, *b) = (red, green, blue);
                }
            }
        }
        Box::new(State {
            rim,
            spark: tint(VIOLET, palette),
            glow: tint(LAVENDER, palette),
            // An outward travel of about three fifths of a blade length.
            reach: f64::from(Ruler::size(RULER.blade, 0.6)),
        })
    }
}

fn tint(color: Rgba, palette: &[Rgba]) -> Rgba {
    match palette {
        [] => color,
        [swatch] => *swatch,
        _ => {
            let dark = luma(SHADOW);
            blend(palette, (luma(color) - dark) / (luma(HIGHLIGHT) - dark))
        }
    }
}

struct State {
    rim: Cow<'static, [u8]>,
    spark: Rgba,
    glow: Rgba,
    reach: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let pose = js_round(t * f64::from(FPS)) as usize % POSES;
        let sprite = &self.rim.as_chunks::<FRAME_BYTES>().0[pose];
        if p.texel() < 1.0 {
            p.sprite_smooth(-17.0, -20.0, WIDTH, HEIGHT, 1.0, sprite);
        } else {
            p.sprite(-17.0, -20.0, WIDTH, HEIGHT, sprite);
        }

        for (dx, dy, offset) in SPARKS {
            let age = (pose + offset) % POSES;
            let travel = self.reach * age as f64 / POSES as f64;
            let x = dx * (18.0 + travel);
            let y = dy * (21.0 + travel);
            let color = if age < 2 { self.glow } else { self.spark };
            p.px(x, y, color);
            p.px(x + dx, y + dy, color);
            if age < 5 {
                let gap = if age < 2 { 2.0 } else { 4.0 };
                p.px(x + dx * gap, y + dy * gap, color);
            }
            if age < 2 {
                p.glow(x, y, 2.0, self.glow, 0.15);
            }
        }
    }
}
