//! The holy effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/holy/*.js`).

use pixfx_effects::effect;

#[test]
fn feather_burst_meta_matches_pixelfx() {
    let m = effect("holy/feather-burst").map(|e| *e.meta());
    assert!(m.is_some(), "holy/feather-burst is not registered");
    assert_eq!(m.map(|m| m.size), Some((160, 160)));
    assert_eq!(m.map(|m| m.origin), Some((80, 60)));
    assert_eq!(m.map(|m| m.duration), Some(3.0));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(4));
}
