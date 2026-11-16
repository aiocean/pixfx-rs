//! The default sheet holds the game's ruler figure.

use pixfx_pose::{Pose, facing, skeleton};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn the_default_figure_on_the_default_sheet_is_the_duels_fighter() {
    let r = pixfx_draw::RULER;
    let sheet = pixfx_pose::PoseSheet::default();
    assert_eq!(sheet.cell, (r.cell, r.cell));
    let (cw, ch) = (f64::from(sheet.cell.0), f64::from(sheet.cell.1));
    let ground = ch - f64::from(sheet.ground);
    let sk = skeleton(&sheet.body, &Pose::default(), facing::RIGHT).placed(cw, ground, 0.0);
    let [_, top, _, bottom] = sk.body_bounds();
    // the soles on the fighter's feet row (its bottom edge), the painted body's height, the face's width
    assert!(close(bottom, f64::from(r.feet.1 + 1)), "soles: {bottom}");
    assert!(
        close(bottom - top, f64::from(r.figure())),
        "height: {}",
        bottom - top
    );
    assert!(
        close(2.0 * sheet.body.head, f64::from(r.head)),
        "head: {}",
        sheet.body.head
    );
    assert!(close(sheet.body.sword, f64::from(r.blade)));
}
