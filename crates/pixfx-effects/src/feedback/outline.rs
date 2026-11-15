//! `feedback/outline`: a ring round the sprite it wraps (a character, a sword, a hat), so it reads over any ground or
//! shows it is picked. Made here, not ported: no other effect draws an outline, and one sprite takes as many rings
//! as the game plays. Built on [`pixfx_core::Silhouette`]: the ring follows every frame the game hands it.

use std::sync::Arc;

use pixfx_core::{
    Actor, Area, Canvas, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette,
};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

pub struct Outline;

pub static OUTLINE: Outline = Outline;

const META: Meta = Meta {
    id: "feedback/outline",
    name: "Outline",
    desc: "A ring round the sprite it wraps, following each frame: to read a character over busy ground, or to show what is picked or hovered.",
    tags: &[
        "feedback",
        "outline",
        "highlight",
        "select",
        "hover",
        "character",
        "loop",
    ],
    // `canvas` sizes it from `w` and `h`; this is their default
    size: (128, 128),
    origin: (64, 64),
    duration: 1.0,
    looping: true,
    // it does not move: one frame, drawn again only when the sprite changes
    fps: 1,
    seed: 1,
    state: None,
    options: &[WIDTH.spec(), CORNERS.spec(), W.spec(), H.spec()],
};

/// How many pixels wide the ring is.
const WIDTH: NumOpt = NumOpt::new("width", 1.0).within(1.0, 3.0);
/// 1 fills the ring's corners (a square ring), 0 cuts them (a rounded, pixel-art ring).
const CORNERS: NumOpt = NumOpt::new("corners", 0.0).within(0.0, 1.0);
/// The box the sprite and its ring fit in, centred on where the effect plays: a sprite reaching past it is ringed
/// only inside it.
const W: NumOpt = NumOpt::new("w", 128.0).within(8.0, 512.0);
const H: NumOpt = NumOpt::new("h", 128.0).within(8.0, 512.0);

/// The engine's dark outline (as the rocks' and the fighter's), the ring's colour unless `palette` gives one: a
/// near-black violet that reads over light and dark ground alike.
const DARK: Rgba = Rgba::hex24(0x1a1422);
const PALETTE: [Rgba; 1] = [DARK];

impl Effect for Outline {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn canvas(&self, opts: &Options) -> Canvas {
        let (w, h) = (opts.num(&W) as u32, opts.num(&H) as u32);
        Canvas {
            size: (w, h),
            origin: ((w / 2) as i32, (h / 2) as i32),
        }
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            color: opts.palette_or(&PALETTE).first().copied().unwrap_or(DARK),
            width: opts.num(&WIDTH) as u32,
            corners: opts.num(&CORNERS) >= 0.5,
            ring: Vec::new(),
        })
    }
}

struct State {
    color: Rgba,
    width: u32,
    corners: bool,
    /// The ring's cells round the sprite as it is now; none without one.
    ring: Vec<(i32, i32)>,
}

impl EffectState for State {
    fn update(&mut self, _dt: f64) {}

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.ring = actor.map_or_else(Vec::new, |a| {
            // the sprite's box with room for the ring on every side
            let pad = self.width as i32;
            let area = Area {
                x: a.x() - pad,
                y: a.y() - pad,
                w: a.w() + 2 * self.width,
                h: a.h() + 2 * self.width,
            };
            Silhouette::of(&a, area)
                .ring(self.width, self.corners)
                .collect()
        });
    }

    /// The ring goes behind the sprite: it lies off the body, so the sprite drawn over it hides none of it.
    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer != Layer::Back {
            return;
        }
        for &(x, y) in &self.ring {
            p.px(f64::from(x), f64::from(y), self.color);
        }
    }
}
