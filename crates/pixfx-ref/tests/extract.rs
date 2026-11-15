//! A sheet as the model returns it, made by hand: the template's labels in near-black on a transparent page, and in
//! one cell an opaque block, checkered in two colours so its pixels show, with a see-through glow beside it.

mod common;

use common::{BLOCK, BLOCK_DARK, GLOW, SPEC, sheet};
use pixfx_ref::extract::extract;
use pixfx_ref::{RgbaImage, Spec};

#[test]
fn opaque_pixels_become_colour_and_see_through_ones_light_with_each_row_its_own_length() -> Result<(), String> {
    let spec = Spec::parse(SPEC)?;
    let ex = extract(&sheet(&spec)?, &spec)?;
    let names: Vec<&str> = ex.cells.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["init-1", "init-2", "init-3", "hit-right-1", "hit-right-2"]);
    let diffs: Vec<&str> = ex.diffs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(diffs, ["init-1-2", "init-2-3", "hit-right-1-2"]);
    assert!(ex.found, "the checker shows the grid");
    assert_eq!(ex.size, (96, 96), "the template's sprite pixels, drawn 3 sheet pixels each: pitch {:?}", ex.pitch);
    assert!(ex.cells.iter().all(|c| c.colour.w == 96 && c.colour.h == 96), "every frame the same size");
    let c = &ex.cells[1];
    assert!(c.strength.0 > 0.5 && c.strength.1 > 0.5, "the checkered cell follows its cuts: {:?}", c.strength);
    // found again within a sprite pixel of where it was drawn
    let near = |img: &RgbaImage, x: u32, y: u32| (0..3).flat_map(|d| (0..3).map(move |e| (d, e))).map(|(d, e)| img.get(x + d - 1, y + e - 1)).collect::<Vec<_>>();
    assert!(near(&c.colour, 48, 48).contains(&Some(BLOCK)), "the block is colour, exactly as drawn");
    assert_eq!(c.colour.get(65, 48).map(|p| p[3]), Some(0), "the glow is not colour");
    let l = c.light.get(65, 48).unwrap_or([0; 4]);
    let want = GLOW.map(|v| (f32::from(v) * 128.0 / 255.0) as u8);
    assert!((0..3).all(|i| l[i].abs_diff(want[i]) <= 1), "the glow is light: its colour times its alpha, got {l:?}");
    assert!(c.solid >= 14 * 14 && c.solid <= 18 * 18, "about the 16x16 block: {}", c.solid);
    let dark = (40..56).filter(|&x| near(&c.colour, x, 48).contains(&Some(BLOCK_DARK))).count();
    assert!(dark >= 14, "the checker comes back pixel for pixel, its dark half too: {dark}");
    assert!(c.mean_err < 1.0, "colour or light adds back up to the cell over black: {}", c.mean_err);
    assert_eq!(ex.cells[0].solid, 0, "an empty cell has no colour");
    assert!(ex.diffs[0].changed > 0 && ex.diffs[1].changed > 0, "the block shows up and goes again");
    Ok(())
}

#[test]
fn a_sheet_without_its_labels_is_refused() -> Result<(), String> {
    let spec = Spec::parse(SPEC)?;
    let blank = RgbaImage::new(600, 800);
    assert!(extract(&blank, &spec).is_err_and(|e| e.contains("label lines")));
    Ok(())
}
