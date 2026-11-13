//! The camera pass: shake offsets the view, ripples bend it, flash lifts it toward white.

use pixfx_core::Frame;
use pixfx_core::screen::{Ripple, flash, ripple, ripple_rgba};

fn gradient(w: u32, h: u32) -> Frame {
    let mut f = Frame::new(w, h, 0, 0);
    for (i, p) in f.color.iter_mut().enumerate() {
        *p = 0xff00_0000 | ((i as u32 * 7 + 3) % 0x00ff_ffff);
    }
    f
}

#[test]
fn flash_one_is_white() {
    let mut f = gradient(8, 8);
    flash(&mut f, 1.0);
    assert!(f.color.iter().all(|&p| p == 0xffff_ffff));
    assert!(f.light.iter().all(|&p| p & 0x00ff_ffff == 0x00ff_ffff));
}

#[test]
fn flash_zero_is_identity() {
    let mut f = gradient(8, 8);
    let before = f.clone();
    flash(&mut f, 0.0);
    assert_eq!(f, before);
}

#[test]
fn ripple_outside_ring_is_identity() {
    let src = gradient(40, 40);
    let mut dst = Frame::new(40, 40, 0, 0);
    // ring at radius 5 of a 10-wide band, centred far away from the corner pixels we compare
    let r = Ripple {
        x: 20.0,
        y: 20.0,
        r0: 5.0,
        r1: 5.0,
        dur: 1.0,
        amp: 1.0,
        age: 0.5,
    };
    ripple(&src, &mut dst, &[r], (0, 0));
    assert_eq!(dst.color[0], src.color[0]);
    assert_eq!(dst.color[40 * 40 - 1], src.color[40 * 40 - 1]);
}

#[test]
fn ripple_on_the_ring_bends_the_view() {
    let src = gradient(40, 40);
    let mut dst = Frame::new(40, 40, 0, 0);
    let r = Ripple {
        x: 20.0,
        y: 20.0,
        r0: 8.0,
        r1: 8.0,
        dur: 1.0,
        amp: 1.0,
        age: 0.0,
    };
    ripple(&src, &mut dst, &[r], (0, 0));
    assert_ne!(dst.color[20 * 40 + 28], src.color[20 * 40 + 28]);
}

#[test]
fn shake_offsets_the_view() {
    let src = gradient(10, 10);
    let mut dst = Frame::new(10, 10, 0, 0);
    ripple(&src, &mut dst, &[], (2, 1));
    // the view moves by (dx, dy): dst(x, y) shows src(x - dx, y - dy), clamped at the edge
    assert_eq!(dst.color[5 * 10 + 5], src.color[4 * 10 + 3]);
    assert_eq!(dst.color[0], src.color[0]);
}

/// A w x h RGBA image whose every pixel differs from its neighbours.
fn rgba(w: u32, h: u32) -> Vec<u8> {
    (0..w * h)
        .flat_map(|i| {
            let [a, b, c, _] = i.wrapping_mul(2_654_435_761).to_le_bytes();
            [a, b, c, 255]
        })
        .collect()
}

fn px(img: &[u8], w: u32, x: u32, y: u32) -> &[u8] {
    let i = ((y * w + x) * 4) as usize;
    img.get(i..i + 4).unwrap_or(&[])
}

#[test]
fn a_canvas_with_no_ripples_is_copied_as_drawn() {
    let src = rgba(16, 12);
    let mut dst = vec![0; src.len()];
    ripple_rgba(&src, &mut dst, 16, 12, &[], 3.0);
    assert_eq!(dst, src);
}

#[test]
fn a_ripple_bends_a_scaled_canvas_on_its_ring_and_leaves_the_rest() {
    // game-pixel ring of radius 8 round (20, 20), on a canvas drawn 3x: the ring sits 24 canvas pixels out
    let (w, h, unit) = (120, 120, 3.0);
    let r = Ripple {
        x: 20.0,
        y: 20.0,
        r0: 8.0,
        r1: 8.0,
        dur: 1.0,
        amp: 1.0,
        age: 0.0,
    };
    let src = rgba(w, h);
    let mut dst = vec![0; src.len()];
    ripple_rgba(&src, &mut dst, w, h, &[r], unit);
    assert_ne!(
        px(&dst, w, 60 + 24, 60),
        px(&src, w, 60 + 24, 60),
        "on the ring"
    );
    assert_eq!(px(&dst, w, 60, 60), px(&src, w, 60, 60), "the centre stays");
    assert_eq!(px(&dst, w, 0, 0), px(&src, w, 0, 0), "far away stays");
}
