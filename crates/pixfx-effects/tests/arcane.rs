//! The arcane effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/arcane/*.js`).

use pixfx_effects::effect;

#[test]
fn rune_circle_meta_matches_pixelfx() {
    let m = effect("arcane/rune-circle").map(|e| *e.meta());
    assert!(m.is_some(), "arcane/rune-circle is not registered");
    assert_eq!(m.map(|m| m.size), Some((136, 100)));
    assert_eq!(m.map(|m| m.origin), Some((68, 80)));
    assert_eq!(m.map(|m| m.duration), Some(4.0));
    assert_eq!(m.map(|m| m.looping), Some(true));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(8));
}
