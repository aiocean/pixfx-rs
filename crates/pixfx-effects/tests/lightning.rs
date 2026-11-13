//! The lightning effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/lightning/*.js`).

use pixfx_effects::effect;

#[test]
fn ball_lightning_meta_matches_pixelfx() {
    let m = effect("lightning/ball-lightning").map(|e| *e.meta());
    assert!(m.is_some(), "lightning/ball-lightning is not registered");
    assert_eq!(m.map(|m| m.size), Some((136, 104)));
    assert_eq!(m.map(|m| m.origin), Some((68, 92)));
    assert_eq!(m.map(|m| m.duration), Some(4.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(9));
}
