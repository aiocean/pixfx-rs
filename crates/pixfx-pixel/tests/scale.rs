use pixfx_draw::{RULER, RgbaImage, Ruler};
use pixfx_pixel::critique::Severity;
use pixfx_pixel::scale::{against, part, standing};

/// A `w` x `h` block of painted cells in a 64x64 picture, its bottom on row 60.
fn block(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(64, 64);
    for y in (61 - h)..61 {
        for x in 20..20 + w {
            img.set(x, y, [200, 100, 50, 255]);
        }
    }
    img
}

#[test]
fn a_standing_character_at_the_rulers_height_passes_and_one_drawn_bigger_is_an_error() {
    assert!(standing(&block(RULER.width, RULER.height), &RULER).is_empty());
    let big = standing(&block(RULER.width, 56), &RULER);
    assert_eq!(big.len(), 1);
    assert_eq!(big[0].severity, Severity::Error);
    assert!(
        big[0]
            .message
            .contains("56 rows tall where the ruler says 40"),
        "{}",
        big[0].message
    );
    let wide = standing(&block(30, RULER.height), &RULER);
    assert!(wide.iter().all(|i| i.severity == Severity::Warning) && wide.len() == 1);
}

#[test]
fn a_thing_is_measured_against_a_body_part_and_never_asked_under_one_cell() {
    let eye = part(&RULER, "eye").unwrap_or(0);
    // glasses two eyes wide, drawn 2 across: right; drawn 6 across: too big
    assert!(against(&block(2, 1), "eye x 2", eye, 2.0).is_empty());
    assert_eq!(against(&block(6, 1), "eye x 2", eye, 2.0).len(), 1);
    // an ant's eye is still a cell
    assert!(against(&block(1, 1), "eye x 0.1", eye, 0.1).is_empty());
    assert_eq!(Ruler::size(eye, 0.1), 1);
    assert_eq!(part(&RULER, "tail"), None);
}
