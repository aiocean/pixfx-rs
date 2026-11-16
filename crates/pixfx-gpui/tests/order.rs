//! An ordered volley in a gpui window: the shots it launched (and their trails) are named so a view paints them
//! where it paints the formation, and each strike on a body comes back as an event naming it.

use pixfx_core::Options;
use pixfx_core::volley::Aim;
use pixfx_gpui::FxScene;
use pixfx_scene::{Body, StageEvent};

#[test]
fn an_ordered_formation_launches_shots_a_view_can_paint_and_reports_each_strike()
-> Result<(), Box<dyn std::error::Error>> {
    let fx = pixfx_effects::effect("arcane/sword-formation").ok_or("no sword formation")?;
    let mut s = FxScene::default();
    let h = s.play(fx, Options::default())?;
    for _ in 0..90 {
        s.tick(1.0 / 60.0)?;
    }
    assert!(s.launched(h).is_empty());
    s.set_body(
        1,
        Some(Body {
            x: 290.0,
            y: -40.0,
            w: 20.0,
            h: 40.0,
        }),
    );
    assert!(s.order(h, 300.0, -20.0, Aim::Converge, None)?);
    let mut launched = 0;
    let mut strikes = 0;
    for _ in 0..240 {
        s.tick(1.0 / 60.0)?;
        launched = launched.max(s.launched(h).len());
        strikes += s
            .events()
            .iter()
            .filter(|e| matches!(e, StageEvent::Strike { body: Some(1), .. }))
            .count();
    }
    assert!(launched >= 2, "{launched}");
    assert_eq!(strikes, 8);
    assert!(s.done(h));
    assert!(s.launched(h).is_empty());
    Ok(())
}
