use pixfx_pixel::reference::RgbaImage;
use pixfx_pixel::reference::compare::{ENGINE_CELL, FEET, Pair, board, effect_cell};

#[test]
fn the_guide_figure_is_the_duels_fighter_in_engine_pixels() -> Result<(), String> {
    let r = pixfx_draw::RULER;
    let bones = pixfx_pixel::sheet::parse_bones(pixfx_pixel::reference::spec::FIGURE)?;
    let pts: Vec<(u32, u32)> = bones.iter().flat_map(|(_, b)| [b[0], b[1]]).collect();
    let (top, bottom) = (pts.iter().map(|p| p.1).min(), pts.iter().map(|p| p.1).max());
    let (left, right) = (pts.iter().map(|p| p.0).min(), pts.iter().map(|p| p.0).max());
    assert_eq!(bottom, Some(FEET), "soles on the feet row");
    assert_eq!(
        bottom.zip(top).map(|(b, t)| b - t + 1),
        Some(r.figure()),
        "crown to soles"
    );
    assert_eq!(
        left.zip(right).map(|(l, rr)| (l + rr) / 2),
        Some(ENGINE_CELL / 2),
        "centred"
    );
    let crown: Vec<u32> = pts
        .iter()
        .filter(|p| Some(p.1) == top)
        .map(|p| p.0)
        .collect();
    let head = crown
        .iter()
        .max()
        .zip(crown.iter().min())
        .map(|(a, b)| a - b + 1);
    assert_eq!(head, Some(r.head), "the head as wide as the face");
    Ok(())
}

#[test]
fn the_effects_origin_lands_where_the_figure_stands_in_the_cell() {
    // a 128 x 120 effect frame with its origin at (64, 106), a white 3 x 3 block on it
    let (w, h, origin) = (128u32, 120u32, (64i32, 106i32));
    let mut f = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let on = (x as i32 - origin.0).abs() <= 1 && (y as i32 - origin.1).abs() <= 1;
            f.set(
                x,
                y,
                if on {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                },
            );
        }
    }
    let cell = effect_cell(&f, origin, 64, 64);
    assert_eq!((cell.w, cell.h), (64, 64));
    let (fx, fy) = (64 / 2, FEET * 64 / ENGINE_CELL);
    let bright = |x: u32, y: u32| cell.get(x, y).map_or(0, |p| p[0]);
    let peak = (fy - 1..=fy + 1)
        .flat_map(|y| (fx - 1..=fx + 1).map(move |x| (x, y)))
        .map(|(x, y)| bright(x, y))
        .max();
    assert!(
        peak.is_some_and(|v| v > 100),
        "the block shows at the feet ({fx}, {fy})"
    );
    assert_eq!(bright(5, 5), 0, "nothing elsewhere");
}

#[test]
fn the_board_has_a_line_of_sheet_cells_over_a_line_of_effect_frames_per_row() {
    let pic = RgbaImage::new(8, 8);
    let pairs = [
        Pair {
            sheet: vec![pic.clone(); 3],
            effect: vec![pic.clone(); 3],
        },
        Pair {
            sheet: vec![pic.clone(); 2],
            effect: vec![pic; 2],
        },
    ];
    let b = board(&pairs, 20, 4);
    assert_eq!((b.w, b.h), (3 * 24 + 4, 4 * 24 + 4));
}
