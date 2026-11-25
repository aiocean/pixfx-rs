//! Incinerate follows the actor's visible pixels, including an offset pose and empty frames.
#![cfg(feature = "disappear-incinerate")]

use std::sync::Arc;

use pixfx_core::{Actor, Options, render_frames_with_actor};
use pixfx_draw::color::Rgba;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn offset_sprite_keeps_its_contour_through_the_flare() -> Result<(), String> {
    let fx = pixfx_effects::effect("disappear/incinerate").ok_or("effect missing")?;
    let teal = Rgba(20, 160, 150, 255).pack();
    let x = 25;
    let y = -10;
    let pixels = (0..12).flat_map(|_| [teal, 0, teal]).collect();
    let actor = Arc::new(Actor::new(x, y, 3, 12, pixels).map_err(|e| e.to_string())?);
    let rendered = render_frames_with_actor(fx, &Options::default(), 0, |_| Some(actor.clone()));
    let at = |k: usize, x: i32, y: i32| {
        let f = &rendered.frames[k];
        let density = f.density().factor() as i32;
        let (x, y) = (x * density, y * density);
        f.color[(y + rendered.origin.1) as usize * f.w as usize + (x + rendered.origin.0) as usize]
    };

    assert_eq!(at(0, x, y), teal, "the unburned crown keeps its colour");
    assert_eq!(at(0, x + 1, y), 0, "transparent crown pixels stay clear");
    assert_ne!(at(0, x, y + 11), teal, "ignition starts at the actual feet");
    for col in [x, x + 2] {
        let lit = Rgba::unpack(at(4, col, y));
        assert!(lit == Rgba::hex24(0xffffff) || lit == Rgba::hex24(0xffe066));
        assert_ne!(at(5, col, y), 0, "the charred crown tracks the pose");
        assert_eq!(at(17, col, y), 0, "the body has crumbled");
    }
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn transparent_actor_emits_neither_ash_nor_light() -> Result<(), String> {
    let fx = pixfx_effects::effect("disappear/incinerate").ok_or("effect missing")?;
    let actor = Arc::new(Actor::new(-1, -1, 2, 2, vec![0; 4]).map_err(|e| e.to_string())?);
    let rendered = render_frames_with_actor(fx, &Options::default(), 0, |_| Some(actor.clone()));
    assert!(rendered.frames.iter().all(|frame| {
        frame
            .color
            .iter()
            .chain(&frame.light)
            .all(|&pixel| pixel == 0)
    }));
    Ok(())
}
