use pixfx_draw::{RULER, RgbaImage, Ruler};
use pixfx_pixel::critique::Severity;
use pixfx_pixel::scale::{against, part, standing};

/// A `w` x `h` block of painted cells in a picture of the ruler's cell, its bottom on the ruler's feet row.
fn block(w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(RULER.cell, RULER.cell);
    let bottom = RULER.feet.1 as u32 + 1;
    for y in (bottom - h)..bottom {
        for x in 20..20 + w {
            img.set(x, y, [200, 100, 50, 255]);
        }
    }
    img
}

#[test]
fn a_standing_character_at_the_rulers_height_passes_and_one_drawn_bigger_is_an_error() {
    assert!(standing(&block(RULER.width, RULER.height), &RULER).is_empty());
    let big = standing(&block(RULER.width, RULER.height + 16), &RULER);
    assert_eq!(big.len(), 1);
    assert_eq!(big[0].severity, Severity::Error);
    assert!(
        big[0].message.contains(&format!(
            "{} rows tall where the ruler says {}",
            RULER.height + 16,
            RULER.height
        )),
        "{}",
        big[0].message
    );
    let wide = standing(&block(RULER.width + 10, RULER.height), &RULER);
    assert!(wide.iter().all(|i| i.severity == Severity::Warning) && wide.len() == 1);
}

#[test]
fn a_thing_is_measured_against_a_body_part_and_never_asked_under_one_cell() {
    let eye = part(&RULER, "eye").unwrap_or(0);
    // glasses two eyes wide, drawn that many across: right; three times as wide: too big
    let glasses = Ruler::size(eye, 2.0);
    assert!(against(&block(glasses, 1), "eye x 2", eye, 2.0).is_empty());
    assert_eq!(
        against(&block(3 * glasses, 1), "eye x 2", eye, 2.0).len(),
        1
    );
    // an ant's eye is still a cell
    assert!(against(&block(1, 1), "eye x 0.1", eye, 0.1).is_empty());
    assert_eq!(Ruler::size(eye, 0.1), 1);
    assert_eq!(part(&RULER, "tail"), None);
}
