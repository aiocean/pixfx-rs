//! The camera pass: shake offsets the view, ripples bend it, flash lifts it toward white.

use pixfx_core::Frame;
use pixfx_core::screen::{Ripple, flash, ripple};

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
