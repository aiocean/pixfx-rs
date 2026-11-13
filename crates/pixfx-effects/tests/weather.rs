//! The weather effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/weather/*.js`).

use pixfx_effects::effect;

#[test]
fn snow_meta_matches_pixelfx() {
    let m = effect("weather/snow").map(|e| *e.meta());
    assert!(m.is_some(), "weather/snow is not registered");
    assert_eq!(m.map(|m| m.size), Some((320, 180)));
    assert_eq!(m.map(|m| m.origin), Some((160, 170)));
    assert_eq!(m.map(|m| m.duration), Some(12.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(6));
    assert_eq!(m.map(|m| m.is_screen()), Some(true));
}
