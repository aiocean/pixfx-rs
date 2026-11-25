//! `pixfx_draw::RULER` is what the painter paints: the idle fighter measured pixel by pixel.

use pixfx_draw::{PixelDensity, RULER, Rgba};
use pixfx_fighter::look::Look;
use pixfx_fighter::paint::{
    Appearance, CX, FEET, SPRITE, Sprite, outline, paint, paint_at_density,
};
use pixfx_fighter::pose::Pose;

fn standing(pose: Pose) -> Appearance {
    Appearance {
        pose,
        pose_t: 0.2,
        face: 1.0,
        x: 0.0,
        air: 0.0,
        vx: 0.0,
        vy: 0.0,
        out: false,
        flash: false,
        untouchable: false,
        frozen: false,
        shocked: false,
        hit_t: 0.2,
        hit_power: 6.0,
        dodged: 0.0,
        act: None,
        jolt: [0, 0],
    }
}

/// Columns and rows of the pixels of `s` in `rows` that `keep` takes, as (x0, y0, w, h).
fn bounds(
    s: &Sprite,
    rows: core::ops::Range<usize>,
    keep: impl Fn(Rgba) -> bool,
) -> (u32, u32, u32, u32) {
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for y in rows {
        for x in 0..SPRITE {
            if let Some(p) = s.pixel(x, y)
                && p.3 > 0
                && keep(p)
            {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    if x0 == usize::MAX {
        return (0, 0, 0, 0);
    }
    let n = |v: usize| u32::try_from(v).unwrap_or(u32::MAX);
    (n(x0), n(y0), n(x1 + 1 - x0), n(y1 + 1 - y0))
}

fn same(a: Rgba, b: Rgba) -> bool {
    (a.0, a.1, a.2) == (b.0, b.1, b.2)
}

#[test]
fn dense_fighter_has_twice_as_many_visible_rows() {
    let look = Look::thunder();
    let mut sprite = paint_at_density(&look, &standing(Pose::Idle), PixelDensity::Two);
    outline(&mut sprite, look.outline);
    let image = sprite.to_image();
    assert_eq!(image.w, RULER.cell * 2);
    let rows = image
        .rgba
        .chunks_exact(image.w as usize * 4)
        .filter(|row| row.as_chunks::<4>().0.iter().any(|pixel| pixel[3] != 0))
        .count();
    assert_eq!(rows, RULER.height as usize * 2);
}

#[test]
fn the_standing_fighter_on_screen_is_as_tall_and_wide_as_the_ruler_and_stands_on_its_feet() {
    let look = Look::thunder();
    let mut s = paint(&look, &standing(Pose::Idle));
    outline(&mut s, look.outline);
    let (x, y, w, h) = bounds(&s, 0..SPRITE, |_| true);
    assert_eq!((RULER.cell as usize, RULER.feet), (SPRITE, (CX, FEET)));
    assert_eq!((w, h), (RULER.width, RULER.height), "idle at ({x}, {y})");
    let soles = i64::from(y) + i64::from(h) - 1 - i64::from(RULER.outline);
    assert_eq!(
        soles,
        i64::from(RULER.feet.1),
        "the soles stand on the feet row"
    );
    let middle = i64::from(x) + i64::from(w / 2);
    assert!(
        (middle - i64::from(RULER.feet.0)).abs() <= 4,
        "centred on the feet: {middle}"
    );
}

#[test]
fn the_face_the_eye_the_hand_and_the_blade_are_the_rulers_parts() {
    let look = Look::thunder();
    let idle = paint(&look, &standing(Pose::Idle));
    let skin = |p: Rgba| same(p, look.skin[0]) || same(p, look.skin[1]);
    let (_, fy, fw, fh) = bounds(&idle, 0..59, skin);
    // the face is painted head x head; the hair covers its top row
    assert_eq!(
        (fw, fh),
        (RULER.head, RULER.head - 1),
        "the face, from row {fy}"
    );
    let face = (fy as usize)..(fy + fh) as usize;
    let (_, _, ew, eh) = bounds(&idle, face, |p| same(p, look.outline));
    assert_eq!((ew, eh), RULER.eye, "the open eye");
    let (_, _, hw, hh) = bounds(&idle, (fy + fh + 2) as usize..SPRITE, |p| {
        same(p, look.skin[0])
    });
    assert_eq!((hw, hh), (RULER.hand, RULER.hand), "the hand");
    // on guard the blade stands upright, so its length is its rows
    let guard = paint(&look, &standing(Pose::Guard));
    let (_, _, _, bh) = bounds(&guard, 0..SPRITE, |p| {
        look.blade.iter().any(|&c| same(p, c))
    });
    assert_eq!(bh, RULER.blade, "the blade");
}

#[test]
fn the_painted_body_is_the_rulers_figure_about_five_and_a_half_heads_tall() {
    assert_eq!(RULER.figure(), 61);
    assert!((RULER.heads() - 61.0 / 11.0).abs() < 1e-9);
}
