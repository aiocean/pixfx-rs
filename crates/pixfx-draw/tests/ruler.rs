use pixfx_draw::Ruler;

const R: Ruler = Ruler {
    cell: 64,
    feet: (32, 60),
    height: 40,
    width: 20,
    outline: 1,
    head: 7,
    eye: (1, 2),
    hand: 2,
    limb: 3,
    blade: 16,
};

#[test]
fn a_thing_is_its_part_times_the_ratio_rounded_and_never_under_one_pixel() {
    // glasses: two lenses, each as wide as the 1-pixel eye
    assert_eq!(Ruler::size(R.eye.0, 2.0), 2);
    // a hat brim half again as wide as the 7-pixel head
    assert_eq!(Ruler::size(R.head, 1.5), 11);
    // an ant's eye, a tenth of ours, is still a pixel
    assert_eq!(Ruler::size(R.eye.0, 0.1), 1);
    assert_eq!(Ruler::size(R.eye.0, f64::NAN), 1);
}

#[test]
fn the_figure_is_the_height_without_the_outline_and_counts_its_heads() {
    assert_eq!(R.figure(), 38);
    assert!((R.heads() - 38.0 / 7.0).abs() < 1e-9);
}

#[test]
fn a_scaled_ruler_keeps_its_proportions_and_its_smallest_parts() {
    // a model's sheet at 64 squares for 96 engine pixels: two thirds
    let s = R.scaled(64.0 / 96.0);
    assert_eq!((s.height, s.head, s.blade), (27, 5, 11));
    assert_eq!(s.eye, (1, 1), "a part under one square stays one");
    assert_eq!(R.scaled(1.0), R);
}
