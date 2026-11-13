//! Palette presets as a caller uses them: a word for a ramp every effect can take.

use pixfx_core::Options;
use pixfx_draw::Rgba;
use pixfx_draw::palette::{PRESETS, parse, preset};

fn luma(c: Rgba) -> f64 {
    0.2126 * f64::from(c.0) + 0.7152 * f64::from(c.1) + 0.0722 * f64::from(c.2)
}

#[test]
fn every_preset_runs_dark_to_light_as_effect_palettes_do() {
    for (name, ramp) in PRESETS {
        assert!(
            ramp.windows(2).all(|w| luma(w[0]) < luma(w[1])),
            "{name} is not dark to light"
        );
    }
}

#[test]
fn a_preset_resamples_to_any_effects_palette_length() {
    let opts = Options {
        palette: parse("ice").ok(),
        ..Options::default()
    };
    let default5 = [Rgba::hex24(0x111111); 5];
    let got = opts.palette_or(&default5);
    assert_eq!(got.len(), 5);
    assert_eq!(got.first(), preset("ice").and_then(|r| r.first()));
    assert_eq!(got.last(), preset("ice").and_then(|r| r.last()));
}

#[test]
fn an_unknown_word_names_every_preset() {
    let err = parse("lava")
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(
        err.contains("lava") && err.contains("ice") && err.contains("gold"),
        "{err}"
    );
}
