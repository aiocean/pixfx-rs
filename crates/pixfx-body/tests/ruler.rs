//! The painted mannequin stands on the game's ruler: 38 rows painted, soles on row 60 of a 64 cell, a 7 pixel head.

use pixfx_body::{Body, Camera, Cell, Dir4, Layers, Look, Part, catalog, sprite};

#[test]
fn the_idle_mannequin_is_the_rulers_size() {
    let b = Body::default();
    let Some(idle) = catalog::by_name("idle") else {
        unreachable!("idle is in the catalog and the look")
    };
    let s = sprite(
        &b,
        &idle,
        0,
        Camera::side(Dir4::S),
        Cell::RULER,
        &Look::mannequin(),
        Layers::ALL,
    );
    let rows: Vec<u32> = (0..64)
        .filter(|y| (0..64).any(|x| s.parts.id(x, *y) != 0))
        .collect();
    let (top, bottom) = (
        rows.first().copied().unwrap_or(0),
        rows.last().copied().unwrap_or(0),
    );
    let r = pixfx_draw::RULER;
    assert_eq!(bottom, r.feet.1 as u32, "soles on the feet row");
    let tall = bottom - top + 1;
    assert!(
        (tall as i64 - i64::from(r.figure())).abs() <= 1,
        "{tall} rows"
    );
    let head = Part::Head.id();
    let head_w = (0..64)
        .filter(|x| (0..64).any(|y| s.parts.id(*x, y) == head))
        .count();
    assert!(
        (head_w as i64 - i64::from(r.head)).abs() <= 1,
        "head {head_w}"
    );
}
