//! The frames as pixel matrices: every pixel of the hand-made sheet comes back as empty, colour or light with the
//! exact value drawn, and the shared palette holds the sheet's colours.

mod common;

use common::{BLOCK, BLOCK_DARK, GLOW, SPEC, sheet};
use pixfx_pixel::reference::Spec;
use pixfx_pixel::reference::extract::extract;
use pixfx_pixel::reference::matrix::{Px, matrix, paletted, to_json};

#[test]
fn every_pixel_is_empty_colour_or_light_with_its_exact_value_and_a_palette_entry()
-> Result<(), String> {
    let spec = Spec::parse(SPEC)?;
    let ex = extract(&sheet(&spec)?, &spec)?;
    let m = matrix(&ex);
    assert_eq!((m.w, m.h), ex.size);
    assert_eq!(m.frames.len(), ex.cells.len());
    let f = &m.frames[1];
    let at = |x: u32, y: u32| f.px[(y * m.w + x) as usize];
    let rgb = |c: [u8; 4]| [c[0], c[1], c[2]];
    // the checkered block, near (48, 48): both its colours, exactly, as colour pixels
    let block: Vec<Px> = (40..56).map(|x| at(x, 48)).collect();
    let exact: Vec<[u8; 3]> = block
        .iter()
        .filter_map(|p| match p {
            Px::Colour { rgb, .. } => Some(*rgb),
            _ => None,
        })
        .collect();
    assert!(
        exact.contains(&rgb(BLOCK)) && exact.contains(&rgb(BLOCK_DARK)),
        "{block:?}"
    );
    assert!(
        m.colours.contains(&rgb(BLOCK)) && m.colours.contains(&rgb(BLOCK_DARK)),
        "palette {:?}",
        m.colours
    );
    // the glow is light: its colour times its alpha
    let Px::Light { rgb: light, ink } = at(65, 48) else {
        return Err(format!("not light: {:?}", at(65, 48)));
    };
    let want = rgb(GLOW).map(|v| (f32::from(v) * 128.0 / 255.0) as u8);
    assert!(
        (0..3).all(|i| light[i].abs_diff(want[i]) <= 1),
        "{light:?} vs {want:?}"
    );
    assert!(usize::from(ink) < m.lights.len());
    assert_eq!(at(5, 5), Px::Empty);
    assert!(
        m.frames[0].px.iter().all(|p| *p == Px::Empty),
        "an empty cell is empty"
    );
    // few colours: the palette holds them all, so painting from it misses nothing
    assert!(
        m.colour_err < 0.5 && m.light_err < 2.0,
        "{} {}",
        m.colour_err,
        m.light_err
    );
    let through = paletted(&m, f);
    assert_eq!(through.get(48, 48), ex.cells[1].rebuilt.get(48, 48));
    Ok(())
}

#[test]
fn the_json_has_a_text_row_per_pixel_row_and_an_exact_value_per_pixel() -> Result<(), String> {
    let spec = Spec::parse(SPEC)?;
    let ex = extract(&sheet(&spec)?, &spec)?;
    let m = matrix(&ex);
    let j = to_json(&m);
    let f = &j["frames"][1];
    let rows = f["colour"].as_array().ok_or("no colour rows")?;
    assert_eq!(rows.len() as u32, m.h);
    assert!(
        rows.iter()
            .all(|r| r.as_str().is_some_and(|s| s.chars().count() as u32 == m.w))
    );
    let letters: usize = rows
        .iter()
        .filter_map(|r| r.as_str())
        .map(|s| s.chars().filter(|&c| c != '.').count())
        .sum();
    let exact = f["exact"]["colour"]
        .as_array()
        .ok_or("no exact colours")?
        .len();
    assert_eq!(letters, exact, "one exact value per colour pixel");
    assert_eq!(letters, ex.cells[1].solid);
    assert_eq!(j["palette"]["colour"][0]["letter"], "a");
    Ok(())
}
