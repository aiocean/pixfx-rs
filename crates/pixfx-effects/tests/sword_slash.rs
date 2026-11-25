//! `melee/sword-slash` sweeps a blade's smear through an overhead cut on the character it plays on: an arc outside
//! the sprite on its facing side, read from the sprite's shoulder, nothing before the swing or after the smear, and
//! a light blow when the blade arrives.

use std::sync::Arc;

use pixfx_core::cue::Cue;
use pixfx_core::effect::OptValue;
use pixfx_core::{Actor, Effect, Options, Rendered, render_frames_with_actor};
use pixfx_draw::color::Rgba;
use pixfx_draw::{PixelDensity, RULER};

const RED: Rgba = Rgba(200, 30, 40, 255);
const TEAL: Rgba = Rgba(20, 160, 150, 255);

/// The frame the swing ends on: `SWING` (0.1 s) at 60 fps.
const SWING_FRAME: usize = 6;

/// A 12 x 20 sprite standing on the origin: a red top half over a teal bottom half. Its centroid is (0, -10), so
/// the effect's pivot is one head above it.
fn sprite() -> Result<Arc<Actor>, String> {
    let (w, h) = (12u32, 20u32);
    let pixels = (0..h)
        .flat_map(|j| (0..w).map(move |_| if j < h / 2 { RED } else { TEAL }.pack()))
        .collect();
    Actor::new(-6, -20, w, h, pixels)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

fn effect() -> Result<&'static dyn Effect, String> {
    pixfx_effects::effect("melee/sword-slash").ok_or_else(|| "not registered".to_string())
}

fn options(pixel_density: PixelDensity) -> Options {
    Options {
        pixel_density,
        ..Options::default()
    }
}

fn render() -> Result<Rendered, String> {
    render_with_density(PixelDensity::One)
}

fn render_with_density(density: PixelDensity) -> Result<Rendered, String> {
    let fx = effect()?;
    let actor = sprite()?;
    Ok(render_frames_with_actor(fx, &options(density), 0, |_| {
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

/// Whether (x, y) lies in the 12 x 20 sprite's box.
fn in_sprite(x: i32, y: i32) -> bool {
    (-6..6).contains(&x) && (-20..0).contains(&y)
}

#[test]
fn the_swing_draws_an_arc_outside_the_sprite_on_its_facing_side() -> Result<(), String> {
    let r = render()?;
    let px = drawn(&r, SWING_FRAME);
    let outside: Vec<_> = px.iter().filter(|&&(x, y, _)| !in_sprite(x, y)).collect();
    if outside.len() < 100 {
        return Err(format!(
            "{} pixels outside the sprite, wanted >= 100",
            outside.len()
        ));
    }
    let max_x = px.iter().map(|&(x, _, _)| x).max().unwrap_or(i32::MIN);
    let blade = RULER.blade as i32;
    if max_x < blade {
        return Err(format!(
            "the farthest pixel is at x {max_x}, wanted >= {blade}"
        ));
    }
    if let Some(&(x, y, _)) = px.iter().find(|&&(_, y, _)| y > -16) {
        return Err(format!("({x}, {y}) is drawn below the shoulder"));
    }
    let forward = px.iter().filter(|&&(x, _, _)| x > 5).count();
    let behind = px.iter().filter(|&&(x, _, _)| x < -6).count();
    if forward <= behind {
        return Err(format!(
            "{forward} pixels forward, {behind} behind: wanted more forward"
        ));
    }
    Ok(())
}

#[test]
fn nothing_is_drawn_outside_the_sprite_before_the_swing_or_after_it() -> Result<(), String> {
    let r = render()?;
    let last = r.frames.len().saturating_sub(1);
    if last < SWING_FRAME {
        return Err(format!("only {} frames", r.frames.len()));
    }
    let first = drawn(&r, 0);
    if !first.is_empty() {
        return Err(format!("{} pixels drawn on frame 0", first.len()));
    }
    let end = drawn(&r, last);
    if !end.is_empty() {
        return Err(format!(
            "{} pixels drawn on the last frame {last}",
            end.len()
        ));
    }
    Ok(())
}

#[test]
fn the_frames_differ_with_and_without_the_actor() -> Result<(), String> {
    let fx = effect()?;
    for density in [PixelDensity::One, PixelDensity::Two] {
        let with = drawn(&render_with_density(density)?, SWING_FRAME);
        let without = drawn(
            &render_frames_with_actor(fx, &options(density), 0, |_| None),
            SWING_FRAME,
        );
        if with.is_empty() || without.is_empty() {
            return Err(format!(
                "{density:?}: {} pixels with the sprite, {} without: wanted both drawn",
                with.len(),
                without.len()
            ));
        }
        if with == without {
            return Err(format!(
                "{density:?}: the same pixels with and without the sprite"
            ));
        }
    }
    Ok(())
}

#[test]
fn it_fires_an_impact_cue() -> Result<(), String> {
    let fx = effect()?;
    let cues = fx.cues(&Options::default());
    if cues.is_empty() {
        return Err("no cues".to_string());
    }
    if let Some(c) = cues.iter().find(|c| (c.t - 0.1).abs() > 1e-9) {
        return Err(format!("a cue at t {}, wanted 0.1", c.t));
    }
    if !cues.iter().any(|c| matches!(c.cue, Cue::Shake { .. })) {
        return Err("no shake".to_string());
    }
    let stop = cues.iter().find_map(|c| match c.cue {
        Cue::Hitstop { dur } => Some(dur),
        _ => None,
    });
    match stop {
        Some(dur) if (dur - 4.0 / 60.0).abs() < 1e-9 => {}
        other => return Err(format!("hit-stop {other:?}, wanted 4 frames at 60")),
    }
    if cues
        .iter()
        .any(|c| matches!(c.cue, Cue::Ripple { .. } | Cue::Flash { .. }))
    {
        return Err("a ripple or flash: too heavy for a light blow".to_string());
    }
    let mut quiet = Options::default();
    quiet.extra.insert("feel".to_string(), OptValue::Num(0.0));
    if !fx.cues(&quiet).is_empty() {
        return Err("cues at feel 0".to_string());
    }
    Ok(())
}
