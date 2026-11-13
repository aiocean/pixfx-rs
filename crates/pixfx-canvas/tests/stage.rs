//! The stage's pure logic, natively: handles, removal of finished instances, draw order, target rects and
//! the frame-to-pixels conversion the canvas shell blits.

use std::collections::BTreeMap;
use std::error::Error;
use std::num::NonZeroU32;

use pixfx_canvas::stage::{
    Pass, Rect, Scene, StageError, StageEvent, color_rgba, light_rgba, live_player,
};
use pixfx_core::{
    Canvas, Effect, EffectState, Frame, Layer, Meta, OptValue, Options, Painter, Placement, Rgba,
    composite_px, render_frames,
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

fn passes(scene: &mut Scene<()>) -> Vec<Pass> {
    let mut seen = Vec::new();
    let ok: Result<(), ()> = scene.draw(|pass, _frame, _surface| {
        seen.push(pass);
        Ok(())
    });
    assert_eq!(ok, Ok(()));
    seen
}

#[test]
fn handles_are_unique_and_never_reused() -> Res {
    let mut scene = Scene::new(scale(4)?);
    let a = scene.play(player("fire/candle")?, 10.0, 10.0, ())?;
    let b = scene.play(player("fire/campfire")?, 20.0, 10.0, ())?;
    assert!(scene.remove(b));
    let c = scene.play(player("fire/brazier")?, 30.0, 10.0, ())?;
    let d = scene.play(player("fire/candle")?, 40.0, 10.0, ())?;
    let all = [a, b, c, d];
    for (i, h) in all.iter().enumerate() {
        assert_ne!(*h, 0, "0 is never a handle");
        assert!(!all[i + 1..].contains(h), "handle {h} given twice: {all:?}");
    }
    assert_eq!(scene.handles().collect::<Vec<_>>(), vec![a, c, d]);
    Ok(())
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
fn stopped_loop_is_removed_after_its_fade() -> Res {
    let mut scene = Scene::new(scale(1)?);
    let fire = scene.play(player("fire/campfire")?, 0.0, 0.0, ())?;
    scene.update(0.1)?;
    assert!(scene.stop(fire));
    scene.update(0.2)?;
    assert_eq!(
        scene.handles().collect::<Vec<_>>(),
        vec![fire],
        "still fading"
    );
    scene.update(0.2)?;
    assert_eq!(scene.handles().count(), 0, "faded out after 0.3 s");
    Ok(())
}

#[test]
fn unknown_handles_are_reported() -> Res {
    let mut scene: Scene<()> = Scene::new(scale(1)?);
    assert!(!scene.stop(7));
    assert!(!scene.remove(7));
    assert_eq!(scene.rect(7), None);
    Ok(())
}

#[test]
fn draws_every_back_layer_before_any_front_layer() -> Res {
    let mut scene = Scene::new(scale(2)?);
    let a = scene.play(player("fire/candle")?, 30.0, 70.0, ())?;
    let b = scene.play(player("fire/campfire")?, 100.0, 120.0, ())?;
    let c = scene.play(player("fire/brazier")?, 200.0, 140.0, ())?;
    let order: Vec<(Layer, u32)> = passes(&mut scene)
        .iter()
        .map(|p| (p.layer, p.handle))
        .collect();
    assert_eq!(
        order,
        vec![
            (Layer::Back, a),
            (Layer::Back, b),
            (Layer::Back, c),
            (Layer::Front, a),
            (Layer::Front, b),
            (Layer::Front, c),
        ]
    );
    Ok(())
}

fn layer_passes(scene: &mut Scene<()>, layer: Layer) -> Vec<Pass> {
    let mut seen = Vec::new();
    let ok: Result<(), ()> = scene.draw_layer(layer, |pass, _frame, _surface| {
        seen.push(pass);
        Ok(())
    });
    assert_eq!(ok, Ok(()));
    seen
}

#[test]
fn drawing_one_layer_hands_only_that_layer_in_instance_order() -> Res {
    let mut scene = Scene::new(scale(2)?);
    let a = scene.play(player("fire/candle")?, 30.0, 70.0, ())?;
    let b = scene.play(player("fire/campfire")?, 100.0, 120.0, ())?;
    let c = scene.play(player("fire/brazier")?, 200.0, 140.0, ())?;
    for layer in [Layer::Back, Layer::Front] {
        let got: Vec<(Layer, u32)> = layer_passes(&mut scene, layer)
            .iter()
            .map(|p| (p.layer, p.handle))
            .collect();
        assert_eq!(got, vec![(layer, a), (layer, b), (layer, c)]);
    }
    Ok(())
}

#[test]
fn drawing_both_is_back_then_front() -> Res {
    let mut scene = Scene::new(scale(2)?);
    scene.play(player("fire/candle")?, 30.0, 70.0, ())?;
    scene.play(player("fire/campfire")?, 100.0, 120.0, ())?;
    let mut split = layer_passes(&mut scene, Layer::Back);
    split.extend(layer_passes(&mut scene, Layer::Front));
    assert_eq!(passes(&mut scene), split);
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

#[test]
fn passes_carry_the_drawn_layer() -> Res {
    // the frame handed to each pass is the effect drawn with that pass's layer, not a stale one
    let mut scene = Scene::new(scale(1)?);
    scene.play(player("fire/campfire")?, 64.0, 112.0, ())?;
    scene.update(0.5)?;
    let mut lit = Vec::new();
    let ok: Result<(), ()> = scene.draw(|pass, frame, _| {
        lit.push((pass.layer, frame.light.iter().filter(|&&l| l != 0).count()));
        Ok(())
    });
    assert_eq!(ok, Ok(()));
    let mut fresh = player("fire/campfire")?;
    fresh.update(0.5);
    let want: Vec<(Layer, usize)> = [Layer::Back, Layer::Front]
        .into_iter()
        .map(|layer| {
            let place = fresh.placement();
            fresh.draw(layer, place);
            (
                layer,
                fresh.frame().light.iter().filter(|&&l| l != 0).count(),
            )
        })
        .collect();
    assert_eq!(lit, want);
    Ok(())
}

#[test]
fn rects_place_the_origin_and_follow_the_scale() -> Res {
    let mut scene = Scene::new(scale(4)?);
    // candle: 56x76, origin (28, 64); margin half its longer side, 38, on every side
    let h = scene.play(player("fire/candle")?, 100.0, 150.0, ())?;
    let at4 = Rect {
        x: (100 - 28 - 38) * 4,
        y: (150 - 64 - 38) * 4,
        w: (56 + 2 * 38) * 4,
        h: (76 + 2 * 38) * 4,
    };
    assert_eq!(scene.rect(h), Some(at4));
    assert_eq!(passes(&mut scene).first().map(|p| p.rect), Some(at4));

    scene.set_scale(scale(2)?);
    let at2 = Rect {
        x: (100 - 28 - 38) * 2,
        y: (150 - 64 - 38) * 2,
        w: (56 + 2 * 38) * 2,
        h: (76 + 2 * 38) * 2,
    };
    assert_eq!(scene.scale(), scale(2)?);
    assert_eq!(scene.rect(h), Some(at2));
    assert_eq!(passes(&mut scene).first().map(|p| p.rect), Some(at2));
    Ok(())
}

#[test]
fn the_origin_lands_where_the_caller_put_it() -> Res {
    // the effect's origin is pixel (origin + margin) of its frame; the rect puts that pixel on (x, y)
    let mut scene = Scene::new(scale(3)?);
    for id in ["fire/candle", "fire/campfire", "fire/brazier"] {
        let p = player(id)?;
        let (o, m) = (p.meta().origin, i64::from(p.margin()));
        assert_eq!(p.margin(), p.meta().glow_margin());
        let h = scene.play(p, 40.0, 90.0, ())?;
        let r = scene.rect(h).ok_or("no rect")?;
        assert_eq!(
            (
                r.x + (i64::from(o.0) + m) * 3,
                r.y + (i64::from(o.1) + m) * 3
            ),
            (40 * 3, 90 * 3),
            "{id}"
        );
    }
    Ok(())
}

#[test]
fn stage_shows_glow_that_spills_past_the_canvas() -> Res {
    // the candle's halo (radius up to 38 * flicker) reaches past its 56x76 canvas when it flares: the stage
    // shows those pixels instead of cutting them at the canvas edge
    for id in ["fire/candle", "fire/campfire", "fire/brazier"] {
        let fx = effect(id).ok_or("not registered")?;
        let (w, h) = fx.meta().size;
        let mut live = live_player(fx, Options::default());
        let m = live.margin();
        let mut spilled = 0;
        for _ in 0..60 {
            live.update(1.0 / 30.0);
            let place = live.placement();
            live.draw(Layer::Both, place);
            let f = live.frame();
            spilled += (0..f.h)
                .flat_map(|y| (0..f.w).map(move |x| (x, y)))
                .filter(|&(x, y)| x < m || y < m || x >= m + w || y >= m + h)
                .filter(|&(x, y)| f.light[(y * f.w + x) as usize] & 0x00ff_ffff != 0)
                .count();
        }
        assert!(spilled > 0, "{id}: no glow past its canvas in 2 s");
    }
    Ok(())
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

#[test]
fn player_frames_equal_render_frames_at_its_margin() -> Res {
    // margin 0 (Fx: edge fade, sprite-sheet frames) and the stage's margin (no fade) both match the render
    let fx = effect("fire/candle").ok_or("candle not registered")?;
    let opts = Options {
        seed: Some(4),
        ..Options::default()
    };
    let fps = f64::from(fx.meta().fps);
    for m in [0, fx.meta().glow_margin()] {
        let want = render_frames(fx, &opts, m).frames;
        let mut live = Player::new(fx, opts.clone(), m);
        let mut stepped = 0;
        for n in [0usize, 1, 45] {
            while stepped < n {
                live.update(1.0 / fps);
                stepped += 1;
            }
            let place = live.placement();
            live.draw(Layer::Both, place);
            assert_eq!(Some(live.frame()), want.get(n), "margin {m}, frame {n}");
        }
    }
    Ok(())
}

#[test]
fn positions_snap_to_whole_game_pixels() -> Res {
    let mut scene = Scene::new(scale(3)?);
    let h = scene.play(player("fire/candle")?, 99.5, 149.4, ())?;
    assert_eq!(
        scene.rect(h).map(|r| (r.x, r.y)),
        Some(((100 - 28 - 38) * 3, (149 - 64 - 38) * 3))
    );
    let left = scene.play(player("fire/candle")?, -0.5, -10.6, ())?;
    assert_eq!(
        scene.rect(left).map(|r| (r.x, r.y)),
        Some(((0 - 28 - 38) * 3, (-11 - 64 - 38) * 3))
    );
    Ok(())
}

#[test]
fn moving_an_effect_places_its_origin_and_keeps_its_time() -> Res {
    let mut scene = Scene::new(scale(2)?);
    let h = scene.play(player("fire/candle")?, 100.0, 150.0, ())?;
    scene.update(0.2)?;
    let t = scene.t(h);
    assert_eq!(scene.move_to(h, 40.6, 60.4), Ok(true));
    assert_eq!(
        scene.rect(h).map(|r| (r.x, r.y)),
        Some(((41 - 28 - 38) * 2, (60 - 64 - 38) * 2))
    );
    assert_eq!(scene.t(h), t, "a move must not restart the effect");
    assert_eq!(passes(&mut scene).first().map(|p| p.rect), scene.rect(h));
    Ok(())
}

#[test]
fn moving_refuses_non_finite_positions_and_reports_unknown_handles() -> Res {
    let mut scene = Scene::new(scale(1)?);
    let h = scene.play(player("fire/candle")?, 10.0, 20.0, ())?;
    let before = scene.rect(h);
    assert_eq!(scene.move_to(h, f64::NAN, 0.0), Err(StageError::NotFinite));
    assert_eq!(
        scene.move_to(h, 0.0, f64::INFINITY),
        Err(StageError::NotFinite)
    );
    assert_eq!(scene.rect(h), before);
    assert_eq!(scene.move_to(h + 1, 0.0, 0.0), Ok(false));
    Ok(())
}

#[test]
fn non_finite_positions_are_refused() -> Res {
    let mut scene = Scene::new(scale(1)?);
    for (x, y) in [
        (f64::NAN, 0.0),
        (0.0, f64::INFINITY),
        (f64::NEG_INFINITY, 1.0),
    ] {
        let got = scene.play(player("fire/candle")?, x, y, ());
        assert_eq!(got, Err(StageError::NotFinite));
    }
    assert_eq!(scene.handles().count(), 0);
    Ok(())
}

#[test]
fn draw_stops_at_the_first_blit_error() -> Res {
    let mut scene = Scene::new(scale(1)?);
    scene.play(player("fire/candle")?, 0.0, 0.0, ())?;
    scene.play(player("fire/candle")?, 0.0, 0.0, ())?;
    let mut calls = 0;
    let got = scene.draw(|_, _, _| {
        calls += 1;
        Err("blit failed")
    });
    assert_eq!((got, calls), (Err("blit failed"), 1));
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
fn light_words_become_light_over_transparency() {
    // xBGR words; the top byte is ignored. A lit pixel is light over nothing (alpha its brightest channel, rgb
    // scaled up to it) so faint glow stays faint on a transparent canvas; dark pixels are alpha 0.
    let f = tiny([0; 3], [0x7700_0000, 0xab00_050a, 0x00ff_8040]);
    let mut out = [9u8; 12];
    assert!(light_rgba(&f, &mut out));
    assert_eq!(
        out,
        [0, 0, 0, 0, 255, 128, 0, 10, 0x40, 0x80, 0xff, 255],
        "(10, 5, 0) is (255, 128, 0) at alpha 10; a full channel keeps the light as it is"
    );
    for (i, px) in out.chunks(4).enumerate() {
        assert_eq!(
            px,
            composite_px(0, f.light[i], None),
            "the same pixel core's composite makes"
        );
    }
    let mut empty = [9u8; 12];
    assert!(
        !light_rgba(&tiny([0xffff_ffff; 3], [0xff00_0000; 3]), &mut empty),
        "no light"
    );
    assert_eq!(empty, [0; 12]);
}

#[test]
fn unlit_pixels_are_transparent_and_only_a_lit_one_counts() {
    // unlit whatever the unused top byte holds, and whatever the buffer held before
    let dark = tiny([0xffff_ffff; 3], [0, 0xff00_0000, 0x1200_0000]);
    let mut out = [9u8; 12];
    assert!(!light_rgba(&dark, &mut out), "a frame with no lit pixel");
    assert_eq!(out, [0; 12]);

    let one = tiny([0; 3], [0xff00_0000, 0x0000_0100, 0]);
    let mut out = [9u8; 12];
    assert!(
        light_rgba(&one, &mut out),
        "a frame with exactly one lit pixel"
    );
    assert_eq!(&out[0..4], &[0; 4]);
    assert_eq!(&out[4..8], &composite_px(0, 0x0000_0100, None));
    assert_eq!(&out[8..12], &[0; 4]);
}

#[test]
fn bad_time_steps_are_refused_and_leave_time_alone() -> Res {
    let mut scene = Scene::new(scale(1)?);
    let h = scene.play(player("fire/candle")?, 0.0, 0.0, ())?;
    scene.update(0.1)?;
    let t0 = scene.t(h);
    assert!(t0.is_some_and(|t| t > 0.0));
    for dt in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01] {
        assert_eq!(scene.update(dt), Err(StageError::BadStep), "dt {dt}");
        assert_eq!(scene.t(h), t0, "dt {dt} moved time");
    }
    scene.update(0.0)?;
    assert_eq!(scene.t(h), t0);
    assert_eq!(scene.t(99), None);
    Ok(())
}

#[test]
fn far_off_positions_saturate_instead_of_overflowing() -> Res {
    let mut scene = Scene::new(scale(4)?);
    let h = scene.play(player("fire/candle")?, 1e300, -1e300, ())?;
    assert_eq!(
        scene.rect(h).map(|r| (r.x, r.y)),
        Some((i64::MAX, i64::MIN))
    );
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
