//! `feedback/frozen` reads the sprite it wraps: its front layer covers every body cell with the chilled body on
//! every frame, an icicle hangs under each arm, frost ferns reach white tips inside the body between shivers, and
//! its back layer lays a frost crust on the floor row.

use std::sync::Arc;

use pixfx_core::{Actor, Layer, Options, Sim};
use pixfx_draw::color::{Rgba, WHITE};
use pixfx_draw::frame::Frame;

const RED: Rgba = Rgba(200, 30, 40, 255);
const TEAL: Rgba = Rgba(20, 160, 150, 255);
const W: i32 = 16;
const H: i32 = 32;

/// Whether the sprite with arms covers cell (x, y): a trunk x -4..3 for y -24..-1, a head x -3..2 for y -32..-25,
/// and two arms x -8..-5 and 4..7 for y -24..-18, so there is empty space under each arm.
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
    let fx = pixfx_effects::effect("feedback/frozen").ok_or("not registered")?;
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

fn body() -> impl Iterator<Item = (i32, i32)> {
    (-H..0)
        .flat_map(|y| (-W / 2..W / 2).map(move |x| (x, y)))
        .filter(|&(x, y)| covers(x, y))
}

fn frames() -> u64 {
    pixfx_effects::effect("feedback/frozen").map_or(0, |fx| fx.meta().frames())
}

#[test]
fn f1_the_chilled_body_covers_every_body_cell_on_every_frame() -> Result<(), String> {
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
fn f2_an_icicle_hangs_under_each_arm() -> Result<(), String> {
    // frame 4: the first frame after the shiver, where the overlay does not jolt sideways
    let (f, o) = render(4, Layer::Front)?;
    for (x, y) in [(-5, -17), (4, -17)] {
        assert!(
            at(&f, o, x, y).is_some(),
            "no icicle at ({x}, {y}) under the arm"
        );
    }
    Ok(())
}

#[test]
fn f3_fern_tips_reach_white_inside_the_body_between_shivers() -> Result<(), String> {
    let (f, o) = render(8, Layer::Front)?;
    let inside = |x: i32, y: i32| {
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .all(|&(dx, dy)| covers(x + dx, y + dy))
    };
    let white = body()
        .filter(|&(x, y)| inside(x, y) && at(&f, o, x, y) == Some(WHITE))
        .count();
    assert!(
        white >= 3,
        "{white} white cells strictly inside the body at frame 8"
    );
    Ok(())
}

#[test]
fn f4_a_frost_crust_lies_on_the_floor_row() -> Result<(), String> {
    let (f, o) = render(0, Layer::Back)?;
    let crust = (-32..32).filter(|&x| at(&f, o, x, 0).is_some()).count();
    assert!(crust >= 20, "{crust} painted cells on row 0");
    Ok(())
}
