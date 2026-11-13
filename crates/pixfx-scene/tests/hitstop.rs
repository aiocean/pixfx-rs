//! An effect marked to play through a hit-stop keeps its clock running while the rest of the scene freezes.

#[path = "common/hitter.rs"]
mod hitter;

use std::error::Error;
use std::num::NonZeroU32;

use hitter::HITTER;
use pixfx_core::{Options, Player};
use pixfx_draw::Frame;
use pixfx_effects::effect;
use pixfx_scene::Scene;

type Res = Result<(), Box<dyn Error>>;

const FRAME: f64 = 1.0 / 60.0;

fn candle() -> Result<Player, Box<dyn Error>> {
    let fx = effect("fire/candle").ok_or("fire/candle not registered")?;
    Ok(Player::live(fx, Options::default()))
}

fn scene() -> Result<Scene<Frame>, Box<dyn Error>> {
    Ok(Scene::new(NonZeroU32::new(1).ok_or("scale 0")?))
}

/// Play until the hitter's stop lands (at 0.1 s).
fn until_frozen(scene: &mut Scene<Frame>) -> Res {
    let mut played = 0.0;
    while !scene.feel().is_some_and(|f| f.frozen()) {
        played += scene.update(FRAME)?;
        if played >= 0.2 {
            return Err("the hit at 0.1 s never froze play".into());
        }
    }
    Ok(())
}

fn t(scene: &Scene<Frame>, h: u32) -> Result<f64, Box<dyn Error>> {
    scene.t(h).ok_or_else(|| format!("no instance {h}").into())
}

#[test]
fn a_marked_effect_plays_on_while_the_rest_freeze() -> Res {
    let mut scene = scene()?;
    let hitter = scene.play(Player::live(&HITTER, Options::default()), 0.0, 0.0)?;
    let candle = scene.play(candle()?, 0.0, 0.0)?;
    assert!(scene.set_plays_through_hitstop(candle, true));
    until_frozen(&mut scene)?;
    let (t_hitter, t_candle) = (t(&scene, hitter)?, t(&scene, candle)?);
    for _ in 0..5 {
        assert_eq!(
            scene.update(FRAME)?,
            0.0,
            "the game is still told to freeze"
        );
    }
    assert_eq!(t(&scene, hitter)?, t_hitter, "unmarked: frozen");
    assert!(
        (t(&scene, candle)? - (t_candle + 5.0 * FRAME)).abs() < 1e-9,
        "marked: plays real time"
    );
    Ok(())
}

#[test]
fn the_effect_that_fires_the_stop_can_play_through_it() -> Res {
    let mut scene = scene()?;
    let hitter = scene.play(Player::live(&HITTER, Options::default()), 0.0, 0.0)?;
    let candle = scene.play(candle()?, 0.0, 0.0)?;
    assert!(scene.set_plays_through_hitstop(hitter, true));
    until_frozen(&mut scene)?;
    let (t_hitter, t_candle) = (t(&scene, hitter)?, t(&scene, candle)?);
    for _ in 0..3 {
        scene.update(FRAME)?;
    }
    assert!(t(&scene, hitter)? > t_hitter, "the hit spark plays on");
    assert_eq!(t(&scene, candle)?, t_candle, "the rest holds");
    Ok(())
}

#[test]
fn slow_motion_slows_a_marked_effect_too() -> Res {
    let mut scene = scene()?;
    let candle = scene.play(candle()?, 0.0, 0.0)?;
    assert!(scene.set_plays_through_hitstop(candle, true));
    scene.update(FRAME)?;
    scene.feel_mut().ok_or("no feel")?.slow(1.0, 0.5);
    let before = t(&scene, candle)?;
    scene.update(FRAME)?;
    assert!((t(&scene, candle)? - (before + 0.5 * FRAME)).abs() < 1e-9);
    Ok(())
}

#[test]
fn unmarking_freezes_it_again_and_an_unknown_handle_is_false() -> Res {
    let mut scene = scene()?;
    scene.play(Player::live(&HITTER, Options::default()), 0.0, 0.0)?;
    let candle = scene.play(candle()?, 0.0, 0.0)?;
    assert!(scene.set_plays_through_hitstop(candle, true));
    assert!(scene.set_plays_through_hitstop(candle, false));
    assert!(!scene.set_plays_through_hitstop(999, true));
    until_frozen(&mut scene)?;
    let held = t(&scene, candle)?;
    scene.update(FRAME)?;
    assert_eq!(t(&scene, candle)?, held);
    Ok(())
}
