//! The explosion effects are registered under their pixelfx ids with pixelfx's meta (`src/fx/explosions/*.js`).

use pixfx_core::Meta;
use pixfx_effects::effect;

fn meta(id: &str) -> Option<Meta> {
    effect(id).map(|e| *e.meta())
}

#[test]
fn dirt_explosion_meta_matches_pixelfx() {
    let m = meta("explosions/dirt-explosion");
    assert!(m.is_some(), "explosions/dirt-explosion is not registered");
    assert_eq!(m.map(|m| m.size), Some((260, 240)));
    assert_eq!(m.map(|m| m.origin), Some((130, 206)));
    assert_eq!(m.map(|m| m.duration), Some(3.6));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(3));
}

#[test]
fn small_explosion_meta_matches_pixelfx() {
    let m = meta("explosions/small-explosion");
    assert!(m.is_some(), "explosions/small-explosion is not registered");
    assert_eq!(m.map(|m| m.size), Some((300, 240)));
    assert_eq!(m.map(|m| m.origin), Some((150, 196)));
    assert_eq!(m.map(|m| m.duration), Some(5.4));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(7));
}

#[test]
fn implosion_meta_matches_pixelfx() {
    let m = meta("explosions/implosion");
    assert!(m.is_some(), "explosions/implosion is not registered");
    assert_eq!(m.map(|m| m.size), Some((208, 184)));
    assert_eq!(m.map(|m| m.origin), Some((104, 92)));
    assert_eq!(m.map(|m| m.duration), Some(1.65));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(2));
}

#[test]
fn plasma_explosion_meta_matches_pixelfx() {
    let m = meta("explosions/plasma-explosion");
    assert!(m.is_some(), "explosions/plasma-explosion is not registered");
    assert_eq!(m.map(|m| m.size), Some((224, 200)));
    assert_eq!(m.map(|m| m.origin), Some((112, 100)));
    assert_eq!(m.map(|m| m.duration), Some(1.7));
    assert_eq!(m.map(|m| m.looping), Some(false));
    assert_eq!(m.map(|m| m.fps), Some(30));
    assert_eq!(m.map(|m| m.seed), Some(7));
}
