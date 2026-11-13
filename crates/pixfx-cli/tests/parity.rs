use pixfx_cli::parity::compare;
use pixfx_core::{Frame, Rendered};

/// One 10 x 10 frame, every pixel the same colour and light.
fn flat(color: u32, light: u32) -> Rendered {
    let mut f = Frame::new(10, 10, 0, 0);
    f.color.fill(color);
    f.light.fill(light);
    Rendered {
        w: 10,
        h: 10,
        fps: 30,
        origin: (0, 0),
        margin: 0,
        frames: vec![f],
    }
}

#[test]
fn one_colour_pixel_of_a_hundred_differing_fails() {
    let a = flat(0xff10_2030, 0);
    let mut b = flat(0xff10_2030, 0);
    b.frames[0].color[42] = 0xff10_2031;
    let r = compare(&a, &b);
    assert_eq!(r.colour_diff_frac, 0.01);
    assert_eq!(r.light_diff_frac, 0.0);
    assert!(!r.pass());
}
