// OKLab: white is lightness 1 with no colour, black is 0, and distances follow how different colours look.
use pixfx_pixel::Rgba;
use pixfx_pixel::oklab::{delta_e, lab};

#[test]
fn white_and_black_sit_at_the_ends_of_lightness() {
    let w = lab(Rgba(255, 255, 255, 255));
    assert!((w.l - 1.0).abs() < 1e-6 && w.a.abs() < 1e-6 && w.b.abs() < 1e-6, "{w:?}");
    assert_eq!(lab(Rgba(0, 0, 0, 255)).l, 0.0);
}

#[test]
fn distances_follow_how_different_colours_look() {
    let (red, orange, blue) = (
        lab(Rgba(220, 30, 30, 255)),
        lab(Rgba(230, 120, 30, 255)),
        lab(Rgba(30, 30, 220, 255)),
    );
    assert_eq!(delta_e(red, red), 0.0);
    assert!(delta_e(red, orange) < delta_e(red, blue));
}

#[test]
fn the_same_colours_give_the_same_bits() {
    let all = |_: ()| -> Vec<u64> {
        (0..4096u32)
            .map(|i| {
                let c = Rgba((i >> 4) as u8, (i * 7) as u8, (i * 13) as u8, 255);
                let l = lab(c);
                l.l.to_bits() ^ l.a.to_bits().rotate_left(21) ^ l.b.to_bits().rotate_left(42)
            })
            .collect()
    };
    assert_eq!(all(()), all(()));
}
