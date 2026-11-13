//! The projectile effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/projectiles/*.js`).

use pixfx_effects::effect;

#[test]
fn laser_bolt_meta_matches_pixelfx() {
    let m = effect("projectiles/laser-bolt").map(|e| *e.meta());
    assert!(m.is_some(), "projectiles/laser-bolt is not registered");
    assert_eq!(m.map(|m| m.size), Some((128, 32)));
    assert_eq!(m.map(|m| m.origin), Some((108, 16)));
    assert_eq!(m.map(|m| m.duration), Some(0.5));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(2));
}
