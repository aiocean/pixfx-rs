//! The stage's pure logic, natively: removal of finished instances, draw order, and the frame-to-pixels
//! conversion the canvas shell blits.

use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZeroU32;

use pixfx_canvas::stage::{Pass, Rect, Scene, Shell, StageEvent, color_rgba, live_player};
use pixfx_core::{
    Canvas, Effect, EffectState, Frame, Layer, Meta, OptValue, Options, Painter, Placement, Rgba,
    render_frames,
};
use pixfx_effects::effect;
use pixfx_wasm::Player;

type Res = Result<(), Box<dyn Error>>;

/// A shell that keeps each drawn layer as a frame and records what it draws and shows.
#[derive(Default)]
struct Recorder {
    /// Layers drawn into a new surface or refilled.
    made: usize,
    refilled: usize,
    shown: Vec<(Pass, Frame)>,
}

impl Shell for Recorder {
    type Surface = Frame;
    type Error = ();
    fn make(&mut self, frame: &Frame) -> Result<Frame, ()> {
        self.made += 1;
        Ok(frame.clone())
    }
    fn refill(&mut self, surface: &mut Frame, frame: &Frame) -> Result<(), ()> {
        self.refilled += 1;
        surface.clone_from(frame);
        Ok(())
    }
    fn show(&mut self, pass: Pass, surface: &Frame) -> Result<(), ()> {
        self.shown.push((pass, surface.clone()));
        Ok(())
    }
}

/// What `layer` of every instance shows, oldest first.
fn shown(scene: &mut Scene<Frame>, layer: Layer) -> Vec<Frame> {
    let mut r = Recorder::default();
    assert_eq!(scene.draw_layer(layer, &mut r), Ok(()));
    r.shown.into_iter().map(|(_, f)| f).collect()
}

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
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    let shot = scene.play(live_player(&BLIP, Options::default()), 0.0, 0.0)?;
    let fire = scene.play(player("fire/candle")?, 0.0, 0.0)?;
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
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    scene.play(live_player(&LAYERED, Options::default()), 20.0, 20.0)?;
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
    for f in shown(&mut scene, Layer::Back) {
        events.push(seen("back", &f));
    }
    events.push(("host", false, false));
    for f in shown(&mut scene, Layer::Front) {
        events.push(seen("front", &f));
    }
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
    let mut scene: Scene<Frame> = Scene::new(scale(2)?);
    let h = scene.play(live, 200.0, 120.0)?;
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
fn fronts(scene: &mut Scene<Frame>) -> Vec<Vec<u32>> {
    shown(scene, Layer::Front)
        .into_iter()
        .map(|f| f.color)
        .collect()
}

#[test]
fn setting_flip_mid_play_draws_it_flipped_without_restarting() -> Res {
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    let plain = scene.play(live_player(&LAYERED, Options::default()), 10.0, 10.0)?;
    let flipped = Options {
        flip: true,
        ..Options::default()
    };
    scene.play(live_player(&LAYERED, flipped), 10.0, 10.0)?;
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
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    let blip = scene.play(live_player(&BLIP, Options::default()), 0.0, 0.0)?;
    let held = scene.play(live_player(&LAYERED, Options::default()), 0.0, 0.0)?;
    let kept = scene.play(live_player(&LAYERED, Options::default()), 0.0, 0.0)?;
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

/// A one-shot of 1 s that hits at 0.1 s: a ripple 3 px right of its origin, a hit-stop and a shake.
struct Hitter;

static HITTER: Hitter = Hitter;

const HITTER_META: Meta = Meta {
    id: "test/hitter",
    name: "Hitter",
    desc: "A one-shot that hits at 0.1 s.",
    tags: &[],
    size: (8, 6),
    origin: (4, 5),
    duration: 1.0,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

impl Effect for Hitter {
    fn meta(&self) -> &Meta {
        &HITTER_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(BlipState)
    }
    fn cues(&self, _opts: &Options) -> Vec<pixfx_core::TimedCue> {
        use pixfx_core::Cue;
        let at = |cue| pixfx_core::TimedCue {
            t: 0.1,
            cue,
            pan: None,
        };
        vec![
            at(Cue::Ripple {
                x: 3.0,
                y: -2.0,
                r0: 0.0,
                r1: 10.0,
                dur: 0.3,
                amp: 1.0,
            }),
            at(Cue::Hitstop { dur: 0.1 }),
            at(Cue::Shake { trauma: 1.0 }),
        ]
    }
}

const FRAME: f64 = 1.0 / 60.0;

#[test]
fn an_effects_hit_freezes_every_effect_and_tells_the_host_how_long() -> Res {
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    let hitter = scene.play(live_player(&HITTER, Options::default()), 0.0, 0.0)?;
    let candle = scene.play(player("fire/candle")?, 0.0, 0.0)?;
    // play until the hit lands
    let mut played = 0.0;
    while !scene.feel().is_some_and(|f| f.frozen()) {
        played += scene.update(FRAME)?;
        assert!(played < 0.2, "the hit at 0.1 s never froze play");
    }
    assert!(
        played >= 0.1 - 1e-9,
        "froze only once the hit landed: {played}"
    );
    let (t_hitter, t_candle) = (scene.t(hitter), scene.t(candle));
    // the hit-stop holds every effect, and update says no time played
    for _ in 0..5 {
        assert_eq!(scene.update(FRAME)?, 0.0);
    }
    assert_eq!((scene.t(hitter), scene.t(candle)), (t_hitter, t_candle));
    let mut thawed = false;
    for _ in 0..3 {
        thawed |= scene.update(FRAME)? > 0.0;
    }
    assert!(thawed, "play resumes after the stop");
    assert!(scene.feel().is_some_and(|f| f.trauma() > 0.0));
    Ok(())
}

#[test]
fn cues_come_out_as_events_where_they_fire_on_the_stage() -> Res {
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    scene.set_screen_w(100.0);
    let mirrored = scene.play(live_player(&HITTER, Options::default()), 50.0, 20.0)?;
    scene.set(
        mirrored,
        Placement {
            flip: true,
            ..live_player(&HITTER, Options::default()).placement()
        },
    )?;
    scene.move_to(mirrored, 50.0, 20.0)?;
    for _ in 0..7 {
        scene.update(FRAME)?;
    }
    let ripples: Vec<_> = scene
        .drain_events()
        .into_iter()
        .filter_map(|e| match e {
            StageEvent::Cue {
                handle,
                pan,
                cue: pixfx_core::Cue::Ripple { x, y, .. },
                ..
            } => Some((handle, x, y, pan)),
            _ => None,
        })
        .collect();
    // 3 px right of the origin, mirrored by the flip set on the stage: 3 px left of (50, 20)
    let [(handle, x, y, pan)] = ripples[..] else {
        return Err(format!("one ripple, got {ripples:?}").into());
    };
    assert_eq!((handle, x, y), (mirrored, 47.0, 18.0));
    // 47 of a 100 px stage is just left of the middle
    assert!(pan.is_some_and(|p| (p + 0.06).abs() < 1e-9), "pan {pan:?}");
    let r = scene
        .feel()
        .map(|f| f.ripples().to_vec())
        .unwrap_or_default();
    assert_eq!(
        (r.len(), r.first().map(|r| (r.x, r.y))),
        (1, Some((47.0, 18.0)))
    );
    Ok(())
}

#[test]
fn a_host_with_its_own_answer_gets_the_cues_and_every_second() -> Res {
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    scene.set_feel(None);
    scene.play(live_player(&HITTER, Options::default()), 0.0, 0.0)?;
    let mut played = 0.0;
    for _ in 0..20 {
        played += scene.update(FRAME)?;
    }
    assert!(
        (played - 20.0 * FRAME).abs() < 1e-12,
        "nothing froze: {played}"
    );
    let cues = scene
        .drain_events()
        .iter()
        .filter(|e| matches!(e, StageEvent::Cue { .. }))
        .count();
    assert_eq!(cues, 3);
    assert!(scene.feel().is_none());
    Ok(())
}

/// Draw both layers once with a fresh recorder: what it drew and showed.
fn draw_once(scene: &mut Scene<Frame>) -> Recorder {
    let mut r = Recorder::default();
    assert_eq!(scene.draw(&mut r), Ok(()));
    r
}

#[test]
fn identical_effects_draw_each_frame_once_and_share_it() -> Res {
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    for i in 0..50 {
        scene.play(player("fire/candle")?, f64::from(i * 3), 40.0)?;
    }
    scene.update(0.1)?;
    let r = draw_once(&mut scene);
    assert_eq!(
        r.made, 2,
        "one frame, back and front, drawn once for fifty effects"
    );
    assert_eq!(r.shown.len(), 100, "every effect shows both layers");
    // every back layer, then every front layer: each half shows one shared frame
    let (backs, fronts) = r.shown.split_at(50);
    assert!(backs.iter().all(|(_, f)| *f == backs[0].1));
    assert!(fronts.iter().all(|(_, f)| *f == fronts[0].1));
    Ok(())
}

#[test]
fn nothing_is_drawn_again_until_the_frame_changes() -> Res {
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    scene.play(player("fire/candle")?, 20.0, 40.0)?;
    scene.update(0.1)?;
    assert_eq!(draw_once(&mut scene).made, 2);
    // display refreshes at 120 Hz: a quarter of a 30 fps frame passes, nothing new to draw
    scene.update(1.0 / 120.0)?;
    let r = draw_once(&mut scene);
    assert_eq!((r.made, r.shown.len()), (0, 2), "shown again, not drawn");
    scene.update(1.0 / 30.0)?;
    assert_eq!(draw_once(&mut scene).made, 2, "a new frame is drawn");
    Ok(())
}

#[test]
fn a_fading_effect_redraws_its_own_frame_once_per_clock_frame() -> Res {
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    let h = scene.play(player("fire/candle")?, 20.0, 40.0)?;
    scene.update(0.1)?;
    scene.stop(h);
    assert_eq!(
        draw_once(&mut scene).made,
        2,
        "its own fade frame, back and front"
    );
    scene.update(1.0 / 120.0)?;
    let r = draw_once(&mut scene);
    assert_eq!(
        (r.made, r.refilled),
        (0, 0),
        "same clock frame: shown, not drawn"
    );
    scene.update(1.0 / 30.0)?;
    let r = draw_once(&mut scene);
    assert_eq!(
        (r.made, r.refilled),
        (0, 2),
        "next clock frame: drawn into its own surfaces"
    );
    Ok(())
}

#[test]
fn frames_dropped_over_the_budget_come_back_the_same() -> Res {
    let mut scene: Scene<Frame> = Scene::new(scale(1)?);
    scene.play(player("fire/candle")?, 20.0, 40.0)?;
    scene.update(0.1)?;
    let before = draw_once(&mut scene).shown;
    let held = scene.frame_bytes();
    assert!(held > 0);
    // a budget below one frame keeps only what the current draw shows
    scene.set_frame_budget(1);
    scene.update(1.0 / 30.0)?;
    draw_once(&mut scene);
    assert!(scene.frame_bytes() <= held, "the older frame went");
    // come round to the first frame again: one whole loop of the candle
    let loop_s = effect("fire/candle").ok_or("candle")?.meta().duration;
    let mut left = loop_s - 1.0 / 30.0;
    while left > 1e-9 {
        let dt = left.min(0.2);
        scene.update(dt)?;
        left -= dt;
    }
    let again = draw_once(&mut scene);
    assert_eq!(again.made, 2, "the dropped frame is drawn again");
    assert_eq!(
        again.shown.iter().map(|(_, f)| f).collect::<Vec<_>>(),
        before.iter().map(|(_, f)| f).collect::<Vec<_>>(),
        "byte-equal to the first time"
    );
    Ok(())
}
