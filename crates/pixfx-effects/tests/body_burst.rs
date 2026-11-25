//! `explosions/body-burst` takes apart the sprite it plays on: it draws the body in the sprite's own pixels, blows
//! it out in big chunks that crack down to pixels, lets them heap up on the floor in the sprite's colours, and ends with nothing left.

use std::sync::Arc;

use pixfx_core::{Actor, Options, Rendered, render_frames_with_actor};
use pixfx_draw::PixelDensity;
use pixfx_draw::color::Rgba;

const RED: Rgba = Rgba(200, 30, 40, 255);
const TEAL: Rgba = Rgba(20, 160, 150, 255);

/// A 12 x 20 sprite standing on the origin: a red top half over a teal bottom half.
fn sprite() -> Result<Arc<Actor>, String> {
    let (w, h) = (12u32, 20u32);
    let pixels = (0..h)
        .flat_map(|j| (0..w).map(move |_| if j < h / 2 { RED } else { TEAL }.pack()))
        .collect();
    Actor::new(-6, -20, w, h, pixels)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

fn render() -> Result<Rendered, String> {
    let fx = pixfx_effects::effect("explosions/body-burst").ok_or("not registered")?;
    let actor = sprite()?;
    let options = Options {
        pixel_density: PixelDensity::One,
        ..Options::default()
    };
    Ok(render_frames_with_actor(fx, &options, 0, |_| {
        Some(actor.clone())
    }))
}

/// The colour drawn at effect pixel (x, y) on frame `k`, if any.
fn at(r: &Rendered, k: usize, x: i32, y: i32) -> Option<Rgba> {
    let f = r.frames.get(k)?;
    let (cx, cy) = (x + r.origin.0, y + r.origin.1);
    if cx < 0 || cy < 0 || cx >= f.w as i32 || cy >= f.h as i32 {
        return None;
    }
    let word = *f.color.get(cy as usize * f.w as usize + cx as usize)?;
    (word >> 24 != 0).then(|| Rgba::unpack(word))
}

/// Every drawn colour pixel of frame `k`: its effect-space cell and colour.
fn drawn(r: &Rendered, k: usize) -> Vec<(i32, i32, Rgba)> {
    let Some(f) = r.frames.get(k) else {
        return Vec::new();
    };
    (0..f.h as i32)
        .flat_map(|cy| (0..f.w as i32).map(move |cx| (cx, cy)))
        .filter_map(|(cx, cy)| {
            let (x, y) = (cx - r.origin.0, cy - r.origin.1);
            at(r, k, x, y).map(|c| (x, y, c))
        })
        .collect()
}

#[test]
fn the_first_frame_is_the_sprite_itself() -> Result<(), String> {
    let r = render()?;
    for y in -20..0 {
        for x in -6..6 {
            let want = if y < -10 { RED } else { TEAL };
            assert_eq!(at(&r, 0, x, y), Some(want), "pixel ({x}, {y})");
        }
    }
    assert_eq!(drawn(&r, 0).len(), 12 * 20, "nothing beyond the sprite");
    Ok(())
}

#[test]
fn the_pieces_fly_out_past_the_body() -> Result<(), String> {
    let r = render()?;
    // 0.35 s in: the pieces are in the air, well beyond the sprite's 12 px width
    let k = (0.35 * f64::from(r.fps)) as usize;
    let px = drawn(&r, k);
    let left = px.iter().map(|p| p.0).min().ok_or("nothing drawn")?;
    let right = px.iter().map(|p| p.0).max().ok_or("nothing drawn")?;
    assert!(left < -20 && right > 20, "spread {left}..{right}");
    let still = (-20..0)
        .flat_map(|y| (-6..6).map(move |x| (x, y)))
        .filter(|&(x, y)| at(&r, k, x, y).is_some())
        .count();
    // the default `dust` share (0.3) crumbles where it stands
    assert!(still < 12 * 20 / 2, "{still} pixels still on the body");
    Ok(())
}

#[test]
fn the_pixels_heap_up_on_the_floor_in_the_sprites_colours() -> Result<(), String> {
    let r = render()?;
    let (a, b) = (
        (1.8 * f64::from(r.fps)) as usize,
        (1.95 * f64::from(r.fps)) as usize,
    );
    let heap = drawn(&r, a);
    assert!(heap.len() > 12 * 20 / 2, "only {} pixels left", heap.len());
    assert!(
        heap.iter()
            .all(|&(_, y, c)| (-12..0).contains(&y) && (c == RED || c == TEAL)),
        "every pixel lies low on the floor, in a colour of the sprite"
    );
    // at rest: what lies there does not move, it only crumbles away
    let later = drawn(&r, b);
    let moved: Vec<_> = later.iter().filter(|p| !heap.contains(p)).collect();
    assert!(moved.is_empty(), "moved {moved:?}");
    Ok(())
}

#[test]
fn nothing_is_left_at_the_end() -> Result<(), String> {
    let r = render()?;
    let last = r.frames.len().checked_sub(1).ok_or("no frames")?;
    assert_eq!(drawn(&r, last), Vec::new());
    Ok(())
}

/// The pixels in flight on frame `k` (drawn there and not on the next frame) grouped into pieces that touch side by
/// side: each piece's size, largest first.
fn pieces(r: &Rendered, k: usize) -> Vec<usize> {
    let next: std::collections::HashSet<(i32, i32, Rgba)> = drawn(r, k + 1).into_iter().collect();
    let mut left: std::collections::HashSet<(i32, i32)> = drawn(r, k)
        .into_iter()
        .filter(|p| !next.contains(p))
        .map(|(x, y, _)| (x, y))
        .collect();
    let mut sizes = Vec::new();
    while let Some(&start) = left.iter().next() {
        left.remove(&start);
        let mut todo = vec![start];
        let mut n = 0;
        while let Some((x, y)) = todo.pop() {
            n += 1;
            for next in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if left.remove(&next) {
                    todo.push(next);
                }
            }
        }
        sizes.push(n);
    }
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    sizes
}

#[test]
fn the_body_breaks_into_big_chunks_before_it_crumbles_to_pixels() -> Result<(), String> {
    let r = render()?;
    let fps = f64::from(r.fps);
    // just after the burst: a few big chunks of the sprite fly, not a spray of single pixels
    let early = pieces(&r, (0.2 * fps) as usize);
    let big = early.iter().filter(|&&n| n >= 12).count();
    assert!(big >= 3, "big chunks at 0.2 s: {early:?}");
    assert!(early.len() <= 30, "{} pieces at 0.2 s", early.len());
    // the chunks keep breaking: later what flies is in more, smaller pieces
    let later = pieces(&r, (0.45 * fps) as usize);
    let (Some(&first), Some(&then)) = (early.first(), later.first()) else {
        return Err("nothing drawn".into());
    };
    assert!(
        later.len() > early.len(),
        "{} pieces at 0.45 s, {} at 0.2 s",
        later.len(),
        early.len()
    );
    assert!(
        then < first,
        "largest piece {first} at 0.2 s, {then} at 0.45 s"
    );
    Ok(())
}
