//! The spawn effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/spawn/*.js`).

use pixfx_effects::effect;

#[test]
fn poof_meta_matches_pixelfx() {
    let m = effect("spawn/poof").map(|e| *e.meta());
    assert!(m.is_some(), "spawn/poof is not registered");
    assert_eq!(m.map(|m| m.size), Some((112, 92)));
    assert_eq!(m.map(|m| m.origin), Some((56, 80)));
    assert_eq!(m.map(|m| m.duration), Some(1.6));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(21));
}
