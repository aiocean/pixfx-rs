//! A painted halo: strokes keep their heat and kind, the heat spreads out as a rounded octagon, and only hot pixels
//! are walked.

use pixfx_draw::halo::Halo;

fn heats(h: &Halo) -> Vec<((i32, i32), f32, u8)> {
    h.each()
        .map(|(x, y, v, k)| ((x as i32, y as i32), v, k))
        .collect()
}

fn at(h: &Halo, x: i32, y: i32) -> f32 {
    heats(h)
        .into_iter()
        .find(|(p, _, _)| *p == (x, y))
        .map_or(0.0, |(_, v, _)| v)
}

#[test]
fn a_stroke_spreads_into_a_rounded_octagon_losing_fall_a_pixel() {
    let mut h = Halo::new(20, 20, 10.0, 10.0);
    h.stamp(0.0, 0.0, 3.0, 7);
    h.spread(1.0);
    assert_eq!(at(&h, 0, 0), 3.0);
    // the four sides and the corners one step out: a corner is reached by the eight-neighbour step
    for (x, y) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1)] {
        assert_eq!(at(&h, x, y), 2.0, "({x}, {y})");
    }
    assert_eq!(at(&h, 2, 0), 1.0);
    assert_eq!(at(&h, 2, 1), 1.0);
    // the far corner stays cold, so the halo is not a square
    assert_eq!(at(&h, 2, 2), 0.0);
    assert_eq!(at(&h, 3, 0), 0.0);
}

#[test]
fn only_the_stroke_keeps_its_kind_and_the_hotter_stroke_wins() {
    let mut h = Halo::new(20, 20, 10.0, 10.0);
    h.stamp(0.0, 0.0, 1.0, 1);
    h.stamp(0.0, 0.0, 2.5, 3);
    h.stamp(0.0, 0.0, 2.0, 5);
    h.spread(1.4);
    let all = heats(&h);
    let centre = all.iter().find(|(p, _, _)| *p == (0, 0));
    assert_eq!(centre.map(|c| (c.1, c.2)), Some((2.5, 3)));
    assert!(
        all.iter()
            .filter(|(p, _, _)| *p != (0, 0))
            .all(|(_, _, k)| *k == 0)
    );
}

#[test]
fn strokes_off_the_grid_or_on_its_edge_are_dropped_and_none_takes_nothing() {
    let mut h = Halo::new(10, 10, 0.0, 0.0);
    h.stamp(0.0, 5.0, 3.0, 1);
    h.stamp(9.0, 5.0, 3.0, 1);
    h.stamp(-4.0, 20.0, 3.0, 1);
    h.spread(1.0);
    assert!(heats(&h).is_empty());
    let mut none = Halo::none();
    none.stamp(0.0, 0.0, 3.0, 1);
    none.spread(1.0);
    assert!(heats(&none).is_empty());
}

#[test]
fn a_halo_never_reaches_the_canvas_edge() {
    let mut h = Halo::new(8, 8, 0.0, 0.0);
    h.stamp(1.0, 1.0, 6.0, 1);
    h.spread(1.0);
    assert!(
        heats(&h)
            .iter()
            .all(|((x, y), _, _)| (1..7).contains(x) && (1..7).contains(y))
    );
}
