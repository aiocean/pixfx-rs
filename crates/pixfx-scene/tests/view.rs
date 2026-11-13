//! A scene with a view draws only the effects whose frame reaches into it; the rest keep their time and cues.

use std::error::Error;
use std::num::NonZeroU32;

#[path = "common/hitter.rs"]
mod hitter;
#[path = "common/recorder.rs"]
mod recorder;

use hitter::HITTER;
use pixfx_core::{Layer, Options, Player};
use pixfx_draw::Frame;
use pixfx_effects::effect;
use pixfx_scene::{Scene, View};
use recorder::Recorder;

type Res = Result<(), Box<dyn Error>>;

fn draw_once(scene: &mut Scene<Frame>) -> Recorder {
    let mut r = Recorder::default();
    assert_eq!(scene.draw(&mut r), Ok(()));
    r
}

fn candle() -> Result<Player, Box<dyn Error>> {
    let fx = effect("fire/candle").ok_or("fire/candle not registered")?;
    Ok(Player::live(fx, Options::default()))
}

fn one() -> Result<NonZeroU32, Box<dyn Error>> {
    NonZeroU32::new(1).ok_or_else(|| "scale 0".into())
}

const VIEW: View = View {
    x: 0.0,
    y: 0.0,
    w: 100.0,
    h: 100.0,
};

const FRAME: f64 = 1.0 / 60.0;

#[test]
fn an_effect_outside_the_view_is_neither_drawn_nor_shown() -> Res {
    let mut scene: Scene<Frame> = Scene::new(one()?);
    scene.set_view(Some(VIEW))?;
    let inside = scene.play(candle()?, 50.0, 50.0)?;
    scene.play(candle()?, 1000.0, 1000.0)?;
    scene.update(0.1)?;
    let r = draw_once(&mut scene);
    assert_eq!(r.shown.len(), 2, "only the inside candle, back and front");
    assert!(r.shown.iter().all(|(p, _)| p.handle == inside));
    Ok(())
}

#[test]
fn without_a_view_everything_is_shown() -> Res {
    let mut scene: Scene<Frame> = Scene::new(one()?);
    scene.play(candle()?, 50.0, 50.0)?;
    scene.play(candle()?, 1000.0, 1000.0)?;
    scene.update(0.1)?;
    assert_eq!(draw_once(&mut scene).shown.len(), 4);
    scene.set_view(Some(VIEW))?;
    scene.set_view(None)?;
    assert_eq!(draw_once(&mut scene).shown.len(), 4);
    Ok(())
}

#[test]
fn a_glow_reaching_into_the_view_is_drawn() -> Res {
    let mut scene: Scene<Frame> = Scene::new(one()?);
    scene.set_view(Some(VIEW))?;
    let h = scene.play(candle()?, 0.0, 50.0)?;
    let at_zero = scene.rect(h).ok_or("no rect")?;
    let w = i64::try_from(at_zero.w)?;
    // the frame's right edge one pixel inside the view's left edge: the origin is well outside
    let x = 1 - w - at_zero.x;
    scene.move_to(h, x as f64, 50.0)?;
    assert!(x < 0, "the effect's origin is outside the view");
    scene.update(0.1)?;
    assert_eq!(
        draw_once(&mut scene).shown.len(),
        2,
        "one pixel of glow overlaps"
    );
    // one pixel further left: the frame only touches the view's edge
    scene.move_to(h, (x - 1) as f64, 50.0)?;
    assert_eq!(
        draw_once(&mut scene).shown.len(),
        0,
        "touching is not overlapping"
    );
    Ok(())
}

/// Plays `scene` and `twin` the same, moving `h` in `scene` out of the view for `away` seconds and back, and
/// returns the frames both show after the return.
fn back_in_view(stopped: bool, away: f64) -> Result<(Vec<Frame>, Vec<Frame>), Box<dyn Error>> {
    let mut scene: Scene<Frame> = Scene::new(one()?);
    let mut twin: Scene<Frame> = Scene::new(one()?);
    scene.set_view(Some(VIEW))?;
    let h = scene.play(candle()?, 50.0, 50.0)?;
    let t = twin.play(candle()?, 50.0, 50.0)?;
    for s in [&mut scene, &mut twin] {
        s.update(0.1)?;
        draw_once(s);
    }
    if stopped {
        scene.stop(h);
        twin.stop(t);
        for s in [&mut scene, &mut twin] {
            draw_once(s);
        }
    }
    scene.move_to(h, 1000.0, 1000.0)?;
    let steps = (away / FRAME) as usize;
    for _ in 0..steps {
        for s in [&mut scene, &mut twin] {
            s.update(FRAME)?;
            draw_once(s);
        }
    }
    scene.move_to(h, 50.0, 50.0)?;
    let frames = |r: Recorder| r.shown.into_iter().map(|(_, f)| f).collect::<Vec<_>>();
    Ok((frames(draw_once(&mut scene)), frames(draw_once(&mut twin))))
}

#[test]
fn an_effect_coming_back_shows_its_current_frame() -> Res {
    let (culled, seen) = back_in_view(false, 1.0)?;
    assert_eq!(culled.len(), 2);
    assert_eq!(culled, seen);
    Ok(())
}

#[test]
fn a_fading_effect_coming_back_shows_its_current_fade_frame() -> Res {
    // the fade lasts 0.3 s: come back halfway through it
    let (culled, seen) = back_in_view(true, 0.15)?;
    assert_eq!(culled.len(), 2);
    assert_eq!(culled, seen);
    Ok(())
}

#[test]
fn a_hit_outside_the_view_still_freezes_and_shakes() -> Res {
    let mut scene: Scene<Frame> = Scene::new(one()?);
    scene.set_view(Some(VIEW))?;
    scene.play(Player::live(&HITTER, Options::default()), 1000.0, 1000.0)?;
    let mut played = 0.0;
    while !scene.feel().is_some_and(|f| f.frozen()) {
        played += scene.update(FRAME)?;
        draw_once(&mut scene);
        assert!(played < 0.2, "the hit at 0.1 s never froze play");
    }
    assert!(scene.feel().is_some_and(|f| f.trauma() > 0.0));
    Ok(())
}

#[test]
fn a_view_that_is_not_finite_is_refused_and_changes_nothing() -> Res {
    let mut scene: Scene<Frame> = Scene::new(one()?);
    scene.play(candle()?, 1000.0, 1000.0)?;
    scene.update(0.1)?;
    let bad = View {
        w: f64::NAN,
        ..VIEW
    };
    assert_eq!(
        scene.set_view(Some(bad)),
        Err(pixfx_scene::StageError::NotFinite)
    );
    assert_eq!(draw_once(&mut scene).shown.len(), 2, "still no view: drawn");
    Ok(())
}

fn seeded_candle(seed: u64) -> Result<Player, Box<dyn Error>> {
    let fx = effect("fire/candle").ok_or("fire/candle not registered")?;
    let opts = Options {
        seed: Some(seed),
        ..Options::default()
    };
    Ok(Player::live(fx, opts))
}

#[test]
fn under_a_draw_budget_effects_outside_the_view_take_no_turn() -> Res {
    let mut scene: Scene<Frame> = Scene::new(one()?);
    scene.set_view(Some(VIEW))?;
    scene.set_draw_budget(NonZeroU32::new(1));
    // played first, so they have waited longest: without the view they would take the turns
    scene.play(seeded_candle(1)?, 1000.0, 1000.0)?;
    scene.play(seeded_candle(2)?, 2000.0, 2000.0)?;
    let inside = scene.play(seeded_candle(3)?, 50.0, 50.0)?;
    scene.update(0.1)?;
    let mut r = Recorder::default();
    scene
        .draw_layer(Layer::Front, &mut r)
        .map_err(|()| "draw")?;
    assert_eq!(r.made, 1, "the one turn goes to the effect in view");
    assert_eq!(
        r.shown.iter().map(|(p, _)| p.handle).collect::<Vec<_>>(),
        [inside]
    );
    Ok(())
}
