//! The pickup effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/pickups/*.js`).

use pixfx_effects::effect;

#[test]
fn coin_spin_meta_matches_pixelfx() {
    let m = effect("pickups/coin-spin").map(|e| *e.meta());
    assert!(m.is_some(), "pickups/coin-spin is not registered");
    assert_eq!(m.map(|m| m.size), Some((56, 56)));
    assert_eq!(m.map(|m| m.origin), Some((28, 48)));
    assert_eq!(m.map(|m| m.duration), Some(1.6));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(3));
}
