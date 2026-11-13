//! The fire effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/fire/*.js`).

use pixfx_effects::effect;

#[test]
fn candle_meta_matches_pixelfx() {
    let m = effect("fire/candle").map(|e| *e.meta());
    assert!(m.is_some(), "fire/candle is not registered");
    assert_eq!(m.map(|m| m.size), Some((56, 76)));
    assert_eq!(m.map(|m| m.origin), Some((28, 64)));
    assert_eq!(m.map(|m| m.duration), Some(4.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
}

#[test]
fn campfire_and_brazier_are_registered() {
    let size = |id| effect(id).map(|e| e.meta().size);
    assert_eq!(size("fire/campfire"), Some((128, 128)));
    assert_eq!(size("fire/brazier"), Some((112, 148)));
}

#[test]
fn huge_campfire_height_draws_in_bounded_time() -> Result<(), &'static str> {
    use std::time::{Duration, Instant};

    use pixfx_core::{Frame, Instance, Layer, OptValue, Options};

    let fx = effect("fire/campfire").ok_or("fire/campfire is not registered")?;
    let (w, h) = fx.meta().size;
    let mut opts = Options {
        x: f64::from(fx.meta().origin.0),
        y: f64::from(fx.meta().origin.1),
        ..Options::default()
    };
    opts.extra.insert("height".into(), OptValue::Num(1e9));
    let mut inst = Instance::new(fx, opts);
    inst.update(0.2);
    let mut frame = Frame::new(w, h, 0, 0);
    let start = Instant::now();
    inst.draw(&mut frame, Layer::Both, inst.placement());
    let took = start.elapsed();
    assert!(took < Duration::from_secs(1), "one draw took {took:?}");
    assert!(frame.color.iter().any(|&c| c != 0), "the fire still draws");
    Ok(())
}

#[test]
fn spirit_flame_meta_matches_pixelfx() {
    let m = effect("fire/spirit-flame").map(|e| *e.meta());
    assert!(m.is_some(), "fire/spirit-flame is not registered");
    assert_eq!(m.map(|m| m.size), Some((80, 124)));
    assert_eq!(m.map(|m| m.origin), Some((40, 94)));
    assert_eq!(m.map(|m| m.duration), Some(3.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(7));
}
