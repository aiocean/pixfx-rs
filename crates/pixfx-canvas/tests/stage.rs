//! What the stage's own loop culls to: its canvas, in game pixels, grown by how far the shake can move it.

use std::error::Error;
use std::num::NonZeroU32;

use pixfx_canvas::{own_view, shake_reach, shake_strength};
use pixfx_core::{Feel, FeelSpec};
use pixfx_scene::{StageError, View};

#[test]
fn the_own_view_reaches_as_far_as_the_shake() -> Result<(), Box<dyn Error>> {
    let two = NonZeroU32::new(2).ok_or("scale 0")?;
    assert_eq!(
        own_view(320, 180, two, 16.0),
        View {
            x: -16.0,
            y: -16.0,
            w: 192.0,
            h: 122.0
        }
    );
    assert_eq!(
        own_view(320, 180, two, 0.0),
        View {
            x: 0.0,
            y: 0.0,
            w: 160.0,
            h: 90.0
        }
    );
    Ok(())
}

#[test]
fn the_own_loop_reaches_as_far_as_the_feel_shake_times_the_strength() {
    let feel = Feel::with_spec(FeelSpec {
        shake: 10.0,
        ..FeelSpec::default()
    });
    assert_eq!(
        shake_reach(Some(&feel), 2.5),
        25.0,
        "a stronger setting shakes further"
    );
    assert_eq!(shake_reach(Some(&feel), 0.0), 0.0, "shake turned off");
    assert_eq!(shake_reach(None, 2.5), 0.0, "the feel off: no shake");
}

#[test]
fn a_shake_strength_that_is_not_finite_is_refused() {
    assert_eq!(shake_strength(f64::INFINITY), Err(StageError::NotFinite));
    assert_eq!(shake_strength(f64::NAN), Err(StageError::NotFinite));
    assert_eq!(shake_strength(1.5), Ok(1.5));
    assert_eq!(
        shake_strength(-1.0),
        Ok(0.0),
        "below 0 holds the screen still"
    );
}
