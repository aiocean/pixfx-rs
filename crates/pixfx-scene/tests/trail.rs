//! A trail (`Scene::trail`) draws the way its mover came: straight behind a mover going straight, bending where
//! it turned, longer the faster it went, and gone soon after its mover.

use std::error::Error;

#[path = "common/recorder.rs"]
mod recorder;

use pixfx_core::{Clip, Layer, OptValue, Options};
use pixfx_draw::Frame;
use pixfx_effects::effect;
use pixfx_scene::{Frames, InlinePainter, Scene, StageEvent};
use recorder::Recorder;

type Res = Result<(), Box<dyn Error>>;

const STEP: f64 = 1.0 / 60.0;

fn clip(id: &str, extra: &[(&str, f64)]) -> Result<Clip, Box<dyn Error>> {
    let fx = effect(id).ok_or("effect not built")?;
    let mut o = Options::default();
    for &(k, v) in extra {
        o.extra.insert(k.to_string(), OptValue::Num(v));
    }
    Ok(Clip::live(fx, o))
}

/// Play a mover with a trail, move it through `path` a point a step, and return the trail's pixels' box
/// (width, height) as drawn after the last step.
fn trail_box(path: &[(f64, f64)]) -> Result<(u32, u32), Box<dyn Error>> {
    let mut frames: Frames<Frame, InlinePainter> = Frames::default();
    let mut s: Scene<Frame> = frames.scene(std::num::NonZeroU32::MIN);
    let (x0, y0) = path.first().copied().ok_or("no path")?;
    let mover = s.play(clip("feedback/power-up", &[])?, x0, y0)?;
    let tail = s
        .trail(
            mover,
            clip("projectiles/trail", &[("secs", 0.2), ("wave", 0.0)])?,
        )?
        .ok_or("the trail was refused")?;
    for &(x, y) in &path[1..] {
        s.move_to(mover, x, y)?;
        s.update(STEP)?;
    }
    let mut r = Recorder::default();
    assert_eq!(s.draw(&mut frames, &mut r), Ok(()));
    let (_, f) = r
        .shown
        .iter()
        .find(|(p, _)| p.handle == tail && p.layer == Layer::Back)
        .ok_or("the trail was not shown")?;
    let (mut lo, mut hi) = ((u32::MAX, u32::MAX), (0, 0));
    for (i, &c) in f.color.iter().enumerate() {
        if c >> 24 == 0 {
            continue;
        }
        let (x, y) = (i as u32 % f.w, i as u32 / f.w);
        lo = (lo.0.min(x), lo.1.min(y));
        hi = (hi.0.max(x), hi.1.max(y));
    }
    if lo.0 > hi.0 {
        return Ok((0, 0));
    }
    let d = f.density().factor();
    Ok(((hi.0 - lo.0 + 1).div_ceil(d), (hi.1 - lo.1 + 1).div_ceil(d)))
}

#[test]
fn a_trail_lies_straight_behind_a_straight_mover_and_bends_round_a_turning_one() -> Res {
    let straight: Vec<(f64, f64)> = (0..20)
        .map(|i| (100.0 + 3.0 * f64::from(i), 100.0))
        .collect();
    let (w, h) = trail_box(&straight)?;
    assert!(w >= 25 && h <= 4, "straight: {w} x {h}");
    // right, then turning down for its last four steps
    let turn: Vec<(f64, f64)> = (0..20)
        .map(|i| {
            let i = f64::from(i);
            if i < 16.0 {
                (100.0 + 3.0 * i, 100.0)
            } else {
                (145.0, 100.0 + 3.0 * (i - 15.0))
            }
        })
        .collect();
    let (w, h) = trail_box(&turn)?;
    assert!(w >= 8 && h >= 10, "turning: {w} x {h}");
    Ok(())
}

#[test]
fn a_fast_mover_leaves_a_longer_trail_than_a_slow_one() -> Res {
    let line = |speed: f64| -> Vec<(f64, f64)> {
        (0..30)
            .map(|i| (100.0 + speed * f64::from(i), 100.0))
            .collect()
    };
    let (fast, _) = trail_box(&line(4.0))?;
    let (slow, _) = trail_box(&line(1.0))?;
    assert!(fast >= 2 * slow && slow > 0, "fast {fast}, slow {slow}");
    Ok(())
}

#[test]
fn a_trail_ends_soon_after_its_mover_and_an_effect_that_draws_no_trace_is_refused() -> Res {
    let mut frames: Frames<Frame, InlinePainter> = Frames::default();
    let mut s: Scene<Frame> = frames.scene(std::num::NonZeroU32::MIN);
    let mover = s.play(clip("feedback/power-up", &[])?, 0.0, 0.0)?;
    assert_eq!(s.trail(mover, clip("feedback/power-up", &[])?)?, None);
    assert_eq!(s.trail(999, clip("projectiles/trail", &[])?)?, None);
    let tail = s
        .trail(mover, clip("projectiles/trail", &[("secs", 0.2)])?)?
        .ok_or("refused")?;
    for i in 1..=10 {
        s.move_to(mover, 4.0 * f64::from(i), 0.0)?;
        s.update(STEP)?;
    }
    assert!(s.remove(mover));
    let mut ended = None;
    for i in 1..=30 {
        s.update(STEP)?;
        if s.drain_events().contains(&StageEvent::End { handle: tail }) {
            ended = Some(f64::from(i) * STEP);
            break;
        }
    }
    let ended = ended.ok_or("the trail never ended")?;
    assert!(ended <= 0.2 + 2.0 * STEP, "ended {ended} s after its mover");
    Ok(())
}

#[test]
fn a_trail_played_on_its_own_follows_where_the_game_moves_it_and_ends_once_stopped() -> Res {
    let mut frames: Frames<Frame, InlinePainter> = Frames::default();
    let mut s: Scene<Frame> = frames.scene(std::num::NonZeroU32::MIN);
    let tail = s.play(
        clip("projectiles/trail", &[("secs", 0.2), ("wave", 0.0)])?,
        100.0,
        100.0,
    )?;
    // standing still it draws nothing
    s.update(STEP)?;
    let mut r = Recorder::default();
    assert_eq!(s.draw(&mut frames, &mut r), Ok(()));
    let lit = |r: &Recorder| {
        r.shown
            .iter()
            .filter(|(p, _)| p.handle == tail && p.layer == Layer::Back)
            .map(|(_, f)| f.color.iter().filter(|&&c| c >> 24 != 0).count())
            .sum::<usize>()
    };
    assert_eq!(lit(&r), 0, "a trail that has not moved drew pixels");
    // moved along like a fighter gliding, it draws behind where it went
    for i in 1..=20 {
        s.move_to(tail, 100.0 + 3.0 * f64::from(i), 100.0)?;
        s.update(STEP)?;
    }
    let mut r = Recorder::default();
    assert_eq!(s.draw(&mut frames, &mut r), Ok(()));
    assert!(lit(&r) >= 25, "the moved trail drew {} pixels", lit(&r));
    assert!(s.stop(tail));
    let mut ended = false;
    for _ in 0..30 {
        s.update(STEP)?;
        if s.drain_events().contains(&StageEvent::End { handle: tail }) {
            ended = true;
            break;
        }
    }
    assert!(ended, "the stopped trail never ended");
    Ok(())
}

#[test]
fn a_trail_that_turns_with_its_mover_sweeps_an_arc() -> Res {
    // a mover standing still while it turns a quarter turn: only a trail turned with it sweeps anything
    let lit = |turn: f64| -> Result<usize, Box<dyn Error>> {
        let mut frames: Frames<Frame, InlinePainter> = Frames::default();
        let mut s: Scene<Frame> = frames.scene(std::num::NonZeroU32::MIN);
        let tail = s.play(
            clip(
                "projectiles/trail",
                &[("secs", 0.3), ("width", 7.0), ("wave", 0.0), ("turn", turn)],
            )?,
            100.0,
            100.0,
        )?;
        for i in 1..=10 {
            let p = s.placement(tail).ok_or("no placement")?;
            s.set(
                tail,
                pixfx_core::Placement {
                    angle: f64::from(i) * 0.15,
                    ..p
                },
            )?;
            s.update(STEP)?;
        }
        let mut r = Recorder::default();
        assert_eq!(s.draw(&mut frames, &mut r), Ok(()));
        Ok(r.shown
            .iter()
            .filter(|(p, _)| p.handle == tail && p.layer == Layer::Back)
            .map(|(_, f)| f.color.iter().filter(|&&c| c >> 24 != 0).count())
            .sum())
    };
    assert_eq!(
        lit(0.0)?,
        0,
        "a ribbon square to its way drew while its mover stood still"
    );
    assert!(
        lit(1.0)? >= 12,
        "a ribbon turned with its mover drew {} pixels",
        lit(1.0)?
    );
    Ok(())
}
