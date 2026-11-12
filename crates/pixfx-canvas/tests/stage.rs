//! The stage's pure logic, natively: handles, removal of finished instances, draw order, target rects and
//! the frame-to-pixels conversion the canvas shell blits.

use std::error::Error;
use std::num::NonZeroU32;

use pixfx_canvas::stage::{Pass, Rect, Scene, StageError, color_rgba, light_rgba};
use pixfx_core::{Effect, EffectState, Frame, Layer, Meta, Options, Painter};
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

fn player(id: &str) -> Result<Player, Box<dyn Error>> {
    let fx = effect(id).ok_or_else(|| format!("{id} not registered"))?;
    Ok(Player::new(fx, Options::default()))
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
    let shot = scene.play(Player::new(&BLIP, Options::default()), 0.0, 0.0, ())?;
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
            fresh.draw(layer);
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
    // candle: 56x76, origin (28, 64)
    let h = scene.play(player("fire/candle")?, 100.0, 150.0, ())?;
    let at4 = Rect {
        x: (100 - 28) * 4,
        y: (150 - 64) * 4,
        w: 56 * 4,
        h: 76 * 4,
    };
    assert_eq!(scene.rect(h), Some(at4));
    assert_eq!(passes(&mut scene).first().map(|p| p.rect), Some(at4));

    scene.set_scale(scale(2)?);
    let at2 = Rect {
        x: (100 - 28) * 2,
        y: (150 - 64) * 2,
        w: 56 * 2,
        h: 76 * 2,
    };
    assert_eq!(scene.scale(), scale(2)?);
    assert_eq!(scene.rect(h), Some(at2));
    assert_eq!(passes(&mut scene).first().map(|p| p.rect), Some(at2));
    Ok(())
}

#[test]
fn positions_snap_to_whole_game_pixels() -> Res {
    let mut scene = Scene::new(scale(3)?);
    let h = scene.play(player("fire/candle")?, 99.5, 149.4, ())?;
    assert_eq!(
        scene.rect(h).map(|r| (r.x, r.y)),
        Some(((100 - 28) * 3, (149 - 64) * 3))
    );
    let left = scene.play(player("fire/candle")?, -0.5, -10.6, ())?;
    assert_eq!(
        scene.rect(left).map(|r| (r.x, r.y)),
        Some(((0 - 28) * 3, (-11 - 64) * 3))
    );
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
fn light_words_become_opaque_rgb_where_lit() {
    // xBGR words; the top byte is ignored. Lit pixels get alpha 255 (dst += src under "lighter"), dark ones
    // alpha 0 so a transparent canvas stays transparent there.
    let f = tiny([0; 3], [0x7700_0000, 0xab03_0201, 0x00ff_8040]);
    let mut out = [9u8; 12];
    assert!(light_rgba(&f, &mut out));
    assert_eq!(out, [0, 0, 0, 0, 1, 2, 3, 255, 0x40, 0x80, 0xff, 255]);
    let mut empty = [9u8; 12];
    assert!(
        !light_rgba(&tiny([0xffff_ffff; 3], [0xff00_0000; 3]), &mut empty),
        "no light"
    );
    assert_eq!(empty, [0; 12]);
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
