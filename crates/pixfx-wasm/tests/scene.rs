//! The scene JS shells follow, natively: the draw list it hands a shell, and frames painted on a worker, sent to
//! the host as bytes with only their shape passing through the scene, equal to frames painted on the spot.

use std::collections::HashMap;
use std::error::Error;
use std::num::NonZeroU32;

use pixfx_core::{Clip, Options};
use pixfx_effects::effect;
use pixfx_wasm::scene::{
    Cut, Frames, OP_FREE, OP_MAKE, OP_REFILL, OP_SHOW, OP_STRIDE, Painting, Scene, Shape, Spec,
};

type Res = Result<(), Box<dyn Error>>;

fn candle() -> Result<(Clip, Spec), Box<dyn Error>> {
    let fx = effect("fire/candle").ok_or("fire/candle not registered")?;
    Ok((
        Clip::live(fx, Options::default()),
        Spec {
            effect: "fire/candle".into(),
            opts: String::new(),
        },
    ))
}

fn scale(n: u32) -> Result<NonZeroU32, Box<dyn Error>> {
    NonZeroU32::new(n).ok_or_else(|| "scale 0".into())
}

/// The ops of a draw list, one slice each.
fn ops(list: &[i32]) -> Vec<&[i32]> {
    list.chunks(OP_STRIDE).collect()
}

fn kinds(list: &[i32]) -> Vec<i32> {
    ops(list).iter().map(|o| o[0]).collect()
}

#[test]
fn the_first_draw_makes_each_surface_before_showing_it() -> Res {
    let mut frames = Frames::with(false);
    let mut scene = Scene::new(&mut frames, scale(2)?);
    let (clip, spec) = candle()?;
    scene.play_clip(&mut frames, clip, spec, 30.0, 40.0)?;
    scene.scene().update(0.1)?;
    let list = scene.draw_ops(&mut frames)?;
    let mut made = Vec::new();
    for op in ops(&list) {
        match op[0] {
            OP_MAKE => made.push(op[1]),
            OP_SHOW => assert!(made.contains(&op[1]), "shown before it was made: {list:?}"),
            k => return Err(format!("unexpected op {k}").into()),
        }
    }
    assert!(!made.is_empty());
    Ok(())
}

#[test]
fn a_surface_is_shown_where_its_lit_pixels_land() -> Res {
    let mut frames = Frames::with(false);
    let mut scene = Scene::new(&mut frames, scale(3)?);
    let (clip, spec) = candle()?;
    let h = scene.play_clip(&mut frames, clip, spec, 30.0, 40.0)?;
    scene.scene().update(0.1)?;
    let rect = scene.scene().rect(h).ok_or("no rect")?;
    let list = scene.draw_ops(&mut frames)?;
    for op in ops(&list).iter().filter(|o| o[0] == OP_SHOW) {
        let made = ops(&list)
            .into_iter()
            .find(|m| m[0] == OP_MAKE && m[1] == op[1])
            .ok_or("no make")?;
        // the crop's size, scaled, inside the effect's rect
        assert_eq!((op[4], op[5]), (made[2] * 3, made[3] * 3));
        assert!(i64::from(op[2]) >= rect.x && i64::from(op[3]) >= rect.y);
        assert!(i64::from(op[2] + op[4]) <= rect.x + rect.w as i64);
        assert_eq!(op[6], h as i32);
    }
    Ok(())
}

#[test]
fn a_hit_stop_paints_nothing_new() -> Res {
    let mut frames = Frames::with(false);
    let mut scene = Scene::new(&mut frames, scale(1)?);
    let (clip, spec) = candle()?;
    scene.play_clip(&mut frames, clip, spec, 30.0, 40.0)?;
    scene.scene().update(0.1)?;
    scene.draw_ops(&mut frames)?;
    frames.end_frame_ops()?;
    scene.scene().update(0.0)?;
    let list = scene.draw_ops(&mut frames)?;
    assert!(
        kinds(&list).iter().all(|&k| k == OP_SHOW),
        "only shows: {list:?}"
    );
    Ok(())
}

/// A JS host as the test sees it: the bytes each surface holds, filled the way a shell fills them.
#[derive(Default)]
struct Host {
    surfaces: HashMap<i32, Vec<u8>>,
}

impl Host {
    /// Carry out `list`: a make or refill takes its bytes from `bytes` (the worker's, for a delivery) or, painted on
    /// the spot, from the frames.
    fn apply(&mut self, list: &[i32], frames: &mut Frames, bytes: Option<&[u8]>) -> Res {
        for op in ops(list) {
            match op[0] {
                OP_MAKE | OP_REFILL => {
                    let got = match bytes {
                        Some(b) => b.to_vec(),
                        None => frames
                            .recorder()
                            .color(u32::try_from(op[1])?)
                            .ok_or("no pixels on the spot")?
                            .to_vec(),
                    };
                    assert_eq!(got.len(), (op[2] * op[3] * 4) as usize, "bytes fit {op:?}");
                    self.surfaces.insert(op[1], got);
                }
                OP_FREE => {
                    self.surfaces.remove(&op[1]);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Play the same candle in `frames` and draw `steps` host frames of 1/30 s, painting jobs on a worker when there
/// are any; returns the colour bytes of every surface shown in the last draw, as the host holds them.
fn shown_pixels(
    frames: &mut Frames,
    steps: usize,
    stop_at: usize,
) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let mut scene = Scene::new(frames, scale(1)?);
    let mut worker = Painting::default();
    let mut host = Host::default();
    let (clip, spec) = candle()?;
    let h = scene.play_clip(frames, clip, spec, 30.0, 40.0)?;
    let mut last = Vec::new();
    for step in 0..steps {
        if step == stop_at {
            scene.scene().stop(h);
        }
        scene.scene().update(1.0 / 30.0)?;
        // ask, paint on the worker, take back, then draw what is held
        let list = scene.draw_ops(frames)?;
        host.apply(&list, frames, None)?;
        let (jobs, _) = frames.take_jobs();
        for job in &jobs {
            let (clip, _) = candle()?;
            let cut = Cut::of(worker.paint_with(job, || Some(clip)).ok_or("not painted")?);
            let list = scene.deliver_ops(frames, job.job, cut.shape)?;
            host.apply(&list, frames, Some(&cut.color))?;
        }
        last = scene.draw_ops(frames)?;
        host.apply(&last, frames, None)?;
        let list = frames.end_frame_ops()?;
        host.apply(&list, frames, None)?;
    }
    let mut pixels = Vec::new();
    for op in ops(&last).iter().filter(|o| o[0] == OP_SHOW) {
        pixels.push(host.surfaces.get(&op[1]).ok_or("shown, not held")?.clone());
    }
    Ok(pixels)
}

#[test]
fn a_shape_that_does_not_fit_the_frame_asked_for_is_refused() -> Res {
    let mut frames = Frames::with(true);
    let mut scene = Scene::new(&mut frames, scale(1)?);
    let (clip, spec) = candle()?;
    scene.play_clip(&mut frames, clip, spec, 30.0, 40.0)?;
    scene.scene().update(0.1)?;
    scene.draw_ops(&mut frames)?;
    let (jobs, _) = frames.take_jobs();
    let job = jobs.first().ok_or("no job")?;
    let shape = Shape {
        frame_w: 1,
        x: 0,
        y: 0,
        w: 1,
        h: 1,
        sides: 1,
    };
    assert!(scene.deliver_ops(&mut frames, job.job, shape).is_err());
    Ok(())
}

#[test]
fn frames_painted_on_a_worker_equal_frames_painted_on_the_spot() -> Res {
    for (steps, stop_at) in [(5, usize::MAX), (8, 4)] {
        let inline = shown_pixels(&mut Frames::with(false), steps, stop_at)?;
        let workers = shown_pixels(&mut Frames::with(true), steps, stop_at)?;
        assert!(!inline.is_empty());
        assert_eq!(inline, workers, "steps {steps}, stopped at {stop_at}");
    }
    Ok(())
}

#[test]
fn every_surface_made_is_freed_once_the_scene_clears() -> Res {
    let mut frames = Frames::with(false);
    let mut scene = Scene::new(&mut frames, scale(1)?);
    let (clip, spec) = candle()?;
    let h = scene.play_clip(&mut frames, clip, spec, 30.0, 40.0)?;
    let mut all = Vec::new();
    for step in 0..10 {
        if step == 5 {
            scene.scene().stop(h);
        }
        scene.scene().update(1.0 / 30.0)?;
        all.extend(scene.draw_ops(&mut frames)?);
        all.extend(frames.end_frame_ops()?);
    }
    all.extend(scene.clear_ops(&mut frames)?);
    all.extend(frames.end_frame_ops()?);
    // the shared frames stay for other scenes until the game clears them
    let made = ops(&all).iter().filter(|o| o[0] == OP_MAKE).count();
    let refilled = ops(&all).iter().filter(|o| o[0] == OP_REFILL).count();
    let freed = ops(&all).iter().filter(|o| o[0] == OP_FREE).count();
    assert!(refilled > 0, "a fading effect refills its own surfaces");
    assert!(freed > 0 && freed <= made);
    Ok(())
}
