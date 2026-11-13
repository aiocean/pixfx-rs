//! What the stage's own loop culls to: its canvas, in game pixels, grown by how far the shake can move it.

use std::error::Error;
use std::num::NonZeroU32;

use pixfx_canvas::own_view;
use pixfx_scene::View;

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
