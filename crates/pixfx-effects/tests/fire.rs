//! A fire option far out of range still draws in bounded time.

use pixfx_effects::effect;

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
    inst.draw(&mut frame, Layer::Both);
    let took = start.elapsed();
    assert!(took < Duration::from_secs(1), "one draw took {took:?}");
    assert!(frame.color.iter().any(|&c| c != 0), "the fire still draws");
    Ok(())
}
