//! An actor as the host shows it: stretched and turned the way the sprite is drawn.

use pixfx_core::{Actor, ActorError};

fn two_by_one() -> Result<Actor, ActorError> {
    Actor::new(-1, -1, 2, 1, vec![1, 2])
}

#[test]
fn a_sprite_drawn_twice_its_size_doubles_every_pixel() -> Result<(), ActorError> {
    let a = two_by_one()?.shown(4, 2, false);
    assert_eq!((a.x(), a.y(), a.w(), a.h()), (-1, -1, 4, 2));
    assert_eq!(a.pixels(), &[1, 1, 2, 2, 1, 1, 2, 2]);
    Ok(())
}

#[test]
fn a_sprite_drawn_flipped_mirrors_in_its_own_box() -> Result<(), ActorError> {
    let a = two_by_one()?.shown(2, 1, true);
    assert_eq!((a.x(), a.pixels()), (-1, &[2, 1][..]));
    Ok(())
}

#[test]
fn rgba_bytes_that_do_not_fill_the_size_are_refused() {
    assert!(Actor::from_rgba(0, 0, 2, 2, &[0; 12]).is_err());
    assert!(Actor::from_rgba(0, 0, 2, 2, &[0; 15]).is_err());
}
