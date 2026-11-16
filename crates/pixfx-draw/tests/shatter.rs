//! What a shattered shape must keep: the same pieces from the same seed, pieces that are real pieces of the shape,
//! walled in, and the held part never falling.

use pixfx_draw::shatter::{Cracks, Hold, Shatter, shatter};

fn pane(x: i32, y: i32) -> bool {
    let (u, v) = (f64::from(x) / 46.0, f64::from(y) / 20.0);
    u * u + v * v < 1.0
}

fn wall(x: i32, y: i32) -> bool {
    x.abs() < 28 && y > -38
}

fn pane_cracks(seed: f64) -> Cracks {
    Cracks {
        strikes: 14,
        bounds: (-48, -24, 47, 23),
        hold: Hold::Rim,
        seed,
        ..Cracks::default()
    }
}

fn wall_cracks() -> Cracks {
    Cracks {
        strikes: 12,
        bounds: (-30, -40, 29, 39),
        hold: Hold::At(0, 39),
        inner: 0.5,
        reach: (8.0, 26.0),
        seed: 9.0,
        ..Cracks::default()
    }
}

fn cracked(s: &Shatter) -> std::collections::BTreeSet<(i32, i32)> {
    s.strikes
        .iter()
        .flat_map(|st| st.cells.iter().copied())
        .collect()
}

#[test]
fn the_same_seed_breaks_the_same_way() {
    assert_eq!(
        shatter(pane_cracks(4.0), pane),
        shatter(pane_cracks(4.0), pane)
    );
    assert_ne!(
        shatter(pane_cracks(4.0), pane),
        shatter(pane_cracks(5.0), pane)
    );
}

#[test]
fn strikes_close_loops_and_pieces_fall() {
    for seed in [1.0, 4.0, 7.0] {
        let s = shatter(pane_cracks(seed), pane);
        assert!(!s.pieces.is_empty(), "seed {seed}: no piece broke away");
    }
}

#[test]
fn every_piece_is_of_the_shape_and_no_cell_falls_twice() {
    let s = shatter(pane_cracks(4.0), pane);
    let mut seen = std::collections::BTreeSet::new();
    for p in &s.pieces {
        for &(x, y) in &p.cells {
            assert!(pane(x, y), "({x}, {y}) is off the shape");
            assert!(seen.insert((x, y)), "({x}, {y}) fell twice");
        }
    }
}

#[test]
fn a_piece_held_by_the_rim_never_falls() {
    // Hold::Rim: the whole (uncracked) cells of a falling piece never touch the outline
    let s = shatter(pane_cracks(4.0), pane);
    let crack = cracked(&s);
    for p in &s.pieces {
        for &(x, y) in p.cells.iter().filter(|c| !crack.contains(c)) {
            let on_rim = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|&(dx, dy)| !pane(x + dx, y + dy));
            assert!(!on_rim, "({x}, {y}) is on the rim but fell");
        }
    }
}

#[test]
fn a_piece_is_one_block_walled_in_by_cracks() {
    // its whole cells all join across sides: one block, not scattered cells
    let s = shatter(pane_cracks(4.0), pane);
    let crack = cracked(&s);
    for p in &s.pieces {
        let whole: std::collections::BTreeSet<(i32, i32)> = p
            .cells
            .iter()
            .copied()
            .filter(|c| !crack.contains(c))
            .collect();
        let Some(&first) = whole.iter().next() else {
            continue;
        };
        let mut reached = std::collections::BTreeSet::from([first]);
        let mut todo = vec![first];
        while let Some((x, y)) = todo.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = (x + dx, y + dy);
                if whole.contains(&n) && reached.insert(n) {
                    todo.push(n);
                }
            }
        }
        assert_eq!(reached.len(), whole.len(), "a piece is in bits");
    }
}

#[test]
fn the_part_held_at_a_point_stays_and_the_rest_breaks_off() {
    let s = shatter(wall_cracks(), wall);
    assert!(!s.pieces.is_empty(), "nothing broke off the wall");
    assert!(
        s.pieces.iter().all(|p| !p.cells.contains(&(0, 39))),
        "the held base fell"
    );
}

#[test]
fn every_whole_cell_falls_or_stands_in_exactly_one_piece() {
    let s = shatter(pane_cracks(4.0), pane);
    let crack = cracked(&s);
    let mut owner = std::collections::BTreeMap::new();
    for (i, p) in s.pieces.iter().chain(&s.rest).enumerate() {
        for &c in &p.cells {
            assert!(owner.insert(c, i).is_none(), "{c:?} is in two pieces");
        }
    }
    assert!(!s.rest.is_empty(), "nothing was left standing");
    for y in -24..=23 {
        for x in -48..=47 {
            if pane(x, y) && !crack.contains(&(x, y)) {
                assert!(owner.contains_key(&(x, y)), "({x}, {y}) is in no piece");
            }
        }
    }
}
