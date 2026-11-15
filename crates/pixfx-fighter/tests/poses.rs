// The pose sheets: a cell per pose, painted as the duel draws it, the blanked poses left empty, and a pose in every state.
use pixfx_fighter::look::Look;
use pixfx_fighter::paint::SPRITE;
use pixfx_fighter::pose::Pose;
use pixfx_fighter::poses::{MOMENTS, POSES, pose_cells, pose_named, state_cells, states};

fn painted(cell: &[u8]) -> usize {
    cell.chunks(4)
        .filter(|p| p.get(3).is_some_and(|&a| a > 0))
        .count()
}

#[test]
fn every_pose_is_painted_except_the_blanked_ones() {
    let blank = [Pose::Down, Pose::Launched];
    let cells = pose_cells(&Look::thunder(), &blank);
    assert_eq!(cells.len(), POSES.len());
    for (cell, &(name, pose)) in cells.iter().zip(&POSES) {
        assert_eq!(cell.len(), SPRITE * SPRITE * 4, "{name}");
        if blank.contains(&pose) {
            assert_eq!(painted(cell), 0, "{name} should be blank");
        } else {
            assert!(painted(cell) > 50, "{name} should be painted");
        }
    }
}

#[test]
fn poses_are_known_by_their_lowercase_names() {
    assert_eq!(pose_named("tumble"), Some(Pose::Tumble));
    assert_eq!(pose_named("Tumble"), None);
}

#[test]
fn alchemist_is_a_preset_painted_in_her_own_colours() {
    let look = Look::preset("alchemist");
    assert_eq!(look, Some(Look::alchemist()));
    let look = Look::alchemist();
    let cells = pose_cells(&look, &[]);
    let robe = [look.robe[0].0, look.robe[0].1, look.robe[0].2, 255];
    for (cell, &(name, _)) in cells.iter().zip(&POSES) {
        assert!(painted(cell) > 50, "{name} should be painted");
    }
    let idle = cells.first().map(Vec::as_slice).unwrap_or_default();
    assert!(
        idle.chunks(4).any(|p| p == robe),
        "idle shows her white dress"
    );
}

#[test]
fn a_state_sheet_has_one_painted_cell_per_state_and_moment() {
    let rows = states();
    assert_eq!(rows.len(), 16);
    let cells = state_cells(&Look::thunder(), Pose::Down, &rows);
    assert_eq!(cells.len(), rows.len() * MOMENTS.len());
    for (i, (cell, state)) in cells
        .iter()
        .zip(rows.iter().flat_map(|s| [s; MOMENTS.len()]))
        .enumerate()
    {
        assert_eq!(cell.len(), SPRITE * SPRITE * 4);
        let painted = cell
            .chunks(4)
            .filter(|p| p.get(3).is_some_and(|&a| a > 0))
            .count();
        assert!(
            painted > 50,
            "cell {i} ({}) has {painted} painted pixels",
            state.label()
        );
    }
}
