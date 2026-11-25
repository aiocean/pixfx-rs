#![cfg(feature = "feedback-volt-orbit")]

use std::sync::Arc;

use pixfx_core::{Actor, Options, render_frames_with_actor};
use pixfx_draw::color::WHITE;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn volt_orbit_tracks_opaque_pose_bounds() -> Result<(), String> {
    let effect = pixfx_effects::effect("feedback/volt-orbit").ok_or("volt orbit missing")?;
    assert!(effect.wraps_actor());

    let render_pose = |x, y, w, h, pixels| {
        let actor = Arc::new(Actor::new(x, y, w, h, pixels).map_err(|e| e.to_string())?);
        Ok::<_, String>(render_frames_with_actor(
            effect,
            &Options::default(),
            0,
            |_| Some(Arc::clone(&actor)),
        ))
    };
    let pixel = WHITE.pack();
    let tight = render_pose(0, -12, 1, 1, vec![pixel])?;
    let padded = render_pose(-1, -13, 3, 3, vec![0, 0, 0, 0, pixel, 0, 0, 0, 0])?;
    let moved = render_pose(12, -12, 1, 1, vec![pixel])?;
    let empty = render_pose(0, -12, 1, 1, vec![0])?;

    assert_eq!(tight, padded);
    assert_ne!(tight, moved);
    assert!(tight.frames.iter().all(|f| f.color.iter().any(|&p| p != 0)));
    assert!(
        empty
            .frames
            .iter()
            .all(|f| f.color.iter().all(|&p| p == 0) && f.light.iter().all(|&p| p == 0))
    );
    Ok(())
}
