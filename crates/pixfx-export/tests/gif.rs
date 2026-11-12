//! `write_gif` on a real effect: a scale too big for gif's 16-bit size is an error, not an overflow.

use pixfx_core::{Options, render_frames};
use pixfx_export::{ExportError, write_gif};

#[test]
fn oversized_scale_is_an_error() -> Result<(), Box<dyn std::error::Error>> {
    let effect = pixfx_effects::effect("fire/candle").ok_or("no candle")?;
    let r = render_frames(effect, &Options::default(), 0);
    let path = std::env::temp_dir().join(format!("pixfx-gif-too-big-{}.gif", std::process::id()));
    // 56 * 2000 fits a u32 but not gif's u16; 56 * u32::MAX overflows a u32
    for scale in [2000, u32::MAX] {
        let got = write_gif(&path, &r, scale);
        assert!(
            matches!(got, Err(ExportError::TooBig { w: 56, h: 76, scale: s }) if s == scale),
            "scale {scale}: {got:?}"
        );
        assert!(!path.exists(), "scale {scale} created a file");
    }
    Ok(())
}
