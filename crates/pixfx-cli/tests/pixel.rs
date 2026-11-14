// `pixfx pixel` verbs, driven through `pixfx_cli::pixel` as `main` drives them, without files where it can.
use std::path::PathBuf;

use pixfx_cli::pixel::{
    CleanArgs, Command, Verb, clean, draw, look_names, look_palette, parse_args, parse_palette,
    read,
};
use pixfx_cli::poses::{MOMENTS, state_cells, states};
use pixfx_draw::RgbaImage;
use pixfx_export::decode_png_rgba;
use pixfx_fighter::look::Look;
use pixfx_fighter::paint::SPRITE;
use pixfx_fighter::pose::Pose;
use pixfx_pixel::colors;

type Check = Result<(), String>;

fn parse(words: &str) -> Result<Command, String> {
    let args: Vec<String> = words.split_whitespace().map(String::from).collect();
    parse_args(&args)
}

fn verb(words: &str) -> Result<Verb, String> {
    parse(words).map(|c| c.verb)
}

fn fixture(name: &str) -> Result<RgbaImage, String> {
    let path = format!(
        "{}/../pixfx-pixel/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
    decode_png_rgba(&bytes)
}

fn clean_args(picture: &str) -> Result<CleanArgs, String> {
    match verb(&format!("clean {picture}"))? {
        Verb::Clean(a) => Ok(a),
        other => Err(format!("clean parsed as {other:?}")),
    }
}

#[test]
fn every_verb_parses_its_files_and_flags() -> Check {
    let c = parse("clean a.png --cut 40 --height 40 --palette thunder --out /tmp/x")?;
    assert_eq!(c.out, PathBuf::from("/tmp/x"));
    let Verb::Clean(a) = c.verb else {
        return Err(format!("{:?}", c.verb));
    };
    assert_eq!((a.cut, a.height, a.max_colors), (Some(40), Some(40), 32));
    assert!(
        a.palette
            .is_some_and(|p| p.names.contains(&"boot".to_string()))
    );

    assert!(matches!(
        verb("read s.png --region 1,2,3,4 --rects")?,
        Verb::Read {
            region: Some([1, 2, 3, 4]),
            rects: true,
            ..
        }
    ));
    assert!(matches!(verb("draw s.txt")?, Verb::Draw { .. }));
    assert!(matches!(
        verb("view s.png --max-side 256")?,
        Verb::View { max_side: 256, .. }
    ));
    assert_eq!(
        verb("critique a.png b.png --frames --intended 3,4;5,6")?,
        Verb::Critique {
            sprites: vec!["a.png".into(), "b.png".into()],
            frames: true,
            intended: vec![(3, 4), (5, 6)],
        }
    );
    let Verb::Sheet(s) = verb("sheet --cell 64x64 --grid 4x2 --scale 6 --pixel-lines")? else {
        return Err("sheet".into());
    };
    assert_eq!(
        (s.cell, s.grid, s.scale, s.pixel_lines, s.gap),
        ((64, 64), (4, 2), 6, true, 2)
    );
    assert!(matches!(verb("cut d.png sheet.json")?, Verb::Cut { .. }));
    assert!(matches!(verb("parts s.png spec.txt")?, Verb::Parts { .. }));
    Ok(())
}

#[test]
fn wrong_arguments_say_what_is_wrong() {
    for words in [
        "",
        "paint a.png",
        "clean",
        "clean a.png b.png",
        "clean a.png --height tall",
        "sheet --cell 64x64",
        "cut d.png",
        "read a.png --region 1,2,3",
        "read a.png --bogus",
    ] {
        assert!(parse(words).is_err(), "{words:?} should not parse");
    }
}

#[test]
fn a_look_palette_comes_with_a_name_per_colour() -> Check {
    let look = Look::thunder();
    let (palette, names) = (look_palette(&look), look_names(&look));
    assert_eq!(palette.len(), names.len());
    assert!(names.iter().all(|n| !n.is_empty()));
    let parsed = parse_palette("thunder")?;
    assert_eq!((parsed.colors, parsed.names), (palette, names));
    Ok(())
}

#[test]
fn clean_finds_the_hero_figure_at_its_drawn_size() -> Check {
    let a = CleanArgs {
        cut: Some(40),
        ..clean_args("hero-style.png")?
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
    let c = clean(&img, &clean_args("many.png")?)?;
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
    let text = read(&sprite, Some(&parse_palette("thunder")?), None, false)?;
    assert!(text.contains(" boot #"), "{text}");
    let back = draw(&text)?;
    assert_eq!(pixfx_pixel::diff(&sprite, &back).0.differing, 0);
    Ok(())
}

#[test]
fn a_state_sheet_has_one_painted_cell_per_state_and_moment() {
    let rows = states();
    assert_eq!(rows.len(), 16);
    let cells = state_cells(&Look::thunder(), Pose::Down, &rows);
    assert_eq!(cells.len(), rows.len() * MOMENTS.len());
    for (i, (cell, state)) in cells
        .iter()
        .zip(rows.iter().flat_map(|s| [s; MOMENTS.len()]))
        .enumerate()
    {
        assert_eq!(cell.len(), SPRITE * SPRITE * 4);
        let painted = cell
            .chunks(4)
            .filter(|p| p.get(3).is_some_and(|&a| a > 0))
            .count();
        assert!(
            painted > 50,
            "cell {i} ({}) has {painted} painted pixels",
            state.label()
        );
    }
}

#[test]
fn critique_suggests_a_clean_command_the_cli_takes() -> Check {
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
    let words = command
        .strip_prefix("pixfx pixel ")
        .ok_or(format!("not a pixfx pixel command: {command}"))?
        .replace("<sprite.png>", "s.png");
    assert!(matches!(verb(&words)?, Verb::Clean(_)), "{command}");
    Ok(())
}
