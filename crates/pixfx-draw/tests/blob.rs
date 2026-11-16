//! A blob painted as a caller paints liquid: points splatted, then painted in steps.

use pixfx_draw::Frame;
use pixfx_draw::Painter;
use pixfx_draw::blob::{Blob, BlobLook};
use pixfx_draw::color::Rgba;

const LOOK: BlobLook = BlobLook {
    edge: Rgba::hex24(0x101010),
    body: Rgba::hex24(0x404040),
    deep: Rgba::hex24(0x202020),
    rim: Rgba::hex24(0xf0f0f0),
    iso: 0.4,
    deep_at: 2.0,
    shine: 0.0,
};

fn paint(blob: &Blob) -> Frame {
    let mut f = Frame::new(32, 32, 0, 0);
    let mut p = Painter::new(&mut f);
    p.place(16.0, 16.0, false, 0.0);
    blob.paint(&mut p, &LOOK);
    f
}

fn color(f: &Frame, x: usize, y: usize) -> u32 {
    f.color[y * f.w as usize + x]
}

#[test]
fn a_lone_point_is_a_round_drop_lit_on_top_and_outlined_below() {
    let mut b = Blob::new(-16.0, -16.0, 16.0, 16.0);
    b.splat(0.0, 0.0, 3.0, 1.0);
    let f = paint(&b);
    let top = (0..32).find(|&y| color(&f, 16, y) != 0).unwrap_or(99);
    let bottom = (0..32).rev().find(|&y| color(&f, 16, y) != 0).unwrap_or(0);
    assert_eq!(color(&f, 16, top), LOOK.rim.pack(), "lit on top");
    assert_eq!(color(&f, 16, bottom), LOOK.edge.pack(), "outlined below");
    assert!(bottom > top, "more than a pixel tall");
}

#[test]
fn points_close_together_merge_and_a_thick_middle_is_deep() {
    let mut b = Blob::new(-16.0, -16.0, 16.0, 16.0);
    for i in -4..=4 {
        for j in -2..=2 {
            b.splat(f64::from(i), f64::from(j), 2.6, 1.0);
        }
    }
    let f = paint(&b);
    // one connected row through the middle, no gaps
    let row: Vec<u32> = (8..24).map(|x| color(&f, x, 16)).collect();
    let first = row.iter().position(|&c| c != 0).unwrap_or(0);
    let last = row.iter().rposition(|&c| c != 0).unwrap_or(0);
    assert!(row[first..=last].iter().all(|&c| c != 0), "merged: {row:?}");
    assert_eq!(color(&f, 16, 16), LOOK.deep.pack(), "deep in the middle");
}
