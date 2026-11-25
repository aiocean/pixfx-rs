//! Ordering an effect that takes orders (`Scene::order`): a formation (a test effect, `common/formation.rs`)
//! answers with a volley of eight flying blades the scene launches, flies and strikes with, at any distance.

use std::error::Error;

#[path = "common/formation.rs"]
mod formation;

use pixfx_core::volley::{Aim, Order};
use pixfx_core::{Clip, Options};
use pixfx_draw::Frame;
use pixfx_draw::math::js_round;
use pixfx_effects::effect;
use pixfx_scene::{Body, Frames, InlinePainter, Scene, StageEvent};

type Res = Result<(), Box<dyn Error>>;

const STEP: f64 = 1.0 / 60.0;

fn scene() -> Scene<Frame> {
    let mut frames: Frames<Frame, InlinePainter> = Frames::default();
    frames.scene(std::num::NonZeroU32::MIN)
}

fn formation() -> Result<Clip, Box<dyn Error>> {
    Ok(Clip::live(&formation::FORMATION, Options::default()))
}

/// Run the scene for `secs`, returning every event with the scene time it came at.
fn run(
    s: &mut Scene<Frame>,
    from: f64,
    secs: f64,
) -> Result<Vec<(f64, StageEvent)>, Box<dyn Error>> {
    let mut out = Vec::new();
    let steps = js_round(secs / STEP) as u32;
    for i in 1..=steps {
        s.update(STEP)?;
        let t = from + f64::from(i) * STEP;
        out.extend(s.drain_events().into_iter().map(|e| (t, e)));
    }
    Ok(out)
}

fn order(x: f64, y: f64, aim: Aim) -> Order {
    Order {
        x,
        y,
        aim,
        caster: None,
    }
}

/// A body 20 by 40 game pixels round (`x`, `y`).
fn body(x: f64, y: f64) -> Body {
    Body {
        x: x - 10.0,
        y: y - 20.0,
        w: 20.0,
        h: 40.0,
    }
}

fn strikes(events: &[(f64, StageEvent)], by: u32) -> Vec<(f64, u32, f64, f64)> {
    events
        .iter()
        .filter_map(|(t, e)| match *e {
            StageEvent::Strike {
                handle,
                index,
                x,
                y,
                ..
            } if handle == by => Some((*t, index, x, y)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_converging_volley_strikes_a_far_body_eight_times_in_turn_then_the_formation_ends() -> Res {
    let mut s = scene();
    let h = s.play(formation()?, 100.0, 200.0)?;
    run(&mut s, 0.0, 1.5)?;
    // 400 pixels away: far past the formation's own canvas
    let target = (500.0, 170.0);
    s.set_body(1, Some(body(target.0, target.1)));
    assert!(s.order(h, order(target.0, target.1, Aim::Converge))?);
    // a second order changes nothing
    assert!(!s.order(h, order(0.0, 0.0, Aim::Pierce))?);
    let events = run(&mut s, 0.0, 4.0)?;
    let hits = strikes(&events, h);
    assert_eq!(hits.len(), 8, "{hits:?}");
    let mut indices: Vec<u32> = hits.iter().map(|h| h.1).collect();
    indices.sort_unstable();
    assert_eq!(indices, (0..8).collect::<Vec<_>>());
    for &(_, _, x, y) in &hits {
        assert!(
            (x - target.0).abs() <= 11.0 && (y - target.1).abs() <= 21.0,
            "struck at {x}, {y}"
        );
    }
    // launched a tenth of a second apart, so they strike in turn, never all at once
    let first = hits.iter().map(|h| h.0).fold(f64::INFINITY, f64::min);
    let last = hits.iter().map(|h| h.0).fold(0.0, f64::max);
    assert!(last - first >= 0.5, "strikes from {first} to {last}");
    let ended = events
        .iter()
        .find(|(_, e)| *e == StageEvent::End { handle: h })
        .map(|(t, _)| *t)
        .ok_or("the formation never ended")?;
    assert!(ended >= last);
    assert!(!s.handles().any(|x| x == h));
    Ok(())
}

#[test]
fn a_volley_names_what_it_launched_so_a_host_can_paint_it_where_the_formation_was() -> Res {
    let mut s = scene();
    let h = s.play(formation()?, 0.0, 0.0)?;
    let other = s.play(formation()?, 50.0, 0.0)?;
    run(&mut s, 0.0, 1.5)?;
    assert_eq!(s.launched(h).count(), 0);
    assert!(s.order(h, order(200.0, -20.0, Aim::Converge))?);
    run(&mut s, 0.0, 0.35)?;
    let launched: Vec<u32> = s.launched(h).collect();
    assert!(launched.len() >= 2, "{launched:?}");
    assert!(launched.iter().all(|&x| x != h && x != other));
    assert_eq!(s.launched(other).count(), 0);
    // every instance playing is the two formations or something the ordered one launched
    assert_eq!(s.handles().count(), 2 + launched.len());
    run(&mut s, 0.0, 4.0)?;
    assert_eq!(s.launched(h).count(), 0, "all spent");
    Ok(())
}

#[test]
fn a_piercing_volley_strikes_the_body_in_its_way_and_flies_on_through_it() -> Res {
    let mut s = scene();
    let h = s.play(formation()?, 100.0, 200.0)?;
    run(&mut s, 0.0, 1.5)?;
    s.set_body(1, Some(body(300.0, 180.0)));
    let before: Vec<u32> = s.handles().collect();
    assert!(s.order(h, order(300.0, 180.0, Aim::Pierce))?);
    let events = run(&mut s, 0.0, 1.0)?;
    let hits = strikes(&events, h);
    assert!(!hits.is_empty());
    for &(_, _, x, y) in &hits {
        assert!(
            (x - 300.0).abs() <= 11.0 && (y - 180.0).abs() <= 21.0,
            "struck at {x}, {y}"
        );
    }
    // a sword that struck is still in the air, past the target
    let flying: Vec<u32> = s.handles().filter(|x| !before.contains(x)).collect();
    let past = flying
        .iter()
        .filter_map(|&f| s.placement(f))
        .any(|p| p.x > 305.0);
    assert!(past, "no sword flew on past the target");
    Ok(())
}

#[test]
fn every_sword_hangs_where_it_was_from_the_order_on_so_the_ring_hands_over_without_a_gap() -> Res {
    let mut s = scene();
    let h = s.play(formation()?, 100.0, 200.0)?;
    run(&mut s, 0.0, 1.5)?;
    assert!(s.order(h, order(400.0, 200.0, Aim::Converge))?);
    // no step yet: all eight already stand in the ring's place
    assert_eq!(s.launched(h).count(), 8);
    for f in s.launched(h).collect::<Vec<_>>() {
        let p = s.placement(f).ok_or("a held sword has no placement")?;
        assert!(
            (p.x - 100.0).abs() < 30.0 && (p.y - 170.0).abs() < 30.0,
            "held at {}, {}",
            p.x,
            p.y
        );
    }
    Ok(())
}

#[test]
fn a_mirrored_formation_flies_mirrored_swords_whichever_way_they_go() -> Res {
    for flip in [false, true] {
        let mut s = scene();
        let h = s.play(formation()?, 300.0, 200.0)?;
        let place = s.placement(h).ok_or("no placement")?;
        s.set(h, pixfx_core::Placement { flip, ..place })?;
        run(&mut s, 0.0, 1.5)?;
        // the target on the other side for the mirrored one: a caster on the right striking left
        let x = if flip { 50.0 } else { 550.0 };
        assert!(s.order(h, order(x, 200.0, Aim::Pierce))?);
        let mut seen = 0;
        for _ in 0..60 {
            run(&mut s, 0.0, STEP)?;
            for f in s.launched(h).collect::<Vec<_>>() {
                if let Some(sword) = s.follows(f) {
                    // a trail draws the way it came, unturned
                    assert!(s.placement(sword).is_some(), "a trail follows no sword");
                    continue;
                }
                let p = s.placement(f).ok_or("no placement")?;
                assert_eq!(p.flip, flip, "a piece lost its caster's mirroring");
                seen += 1;
            }
        }
        assert!(seen > 0);
    }
    Ok(())
}

#[test]
fn a_volley_that_meets_nothing_strikes_nothing_and_flies_on_past_the_point() -> Res {
    let mut s = scene();
    let h = s.play(formation()?, 100.0, 200.0)?;
    run(&mut s, 0.0, 1.5)?;
    // a body off to the side, out of the way
    s.set_body(1, Some(body(300.0, 40.0)));
    assert!(s.order(h, order(300.0, 200.0, Aim::Converge))?);
    let mut furthest: f64 = 0.0;
    let mut events = Vec::new();
    for _ in 0..240 {
        events.extend(run(&mut s, 0.0, STEP)?);
        for f in s.launched(h).collect::<Vec<_>>() {
            if s.follows(f).is_none() {
                furthest = furthest.max(s.placement(f).map_or(0.0, |p| p.x));
            }
        }
    }
    assert!(strikes(&events, h).is_empty());
    assert!(furthest > 400.0, "the swords stopped at {furthest}");
    assert!(
        events.contains(&(
            events
                .iter()
                .find(|(_, e)| *e == StageEvent::End { handle: h })
                .map_or(0.0, |e| e.0),
            StageEvent::End { handle: h }
        ))
    );
    Ok(())
}

#[test]
fn a_volley_spares_its_caster_and_the_game_can_burst_a_piece_itself() -> Res {
    let mut s = scene();
    let h = s.play(formation()?, 100.0, 200.0)?;
    run(&mut s, 0.0, 1.5)?;
    // the caster's own body, round where the swords start
    s.set_body(
        7,
        Some(Body {
            x: 60.0,
            y: 140.0,
            w: 80.0,
            h: 80.0,
        }),
    );
    let o = Order {
        caster: Some(7),
        ..order(400.0, 200.0, Aim::Converge)
    };
    assert!(s.order(h, o)?);
    let mut events = run(&mut s, 0.0, 0.5)?;
    // held swords and trails are no pieces in flight: burst refuses them
    let mut burst = None;
    for f in s.launched(h).collect::<Vec<_>>() {
        let at = s.placement(f).ok_or("no placement")?;
        if s.burst(f)? {
            burst = Some((f, at));
            break;
        }
    }
    let (piece, at) = burst.ok_or("no sword in flight")?;
    assert!(!s.burst(piece)?);
    events.extend(run(&mut s, 0.0, STEP)?);
    let hits: Vec<&StageEvent> = events
        .iter()
        .map(|(_, e)| e)
        .filter(|e| matches!(e, StageEvent::Strike { .. }))
        .collect();
    assert_eq!(hits.len(), 1, "{hits:?}");
    if let StageEvent::Strike { x, body, .. } = *hits[0] {
        assert_eq!(body, None);
        assert!((x - at.x).abs() < 20.0);
    }
    assert!(s.placement(piece).is_none());
    Ok(())
}

#[test]
fn an_effect_that_takes_no_orders_refuses_one() -> Res {
    let mut s = scene();
    let fx = effect("feedback/power-up").ok_or("feedback/power-up is not built")?;
    let h = s.play(Clip::live(fx, Options::default()), 10.0, 10.0)?;
    assert!(!s.order(h, order(50.0, 10.0, Aim::Converge))?);
    assert!(!s.order(999, order(50.0, 10.0, Aim::Converge))?);
    Ok(())
}

#[test]
fn a_thrown_piece_lands_once_and_bursts_on_a_far_target() -> Res {
    let mut s = scene();
    let h = s.play(
        Clip::live(&formation::THROW, Options::default()),
        100.0,
        200.0,
    )?;
    run(&mut s, 0.0, 1.5)?;
    let target = (420.0, 178.0);
    assert!(s.order(h, order(target.0, target.1, Aim::Converge))?);
    let events = run(&mut s, 0.0, 3.0)?;
    let hits = strikes(&events, h);
    assert_eq!(hits.len(), 1, "{hits:?}");
    let (_, _, x, y) = hits[0];
    assert!(
        (x - target.0).abs() <= 3.0 && (y - target.1).abs() <= 3.0,
        "struck at {x}, {y}"
    );
    let shook = events
        .iter()
        .any(|(_, e)| matches!(e, StageEvent::Cue { .. }));
    assert!(shook, "the burst fired no cue");
    assert_eq!(
        s.handles().count(),
        0,
        "the thrower, its flight and its burst all ended"
    );
    Ok(())
}
