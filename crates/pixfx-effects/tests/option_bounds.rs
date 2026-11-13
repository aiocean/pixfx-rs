//! A number option set absurdly high must not hang an effect: each option that sizes a loop is held to a
//! documented maximum (`NumOpt::within`). Before the clamps these four never returned.

use pixfx_core::{Frame, Instance, Layer, OptValue, Options};
use pixfx_effects::effect;
use std::time::{Duration, Instant};

/// Effect id and the number option that sizes its loops.
const CLAMPED: [(&str, &str); 4] = [
    ("water/water-jet", "length"),
    ("nature/poison-cloud", "spread"),
    ("projectiles/laser-bolt", "length"),
    ("explosions/small-explosion", "size"),
];

#[test]
fn huge_number_options_still_draw_quickly() -> Result<(), String> {
    for (id, key) in CLAMPED {
        let fx = effect(id).ok_or(format!("{id} is not registered"))?;
        for huge in [1e9, f64::INFINITY] {
            let mut opts = Options::default();
            opts.extra.insert(key.to_string(), OptValue::Num(huge));
            let (w, h) = fx.meta().size;
            let started = Instant::now();
            let mut inst = Instance::new(fx, opts);
            inst.update(fx.meta().duration / 2.0);
            let mut frame = Frame::new(w, h, 0, 0);
            inst.draw(&mut frame, Layer::Both, inst.placement());
            // debug builds are several times slower, so the bound only applies to release builds: a hang
            // fails there by never returning
            if !cfg!(debug_assertions) {
                let took = started.elapsed();
                if took > Duration::from_secs(2) {
                    return Err(format!("{id} {key}={huge} took {took:?}"));
                }
            }
        }
    }
    Ok(())
}
