//! The impact effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/impacts/*.js`).

use pixfx_effects::effect;

#[test]
fn parry_meta_matches_pixelfx() {
    let m = effect("impacts/parry").map(|e| *e.meta());
    assert!(m.is_some(), "impacts/parry is not registered");
    assert_eq!(m.map(|m| m.size), Some((200, 112)));
    assert_eq!(m.map(|m| m.origin), Some((100, 96)));
    assert_eq!(m.map(|m| m.duration), Some(1.0));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(11));
}
