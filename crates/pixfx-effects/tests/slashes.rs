#![cfg(feature = "slashes")]

use std::collections::BTreeSet;
use std::sync::Arc;

use pixfx_core::{Layer, Options, RenderKind, Trace, TracePoint, render_frames};
use pixfx_draw::{Frame, Painter, frame_hash};
use pixfx_effects::effect;

#[test]
fn each_slash_fades_and_accepts_the_engine_palette() -> Result<(), String> {
    let mut swung = BTreeSet::new();
    for id in ["slashes/slash-1", "slashes/slash-2", "slashes/slash-3"] {
        let fx = effect(id).ok_or(id)?;
        assert_eq!(fx.render_kind().as_str(), RenderKind::Paint.as_str());
        let defaults = render_frames(fx, &Options::default(), 0);
        let custom = render_frames(
            fx,
            &Options {
                palette: Some(vec![pixfx_draw::color::WHITE]),
                ..Options::default()
            },
            0,
        );
        assert!(!fx.meta().looping);
        assert!(
            defaults
                .frames
                .iter()
                .any(|f| f.color.iter().any(|&p| p != 0))
        );
        assert!(
            defaults
                .frames
                .last()
                .ok_or(id)?
                .color
                .iter()
                .all(|&p| p == 0)
        );
        assert_ne!(defaults, custom);
        swung.insert(defaults.frames.iter().map(frame_hash).collect::<Vec<_>>());
    }
    assert_eq!(swung.len(), 3);
    Ok(())
}

#[test]
fn blade_trail_paints_the_movers_positions() -> Result<(), String> {
    let id = "slashes/slash-3";
    let fx = effect(id).ok_or(id)?;
    let opts = Options {
        x: 40.0,
        y: 40.0,
        ..Options::default()
    };
    assert!(fx.traces(&opts).is_some());
    let mut state = fx.spawn(&opts, 1);
    let mut standalone = Frame::new(80, 80, 0, 0);
    let mut painter = Painter::new(&mut standalone);
    painter.place(40.0, 40.0, false, 0.0);
    state.draw(&mut painter, 0.17, Layer::Front);
    state.set_trace(Some(Arc::new(Trace::default())));
    let mut stationary = Frame::new(80, 80, 0, 0);
    let mut painter = Painter::new(&mut stationary);
    painter.place(40.0, 40.0, false, 0.0);
    state.draw(&mut painter, 0.17, Layer::Front);
    assert_eq!(standalone, stationary);
    state.set_trace(Some(Arc::new(Trace {
        points: vec![
            TracePoint {
                x: 13.0,
                y: 15.0,
                angle: 0.0,
                age: 0.0,
            },
            TracePoint {
                x: -13.0,
                y: 15.0,
                angle: 0.0,
                age: 0.10,
            },
        ],
    })));
    let mut trail = Frame::new(80, 80, 0, 0);
    let mut painter = Painter::new(&mut trail);
    painter.place(40.0, 40.0, false, 0.0);
    state.draw(&mut painter, 0.17, Layer::Front);
    assert_ne!(standalone, trail);
    assert!(
        trail.color[55 * 80..56 * 80]
            .iter()
            .any(|&pixel| pixel != 0)
    );
    Ok(())
}
