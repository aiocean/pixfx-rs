#![cfg(feature = "feedback-comet-weave")]

use std::sync::Arc;

use pixfx_core::{Actor, Instance, Layer, OptValue, Options, render_frames_with_actor};
use pixfx_draw::{Frame, color::WHITE};

#[test]
fn weaves_around_the_visible_pose_and_clears_both_layers() -> Result<(), String> {
    let effect = pixfx_effects::effect("feedback/comet-weave").ok_or("comet weave missing")?;
    assert!(effect.wraps_actor());
    assert!(!effect.meta().looping);

    let render = |x, y, w, h, pixels| {
        let actor = Arc::new(Actor::new(x, y, w, h, pixels).map_err(|e| e.to_string())?);
        Ok::<_, String>(render_frames_with_actor(
            effect,
            &Options::default(),
            0,
            |_| Some(Arc::clone(&actor)),
        ))
    };
    let white = WHITE.pack();
    let visible = render(0, -12, 1, 1, vec![white])?;
    let padded = render(-1, -13, 3, 3, vec![0, 0, 0, 0, white, 0, 0, 0, 0])?;
    let moved = render(12, -12, 1, 1, vec![white])?;
    let empty = render(0, -12, 1, 1, vec![0])?;

    let blank = |frame: &Frame| frame.color.iter().chain(&frame.light).all(|&p| p == 0);
    assert_eq!(visible, padded);
    assert_ne!(visible, moved);
    assert!(visible.frames.iter().any(|f| !blank(f)));
    assert!(visible.frames.last().is_some_and(blank));
    assert!(empty.frames.iter().all(blank));
    Ok(())
}

#[test]
fn one_comet_leaves_a_wake_on_both_sides_of_the_sprite() -> Result<(), String> {
    let effect = pixfx_effects::effect("feedback/comet-weave").ok_or("comet weave missing")?;
    let canvas = effect.canvas(&Options::default());
    let mut instance = Instance::new(
        effect,
        Options {
            x: f64::from(canvas.origin.0),
            y: f64::from(canvas.origin.1),
            ..Options::default()
        },
    );
    let visible = |instance: &Instance, layer| {
        let mut frame = Frame::new(canvas.size.0, canvas.size.1, 0, 0);
        instance.draw(&mut frame, layer, instance.placement());
        frame.color.iter().any(|&pixel| pixel != 0)
    };

    assert!(visible(&instance, Layer::Front));
    assert!(!visible(&instance, Layer::Back));
    // The lead comet has crossed behind the sprite and the next has yet to launch.
    instance.update(1.01 / f64::from(effect.meta().fps));
    assert!(visible(&instance, Layer::Front));
    assert!(visible(&instance, Layer::Back));
    Ok(())
}

#[test]
fn length_extends_the_weave_along_the_projectile_heading() -> Result<(), String> {
    let effect = pixfx_effects::effect("feedback/comet-weave").ok_or("comet weave missing")?;
    let actor =
        Arc::new(Actor::new(-6, -3, 13, 7, vec![WHITE.pack(); 13 * 7]).map_err(|e| e.to_string())?);
    let span = |length| {
        let opts = Options {
            angle: std::f64::consts::FRAC_PI_2,
            extra: [("length".into(), OptValue::Num(length))].into(),
            ..Options::default()
        };
        render_frames_with_actor(effect, &opts, 0, |_| Some(Arc::clone(&actor)))
            .frames
            .iter()
            .map(|frame| {
                let width = frame.w as usize;
                let xs = frame.color.iter().enumerate().filter_map(|(i, &pixel)| {
                    (pixel != 0 && pixel != WHITE.pack()).then_some(i % width)
                });
                xs.clone().max().unwrap_or(0) - xs.min().unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    };
    assert!(span(4.0) >= 2 * span(1.0));
    Ok(())
}
