//! The critters are registered under their pixelfx ids with pixelfx's meta (`src/fx/critters/*.js`).

use pixfx_effects::effect;

#[test]
fn wisp_meta_matches_pixelfx() {
    let m = effect("critters/wisp").map(|e| *e.meta());
    assert!(m.is_some(), "critters/wisp is not registered");
    assert_eq!(m.map(|m| m.size), Some((128, 104)));
    assert_eq!(m.map(|m| m.origin), Some((64, 96)));
    assert_eq!(m.map(|m| m.duration), Some(3.2));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(1));
}
