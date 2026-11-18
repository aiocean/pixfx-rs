//! `feedback/petrified` reads the sprite it wraps and opens once before it loops: on frame 0 the stone has only
//! begun at the feet, by the held statue (frame 12) every body cell is grey, the opening is 0.75 s of 28 frames and
//! the held loop still moves (the glint), and with `creep = 0` the statue is set from frame 0. A live clock plays
//! the opening once and then loops the hold.

use std::sync::Arc;

use pixfx_core::{Actor, Instance, Layer, OptValue, Options, Sim};
use pixfx_draw::color::Rgba;
use pixfx_draw::frame::Frame;
use pixfx_draw::frame_hash;

const RED: Rgba = Rgba(200, 30, 40, 255);
const TEAL: Rgba = Rgba(20, 160, 150, 255);
const W: i32 = 16;
const H: i32 = 32;
const ID: &str = "feedback/petrified";

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

fn options(creep: Option<f64>) -> Result<Options, String> {
    let fx = pixfx_effects::effect(ID).ok_or("not registered")?;
    let canvas = fx.canvas(&Options::default());
    let mut opts = Options {
        x: f64::from(canvas.origin.0),
        y: f64::from(canvas.origin.1),
        ..Options::default()
    };
    if let Some(c) = creep {
        opts.extra.insert("creep".into(), OptValue::Num(c));
    }
    Ok(opts)
}

/// Frame `k`'s layer alone on the effect's canvas, with the sprite wrapped, and the canvas origin.
fn render(k: u64, layer: Layer, creep: Option<f64>) -> Result<(Frame, (i32, i32)), String> {
    let fx = pixfx_effects::effect(ID).ok_or("not registered")?;
    let canvas = fx.canvas(&Options::default());
    let mut sim = Sim::new(fx, options(creep)?);
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

#[test]
fn q1_on_frame_0_the_stone_has_only_begun_at_the_feet() -> Result<(), String> {
    let (f, o) = render(0, Layer::Front, None)?;
    let high: Vec<_> = painted(&f, o).into_iter().filter(|p| p.1 < -4).collect();
    assert!(high.is_empty(), "painted above y = -4 on frame 0: {high:?}");
    Ok(())
}

#[test]
fn q2_the_held_statue_covers_every_body_cell_in_stone() -> Result<(), String> {
    let (f, o) = render(12, Layer::Front, None)?;
    for (x, y) in body() {
        let c = at(&f, o, x, y);
        assert!(c.is_some(), "body cell ({x}, {y}) unpainted on frame 12");
        assert!(
            c != Some(RED) && c != Some(TEAL),
            "body cell ({x}, {y}) holds the sprite's own colour {c:?}"
        );
    }
    Ok(())
}

#[test]
fn q3_the_opening_is_three_quarters_of_a_second_and_the_hold_still_moves() -> Result<(), String> {
    let fx = pixfx_effects::effect(ID).ok_or("not registered")?;
    assert_eq!(fx.loop_from(), Some(0.75));
    assert_eq!(fx.meta().frames(), 28);
    let (a, _) = render(12, Layer::Front, None)?;
    let (b, _) = render(18, Layer::Front, None)?;
    assert_ne!(
        frame_hash(&a),
        frame_hash(&b),
        "frames 12 and 18 draw the same pixels"
    );
    Ok(())
}

#[test]
fn q4_with_no_creep_the_statue_is_set_from_frame_0() -> Result<(), String> {
    let (f, o) = render(0, Layer::Front, Some(0.0))?;
    for (x, y) in body() {
        assert!(
            at(&f, o, x, y).is_some(),
            "body cell ({x}, {y}) unpainted on frame 0 with creep = 0"
        );
    }
    Ok(())
}

/// The live check of the first `loop_from` effect: a real clock driven for 5 s at 60 Hz plays frames 0..11 once,
/// then cycles 12..27; and what it draws on its third time round the loop is the frame the render gives.
#[test]
fn q5_a_live_clock_plays_the_opening_once_and_loops_the_hold() -> Result<(), String> {
    let fx = pixfx_effects::effect(ID).ok_or("not registered")?;
    let canvas = fx.canvas(&Options::default());
    let mut inst = Instance::new(fx, options(None)?);
    inst.set_actor(Some(sprite()?));
    let mut shown = vec![inst.frame()];
    let mut live: Vec<(u64, u64)> = Vec::new();
    for step in 0..300 {
        inst.update(1.0 / 60.0);
        let k = inst.frame();
        if shown.last() != Some(&k) {
            shown.push(k);
        }
        if step >= 240 {
            let mut frame = Frame::new(canvas.size.0, canvas.size.1, 0, 0);
            inst.draw(&mut frame, Layer::Both, inst.placement());
            live.push((k, frame_hash(&frame)));
        }
    }
    let opening: Vec<u64> = (0..12).collect();
    assert_eq!(
        &shown[..12],
        &opening[..],
        "the opening plays first: {shown:?}"
    );
    assert!(
        shown[12..].iter().all(|&k| (12..28).contains(&k)),
        "a frame of the opening came back: {shown:?}"
    );
    let laps = shown[12..].iter().filter(|&&k| k == 12).count();
    assert!(laps >= 3, "{laps} times round the hold in 5 s: {shown:?}");
    for (k, hash) in live {
        let mut sim = Sim::new(fx, options(None)?);
        sim.set_actor(Some(sprite()?));
        sim.seek(k);
        let mut frame = Frame::new(canvas.size.0, canvas.size.1, 0, 0);
        sim.draw(&mut frame, Layer::Both, sim.placement(), 1.0);
        assert_eq!(
            hash,
            frame_hash(&frame),
            "live frame {k} differs from the render"
        );
    }
    Ok(())
}
