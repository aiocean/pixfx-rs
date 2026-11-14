//! Frames shared by several scenes, and painters that paint elsewhere: a frame one scene painted shows in every
//! scene without being painted again, and a frame painted on another thread shows once it is delivered.

use std::error::Error;
use std::num::NonZeroU32;

#[path = "common/recorder.rs"]
mod recorder;

use pixfx_core::{Clip, Layer, Options};
use pixfx_draw::Frame;
use pixfx_effects::effect;
use pixfx_scene::{EffectKey, Frames, InlinePainter, Job, Painter, Scene, Target};
use recorder::Recorder;

type Res = Result<(), Box<dyn Error>>;

fn candle() -> Result<Clip, Box<dyn Error>> {
    let fx = effect("fire/candle").ok_or("fire/candle not registered")?;
    Ok(Clip::live(fx, Options::default()))
}

fn one() -> Result<NonZeroU32, Box<dyn Error>> {
    NonZeroU32::new(1).ok_or_else(|| "scale 0".into())
}

/// A painter on another thread, as a test sees it: it paints nothing when asked, keeps every job, and paints
/// them all when the test says the work came back.
#[derive(Default)]
struct Later {
    inline: InlinePainter,
    asked: Vec<Asked>,
    ended: Vec<EffectKey>,
}

/// A job a painter was handed, kept to paint later.
struct Asked {
    effect: EffectKey,
    clip: Clip,
    frame: u64,
    fade: f64,
    place: pixfx_core::Placement,
    layer: Layer,
    target: Target,
}

impl Painter for Later {
    fn paint(&mut self, job: &Job<'_>) -> Option<&Frame> {
        self.asked.push(Asked {
            effect: job.effect,
            clip: job.clip.clone(),
            frame: job.frame,
            fade: job.fade,
            place: job.place,
            layer: job.layer,
            target: job.target.clone(),
        });
        None
    }

    fn end(&mut self, effect: EffectKey) {
        self.ended.push(effect);
        self.inline.end(effect);
    }
}

/// Paint every job asked since the last call and hand each frame back to `frames` or its scene.
fn deliver(
    frames: &mut Frames<Frame, Later>,
    scene: &mut Scene<Frame>,
    shell: &mut Recorder,
) -> Res {
    let asked = std::mem::take(&mut frames.painter_mut().asked);
    for a in asked {
        let job = Job {
            effect: a.effect,
            clip: &a.clip,
            frame: a.frame,
            fade: a.fade,
            place: a.place,
            layer: a.layer,
            target: a.target.clone(),
        };
        let frame = frames
            .painter_mut()
            .inline
            .paint(&job)
            .ok_or("the inline painter paints at once")?
            .clone();
        match a.target {
            Target::Shared(id) => frames
                .deliver(&id, a.layer, &frame, shell)
                .map_err(|()| "deliver")?,
            Target::Own { clock, place } => {
                scene
                    .deliver_own(a.effect.handle, a.layer, clock, place, &frame, shell)
                    .map_err(|()| "deliver own")?;
            }
        }
    }
    Ok(())
}

/// One host frame of `scene` drawn from `frames`: what it showed.
fn host_frame<P: Painter>(
    frames: &mut Frames<Frame, P>,
    scene: &mut Scene<Frame>,
    shell: &mut Recorder,
) -> Result<Vec<Frame>, Box<dyn Error>> {
    let before = shell.shown.len();
    scene.draw(frames, shell).map_err(|()| "draw")?;
    frames.end_frame(shell).map_err(|()| "end frame")?;
    Ok(shell.shown[before..]
        .iter()
        .map(|(_, f)| f.clone())
        .collect())
}

#[test]
fn scenes_sharing_frames_paint_each_frame_once() -> Res {
    let mut frames = Frames::default();
    let (mut a, mut b): (Scene<Frame>, Scene<Frame>) = (frames.scene(one()?), frames.scene(one()?));
    a.play(candle()?, 20.0, 40.0)?;
    b.play(candle()?, 60.0, 10.0)?;
    let mut shell = Recorder::default();
    for _ in 0..30 {
        a.update(1.0 / 60.0)?;
        b.update(1.0 / 60.0)?;
        a.draw(&mut frames, &mut shell).map_err(|()| "draw a")?;
        let made = shell.made;
        b.draw(&mut frames, &mut shell).map_err(|()| "draw b")?;
        assert_eq!(
            shell.made, made,
            "the second scene shows what the first painted"
        );
        frames.end_frame(&mut shell).map_err(|()| "end frame")?;
    }
    assert!(shell.made > 0);
    Ok(())
}

#[test]
fn a_frame_painted_elsewhere_shows_once_delivered_and_is_asked_for_once() -> Res {
    let mut frames = Frames::new(Later::default());
    let mut scene: Scene<Frame> = frames.scene(one()?);
    let mut shell = Recorder::default();
    scene.play(candle()?, 20.0, 40.0)?;
    scene.play(candle()?, 20.0, 40.0)?;
    scene.update(0.1)?;
    // nothing painted yet: nothing to show, and each layer is asked for once though two effects show it
    assert!(host_frame(&mut frames, &mut scene, &mut shell)?.is_empty());
    assert_eq!(frames.painter().asked.len(), 2, "back and front, once each");
    assert!(host_frame(&mut frames, &mut scene, &mut shell)?.is_empty());
    assert_eq!(
        frames.painter().asked.len(),
        2,
        "not asked again while on its way"
    );
    deliver(&mut frames, &mut scene, &mut shell)?;
    let shown = host_frame(&mut frames, &mut scene, &mut shell)?;
    assert_eq!(shown.len(), 4, "both layers of both effects");
    // the same frames an inline painter paints
    let mut inline = Frames::default();
    let mut twin: Scene<Frame> = inline.scene(one()?);
    twin.play(candle()?, 20.0, 40.0)?;
    twin.play(candle()?, 20.0, 40.0)?;
    twin.update(0.1)?;
    assert_eq!(
        shown,
        host_frame(&mut inline, &mut twin, &mut Recorder::default())?
    );
    Ok(())
}

#[test]
fn a_new_frame_on_its_way_leaves_the_last_one_shown() -> Res {
    let mut frames = Frames::new(Later::default());
    let mut scene: Scene<Frame> = frames.scene(one()?);
    let mut shell = Recorder::default();
    scene.play(candle()?, 20.0, 40.0)?;
    scene.update(0.1)?;
    host_frame(&mut frames, &mut scene, &mut shell)?;
    deliver(&mut frames, &mut scene, &mut shell)?;
    let before = host_frame(&mut frames, &mut scene, &mut shell)?;
    scene.update(0.1)?;
    assert_eq!(
        host_frame(&mut frames, &mut scene, &mut shell)?,
        before,
        "the old frame while the new paints"
    );
    deliver(&mut frames, &mut scene, &mut shell)?;
    assert_ne!(host_frame(&mut frames, &mut scene, &mut shell)?, before);
    Ok(())
}

#[test]
fn a_fading_effect_painted_elsewhere_shows_its_own_frames() -> Res {
    let mut frames = Frames::new(Later::default());
    let mut scene: Scene<Frame> = frames.scene(one()?);
    let mut shell = Recorder::default();
    let h = scene.play(candle()?, 20.0, 40.0)?;
    scene.update(0.1)?;
    scene.stop(h);
    scene.update(0.05)?;
    assert!(host_frame(&mut frames, &mut scene, &mut shell)?.is_empty());
    assert!(
        frames
            .painter()
            .asked
            .iter()
            .all(|a| matches!(a.target, Target::Own { .. })),
        "a fading effect asks for its own frames"
    );
    deliver(&mut frames, &mut scene, &mut shell)?;
    let shown = host_frame(&mut frames, &mut scene, &mut shell)?;
    let mut inline = Frames::default();
    let mut twin: Scene<Frame> = inline.scene(one()?);
    let t = twin.play(candle()?, 20.0, 40.0)?;
    twin.update(0.1)?;
    twin.stop(t);
    twin.update(0.05)?;
    assert_eq!(
        shown,
        host_frame(&mut inline, &mut twin, &mut Recorder::default())?
    );
    Ok(())
}

#[test]
fn the_painter_lets_go_of_every_effect_that_ends() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    let mut shell = Recorder::default();
    let stopped = scene.play(candle()?, 20.0, 40.0)?;
    let removed = scene.play(candle()?, 60.0, 40.0)?;
    scene.play(candle()?, 90.0, 40.0)?;
    scene.update(0.1)?;
    host_frame(&mut frames, &mut scene, &mut shell)?;
    assert_eq!(
        frames.painter().len(),
        1,
        "identical effects: the first paints every shared frame"
    );
    scene.stop(stopped);
    scene.update(0.05)?;
    host_frame(&mut frames, &mut scene, &mut shell)?;
    assert_eq!(frames.painter().len(), 2, "a fading effect paints its own");
    scene.remove(removed);
    scene.update(1.0)?;
    host_frame(&mut frames, &mut scene, &mut shell)?;
    assert_eq!(frames.painter().len(), 1, "the faded one let go");
    scene.clear(&mut frames, &mut shell).map_err(|()| "clear")?;
    assert!(frames.painter().is_empty());
    Ok(())
}

#[test]
fn a_painter_that_falls_behind_is_asked_for_one_frame_per_layer_at_a_time() -> Res {
    let mut frames = Frames::new(Later::default());
    let mut scene: Scene<Frame> = frames.scene(one()?);
    let mut shell = Recorder::default();
    let h = scene.play(candle()?, 20.0, 40.0)?;
    for _ in 0..20 {
        scene.update(1.0 / 30.0)?;
        host_frame(&mut frames, &mut scene, &mut shell)?;
    }
    assert_eq!(
        frames.painter().asked.len(),
        2,
        "back and front, once each, however far the clock moved"
    );
    // fading, its own frames are asked one at a time too
    deliver(&mut frames, &mut scene, &mut shell)?;
    scene.stop(h);
    for _ in 0..10 {
        scene.update(1.0 / 30.0)?;
        host_frame(&mut frames, &mut scene, &mut shell)?;
    }
    assert!(frames.painter().asked.len() <= 2, "one own frame per layer");
    Ok(())
}

#[test]
fn a_frame_that_comes_back_after_its_clock_moved_on_still_shows() -> Res {
    let mut frames = Frames::new(Later::default());
    let mut scene: Scene<Frame> = frames.scene(one()?);
    let mut shell = Recorder::default();
    scene.play(candle()?, 20.0, 40.0)?;
    scene.update(0.1)?;
    host_frame(&mut frames, &mut scene, &mut shell)?;
    // the clock moves on while the frame of 0.1 s is painted elsewhere
    scene.update(0.5)?;
    host_frame(&mut frames, &mut scene, &mut shell)?;
    deliver(&mut frames, &mut scene, &mut shell)?;
    let shown = host_frame(&mut frames, &mut scene, &mut shell)?;
    // the frame of 0.1 s, as painted on the spot
    let mut inline = Frames::default();
    let mut twin: Scene<Frame> = inline.scene(one()?);
    twin.play(candle()?, 20.0, 40.0)?;
    twin.update(0.1)?;
    assert!(!shown.is_empty());
    assert_eq!(
        shown,
        host_frame(&mut inline, &mut twin, &mut Recorder::default())?
    );
    // and the frame the clock is on now is asked for
    assert_eq!(frames.painter().asked.len(), 2);
    deliver(&mut frames, &mut scene, &mut shell)?;
    let now = host_frame(&mut frames, &mut scene, &mut shell)?;
    twin.update(0.5)?;
    assert_eq!(
        now,
        host_frame(&mut inline, &mut twin, &mut Recorder::default())?
    );
    Ok(())
}

#[test]
fn a_shared_frame_counts_against_the_budget_as_the_lit_box_a_shell_holds() -> Res {
    let mut frames = Frames::default();
    let mut scene: Scene<Frame> = frames.scene(one()?);
    scene.play(candle()?, 30.0, 40.0)?;
    let mut shell = Recorder::default();
    scene.update(1.0 / 60.0)?;
    let shown = host_frame(&mut frames, &mut scene, &mut shell)?;
    assert!(!shown.is_empty());
    let lit: u64 = shown
        .iter()
        .filter_map(pixfx_core::PixelBox::lit)
        .map(|b| u64::from(b.w) * u64::from(b.h) * 8)
        .sum();
    let whole: u64 = shown
        .iter()
        .map(|f| u64::from(f.w) * u64::from(f.h) * 8)
        .sum();
    assert!(lit < whole, "a candle lights a small part of its frame");
    assert_eq!(frames.bytes(), lit);
    Ok(())
}
