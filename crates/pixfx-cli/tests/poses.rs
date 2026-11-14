// `pixfx poses`: a sheet cell per pose, painted as the duel draws it, and the blanked poses left empty.
use pixfx_cli::poses::{POSES, pose_cells, pose_named};
use pixfx_fighter::look::Look;
use pixfx_fighter::paint::SPRITE;
use pixfx_fighter::pose::Pose;

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
