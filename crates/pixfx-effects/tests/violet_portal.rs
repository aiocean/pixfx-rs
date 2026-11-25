#![cfg(feature = "feedback-violet-portal")]

use pixfx_core::{Options, render_frames};
use pixfx_draw::color::Rgba;
use pixfx_effects::feedback::VIOLET_PORTAL;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn palette_tints_the_rim_sparks_and_light_without_filling_the_aperture() {
    let original = render_frames(&VIOLET_PORTAL, &Options::default(), 0);
    let red = Rgba::hex24(0xff0000);
    let tinted = render_frames(
        &VIOLET_PORTAL,
        &Options {
            palette: Some(vec![red]),
            ..Options::default()
        },
        0,
    );

    for (original, tinted) in original.frames.iter().zip(&tinted.frames) {
        assert!(tinted.light.iter().all(|pixel| pixel & 0x00ff_ff00 == 0));
        if original.light.iter().any(|&pixel| pixel != 0) {
            assert_ne!(original.light, tinted.light);
            assert!(tinted.light.iter().any(|&pixel| pixel != 0));
        }
        for (&before, &after) in original.color.iter().zip(&tinted.color) {
            assert_eq!(before >> 24, after >> 24);
            if after >> 24 != 0 {
                assert_eq!(after & 0x00ff_ffff, red.pack() & 0x00ff_ffff);
            }
        }
    }
}
