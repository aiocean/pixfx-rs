//! The water effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/water/*.js`).

use pixfx_effects::effect;

#[test]
fn water_jet_meta_matches_pixelfx() {
    let m = effect("water/water-jet").map(|e| *e.meta());
    assert!(m.is_some(), "water/water-jet is not registered");
    assert_eq!(m.map(|m| m.size), Some((188, 80)));
    assert_eq!(m.map(|m| m.origin), Some((14, 32)));
    assert_eq!(m.map(|m| m.duration), Some(1.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(3));
}
