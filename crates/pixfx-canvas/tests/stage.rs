//! The stage's pure logic, natively: removal of finished instances, draw order, and the frame-to-pixels
//! conversion the canvas shell blits.

use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZeroU32;

use pixfx_canvas::stage::{Rect, Scene, StageEvent, color_rgba, live_player};
use pixfx_core::{
    Canvas, Effect, EffectState, Frame, Layer, Meta, OptValue, Options, Painter, Placement, Rgba,
    render_frames,
};
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
    state: None,
    options: &[],
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
    state: None,
    options: &[],
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

#[test]
fn a_sized_instance_gets_its_own_canvas_and_rect() -> Res {
    // fireflies 100 x 50: origin mid-width, 10 above the foot; margin still its own canvas's, 160
    let fx = effect("weather/fireflies").ok_or("fireflies not registered")?;
    let opts = Options {
        extra: BTreeMap::from([
            ("width".to_string(), OptValue::Num(100.0)),
            ("height".to_string(), OptValue::Num(50.0)),
        ]),
        ..Options::default()
    };
    let live = live_player(fx, opts);
    assert_eq!(
        live.canvas(),
        Canvas {
            size: (100, 50),
            origin: (50, 40)
        }
    );
    assert_eq!(
        (live.frame().w, live.frame().h),
        (100 + 2 * 160, 50 + 2 * 160)
    );
    let mut scene = Scene::new(scale(2)?);
    let h = scene.play(live, 200.0, 120.0, ())?;
    assert_eq!(
        scene.rect(h),
        Some(Rect {
            x: (200 - 50 - 160) * 2,
            y: (120 - 40 - 160) * 2,
            w: (100 + 2 * 160) * 2,
            h: (50 + 2 * 160) * 2,
        })
    );
    Ok(())
}

/// A player of `fx` with `opts` at margin `m` shows frames 0, 1 and 45 of [`render_frames`] at that margin.
fn player_matches_render(fx: &'static dyn Effect, opts: &Options, m: u32) {
    let fps = f64::from(fx.meta().fps);
    let want = render_frames(fx, opts, m).frames;
    let mut live = Player::new(fx, opts.clone(), m);
    let mut stepped = 0;
    for n in [0usize, 1, 45] {
        while stepped < n {
            live.update(1.0 / fps);
            stepped += 1;
        }
        let place = live.placement();
        live.draw(Layer::Both, place);
        assert_eq!(
            Some(live.frame()),
            want.get(n),
            "{}, margin {m}, frame {n}",
            fx.meta().id
        );
    }
}

#[test]
fn a_sized_player_draws_where_render_frames_does() -> Res {
    // fireflies 12 x 12: the canvas its options give, not its own 320 x 180
    let fx = effect("weather/fireflies").ok_or("fireflies not registered")?;
    let opts = Options {
        seed: Some(3),
        extra: BTreeMap::from([
            ("width".to_string(), OptValue::Num(12.0)),
            ("height".to_string(), OptValue::Num(12.0)),
        ]),
        ..Options::default()
    };
    for m in [0, 3] {
        player_matches_render(fx, &opts, m);
    }
    Ok(())
}

/// The front frame of every instance, oldest first.
fn fronts(scene: &mut Scene<()>) -> Vec<Vec<u32>> {
    let mut seen = Vec::new();
    let ok: Result<(), ()> = scene.draw_layer(Layer::Front, |_pass, frame, _surface| {
        seen.push(frame.color.clone());
        Ok(())
    });
    assert_eq!(ok, Ok(()));
    seen
}

#[test]
fn setting_flip_mid_play_draws_it_flipped_without_restarting() -> Res {
    let mut scene = Scene::new(scale(1)?);
    let plain = scene.play(live_player(&LAYERED, Options::default()), 10.0, 10.0, ())?;
    let flipped = Options {
        flip: true,
        ..Options::default()
    };
    scene.play(live_player(&LAYERED, flipped), 10.0, 10.0, ())?;
    scene.update(0.25)?;
    let before = fronts(&mut scene);
    assert_ne!(
        before[0], before[1],
        "the test effect must look different flipped"
    );

    let p = scene.placement(plain).ok_or("no placement")?;
    assert!(scene.set(plain, Placement { flip: true, ..p })?);

    assert_eq!(scene.t(plain), Some(0.25), "set keeps the time");
    let after = fronts(&mut scene);
    assert_eq!(after[0], after[1], "drawn as one played flipped");
    assert_eq!(scene.placement(plain), Some(Placement { flip: true, ..p }));
    Ok(())
}

#[test]
fn ended_effects_are_drained_once_in_order() -> Res {
    let mut scene = Scene::new(scale(1)?);
    let blip = scene.play(live_player(&BLIP, Options::default()), 0.0, 0.0, ())?;
    let held = scene.play(live_player(&LAYERED, Options::default()), 0.0, 0.0, ())?;
    let kept = scene.play(live_player(&LAYERED, Options::default()), 0.0, 0.0, ())?;
    assert!(scene.remove(held));
    scene.update(0.2)?;

    assert_eq!(
        scene.drain_events(),
        vec![
            StageEvent::End { handle: held },
            StageEvent::End { handle: blip }
        ]
    );
    assert_eq!(scene.drain_events(), vec![]);
    assert_eq!(scene.handles().collect::<Vec<_>>(), vec![kept]);
    Ok(())
}
