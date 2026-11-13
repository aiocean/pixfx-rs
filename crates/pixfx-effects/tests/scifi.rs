//! The scifi effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/scifi/*.js`).

use pixfx_effects::effect;

#[test]
fn jetpack_meta_matches_pixelfx() {
    let m = effect("scifi/jetpack").map(|e| *e.meta());
    assert!(m.is_some(), "scifi/jetpack is not registered");
    assert_eq!(m.map(|m| m.size), Some((96, 116)));
    assert_eq!(m.map(|m| m.origin), Some((48, 30)));
    assert_eq!(m.map(|m| m.duration), Some(2.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(30));
}

#[test]
fn long_play_stays_bounded() -> Result<(), &'static str> {
    use std::time::{Duration, Instant};

    use pixfx_core::{Frame, Instance, Layer, Options};

    let fx = effect("scifi/jetpack").ok_or("scifi/jetpack is not registered")?;
    let (w, h) = fx.meta().size;
    let opts = Options {
        x: f64::from(fx.meta().origin.0),
        y: f64::from(fx.meta().origin.1),
        ..Options::default()
    };
    let mut inst = Instance::new(fx, opts);
    // ten minutes of play at 30 fps
    for _ in 0..18_000 {
        inst.update(1.0 / 30.0);
    }
    let mut frame = Frame::new(w, h, 0, 0);
    let start = Instant::now();
    inst.draw(&mut frame, Layer::Both, inst.placement());
    let took = start.elapsed();
    eprintln!("long_play_stays_bounded: one draw after 18000 updates took {took:?}");
    assert!(took < Duration::from_millis(50), "one draw took {took:?}");
    assert!(
        frame.color.iter().any(|&c| c != 0),
        "the jetpack still draws"
    );
    Ok(())
}
