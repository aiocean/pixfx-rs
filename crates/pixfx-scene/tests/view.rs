//! A scene with a view draws only the effects whose frame reaches into it; the rest keep their time and cues.

use std::error::Error;
use std::num::NonZeroU32;

#[path = "common/hitter.rs"]
mod hitter;
#[path = "common/recorder.rs"]
mod recorder;

use hitter::HITTER;
use pixfx_core::{Clip, Layer, Options};
use pixfx_draw::Frame;
use pixfx_effects::effect;
use pixfx_scene::{Frames, InlinePainter, Scene, View};
use recorder::Recorder;

type Res = Result<(), Box<dyn Error>>;

/// One host frame.
fn draw_once(frames: &mut Frames<Frame, InlinePainter>, scene: &mut Scene<Frame>) -> Recorder {
    let mut r = Recorder::default();
    assert_eq!(scene.draw(frames, &mut r), Ok(()));
    assert_eq!(frames.end_frame(&mut r), Ok(()));
    r
}

fn buff() -> Result<Clip, Box<dyn Error>> {
    let fx = effect("feedback/power-up").ok_or("feedback/power-up not registered")?;
    Ok(Clip::live(fx, Options::default()))
}

fn one() -> Result<NonZeroU32, Box<dyn Error>> {
    NonZeroU32::new(1).ok_or_else(|| "scale 0".into())
}

const VIEW: View = View {
    x: 0.0,
    y: 0.0,
    w: 100.0,
    h: 100.0,
    scroll_x: 0.0,
    scroll_y: 0.0,
};

/// A camera scrolled 1000 game pixels right: it sees world x 1000 to 1100.
const SCROLLED: View = View {
    x: 1000.0,
    scroll_x: 1000.0,
    ..VIEW
};

const FRAME: f64 = 1.0 / 60.0;

#[test]
fn an_effect_outside_the_view_is_neither_drawn_nor_shown() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.set_view(Some(VIEW))?;
    let inside = scene.play(buff()?, 50.0, 50.0)?;
    scene.play(buff()?, 1000.0, 1000.0)?;
    scene.update(0.1)?;
    let r = draw_once(&mut frames, &mut scene);
    assert_eq!(r.shown.len(), 2, "only the inside effect, back and front");
    assert!(r.shown.iter().all(|(p, _)| p.handle == inside));
    Ok(())
}

#[test]
fn without_a_view_everything_is_shown() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.play(buff()?, 50.0, 50.0)?;
    scene.play(buff()?, 1000.0, 1000.0)?;
    scene.update(0.1)?;
    assert_eq!(draw_once(&mut frames, &mut scene).shown.len(), 4);
    scene.set_view(Some(VIEW))?;
    scene.set_view(None)?;
    assert_eq!(draw_once(&mut frames, &mut scene).shown.len(), 4);
    Ok(())
}

#[test]
fn a_glow_reaching_into_the_view_is_drawn() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.set_view(Some(VIEW))?;
    let h = scene.play(buff()?, 0.0, 50.0)?;
    let at_zero = scene.rect(h).ok_or("no rect")?;
    let w = i64::try_from(at_zero.w)?;
    // the frame's right edge one pixel inside the view's left edge: the origin is well outside
    let x = 1 - w - at_zero.x;
    scene.move_to(h, x as f64, 50.0)?;
    assert!(x < 0, "the effect's origin is outside the view");
    scene.update(0.1)?;
    assert_eq!(
        draw_once(&mut frames, &mut scene).shown.len(),
        2,
        "one pixel of glow overlaps"
    );
    // one pixel further left: the frame only touches the view's edge
    scene.move_to(h, (x - 1) as f64, 50.0)?;
    assert_eq!(
        draw_once(&mut frames, &mut scene).shown.len(),
        0,
        "touching is not overlapping"
    );
    Ok(())
}

/// Plays `scene` and `twin` the same, moving `h` in `scene` out of the view for `away` seconds and back, and
/// returns the frames both show after the return.
fn back_in_view(stopped: bool, away: f64) -> Result<(Vec<Frame>, Vec<Frame>), Box<dyn Error>> {
    // each with its own frames, so the twin's frames cannot stand in for the scene's
    let (mut ours, mut theirs) = (Frames::default(), Frames::default());
    let mut scene: Scene<Frame> = ours.scene(one()?);
    let mut twin: Scene<Frame> = theirs.scene(one()?);
    scene.set_view(Some(VIEW))?;
    let h = scene.play(buff()?, 50.0, 50.0)?;
    let t = twin.play(buff()?, 50.0, 50.0)?;
    for (f, s) in [(&mut ours, &mut scene), (&mut theirs, &mut twin)] {
        s.update(0.1)?;
        draw_once(f, s);
    }
    if stopped {
        scene.stop(h);
        twin.stop(t);
        for (f, s) in [(&mut ours, &mut scene), (&mut theirs, &mut twin)] {
            draw_once(f, s);
        }
    }
    scene.move_to(h, 1000.0, 1000.0)?;
    let steps = (away / FRAME) as usize;
    for _ in 0..steps {
        for (f, s) in [(&mut ours, &mut scene), (&mut theirs, &mut twin)] {
            s.update(FRAME)?;
            draw_once(f, s);
        }
    }
    scene.move_to(h, 50.0, 50.0)?;
    let shown = |r: Recorder| r.shown.into_iter().map(|(_, f)| f).collect::<Vec<_>>();
    Ok((
        shown(draw_once(&mut ours, &mut scene)),
        shown(draw_once(&mut theirs, &mut twin)),
    ))
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
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.set_view(Some(VIEW))?;
    scene.play(Clip::live(&HITTER, Options::default()), 1000.0, 1000.0)?;
    let mut played = 0.0;
    while !scene.feel().is_some_and(|f| f.frozen()) {
        played += scene.update(FRAME)?;
        draw_once(&mut frames, &mut scene);
        assert!(played < 0.2, "the hit at 0.1 s never froze play");
    }
    assert!(scene.feel().is_some_and(|f| f.trauma() > 0.0));
    Ok(())
}

#[test]
fn a_view_that_is_not_finite_is_refused_and_changes_nothing() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.play(buff()?, 1000.0, 1000.0)?;
    scene.update(0.1)?;
    let bad = View {
        w: f64::NAN,
        ..VIEW
    };
    assert_eq!(
        scene.set_view(Some(bad)),
        Err(pixfx_scene::StageError::NotFinite)
    );
    assert_eq!(
        draw_once(&mut frames, &mut scene).shown.len(),
        2,
        "still no view: drawn"
    );
    Ok(())
}

fn seeded_buff(seed: u64) -> Result<Clip, Box<dyn Error>> {
    let fx = effect("feedback/power-up").ok_or("feedback/power-up not registered")?;
    let opts = Options {
        seed: Some(seed),
        ..Options::default()
    };
    Ok(Clip::live(fx, opts))
}

#[test]
fn under_a_draw_budget_effects_outside_the_view_take_no_turn() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.set_view(Some(VIEW))?;
    scene.set_draw_budget(NonZeroU32::new(1));
    // played first, so they have waited longest: without the view they would take the turns
    scene.play(seeded_buff(1)?, 1000.0, 1000.0)?;
    scene.play(seeded_buff(2)?, 2000.0, 2000.0)?;
    let inside = scene.play(seeded_buff(3)?, 50.0, 50.0)?;
    scene.update(0.1)?;
    let mut r = Recorder::default();
    scene
        .draw_layer(Layer::Front, &mut frames, &mut r)
        .map_err(|()| "draw")?;
    assert_eq!(r.made, 1, "the one turn goes to the effect in view");
    assert_eq!(
        r.shown.iter().map(|(p, _)| p.handle).collect::<Vec<_>>(),
        [inside]
    );
    Ok(())
}

/// Which handles the scene shows in one host frame.
fn shown_handles(frames: &mut Frames<Frame, InlinePainter>, scene: &mut Scene<Frame>) -> Vec<u32> {
    let mut h: Vec<u32> = draw_once(frames, scene)
        .shown
        .iter()
        .map(|(p, _)| p.handle)
        .collect();
    h.dedup();
    h
}

#[test]
fn an_effect_fixed_to_the_camera_is_seen_wherever_the_camera_scrolls() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.set_view(Some(SCROLLED))?;
    // both at (50, 50): in the world, far left of the camera; fixed to the camera, inside it
    let hud = scene.play(buff()?, 50.0, 50.0)?;
    scene.play(buff()?, 50.0, 50.0)?;
    assert_eq!(scene.set_scroll_factor(hud, 0.0), Ok(true));
    scene.update(0.1)?;
    assert_eq!(shown_handles(&mut frames, &mut scene), [hud]);
    Ok(())
}

#[test]
fn a_parallax_effect_is_seen_where_its_scroll_factor_puts_it() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.set_view(Some(SCROLLED))?;
    // at half the camera's scroll, the camera sees x 500 to 600 of this effect's plane
    let near = scene.play(buff()?, 550.0, 50.0)?;
    let far = scene.play(buff()?, 1050.0, 50.0)?;
    for h in [near, far] {
        assert_eq!(scene.set_scroll_factor(h, 0.5), Ok(true));
    }
    scene.update(0.1)?;
    assert_eq!(shown_handles(&mut frames, &mut scene), [near]);
    Ok(())
}

#[test]
fn a_scroll_factor_that_is_not_finite_is_refused() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    let h = scene.play(buff()?, 50.0, 50.0)?;
    assert_eq!(
        scene.set_scroll_factor(h, f64::INFINITY),
        Err(pixfx_scene::StageError::NotFinite)
    );
    assert_eq!(
        scene.set_scroll_factor(h + 1, 0.0),
        Ok(false),
        "no such effect"
    );
    Ok(())
}
