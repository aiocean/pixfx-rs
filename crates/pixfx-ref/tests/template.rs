use pixfx_pixel::sheet::cell_rect;
use pixfx_ref::Spec;
use pixfx_ref::template::{labels, layout, render};

const SPEC: &str = r#"{"id":"t/x","name":"X","look":"l","subject":"s",
  "rows":[{"name":"INIT","caption":"forms","text":"a","frames":4},{"name":"HIT","caption":"","text":"b","frames":2}]}"#;

#[test]
fn labels_count_each_rows_own_frames_and_leave_the_rest_blank() -> Result<(), String> {
    let s = Spec::parse(SPEC)?;
    assert_eq!(labels(&s), ["INIT 1/4 - forms", "INIT 2/4", "INIT 3/4", "INIT 4/4", "HIT 1/2", "HIT 2/2", "", ""]);
    Ok(())
}

#[test]
fn the_sheet_is_the_grid_with_the_figure_in_used_cells_and_paper_in_the_rest() -> Result<(), String> {
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
    assert!(magenta(&cell_pixels(1, 1)) > 20, "a used cell holds the figure");
    assert!(cell_pixels(2, 1).iter().all(|p| *p == [255, 255, 255, 255]), "an unused cell is plain paper, frame and all");
    Ok(())
}

#[test]
fn a_label_too_long_for_its_gap_is_refused() -> Result<(), String> {
    let mut s = Spec::parse(SPEC)?;
    s.gap = 2;
    assert!(render(&s).is_err_and(|e| e.contains("does not fit")));
    Ok(())
}
