// Sheets: cells laid out for an image model, guides drawn into them, and each cell cut back out of what it drew.
use pixfx_pixel::sheet::{
    BORDER, GUIDE, Guide, Layout, blank, cell_rect, cut, directions, frames, guide, size,
};
use pixfx_pixel::{Rgba, RgbaImage};

const COLORS: [[u8; 4]; 6] = [
    [0x20, 0x30, 0x40, 0xff],
    [0xc0, 0x40, 0x30, 0xff],
    [0x40, 0xa0, 0x50, 0xff],
    [0xe0, 0xd0, 0x90, 0xff],
    [0x30, 0x30, 0xa0, 0xff],
    [0x00, 0x00, 0x00, 0x00],
];

fn px(c: Rgba) -> [u8; 4] {
    [c.0, c.1, c.2, c.3]
}

/// A `w` x `h` sprite seeded by `seed`, colours from a fixed list (one of them transparent).
fn sprite(w: u32, h: u32, seed: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let i = ((x * 7 + y * 13 + seed * 5) / 3 + (x * y + seed) % 4) as usize % COLORS.len();
            img.set(x, y, COLORS[i]);
        }
    }
    img
}

/// `img` resized to `w` x `h` by nearest neighbour, each target pixel taking the source pixel under its centre.
fn resize(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let sx = ((2 * u64::from(x) + 1) * u64::from(img.w) / (2 * u64::from(w))) as u32;
            let sy = ((2 * u64::from(y) + 1) * u64::from(img.h) / (2 * u64::from(h))) as u32;
            if let Some(p) = img.get(sx, sy) {
                out.set(x, y, p);
            }
        }
    }
    out
}

fn paste(sheet: &mut RgbaImage, pic: &RgbaImage, x: u32, y: u32) {
    for py in 0..pic.h {
        for px in 0..pic.w {
            if let Some(p) = pic.get(px, py) {
                sheet.set(x + px, y + py, p);
            }
        }
    }
}

const SIX: Layout = Layout {
    cell_w: 10,
    cell_h: 12,
    cols: 3,
    rows: 2,
    gap: 2,
    scale: 4,
};

/// A sheet with six known pictures, one per cell, and the pictures as they sit on it (in sheet pixels).
fn filled() -> (RgbaImage, Vec<RgbaImage>) {
    let mut sheet = blank(&SIX, false);
    let mut pics = Vec::new();
    for row in 0..SIX.rows {
        for col in 0..SIX.cols {
            let [x, y, w, h] = cell_rect(&SIX, col, row);
            let pic = resize(&sprite(SIX.cell_w, SIX.cell_h, row * SIX.cols + col), w, h);
            paste(&mut sheet, &pic, x, y);
            pics.push(pic);
        }
    }
    (sheet, pics)
}

fn inside(r: [u32; 4], x: u32, y: u32) -> bool {
    x >= r[0] && x < r[0] + r[2] && y >= r[1] && y < r[1] + r[3]
}

#[test]
fn size_and_cells_follow_the_layout() {
    assert_eq!(size(&SIX), ((3 * 10 + 4 * 2) * 4, (2 * 12 + 3 * 2) * 4));
    assert_eq!(cell_rect(&SIX, 0, 0), [8, 8, 40, 48]);
    assert_eq!(cell_rect(&SIX, 2, 1), [8 + 2 * 48, 8 + 56, 40, 48]);
}

#[test]
fn blank_borders_sit_on_cell_edges() {
    let sheet = blank(&SIX, false);
    let rects: Vec<[u32; 4]> = (0..SIX.rows)
        .flat_map(|r| (0..SIX.cols).map(move |c| cell_rect(&SIX, c, r)))
        .collect();
    let on_ring = |x: u32, y: u32| {
        rects.iter().any(|&[rx, ry, rw, rh]| {
            let (x0, y0, x1, y1) = (rx - 1, ry - 1, rx + rw, ry + rh);
            (x >= x0 && x <= x1 && (y == y0 || y == y1))
                || (y >= y0 && y <= y1 && (x == x0 || x == x1))
        })
    };
    for y in 0..sheet.h {
        for x in 0..sheet.w {
            let is_border = sheet.get(x, y) == Some(px(BORDER));
            assert_eq!(is_border, on_ring(x, y), "({x}, {y})");
        }
    }
    for r in &rects {
        assert!(!(r[0]..r[0] + r[2]).any(|x| sheet.get(x, r[1]) == Some(px(BORDER))));
    }
}

#[test]
fn pixel_lines_mark_every_sprite_pixel() {
    let sheet = blank(&SIX, true);
    let [x, y, _, _] = cell_rect(&SIX, 1, 1);
    for i in 0..SIX.cell_w {
        assert_eq!(sheet.get(x + i * 4, y + 1), Some(px(BORDER)));
        assert_ne!(sheet.get(x + i * 4 + 1, y + 1), Some(px(BORDER)));
    }
}

#[test]
fn bones_draw_only_in_guide_inside_their_cell() {
    let before = blank(&SIX, false);
    let mut after = before.clone();
    let bones = Guide::Bones(vec![
        [(5, 1), (5, 6)],
        [(5, 6), (1, 11)],
        [(0, 3), (30, 20)],
    ]);
    guide(&mut after, &SIX, 1, 0, &bones);
    let r = cell_rect(&SIX, 1, 0);
    let mut changed = 0;
    for y in 0..after.h {
        for x in 0..after.w {
            if after.get(x, y) != before.get(x, y) {
                changed += 1;
                assert_eq!(after.get(x, y), Some(px(GUIDE)), "({x}, {y})");
                assert!(inside(r, x, y), "({x}, {y}) outside {r:?}");
            }
        }
    }
    // The vertical bone: six sprite pixels of 4x4.
    assert!(changed >= 6 * 16, "{changed}");
    assert_eq!(after.get(r[0] + 5 * 4 + 2, r[1] + 3 * 4), Some(px(GUIDE)));
}

#[test]
fn pivot_and_ground_land_on_their_sprite_pixels() {
    let mut sheet = blank(&SIX, false);
    guide(&mut sheet, &SIX, 0, 1, &Guide::Pivot { x: 3, y: 4 });
    guide(&mut sheet, &SIX, 0, 1, &Guide::Ground { y: 11 });
    let [x, y, w, _] = cell_rect(&SIX, 0, 1);
    assert_eq!(sheet.get(x + 3 * 4 + 3, y + 4 * 4), Some(px(GUIDE)));
    assert_ne!(sheet.get(x + 4 * 4, y + 4 * 4), Some(px(GUIDE)));
    for i in 0..w {
        assert_eq!(sheet.get(x + i, y + 11 * 4 + 3), Some(px(GUIDE)));
    }
}

#[test]
fn a_filled_sheet_cuts_back_to_its_pictures() {
    let (sheet, pics) = filled();
    assert_eq!(cut(&sheet, &SIX), pics);
}

#[test]
fn guides_come_out_transparent() {
    let mut sheet = blank(&SIX, true);
    guide(&mut sheet, &SIX, 0, 0, &Guide::Ground { y: 5 });
    let cells = cut(&sheet, &SIX);
    assert_eq!(cells.len(), 6);
    let painted = |p: &[u8]| matches!(p, [_, _, _, a] if *a >= 128);
    for cell in &cells {
        for p in cell.rgba.as_chunks::<4>().0 {
            assert!(!painted(p) || *p == [0xff, 0xff, 0xff, 0xff], "{p:?}");
        }
    }
    let row5 = cells.first().map(|c| c.get(7, 5 * 4 + 2));
    assert_eq!(row5, Some(Some([0, 0, 0, 0])));
}

// Review Focus 3: a model returns its own size for the sheet; the cell rectangles scale with it. Two nearest-neighbour
// resamples (1.2x up, then back) move block edges by a sheet pixel, so each cut cell is compared at its sprite
// pixels (one sample at each block's centre), what `recover` brings a cell back to.
#[test]
fn a_sheet_drawn_bigger_cuts_back_scaled() {
    let (sheet, _) = filled();
    let drawn = resize(&sheet, sheet.w * 6 / 5, sheet.h * 6 / 5);
    let cells = cut(&drawn, &SIX);
    assert_eq!(cells.len(), 6);
    for (i, cell) in cells.iter().enumerate() {
        assert!(
            (47..=49).contains(&cell.w) && (57..=59).contains(&cell.h),
            "cell {i}: {}x{}",
            cell.w,
            cell.h
        );
        let original = sprite(SIX.cell_w, SIX.cell_h, i as u32);
        assert_eq!(resize(cell, SIX.cell_w, SIX.cell_h), original, "cell {i}");
    }
}

#[test]
fn directions_put_the_sprite_in_the_centre() {
    let s = sprite(8, 9, 3);
    let (layout, sheet) = directions(&s, 3);
    assert_eq!(
        (
            layout.cols,
            layout.rows,
            layout.cell_w,
            layout.cell_h,
            layout.scale
        ),
        (3, 3, 8, 9, 3)
    );
    assert_eq!((sheet.w, sheet.h), size(&layout));
    let cells = cut(&sheet, &layout);
    let centre = cells.get(4).map(|c| resize(c, 8, 9));
    let Some(centre) = centre else {
        return assert_eq!(cells.len(), 9);
    };
    for y in 0..9 {
        for x in 0..8 {
            let want = s
                .get(x, y)
                .filter(|p| p[3] >= 128)
                .unwrap_or([0xff, 0xff, 0xff, 0xff]);
            assert_eq!(centre.get(x, y), Some(want), "({x}, {y})");
        }
    }
    for (i, c) in cells.iter().enumerate().filter(|(i, _)| *i != 4) {
        assert!(
            c.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [0xff, 0xff, 0xff, 0xff]),
            "cell {i}"
        );
    }
}

#[test]
fn frames_lay_out_one_row() {
    let l = frames(5, 16, 20, 2);
    assert_eq!(
        (l.cols, l.rows, l.cell_w, l.cell_h, l.scale),
        (5, 1, 16, 20, 2)
    );
    assert_eq!(cut(&blank(&l, false), &l).len(), 5);
}

#[test]
fn a_layout_comes_back_from_its_json_and_bones_from_their_text() {
    let layout = Layout {
        cell_w: 32,
        cell_h: 48,
        cols: 3,
        rows: 2,
        gap: 1,
        scale: 6,
    };
    assert_eq!(
        pixfx_pixel::sheet::parse_layout(&pixfx_pixel::sheet::layout_json(&layout)),
        Ok(layout)
    );
    assert!(pixfx_pixel::sheet::parse_layout("{\"cell_w\": 3}").is_err());
    let bones = pixfx_pixel::sheet::parse_bones("# spine\n16,10 16,30\n\n16,30 10,46\n");
    assert_eq!(bones, Ok(vec![[(16, 10), (16, 30)], [(16, 30), (10, 46)]]));
    assert!(pixfx_pixel::sheet::parse_bones("16,10 16").is_err());
}

#[test]
fn a_layout_too_big_or_without_cells_is_refused() {
    use pixfx_pixel::sheet::check;
    let ok = Layout {
        cell_w: 64,
        cell_h: 64,
        cols: 4,
        rows: 2,
        gap: 2,
        scale: 8,
    };
    assert_eq!(check(&ok), Ok(()));
    assert!(
        check(&Layout {
            cell_w: 1_000_000,
            cols: 1000,
            rows: 1,
            cell_h: 1,
            ..ok
        })
        .is_err()
    );
    assert!(check(&Layout { cell_w: 0, ..ok }).is_err());
    assert!(pixfx_pixel::sheet::parse_layout(
        "{\"cell_w\": 70000, \"cell_h\": 70000, \"cols\": 8, \"rows\": 8, \"gap\": 2, \"scale\": 8}"
    )
    .is_err());
}
