//! The ice effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/ice/*.js`).

use pixfx_effects::effect;

#[test]
fn icicle_fall_meta_matches_pixelfx() {
    let m = effect("ice/icicle-fall").map(|e| *e.meta());
    assert!(m.is_some(), "ice/icicle-fall is not registered");
    assert_eq!(m.map(|m| m.size), Some((152, 120)));
    assert_eq!(m.map(|m| m.origin), Some((76, 100)));
    assert_eq!(m.map(|m| m.duration), Some(2.6));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(4));
}
