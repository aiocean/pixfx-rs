//! A sheet as the model returns it, made by hand: the template's labels in near-black on a transparent page, and in
//! one cell an opaque block, checkered in two colours so its pixels show, with a see-through glow beside it.

mod common;

use common::{BLOCK, BLOCK_DARK, GLOW, SPEC, sheet};
use pixfx_ref::extract::extract;
use pixfx_ref::{RgbaImage, Spec};

#[test]
fn opaque_pixels_become_colour_and_see_through_ones_light_with_each_row_its_own_length()
-> Result<(), String> {
    let spec = Spec::parse(SPEC)?;
    let ex = extract(&sheet(&spec)?, &spec)?;
    let names: Vec<&str> = ex.cells.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["init-1", "init-2", "init-3", "hit-right-1", "hit-right-2"]
    );
    let diffs: Vec<&str> = ex.diffs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(diffs, ["init-1-2", "init-2-3", "hit-right-1-2"]);
    assert!(ex.found, "the checker shows the grid");
    assert_eq!(
        ex.size,
        (96, 96),
        "the template's sprite pixels, drawn 3 sheet pixels each: pitch {:?}",
        ex.pitch
    );
    assert!(
        ex.cells
            .iter()
            .all(|c| c.colour.w == 96 && c.colour.h == 96),
        "every frame the same size"
    );
    let c = &ex.cells[1];
    assert!(
        c.strength.0 > 0.5 && c.strength.1 > 0.5,
        "the checkered cell follows its cuts: {:?}",
        c.strength
    );
    // found again within a sprite pixel of where it was drawn
    let near = |img: &RgbaImage, x: u32, y: u32| {
        (0..3)
            .flat_map(|d| (0..3).map(move |e| (d, e)))
            .map(|(d, e)| img.get(x + d - 1, y + e - 1))
            .collect::<Vec<_>>()
    };
    assert!(
        near(&c.colour, 48, 48).contains(&Some(BLOCK)),
        "the block is colour, exactly as drawn"
    );
    assert_eq!(
        c.colour.get(65, 48).map(|p| p[3]),
        Some(0),
        "the glow is not colour"
    );
    let l = c.light.get(65, 48).unwrap_or([0; 4]);
    let want = GLOW.map(|v| (f32::from(v) * 128.0 / 255.0) as u8);
    assert!(
        (0..3).all(|i| l[i].abs_diff(want[i]) <= 1),
        "the glow is light: its colour times its alpha, got {l:?}"
    );
    assert!(
        c.solid >= 14 * 14 && c.solid <= 18 * 18,
        "about the 16x16 block: {}",
        c.solid
    );
    let dark = (40..56)
        .filter(|&x| near(&c.colour, x, 48).contains(&Some(BLOCK_DARK)))
        .count();
    assert!(
        dark >= 14,
        "the checker comes back pixel for pixel, its dark half too: {dark}"
    );
    assert!(
        c.mean_err < 1.0,
        "colour or light adds back up to the cell over black: {}",
        c.mean_err
    );
    assert_eq!(ex.cells[0].solid, 0, "an empty cell has no colour");
    assert!(
        ex.diffs[0].changed > 0 && ex.diffs[1].changed > 0,
        "the block shows up and goes again"
    );
    Ok(())
}

#[test]
fn a_sheet_without_its_labels_is_refused() -> Result<(), String> {
    let spec = Spec::parse(SPEC)?;
    let blank = RgbaImage::new(600, 800);
    assert!(extract(&blank, &spec).is_err_and(|e| e.contains("label lines")));
    Ok(())
}

const GRID: &str = r#"{"id":"t/x","name":"X","look":"l","subject":"s","grid":true,"cell":32,"scale":6,"gap":10,
  "parts":[{"name":"MAIN","text":"m"},{"name":"PETAL","text":"p"},{"name":"FX","text":"f"}],
  "rows":[{"name":"INIT","caption":"forms","text":"a","frames":3},{"name":"HIT","caption":"","text":"b","frames":2}]}"#;

/// The grid sheet as a model returns it: labels boxed in near-black, every cell framed in near-black, a block on the
/// grid in one moment cell and one part cell.
fn boxed_grid_sheet(spec: &Spec) -> Result<RgbaImage, String> {
    use pixfx_pixel::sheet::cell_rect;
    let l = pixfx_ref::template::layout(spec);
    let (w, h) = pixfx_pixel::sheet::size(&l);
    let mut img = RgbaImage::new(w, h);
    let names = pixfx_ref::template::labels(spec);
    let texts: Vec<&str> = names.iter().map(String::as_str).collect();
    pixfx_pose::captions(&mut img, &l, &texts)?;
    let ink = [20, 20, 20, 255];
    let s = l.scale;
    for row in 0..l.rows {
        for col in 0..l.cols {
            let [x, y, cw, ch] = cell_rect(&l, col, row);
            let band = l.gap * s;
            // the label's box, in the gap above the cell, and the cell's frame
            for (x0, y0, x1, y1) in [
                (x, y - band + 4, x + cw - 1, y - 4),
                (x - 2, y - 2, x + cw + 1, y + ch + 1),
            ] {
                for px in x0..=x1 {
                    img.set(px, y0, ink);
                    img.set(px, y1, ink);
                }
                for py in y0..=y1 {
                    img.set(x0, py, ink);
                    img.set(x1, py, ink);
                }
            }
        }
    }
    // a 6 x 6 block at art pixels (10..16, 20..26) of part PETAL (1, 0) and of frame INIT 2 (1, 1)
    for (col, row) in [(1, 0), (1, 1)] {
        let [x0, y0, _, _] = cell_rect(&l, col, row);
        for ay in 20..26 {
            for ax in 10..16 {
                for d in 0..s * s {
                    img.set(x0 + ax * s + d % s, y0 + ay * s + d / s, BLOCK);
                }
            }
        }
    }
    Ok(img)
}

#[test]
fn a_grid_sheet_comes_back_on_its_grid_with_the_parts_named_and_left_out_of_the_frames()
-> Result<(), String> {
    let spec = Spec::parse(GRID)?;
    let ex = extract(&boxed_grid_sheet(&spec)?, &spec)?;
    let names: Vec<&str> = ex.cells.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "parts-main",
            "parts-petal",
            "parts-fx",
            "init-1",
            "init-2",
            "init-3",
            "hit-1",
            "hit-2"
        ],
        "boxes and frames are not taken for labels"
    );
    assert_eq!(ex.size, (32, 32), "the template's grid");
    let diffs: Vec<&str> = ex.diffs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        diffs,
        ["init-1-2", "init-2-3", "hit-1-2"],
        "no diffs between parts"
    );
    for c in [&ex.cells[1], &ex.cells[4]] {
        assert_eq!(
            c.colour.get(12, 22),
            Some(BLOCK),
            "{}: the block where it was drawn",
            c.name
        );
        assert_eq!(
            c.colour.get(8, 22).map(|p| p[3]),
            Some(0),
            "{}: nothing left of it",
            c.name
        );
    }
    assert_eq!(ex.cells[1].shift, (0, 0), "a part is never moved");
    Ok(())
}

/// A parts sheet the model re-laid: cells framed in cyan where it wanted them (two wide cells over three narrow
/// ones, none square, none where the template put them), its own pixels 5 sheet pixels wide, a checkered block in
/// every cell and a green pivot square on the second form.
fn re_laid_parts_sheet() -> RgbaImage {
    const P: u32 = 5;
    let cyan = [0, 230, 240, 255];
    let mut img = RgbaImage::new(700, 780);
    let rows: [(u32, u32, &[u32]); 2] = [(40, 400, &[20, 340]), (440, 740, &[20, 240, 460])];
    for (k, &(y0, y1, xs)) in rows.iter().enumerate() {
        let cw = if k == 0 { 300 } else { 200 };
        for &x0 in xs {
            let x1 = x0 + cw;
            for t in 1..=2 {
                for y in y0 - 2..y1 + 2 {
                    img.set(x0 - t, y, cyan);
                    img.set(x1 + t - 1, y, cyan);
                }
                for x in x0 - 2..x1 + 2 {
                    img.set(x, y0 - t, cyan);
                    img.set(x, y1 + t - 1, cyan);
                }
            }
            // model pixels (10..30, 20..50) checkered
            for ay in 20..50 {
                for ax in 10..30 {
                    let c = if (ax + ay) % 2 == 0 {
                        BLOCK
                    } else {
                        BLOCK_DARK
                    };
                    for d in 0..P * P {
                        img.set(x0 + ax * P + d % P, y0 + ay * P + d / P, c);
                    }
                }
            }
            // one mark on the second form, two apart on the third
            let marks: &[(u32, u32)] = match (k, x0) {
                (1, 240) => &[(20, 40)],
                (1, 460) => &[(8, 28), (12, 38)],
                _ => &[],
            };
            for &(mx, my) in marks {
                for d in 0..P * P {
                    img.set(x0 + mx * P + d % P, y0 + my * P + d / P, [0, 255, 0, 255]);
                }
            }
        }
    }
    img
}

#[test]
fn a_re_laid_sheet_is_cut_on_the_frames_it_kept_at_its_own_pixel_size_and_its_pivots_read()
-> Result<(), String> {
    let spec = Spec::parse(
        r#"{"id":"t/p","name":"P","look":"l","subject":"s","grid":true,"cell":64,"scale":6,"gap":10,
        "parts":[{"name":"MAIN","text":"m"},{"name":"PETAL","text":"p","pivot":"its base","forms":["a","b","c"]}],
        "rows":[{"name":"INIT","caption":"","text":"a","frames":2}]}"#,
    )?
    .parts_sheet();
    let ex = extract(&re_laid_parts_sheet(), &spec)?;
    let names: Vec<&str> = ex.cells.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "parts-main",
            "parts-petal",
            "parts-petal-1",
            "parts-petal-2",
            "parts-petal-3"
        ]
    );
    assert_eq!(
        ex.cells[0].rect,
        [20, 40, 320, 400],
        "the cell inside its frame"
    );
    assert!(
        (ex.pitch.0 - 5.0).abs() < 0.2,
        "its own pixel size: {:?}",
        ex.pitch
    );
    assert_eq!(ex.size, (60, 72), "the largest cell, in its pixels");
    // the narrow cells (40 x 60) sit in the middle of the 60 x 72 frame, counted from the bottom: +10, +12
    let form = &ex.cells[3];
    assert_eq!(form.pivots, [(30.5, 52.5)], "the pivot's middle");
    let filled = form.colour.get(30, 52);
    assert!(
        filled == Some(BLOCK) || filled == Some(BLOCK_DARK),
        "the mark is painted over with the part: {filled:?}"
    );
    assert!(
        ex.cells[2].pivots.is_empty(),
        "a form with no mark has no pivot"
    );
    assert_eq!(
        ex.cells[4].pivots,
        [(18.5, 40.5), (22.5, 50.5)],
        "a cell of two pieces has a pivot each"
    );
    assert!(ex.diffs.is_empty(), "parts are not frames");
    Ok(())
}
