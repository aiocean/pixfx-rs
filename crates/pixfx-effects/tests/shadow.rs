//! The shadow effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/shadow/*.js`).

use pixfx_effects::effect;

#[test]
fn void_orb_meta_matches_pixelfx() {
    let m = effect("shadow/void-orb").map(|e| *e.meta());
    assert!(m.is_some(), "shadow/void-orb is not registered");
    assert_eq!(m.map(|m| m.size), Some((168, 160)));
    assert_eq!(m.map(|m| m.origin), Some((84, 150)));
    assert_eq!(m.map(|m| m.duration), Some(3.2));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(5));
}
