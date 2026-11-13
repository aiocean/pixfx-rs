//! The nature effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/nature/*.js`).

use pixfx_effects::effect;

#[test]
fn poison_cloud_meta_matches_pixelfx() {
    let m = effect("nature/poison-cloud").map(|e| *e.meta());
    assert!(m.is_some(), "nature/poison-cloud is not registered");
    assert_eq!(m.map(|m| m.size), Some((256, 100)));
    assert_eq!(m.map(|m| m.origin), Some((128, 86)));
    assert_eq!(m.map(|m| m.duration), Some(4.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(3));
}
