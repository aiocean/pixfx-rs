//! `feedback/burning` reads the sprite it wraps: on the tick (frame 0) its front layer paints every body cell white,
//! a flame stands on the crown, and between ticks (frame 4) most of the body is left bare; no front frame ever
//! paints the sprite's own colours (the body flash and the scorch are drawn over it, never copied from it).

use std::sync::Arc;

use pixfx_core::{Actor, Layer, Options, Sim};
use pixfx_draw::color::Rgba;
use pixfx_draw::frame::Frame;

const RED: Rgba = Rgba(200, 30, 40, 255);
const TEAL: Rgba = Rgba(20, 160, 150, 255);
const W: i32 = 16;
const H: i32 = 32;

/// A 16 x 32 sprite standing on the origin (x -8..7, y -32..-1): a red top half over a teal bottom half.
fn sprite() -> Result<Arc<Actor>, String> {
    let pixels = (0..H)
        .flat_map(|j| (0..W).map(move |_| if j < H / 2 { RED } else { TEAL }.pack()))
        .collect();
    Actor::new(-W / 2, -H, W as u32, H as u32, pixels)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

/// Frame `k`'s front layer alone on the effect's canvas, with the sprite wrapped, and the canvas origin.
fn front(k: u64) -> Result<(Frame, (i32, i32)), String> {
    let fx = pixfx_effects::effect("feedback/burning").ok_or("not registered")?;
    let canvas = fx.canvas(&Options::default());
    let opts = Options {
        x: f64::from(canvas.origin.0),
        y: f64::from(canvas.origin.1),
        ..Options::default()
    };
    let mut sim = Sim::new(fx, opts);
    sim.set_actor(Some(sprite()?));
    sim.seek(k);
    let mut frame = Frame::new(canvas.size.0, canvas.size.1, 0, 0);
    sim.draw(&mut frame, Layer::Front, sim.placement(), 1.0);
    Ok((frame, canvas.origin))
}

/// The colour painted at effect pixel (x, y), if any.
fn at(f: &Frame, origin: (i32, i32), x: i32, y: i32) -> Option<Rgba> {
    let (cx, cy) = (x + origin.0, y + origin.1);
    if cx < 0 || cy < 0 || cx >= f.w as i32 || cy >= f.h as i32 {
        return None;
    }
    let word = *f.color.get(cy as usize * f.w as usize + cx as usize)?;
    (word >> 24 != 0).then(|| Rgba::unpack(word))
}

fn body() -> impl Iterator<Item = (i32, i32)> {
    (-H..0).flat_map(|y| (-W / 2..W / 2).map(move |x| (x, y)))
}

fn frames() -> u64 {
    pixfx_effects::effect("feedback/burning").map_or(0, |fx| fx.meta().frames())
}

#[test]
fn b1_the_tick_paints_every_body_cell_in_a_colour_not_the_sprites() -> Result<(), String> {
    let (f, o) = front(0)?;
    for (x, y) in body() {
        let c = at(&f, o, x, y);
        assert!(c.is_some(), "body cell ({x}, {y}) unpainted on the tick");
        assert!(
            c != Some(RED) && c != Some(TEAL),
            "body cell ({x}, {y}) holds the sprite's own colour {c:?}"
        );
    }
    Ok(())
}

#[test]
fn b2_a_flame_stands_on_the_crown_on_the_tick() -> Result<(), String> {
    let (f, o) = front(0)?;
    // the hair spot is the top-left up cell (-8, -32): its three tongues stand in the columns round it, at full
    // flare on the tick at least 5 rows tall, 3 columns wide; a stray spark or ember is 1 px
    let above = (-H - 6..-H)
        .flat_map(|y| (-W / 2 - 3..=-W / 2 + 4).map(move |x| (x, y)))
        .filter(|&(x, y)| at(&f, o, x, y).is_some())
        .count();
    assert!(
        above >= 6,
        "{above} painted cells in the 6 rows over the crown round the hair spot: no flame stands there"
    );
    Ok(())
}

#[test]
fn b3_between_ticks_the_body_is_mostly_bare() -> Result<(), String> {
    let (f, o) = front(4)?;
    let bare = body().filter(|&(x, y)| at(&f, o, x, y).is_none()).count();
    let total = (W * H) as usize;
    assert!(
        bare * 4 >= total,
        "{bare} of {total} body cells bare at frame 4: the flash or the blend leaked past the tick"
    );
    Ok(())
}

#[test]
fn b4_no_front_frame_holds_the_sprites_own_colours() -> Result<(), String> {
    let n = frames();
    assert!(n > 0, "no frames");
    for k in 0..n {
        let (f, o) = front(k)?;
        for (x, y) in body() {
            let c = at(&f, o, x, y);
            assert!(
                c != Some(RED) && c != Some(TEAL),
                "frame {k}: body cell ({x}, {y}) holds the sprite's own colour {c:?}"
            );
        }
    }
    Ok(())
}
