//! The fire effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/fire/*.js`).

use pixfx_effects::effect;

#[test]
fn candle_meta_matches_pixelfx() {
    let m = effect("fire/candle").map(|e| *e.meta());
    assert!(m.is_some(), "fire/candle is not registered");
    assert_eq!(m.map(|m| m.size), Some((56, 76)));
    assert_eq!(m.map(|m| m.origin), Some((28, 64)));
    assert_eq!(m.map(|m| m.duration), Some(4.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
}

#[test]
fn campfire_and_brazier_are_registered() {
    let size = |id| effect(id).map(|e| e.meta().size);
    assert_eq!(size("fire/campfire"), Some((128, 128)));
    assert_eq!(size("fire/brazier"), Some((112, 148)));
}
