//! The stage's pure logic, natively: removal of finished instances, draw order, and the frame-to-pixels
//! conversion the canvas shell blits.

use std::error::Error;
use std::num::NonZeroU32;

use pixfx_canvas::stage::{Scene, color_rgba, live_player};
use pixfx_core::{Effect, EffectState, Frame, Layer, Meta, Options, Painter, Rgba};
use pixfx_effects::effect;
use pixfx_wasm::Player;

type Res = Result<(), Box<dyn Error>>;

/// A one-shot lasting 0.1 s that draws nothing: the registry has no one-shot yet.
struct Blip;

static BLIP: Blip = Blip;

const BLIP_META: Meta = Meta {
    id: "test/blip",
    name: "Blip",
    desc: "A one-shot for tests.",
    tags: &[],
    size: (8, 6),
    origin: (4, 5),
    duration: 0.1,
    looping: false,
    fps: 30,
    seed: 1,
};

struct BlipState;

impl EffectState for BlipState {
    fn draw(&self, _p: &mut Painter, _t: f64, _layer: Layer) {}
}

impl Effect for Blip {
    fn meta(&self) -> &Meta {
        &BLIP_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(BlipState)
    }
}

/// A loop that paints one red pixel on its back layer and one blue pixel on its front layer.
struct Layered;

static LAYERED: Layered = Layered;

const LAYERED_META: Meta = Meta {
    id: "test/layered",
    name: "Layered",
    desc: "Red behind, blue in front.",
    tags: &[],
    size: (8, 6),
    origin: (4, 5),
    duration: 1.0,
    looping: true,
    fps: 30,
    seed: 1,
};

struct LayeredState;

impl EffectState for LayeredState {
    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        match layer {
            Layer::Back => p.px(1.0, 1.0, Rgba(255, 0, 0, 255)),
            _ => p.px(2.0, 2.0, Rgba(0, 0, 255, 255)),
        }
    }
}

impl Effect for Layered {
    fn meta(&self) -> &Meta {
        &LAYERED_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(LayeredState)
    }
}

fn player(id: &str) -> Result<Player, Box<dyn Error>> {
    let fx = effect(id).ok_or_else(|| format!("{id} not registered"))?;
    Ok(live_player(fx, Options::default()))
}

fn scale(n: u32) -> Result<NonZeroU32, Box<dyn Error>> {
    NonZeroU32::new(n).ok_or_else(|| "scale 0".into())
}

#[test]
fn finished_one_shot_is_removed() -> Res {
    let mut scene = Scene::new(scale(1)?);
    let shot = scene.play(live_player(&BLIP, Options::default()), 0.0, 0.0, ())?;
    let fire = scene.play(player("fire/candle")?, 0.0, 0.0, ())?;
    scene.update(0.05)?;
    assert_eq!(scene.handles().collect::<Vec<_>>(), vec![shot, fire]);
    scene.update(0.06)?;
    assert_eq!(
        scene.handles().collect::<Vec<_>>(),
        vec![fire],
        "the one-shot ended at 0.1 s"
    );
    Ok(())
}

#[test]
fn a_host_draw_between_the_layers_lands_over_back_and_under_front() -> Res {
    let mut scene = Scene::new(scale(1)?);
    scene.play(live_player(&LAYERED, Options::default()), 20.0, 20.0, ())?;
    scene.update(0.1)?;
    let (red, blue) = (0xff00_00ff_u32, 0xffff_0000_u32);
    // (what happened, red shown, blue shown): the host's draw is the middle event
    let mut events: Vec<(&str, bool, bool)> = Vec::new();
    let seen = |layer: &'static str, frame: &Frame| {
        (
            layer,
            frame.color.contains(&red),
            frame.color.contains(&blue),
        )
    };
    let mut got = Vec::new();
    let ok: Result<(), ()> = scene.draw_layer(Layer::Back, |_, frame, _| {
        got.push(seen("back", frame));
        Ok(())
    });
    assert_eq!(ok, Ok(()));
    events.append(&mut got);
    events.push(("host", false, false));
    let ok: Result<(), ()> = scene.draw_layer(Layer::Front, |_, frame, _| {
        got.push(seen("front", frame));
        Ok(())
    });
    assert_eq!(ok, Ok(()));
    events.append(&mut got);
    assert_eq!(
        events,
        vec![
            ("back", true, false),
            ("host", false, false),
            ("front", false, true)
        ]
    );
    Ok(())
}

fn tiny(color: [u32; 3], light: [u32; 3]) -> Frame {
    let mut f = Frame::new(3, 1, 0, 0);
    f.color.copy_from_slice(&color);
    f.light.copy_from_slice(&light);
    f
}

#[test]
fn colour_words_become_straight_rgba_bytes() {
    // ABGR words: 0xAABBGGRR
    let f = tiny([0x0000_0000, 0xff33_2211, 0x80cc_bbaa], [0; 3]);
    let mut out = [9u8; 12];
    assert!(color_rgba(&f, &mut out));
    assert_eq!(
        out,
        [0, 0, 0, 0, 0x11, 0x22, 0x33, 0xff, 0xaa, 0xbb, 0xcc, 0x80]
    );
    let mut empty = [9u8; 12];
    assert!(
        !color_rgba(&tiny([0; 3], [0x00ff_ffff; 3]), &mut empty),
        "nothing to draw"
    );
    assert_eq!(empty, [0; 12]);
}
