use pixfx_core::{Options, render_frames};
use pixfx_export::{write_gif, write_webp_from_gif};

#[test]
#[ignore = "requires gif2webp on PATH"]
fn gif_conversion_preserves_animation() -> Result<(), Box<dyn std::error::Error>> {
    let effect = pixfx_effects::effect("impacts/parry").ok_or("missing parry effect")?;
    let rendered = render_frames(effect, &Options::default(), 0);
    let base = std::env::temp_dir().join(format!("pixfx-webp-{}", std::process::id()));
    let gif = base.with_extension("gif");
    let webp = base.with_extension("webp");
    let result = (|| {
        write_gif(&gif, &rendered, 1)?;
        write_webp_from_gif(&gif, &webp)?;
        std::fs::read(&webp).map_err(pixfx_export::ExportError::from)
    })();
    let _ = std::fs::remove_file(gif);
    let _ = std::fs::remove_file(webp);

    let bytes = result?;
    assert_eq!(bytes.get(8..12), Some(b"WEBP".as_slice()));
    let animation = bytes
        .windows(4)
        .position(|word| word == b"ANIM")
        .ok_or("missing animation header")?;
    assert_eq!(
        bytes.get(animation + 12..animation + 14),
        Some([0, 0].as_slice())
    );
    assert!(bytes.windows(4).filter(|word| *word == b"ANMF").count() > 1);
    Ok(())
}
