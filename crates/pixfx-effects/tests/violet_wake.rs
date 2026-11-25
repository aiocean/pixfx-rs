#![cfg(feature = "projectiles-violet-wake")]

use std::sync::Arc;

use pixfx_core::{Effect, Layer, Options, Trace, TracePoint, render_frames};
use pixfx_draw::{Frame, Painter, frame_hash};
use pixfx_effects::projectiles::VIOLET_WAKE;

#[test]
fn stationary_preview_rolls_and_fades() -> Result<(), &'static str> {
    let frames = render_frames(&VIOLET_WAKE, &Options::default(), 0).frames;
    let first = frames.first().ok_or("missing first frame")?;
    let last = frames.last().ok_or("missing last frame")?;
    let middle = frames.get(8..=9).ok_or("missing sweep frames")?;
    assert!(first.color.iter().all(|&pixel| pixel == 0));
    assert!(last.color.iter().all(|&pixel| pixel == 0));
    assert!(
        frames
            .iter()
            .any(|frame| frame.light.iter().any(|&pixel| pixel != 0))
    );
    assert_ne!(frame_hash(&middle[0]), frame_hash(&middle[1]));
    Ok(())
}

#[test]
fn movement_can_billow_the_gust_without_translating_or_rotating_it() {
    let mut gust = VIOLET_WAKE.spawn(&Options::default(), 1);
    let draw = |gust: &dyn pixfx_core::EffectState| {
        let mut frame = Frame::new(112, 64, 0, 0);
        let mut painter = Painter::new(&mut frame);
        painter.place(56.0, 32.0, false, 0.0);
        gust.draw(&mut painter, 0.25, Layer::Back);
        frame_hash(&frame)
    };
    let set = |gust: &mut dyn pixfx_core::EffectState, x, y, dx, dy| {
        gust.set_trace(Some(Arc::new(Trace {
            points: vec![
                TracePoint {
                    x,
                    y,
                    angle: 0.0,
                    age: 0.0,
                },
                TracePoint {
                    x: x - dx,
                    y: y - dy,
                    angle: 0.0,
                    age: 0.04,
                },
            ],
        })));
    };
    let stationary = draw(gust.as_ref());
    set(gust.as_mut(), 0.0, 0.0, 9.0, 0.0);
    let moving = draw(gust.as_ref());
    assert_ne!(stationary, moving);
    set(gust.as_mut(), 900.0, 100.0, 0.0, 9.0);
    assert_eq!(moving, draw(gust.as_ref()));
}
