#![cfg(feature = "feedback-outline")]

use std::sync::Arc;

use pixfx_core::{Actor, Options, render_frames_with_actor};
use pixfx_draw::color::{Rgba, WHITE};
use pixfx_draw::palette::preset;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn outline_uses_a_visible_middle_swatch_and_accepts_exact_colours() -> Result<(), String> {
    let effect = pixfx_effects::effect("feedback/outline").ok_or("outline missing")?;
    let actor = Arc::new(Actor::new(0, 0, 1, 1, vec![WHITE.pack()]).map_err(|e| e.to_string())?);
    for (palette, expected) in [
        (None, Rgba::hex24(0x1a1422)),
        (preset("ice").map(<[_]>::to_vec), Rgba::hex24(0x6cc0e6)),
        (Some(vec![WHITE]), WHITE),
    ] {
        let options = Options {
            palette,
            ..Options::default()
        };
        let rendered = render_frames_with_actor(effect, &options, 0, |_| Some(Arc::clone(&actor)));
        let frame = rendered.frames.first().ok_or("outline frame missing")?;
        let (x, y) = rendered.origin;
        let pixel = y as usize * frame.w as usize + (x - 1) as usize;
        assert_eq!(frame.color[pixel], expected.pack());
    }
    Ok(())
}
