//! `feedback/poisoned` reads the sprite it wraps: its front layer tints every body cell on every frame, the wound
//! sits one in from the right edge of the row at 60 % height, veins in their own colour creep out of it, and the far
//! half of the floor ring lies on the back layer above the feet.

use std::sync::Arc;

use pixfx_core::{Actor, Layer, Options, Sim};
use pixfx_draw::color::Rgba;
use pixfx_draw::frame::Frame;

const RED: Rgba = Rgba(200, 30, 40, 255);
const TEAL: Rgba = Rgba(20, 160, 150, 255);
const W: i32 = 16;
const H: i32 = 32;
/// The default `veins_color`.
const VEINS: Rgba = Rgba::hex24(0x2c1a3a);
/// The palette's seventh step, the wound's colour.
const WOUND: Rgba = Rgba::hex24(0xcfff63);

/// Whether the sprite with arms covers cell (x, y): a trunk x -4..3 for y -24..-1, a head x -3..2 for y -32..-25,
/// and two arms x -8..-5 and 4..7 for y -24..-18.
fn covers(x: i32, y: i32) -> bool {
    let trunk = (-4..=3).contains(&x) && (-24..=-1).contains(&y);
    let head = (-3..=2).contains(&x) && (-32..=-25).contains(&y);
    let arms = ((-8..=-5).contains(&x) || (4..=7).contains(&x)) && (-24..=-18).contains(&y);
    trunk || head || arms
}

/// The sprite with arms in a 16 x 32 box standing on the origin: a red top half over a teal bottom half.
fn sprite() -> Result<Arc<Actor>, String> {
    let pixels = (0..H)
        .flat_map(|j| {
            (0..W).map(move |i| {
                let (x, y) = (i - W / 2, j - H);
                if !covers(x, y) {
                    0
                } else if y < -H / 2 {
                    RED.pack()
                } else {
                    TEAL.pack()
                }
            })
        })
        .collect();
    Actor::new(-W / 2, -H, W as u32, H as u32, pixels)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

/// Frame `k`'s layer alone on the effect's canvas, with the sprite wrapped, and the canvas origin.
fn render(k: u64, layer: Layer) -> Result<(Frame, (i32, i32)), String> {
    let fx = pixfx_effects::effect("feedback/poisoned").ok_or("not registered")?;
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
    sim.draw(&mut frame, layer, sim.placement(), 1.0);
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

/// Every painted cell of the frame, in effect space.
fn painted(f: &Frame, origin: (i32, i32)) -> Vec<(i32, i32, Rgba)> {
    (0..f.h as i32)
        .flat_map(|cy| (0..f.w as i32).map(move |cx| (cx - origin.0, cy - origin.1)))
        .filter_map(|(x, y)| at(f, origin, x, y).map(|c| (x, y, c)))
        .collect()
}

fn body() -> impl Iterator<Item = (i32, i32)> {
    (-H..0)
        .flat_map(|y| (-W / 2..W / 2).map(move |x| (x, y)))
        .filter(|&(x, y)| covers(x, y))
}

fn frames() -> u64 {
    pixfx_effects::effect("feedback/poisoned").map_or(0, |fx| fx.meta().frames())
}

#[test]
fn p1_the_tinted_body_covers_every_body_cell_on_every_frame() -> Result<(), String> {
    let n = frames();
    assert!(n > 0, "no frames");
    for k in 0..n {
        let (f, o) = render(k, Layer::Front)?;
        for (x, y) in body() {
            assert!(
                at(&f, o, x, y).is_some(),
                "frame {k}: body cell ({x}, {y}) unpainted"
            );
        }
    }
    Ok(())
}

#[test]
fn p2_the_wound_sits_one_in_from_the_right_edge_at_sixty_percent_height() -> Result<(), String> {
    let (f, o) = render(0, Layer::Front)?;
    assert_eq!(at(&f, o, 2, -13), Some(WOUND), "the wound at (2, -13)");
    Ok(())
}

#[test]
fn p3_veins_in_their_own_colour_creep_out_of_the_wound() -> Result<(), String> {
    let (f, o) = render(6, Layer::Front)?;
    let veins = painted(&f, o).iter().filter(|p| p.2 == VEINS).count();
    assert!(veins >= 4, "{veins} cells in the veins' colour at frame 6");
    Ok(())
}

#[test]
fn p4_the_far_half_of_the_ring_lies_behind_the_feet() -> Result<(), String> {
    let (f, o) = render(2, Layer::Back)?;
    let cells = painted(&f, o);
    let far = cells
        .iter()
        .filter(|&&(x, y, _)| y < 0 && x.abs() <= 16)
        .count();
    assert!(far >= 6, "{far} far-ring cells at frame 2");
    let wide: Vec<_> = cells.iter().filter(|&&(x, _, _)| x.abs() > 16).collect();
    assert!(wide.is_empty(), "painted past the ring's radius: {wide:?}");
    Ok(())
}

/// Frame `k`'s front layer with `veins_color` set to `vein`.
fn front_with_veins(k: u64, vein: Rgba) -> Result<(Frame, (i32, i32)), String> {
    use pixfx_core::OptValue;
    let fx = pixfx_effects::effect("feedback/poisoned").ok_or("not registered")?;
    let canvas = fx.canvas(&Options::default());
    let mut opts = Options {
        x: f64::from(canvas.origin.0),
        y: f64::from(canvas.origin.1),
        ..Options::default()
    };
    opts.extra
        .insert("veins_color".into(), OptValue::Color(vein));
    let mut sim = Sim::new(fx, opts);
    sim.set_actor(Some(sprite()?));
    sim.seek(k);
    let mut frame = Frame::new(canvas.size.0, canvas.size.1, 0, 0);
    sim.draw(&mut frame, Layer::Front, sim.placement(), 1.0);
    Ok((frame, canvas.origin))
}

#[test]
fn p5_veins_color_recolours_the_veins_and_nothing_else() -> Result<(), String> {
    const OTHER: Rgba = Rgba(250, 10, 200, 255);
    let (a, o) = render(6, Layer::Front)?;
    let (b, _) = front_with_veins(6, OTHER)?;
    let before = painted(&a, o);
    let after = painted(&b, o);
    assert_eq!(before.len(), after.len(), "the same cells are painted");
    let mut changed = 0;
    for (p, q) in before.iter().zip(&after) {
        if p.2 == VEINS {
            assert_eq!(q.2, OTHER, "vein cell ({}, {}) recoloured", p.0, p.1);
            changed += 1;
        } else {
            assert_eq!(p, q, "a cell off the veins changed");
        }
    }
    assert!(changed >= 4, "{changed} vein cells recoloured");
    Ok(())
}
