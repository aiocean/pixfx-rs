// A picture into a pixel grid an agent reads: cut it out, match it to a palette, shrink it, merge it into rects,
// show it as text and zoomed, and diff what comes out against what went in.
use pixfx_pixel::{Map, Rgba, RgbaImage, colors, cut, diff, nearest, rects, shrink, text, zoom};

const BACK: [u8; 4] = [42, 36, 56, 255];
const RED: [u8; 4] = [220, 30, 30, 255];
const BLUE: [u8; 4] = [30, 30, 220, 255];

/// A `w` x `h` image of the background with `rect` (x, y, w, h) filled in `p`.
fn picture(w: u32, h: u32, fills: &[([u32; 4], [u8; 4])]) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let p = fills
                .iter()
                .rev()
                .find(|([rx, ry, rw, rh], _)| x >= *rx && x < rx + rw && y >= *ry && y < ry + rh)
                .map_or(BACK, |(_, p)| *p);
            img.set(x, y, p);
        }
    }
    img
}

#[test]
fn cut_clears_the_background_and_crops_to_the_drawing() {
    let img = picture(40, 30, &[([10, 5, 8, 12], RED)]);
    let out = cut(&img, 20);
    assert_eq!((out.w, out.h), (8, 12));
    assert_eq!(out.get(0, 0), Some(RED));
    // a near-background speck (a model's noise) goes with the background
    let noisy = picture(
        40,
        30,
        &[([1, 1, 1, 1], [46, 38, 60, 255]), ([10, 5, 8, 12], RED)],
    );
    assert_eq!((cut(&noisy, 20).w, cut(&noisy, 20).h), (8, 12));
    // a guide line along the edge (under a tenth of the drawing) and a far speck go; a blob near its size stays
    let lined = picture(
        60,
        40,
        &[
            ([0, 38, 30, 1], BLUE),
            ([50, 2, 1, 1], BLUE),
            ([10, 5, 20, 20], RED),
        ],
    );
    assert_eq!((cut(&lined, 20).w, cut(&lined, 20).h), (20, 20));
    let two = picture(60, 40, &[([10, 5, 20, 20], RED), ([35, 10, 8, 8], BLUE)]);
    assert_eq!((cut(&two, 20).w, cut(&two, 20).h), (33, 20));
}

#[test]
fn shrink_keeps_what_most_of_each_square_holds_so_edges_stay_crisp() {
    // 8 x 4 drawn at 4 source pixels a pixel: red on the left, blue on the right, one stray blue pixel in the red
    let img = picture(
        8,
        4,
        &[
            ([0, 0, 4, 4], RED),
            ([4, 0, 4, 4], BLUE),
            ([1, 1, 1, 1], BLUE),
        ],
    );
    let palette = [Rgba(220, 30, 30, 255), Rgba(30, 30, 220, 255)];
    let map = shrink(&nearest(&img, &palette), img.w, 4.0);
    assert_eq!((map.w, map.h), (2, 1));
    assert_eq!(map.cells, vec![Some(0), Some(1)]);
}

#[test]
fn rects_cover_every_painted_cell_exactly_once() {
    let cells = vec![
        Some(0),
        Some(0),
        None,
        Some(1),
        Some(0),
        Some(0),
        Some(1),
        Some(1),
        None,
        Some(2),
        Some(1),
        Some(1),
    ];
    let map = Map {
        w: 4,
        h: 3,
        cells: cells.clone(),
    };
    let mut rebuilt = vec![None; cells.len()];
    let runs = rects(&map);
    for r in &runs {
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                let i = (y * 4 + x) as usize;
                assert_eq!(rebuilt[i], None, "cell {x},{y} covered twice");
                rebuilt[i] = Some(r.value);
            }
        }
    }
    assert_eq!(rebuilt, cells);
    // the 2 x 2 block of 0s is one rect
    assert!(
        runs.iter()
            .any(|r| (r.x, r.y, r.w, r.h, r.value) == (0, 0, 2, 2, 0))
    );
}

#[test]
fn diff_counts_the_pixels_that_differ_and_marks_them() {
    let a = picture(10, 10, &[([2, 2, 3, 3], RED)]);
    let mut b = a.clone();
    b.set(2, 2, BLUE);
    b.set(9, 9, RED);
    let (d, mask) = diff(&a, &b);
    assert_eq!((d.differing, d.total), (2, 100));
    assert_eq!(mask.get(2, 2), Some([255, 40, 40, 255]));
    assert_eq!(diff(&a, &a).0.differing, 0);
}

#[test]
fn a_sprite_that_already_is_pixel_art_round_trips_through_its_own_colours() {
    let img = picture(6, 4, &[([1, 1, 2, 2], RED), ([3, 1, 2, 2], BLUE)]);
    let palette = colors(&img);
    assert_eq!(
        palette,
        vec![
            Rgba(42, 36, 56, 255),
            Rgba(220, 30, 30, 255),
            Rgba(30, 30, 220, 255)
        ]
    );
    let map = shrink(&nearest(&img, &palette), img.w, 1.0);
    assert_eq!(map.paint(&palette), img);
}

#[test]
fn text_names_every_cell_by_letter_under_column_and_row_numbers() {
    let map = Map {
        w: 3,
        h: 2,
        cells: vec![Some(0), None, Some(1), None, Some(1), None],
    };
    let palette = [Rgba(255, 0, 0, 255), Rgba(0, 0, 255, 255)];
    let out = text(&map, &palette).map_err(|e| e.to_string());
    assert_eq!(
        out.as_deref(),
        Ok("3x2, 2 colours, . empty\na #ff0000\nb #0000ff\n  012\n0 a.b\n1 .b.\n")
    );
    // past ten columns the tens come on a line above the units
    let wide = text(&Map::empty(12, 1), &[]).unwrap_or_default();
    assert!(wide.contains("  000000000011\n  012345678901\n0 ............\n"));
    // more colours than letters is refused, not shown wrongly
    assert!(text(&map, &vec![Rgba(0, 0, 0, 255); 63]).is_err());
}

#[test]
fn zoom_draws_each_cell_as_a_square_inside_grid_lines() {
    let map = Map {
        w: 2,
        h: 1,
        cells: vec![Some(0), None],
    };
    let img = zoom(&map, &[Rgba(220, 30, 30, 255)], 4);
    // two 4-pixel cells and three one-pixel lines across, one cell and two lines down
    assert_eq!((img.w, img.h), (11, 6));
    assert_eq!(img.get(2, 2), Some(RED));
    assert_ne!(img.get(7, 2), Some(RED));
    // a line pixel is neither the cell colour nor an empty cell
    let line = img.get(5, 2);
    assert!(line.is_some() && line != Some(RED) && line != img.get(7, 2));
}

#[test]
fn feet_stands_two_figures_on_the_same_spot() {
    // The same 2x3 figure, once at the top left of a 4x4 picture and once at the bottom right of a 6x5 one.
    let figure = |w: u32, h: u32, ox: u32, oy: u32| {
        let mut img = RgbaImage::new(w, h);
        for y in 0..3 {
            for x in 0..2 {
                img.set(ox + x, oy + y, [200, 10, 10, 255]);
            }
        }
        img
    };
    let (a, b) = pixfx_pixel::feet(&figure(4, 4, 0, 0), &figure(6, 5, 4, 2));
    assert_eq!((a.w, a.h), (b.w, b.h));
    assert_eq!(pixfx_pixel::diff(&a, &b).0.differing, 0);
}

#[test]
fn a_huge_cut_tolerance_clears_everything_without_overflow() {
    let mut img = RgbaImage::new(4, 4);
    for y in 0..4 {
        for x in 0..4 {
            img.set(x, y, [x as u8 * 60, 0, 0, 255]);
        }
    }
    let out = cut(&img, u32::MAX);
    assert_eq!((out.w, out.h), (0, 0));
}

#[test]
fn a_text_grid_too_big_to_be_a_sprite_is_refused_on_its_head_line() {
    let e = pixfx_pixel::parse("100000x100000, 1 colours, . empty\na #ffffff\n");
    assert!(e.is_err_and(|e| e.line == 1));
}
