// Frames lined up on one origin: a subject drawn a few pixels off comes back, while a frame whose content changes
// (half of it gone, a spark beside it) or that holds almost nothing stays where it is.
use pixfx_pixel::RgbaImage;
use pixfx_pixel::register::{apply, register};

const W: u32 = 64;
const H: u32 = 64;
const BODY: [u8; 4] = [220, 180, 60, 255];
const LINE: [u8; 4] = [255, 250, 220, 255];
const SPARK: [u8; 4] = [255, 255, 255, 255];

/// A bell-like subject at `(ox, oy)`: a filled octagon with a bright outline and bands, rows below `cut` left out.
fn subject(ox: i32, oy: i32, cut: i32) -> RgbaImage {
    let mut img = RgbaImage::new(W, H);
    for y in 0..40 {
        for x in 0..36 {
            // an octagon 36 wide, 40 tall, corners cut 8
            let (cx, cy) = (x.min(35 - x), y.min(39 - y));
            if cx + cy < 8 || y >= cut {
                continue;
            }
            let edge = cx + cy < 10 || cx == 0 || cy == 0;
            let band = y % 9 == 4 || (x % 7 == 3 && y % 9 > 5);
            let p = if edge || band { LINE } else { BODY };
            let (px, py) = (x + ox, y + oy);
            if (0..W as i32).contains(&px) && (0..H as i32).contains(&py) {
                img.set(px as u32, py as u32, p);
            }
        }
    }
    img
}

#[test]
fn frames_drawn_off_come_back_and_changing_ones_stay() -> Result<(), String> {
    let mut spark = subject(14, 12, 40);
    for i in 0..12 {
        spark.set(52 + (i % 4), 20 + i, SPARK);
        spark.set(50 + i.min(13), 26, SPARK);
    }
    let mut speck = RgbaImage::new(W, H);
    speck.set(30, 30, SPARK);
    let frames = [
        subject(14, 12, 40),
        subject(14, 12, 40),
        subject(14, 12, 40),
        subject(14, 7, 40),  // drawn 5 up
        subject(17, 12, 40), // drawn 3 right
        subject(14, 12, 22), // its bottom gone: dissolving, not moving
        spark,
        speck,
    ];
    let s = register(&frames, 6)?;
    let got: Vec<(i32, i32)> = s.iter().map(|s| (s.dx, s.dy)).collect();
    assert_eq!(got, [(0, 0), (0, 0), (0, 0), (0, 5), (-3, 0), (0, 0), (0, 0), (0, 0)], "{s:?}");
    assert!(s[3].score > s[3].unshifted, "a taken shift correlates better than none");
    let back = apply(&frames[3], s[3]);
    assert_eq!(back.get(14 + 18, 12 + 4), Some(LINE), "the band is back on its row");
    Ok(())
}

#[test]
fn frames_of_different_sizes_are_refused() {
    assert!(register(&[RgbaImage::new(8, 8), RgbaImage::new(9, 8)], 2).is_err());
}

#[test]
fn apply_moves_right_and_down() {
    let mut img = RgbaImage::new(4, 4);
    img.set(1, 1, SPARK);
    let s = pixfx_pixel::register::Shift { dx: 2, dy: -1, score: 1.0, unshifted: 0.0 };
    let out = apply(&img, s);
    assert_eq!(out.get(3, 0), Some(SPARK));
    assert_eq!(out.get(1, 1), Some([0; 4]));
}

#[test]
fn a_best_match_on_the_window_edge_is_not_taken() -> Result<(), String> {
    // drawn 4 down, searched within 4: the match lands on the window's edge, so it is not trusted
    let frames = [subject(14, 12, 40), subject(14, 12, 40), subject(14, 12, 40), subject(14, 16, 40)];
    let s = register(&frames, 4)?;
    assert_eq!((s[3].dx, s[3].dy), (0, 0), "{s:?}");
    let s = register(&frames, 5)?;
    assert_eq!((s[3].dx, s[3].dy), (0, -4), "inside a wider window it is found: {s:?}");
    Ok(())
}
