//! What an effect reads of the sprite it wraps: the cells it covers, its edge, the cells round it and how far.

use pixfx_core::{Actor, Area, Silhouette};
use pixfx_draw::color::Rgba;
use pixfx_draw::math::Rng;

type Res = Result<(), Box<dyn std::error::Error>>;

const RED: u32 = 0xff00_00ff;

/// A 3 x 2 sprite standing on the origin: a full bottom row, one pixel on top in the middle (a hat, a sword hilt:
/// any sprite).
fn sprite() -> Result<Actor, pixfx_core::ActorError> {
    Actor::new(-1, -2, 3, 2, vec![0, RED, 0, RED, RED, RED])
}

const AREA: Area = Area {
    x: -4,
    y: -5,
    w: 8,
    h: 6,
};

#[test]
fn a_silhouette_covers_the_cells_under_the_sprites_opaque_pixels() -> Res {
    let s = Silhouette::of(&sprite()?, AREA);
    assert_eq!(
        s.cells().collect::<Vec<_>>(),
        [(0, -2), (-1, -1), (0, -1), (1, -1)]
    );
    assert!(s.covers(0, -2) && !s.covers(-1, -2) && !s.covers(40, 40));
    assert_eq!(
        s.bounds(),
        Some(Area {
            x: -1,
            y: -2,
            w: 3,
            h: 2
        })
    );
    assert_eq!(s.centroid(), Some((0.5, -0.75)));
    Ok(())
}

#[test]
fn every_cell_of_so_thin_a_body_is_on_its_edge() -> Res {
    let s = Silhouette::of(&sprite()?, AREA);
    assert_eq!(s.edge().count(), 4);
    // a 3 x 3 block has one cell inside its edge
    let block = Silhouette::from_fn(AREA, |x, y| {
        (-1.0..2.0).contains(&x) && (-3.0..0.0).contains(&y)
    });
    assert_eq!(block.cells().count(), 9);
    assert_eq!(block.edge().count(), 8);
    assert!(
        !block.edge().any(|c| c == (0, -2)),
        "the middle is not on the edge"
    );
    Ok(())
}

#[test]
fn cells_round_the_body_come_with_their_distance_in_pixels() -> Res {
    let s = Silhouette::of(&sprite()?, AREA);
    assert_eq!(s.distance(0, -1), Some(0.0));
    assert_eq!(s.distance(0, -3), Some(1.0), "a step above the top pixel");
    assert_eq!(s.distance(-1, -2), Some(1.0));
    assert_eq!(s.distance(-2, -3), Some(7.0 / 3.0), "a step and a diagonal");
    assert_eq!(s.distance(99, 0), None);
    let near: Vec<_> = s.around(1.0).collect();
    assert!(near.iter().all(|&(x, y, d)| d == 1.0 && !s.covers(x, y)));
    assert!(near.contains(&(0, -3, 1.0)));
    assert!(
        s.around(100.0).count() == AREA.w as usize * AREA.h as usize - 4,
        "every other cell"
    );
    Ok(())
}

#[test]
fn a_pick_lands_on_the_body_and_is_the_same_from_the_same_seed() -> Res {
    let s = Silhouette::of(&sprite()?, AREA);
    let picks = |seed| {
        let mut r = Rng::new(seed);
        (0..20).map(|_| s.pick(&mut r, 200)).collect::<Vec<_>>()
    };
    let a = picks(7);
    assert!(a.iter().all(|p| p.is_some_and(|(x, y)| s.covers(x, y))));
    assert_eq!(a, picks(7));
    let empty = Silhouette::from_fn(AREA, |_, _| false);
    assert!(empty.is_empty());
    assert_eq!(empty.pick(&mut Rng::new(1), 40), None);
    assert_eq!(empty.bounds(), None);
    Ok(())
}

#[test]
fn a_sprite_off_the_area_covers_nothing() -> Res {
    let far = sprite()?.moved_to(100, 100);
    assert!(Silhouette::of(&far, AREA).is_empty());
    Ok(())
}

#[test]
fn the_sprites_opaque_pixels_come_with_their_place_and_colour() -> Res {
    let px: Vec<_> = sprite()?.opaque().collect();
    assert_eq!(px.len(), 4);
    assert_eq!(px.first(), Some(&(0, -2, Rgba(255, 0, 0, 255))));
    Ok(())
}
