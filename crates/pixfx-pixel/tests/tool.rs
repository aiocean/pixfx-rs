// The steps of `pixfx_pixel::tool`, driven as their example scripts drive them, without files where they can.
use pixfx_draw::RgbaImage;
use pixfx_export::decode_png_rgba;
use pixfx_fighter::look::Look;
use pixfx_pixel::colors;
use pixfx_pixel::tool::{CleanOptions, Palette, clean, draw, read};

type Check = Result<(), String>;

fn fixture(name: &str) -> Result<RgbaImage, String> {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
    decode_png_rgba(&bytes)
}

fn thunder() -> Palette {
    Palette::named(Look::thunder().colors())
}

#[test]
fn a_look_palette_comes_with_a_name_per_colour() {
    let palette = thunder();
    assert_eq!(palette.colors.len(), palette.names.len());
    assert!(palette.names.iter().all(|n| !n.is_empty()));
    assert!(palette.names.contains(&"boot".to_string()));
    let mut seen = palette.colors.clone();
    seen.dedup();
    assert_eq!(seen.len(), palette.colors.len(), "each colour once");
}

#[test]
fn a_hex_palette_is_unnamed_and_refuses_a_word_that_is_no_colour() -> Check {
    let p = Palette::parse_hex("#ff0000, 00ff00")?;
    assert_eq!((p.colors.len(), p.names), (2, vec![String::new(); 2]));
    assert!(Palette::parse_hex("red").is_err());
    Ok(())
}

#[test]
fn clean_finds_the_hero_figure_at_its_drawn_size() -> Check {
    let a = CleanOptions {
        cut: Some(40),
        ..CleanOptions::default()
    };
    let c = clean(&fixture("hero-style.png")?, &a)?;
    let (w, h) = (c.sprite.w, c.sprite.h);
    assert!(w.abs_diff(34) <= 1 && h.abs_diff(60) <= 1, "{w}x{h}");
    assert!((c.pitch - 10.22).abs() < 0.5, "pitch {}", c.pitch);
    Ok(())
}

#[test]
fn clean_without_a_palette_merges_thousands_of_colours_to_32() -> Check {
    // 64x64 cells of 4 picture pixels each, every cell its own colour: 4096 colours.
    let mut img = RgbaImage::new(256, 256);
    for y in 0..256u32 {
        for x in 0..256u32 {
            let (cx, cy) = (x / 4, y / 4);
            img.set(
                x,
                y,
                [(cx * 4) as u8, (cy * 4) as u8, ((cx + cy) * 2) as u8, 255],
            );
        }
    }
    assert!(colors(&img).len() > 4000);
    let c = clean(&img, &CleanOptions::default())?;
    assert_eq!(
        (c.sprite.w, c.sprite.h),
        (64, 64),
        "the grid of 4-pixel cells"
    );
    assert!(
        colors(&c.sprite).len() > 16,
        "merging keeps a palette, not one colour"
    );
    assert!(
        colors(&c.sprite).len() <= 32,
        "{} colours",
        colors(&c.sprite).len()
    );
    Ok(())
}

#[test]
fn read_then_draw_gives_the_sprite_back() -> Check {
    let sprite = fixture("thunder-idle.png")?;
    let text = read(&sprite, Some(&thunder()), None, false)?;
    assert!(text.contains(" boot #"), "{text}");
    let back = draw(&text)?;
    assert_eq!(pixfx_pixel::diff(&sprite, &back).0.differing, 0);
    Ok(())
}

#[test]
fn critique_suggests_a_clean_script_that_takes_its_flag() -> Check {
    // 64 cells in 64 colours: far over the colour budget.
    let mut img = RgbaImage::new(8, 8);
    for i in 0..64u32 {
        img.set(
            i % 8,
            i / 8,
            [(i * 4) as u8, 255 - (i * 4) as u8, (i % 7 * 30) as u8, 255],
        );
    }
    let report = pixfx_pixel::critique::critique(&img, &[]);
    let issue = report
        .issues
        .iter()
        .find(|i| i.check == "colour_budget")
        .ok_or("no colour_budget issue")?;
    let command = issue.fix.split('`').nth(1).ok_or("no command in the fix")?;
    let rest = command
        .strip_prefix("cargo run -p pixfx-pixel --example ")
        .ok_or(format!("not a pixfx-pixel script: {command}"))?;
    let (script, args) = rest
        .split_once(" -- ")
        .ok_or(format!("no arguments: {command}"))?;
    let path = format!("{}/examples/{script}.rs", env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    for flag in args.split_whitespace().filter(|w| w.starts_with("--")) {
        assert!(
            source.contains(&format!("\"{flag}\"")),
            "{script} takes no {flag}"
        );
    }
    Ok(())
}
