use pixfx_pixel::reference::Spec;
use pixfx_pixel::reference::template::{labels, layout, render};
use pixfx_pixel::sheet::cell_rect;

const SPEC: &str = r#"{"id":"t/x","name":"X","look":"l","subject":"s",
  "rows":[{"name":"INIT","caption":"forms","text":"a","frames":4},{"name":"HIT","caption":"","text":"b","frames":2}]}"#;

#[test]
fn labels_count_each_rows_own_frames_and_leave_the_rest_blank() -> Result<(), String> {
    let s = Spec::parse(SPEC)?;
    assert_eq!(
        labels(&s),
        [
            "INIT 1/4 - forms",
            "INIT 2/4",
            "INIT 3/4",
            "INIT 4/4",
            "HIT 1/2",
            "HIT 2/2",
            "",
            ""
        ]
    );
    Ok(())
}

#[test]
fn the_sheet_is_the_grid_with_the_figure_in_used_cells_and_paper_in_the_rest() -> Result<(), String>
{
    let s = Spec::parse(SPEC)?;
    let img = render(&s)?;
    let l = layout(&s);
    assert_eq!((l.cols, l.rows), (4, 2));
    assert_eq!((img.w, img.h), pixfx_pixel::sheet::size(&l));
    let cell_pixels = |col, row| {
        let [x, y, w, h] = cell_rect(&l, col, row);
        let mut v = Vec::new();
        for py in y.saturating_sub(1)..y + h + 1 {
            for px in x.saturating_sub(1)..x + w + 1 {
                v.push(img.get(px, py).unwrap_or([0; 4]));
            }
        }
        v
    };
    let magenta = |v: &[[u8; 4]]| v.iter().filter(|p| **p == [255, 0, 255, 255]).count();
    assert!(
        magenta(&cell_pixels(1, 1)) > 20,
        "a used cell holds the figure"
    );
    assert!(
        cell_pixels(2, 1).iter().all(|p| *p == [255, 255, 255, 255]),
        "an unused cell is plain paper, frame and all"
    );
    Ok(())
}

#[test]
fn a_label_too_long_for_its_gap_is_refused() -> Result<(), String> {
    let mut s = Spec::parse(SPEC)?;
    s.gap = 2;
    assert!(render(&s).is_err_and(|e| e.contains("does not fit")));
    Ok(())
}

const PARTS: &str = r#"{"id":"t/x","name":"X","look":"l","subject":"s","grid":true,"cell":32,"scale":6,"gap":10,
  "parts":[{"name":"MAIN","text":"m"},{"name":"PETAL","text":"p"},{"name":"FX","text":"f"}],
  "rows":[{"name":"INIT","caption":"forms","text":"a","frames":2}]}"#;

#[test]
fn the_parts_row_comes_first_with_a_label_per_part_and_no_figure() -> Result<(), String> {
    let s = Spec::parse(PARTS)?;
    assert_eq!(
        labels(&s),
        [
            "PARTS 1/3 - MAIN",
            "PARTS 2/3 - PETAL",
            "PARTS 3/3 - FX",
            "INIT 1/2 - forms",
            "INIT 2/2",
            ""
        ]
    );
    let img = render(&s)?;
    let l = layout(&s);
    assert_eq!((l.cols, l.rows), (3, 2));
    let count = |col, row, want: [u8; 4]| {
        let [x, y, w, h] = cell_rect(&l, col, row);
        (y..y + h)
            .flat_map(|py| (x..x + w).map(move |px| (px, py)))
            .filter(|&(px, py)| img.get(px, py) == Some(want))
            .count()
    };
    let magenta = [255, 0, 255, 255];
    assert_eq!(count(0, 0, magenta), 0, "a part's cell holds no figure");
    assert!(count(0, 1, magenta) > 10, "a moment's cell does");
    Ok(())
}

#[test]
fn with_a_grid_every_cell_shows_a_line_round_each_pixel_and_a_darker_one_every_eight()
-> Result<(), String> {
    let s = Spec::parse(PARTS)?;
    let img = render(&s)?;
    let l = layout(&s);
    let [x, y, _, _] = cell_rect(&l, 1, 0);
    assert_eq!(
        img.get(x + 6, y + 3),
        Some(pixfx_pixel::reference::template::MINOR),
        "a thin line after the first pixel"
    );
    assert_eq!(
        img.get(x + 48, y + 3),
        Some(pixfx_pixel::reference::template::DARK),
        "a darker one after the eighth"
    );
    assert_eq!(
        img.get(x + 3, y + 3),
        Some([255, 255, 255, 255]),
        "paper inside a pixel"
    );
    let plain = Spec::parse(SPEC)?;
    let pimg = render(&plain)?;
    let [px, py, _, _] = cell_rect(&layout(&plain), 0, 0);
    assert_eq!(
        pimg.get(px + 3, py + 1),
        Some([255, 255, 255, 255]),
        "no grid without `grid`"
    );
    Ok(())
}

#[test]
fn a_part_with_forms_gets_a_row_of_its_own_with_no_figure() -> Result<(), String> {
    let s = Spec::parse(&PARTS.replace(
        r#"{"name":"PETAL","text":"p"}"#,
        r#"{"name":"PETAL","text":"p","forms":["up","curled"]}"#,
    ))?;
    assert_eq!(
        labels(&s),
        [
            "PARTS 1/3 - MAIN",
            "PARTS 2/3 - PETAL",
            "PARTS 3/3 - FX",
            "PETAL 1/2",
            "PETAL 2/2",
            "",
            "INIT 1/2 - forms",
            "INIT 2/2",
            ""
        ]
    );
    let img = render(&s)?;
    let l = layout(&s);
    let [x, y, w, h] = cell_rect(&l, 0, 1);
    let magenta = (y..y + h)
        .flat_map(|py| (x..x + w).map(move |px| (px, py)))
        .filter(|&(px, py)| img.get(px, py) == Some([255, 0, 255, 255]))
        .count();
    assert_eq!(magenta, 0, "a form's cell holds no figure");
    Ok(())
}
