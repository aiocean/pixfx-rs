//! A three-frame rune sprite with a painted shock ring.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RenderKind};
use pixfx_draw::color::Rgba;
use pixfx_draw::math::js_round;
use pixfx_draw::{Painter, RULER, Ruler};

pub struct RuneFlash;

pub static RUNE_FLASH: RuneFlash = RuneFlash;

const FPS: u32 = 30;
const META: Meta = Meta {
    id: "impacts/rune-flash",
    name: "Rune Flash",
    desc: "A violet sigil flares and breaks apart as a ring of light expands from the impact.",
    tags: &["impact", "magic", "one-shot"],
    size: (48, 48),
    origin: (24, 24),
    duration: 12.0 / FPS as f64,
    looping: false,
    fps: FPS,
    seed: 1,
    state: None,
    options: &[],
};

// Each frame is half a blade wide at one game pixel per texel.
const SIDE: u32 = 13;
const FRAME_BYTES: usize = SIDE as usize * SIDE as usize * 4;
const SPRITES: &[u8; 3 * FRAME_BYTES] = include_bytes!("art/rune_flash.rgba");
const LIGHT: [Rgba; 1] = [Rgba::hex24(0xb582ff)];

impl Effect for RuneFlash {
    fn meta(&self) -> &Meta {
        &META
    }

    fn render_kind(&self) -> RenderKind {
        RenderKind::Hybrid
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            light: opts.palette_or(&LIGHT).first().copied().unwrap_or(LIGHT[0]),
            reach: f64::from(Ruler::size(RULER.blade, 0.75)),
        })
    }
}

struct State {
    light: Rgba,
    reach: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let frame = js_round(t * f64::from(FPS)) as usize;
        if let Some(sprite) = SPRITES.as_chunks::<FRAME_BYTES>().0.get(frame / 3) {
            let origin = -f64::from(SIDE / 2);
            if p.texel() < 1.0 {
                p.sprite_smooth(origin, origin, SIDE, SIDE, 1.0, sprite);
            } else {
                p.sprite(origin, origin, SIDE, SIDE, sprite);
            }
        }

        let progress = t / META.duration;
        if progress < 1.0 {
            let radius = f64::from(SIDE / 2) + self.reach * progress;
            p.glow_ring(0.0, 0.0, radius, 2.0, self.light, 0.85 * (1.0 - progress));
        }
    }
}
