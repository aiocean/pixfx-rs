//! Demo: a spray of blood drops, each a `projectiles/trail` in the blood palette that the script moves along its
//! fall, as a game moves any mover; the trail is the shared sweep (`pixfx_draw::sweep`). Writes
//! `target/out/blood-spray.gif`.
//!
//!   cargo run -p pixfx-export --example blood_spray

use std::num::NonZeroU32;

use pixfx_core::{Clip, Layer, OptValue, Options, Rendered};
use pixfx_draw::color::Rgba;
use pixfx_draw::{Frame, Painter};
use pixfx_export::write_gif;
use pixfx_scene::{Frames, InlinePainter, Pass, Shell};

const W: u32 = 200;
const H: u32 = 120;
const FPS: f64 = 30.0;
const GROUND: f64 = 104.0;
const GRAVITY: f64 = 420.0;
/// Where the drops leave from.
const WOUND: (f64, f64) = (40.0, 70.0);

/// Each drop: its speed (x, y), when it leaves, how wide its trail is.
const DROPS: [(f64, f64, f64, f64); 6] = [
    (150.0, -170.0, 0.0, 3.0),
    (110.0, -210.0, 0.03, 2.0),
    (180.0, -110.0, 0.06, 2.0),
    (90.0, -140.0, 0.1, 2.0),
    (210.0, -150.0, 0.14, 3.0),
    (130.0, -90.0, 0.18, 2.0),
];

const HEAD: Rgba = Rgba::hex24(0xb82a2a);
const SHINE: Rgba = Rgba::hex24(0xf6806a);
const POOL: Rgba = Rgba::hex24(0x5c0e14);
const DARK: Rgba = Rgba::hex24(0x8c1820);

fn main() -> Result<(), String> {
    let fx = pixfx_effects::effect("projectiles/trail").ok_or("projectiles/trail not built")?;
    let mut frames: Frames<Frame, InlinePainter> = Frames::default();
    let mut scene = frames.scene(NonZeroU32::MIN);
    let mut stage = Stage(Frame::new(W, H, 0, 0));
    // the blood ramp, head (bright) to end (dark)
    let palette = vec![
        Rgba::hex24(0xde4a3a),
        Rgba::hex24(0xb82a2a),
        Rgba::hex24(0x8c1820),
        Rgba::hex24(0x5c0e14),
        Rgba::hex24(0x34080c),
    ];
    let dt = 1.0 / FPS;
    let mut handles: Vec<Option<u32>> = vec![None; DROPS.len()];
    let mut landed: Vec<Option<f64>> = vec![None; DROPS.len()];
    let mut shown = Vec::new();
    for n in 0..60 {
        let t = f64::from(n) * dt;
        let mut heads = Vec::new();
        for (k, &(vx, vy, at, width)) in DROPS.iter().enumerate() {
            let s = t - at;
            if s < 0.0 || landed[k].is_some() {
                continue;
            }
            let x = WOUND.0 + vx * s;
            let y = WOUND.1 + vy * s + 0.5 * GRAVITY * s * s;
            let Some(h) = handles[k] else {
                let mut o = Options {
                    palette: Some(palette.clone()),
                    ..Options::default()
                };
                for (name, v) in [
                    ("secs", 0.1),
                    ("width", width),
                    ("wave", 0.3),
                    ("skip", 1.0),
                ] {
                    o.extra.insert(name.to_string(), OptValue::Num(v));
                }
                handles[k] = Some(
                    scene
                        .play(Clip::live(fx, o), x, y)
                        .map_err(|e| e.to_string())?,
                );
                heads.push((x, y, vx, vy, width));
                continue;
            };
            if y >= GROUND {
                scene.move_to(h, x, GROUND).map_err(|e| e.to_string())?;
                scene.stop(h);
                landed[k] = Some(x);
            } else {
                scene.move_to(h, x, y).map_err(|e| e.to_string())?;
                heads.push((x, y, vx, vy + GRAVITY * s, width));
            }
        }
        scene.update(dt).map_err(|e| e.to_string())?;
        stage.0.clear();
        {
            let mut p = Painter::new(&mut stage.0);
            p.rect(
                0.0,
                GROUND + 1.0,
                f64::from(W),
                f64::from(H) - GROUND,
                Rgba::hex24(0x1c1a22),
                1.0,
            );
            p.rect(14.0, GROUND - 40.0, 10.0, 41.0, Rgba::hex24(0x3a3446), 1.0);
        }
        scene
            .draw_layer(Layer::Back, &mut frames, &mut stage)
            .map_err(|e| e.to_string())?;
        scene
            .draw_layer(Layer::Front, &mut frames, &mut stage)
            .map_err(|e| e.to_string())?;
        frames.end_frame(&mut stage).map_err(|e| e.to_string())?;
        let mut p = Painter::new(&mut stage.0);
        for (k, x) in landed.iter().enumerate() {
            let Some(x) = *x else { continue };
            // a splat on the ground, wider for a big drop
            let big = DROPS[k].3 > 2.5;
            let w = if big { 7.0 } else { 5.0 };
            p.rect(libm::floor(x - w / 2.0), GROUND, w, 1.0, POOL, 1.0);
            p.rect(
                libm::floor(x - w / 2.0) + 1.0,
                GROUND,
                w - 2.0,
                1.0,
                DARK,
                1.0,
            );
            p.rect(libm::floor(x) - 1.0, GROUND, 2.0, 1.0, HEAD, 1.0);
            p.dot(libm::floor(x - w / 2.0) - 2.0, GROUND - 1.0, DARK, 1.0);
            p.dot(libm::floor(x + w / 2.0) + 1.0, GROUND - 2.0, DARK, 1.0);
        }
        // each drop's head: a round bead with a dark rim and a lit pixel on its leading side
        for (x, y, vx, vy, width) in heads {
            let (x, y) = (libm::floor(x), libm::floor(y));
            let l = libm::hypot(vx, vy).max(1e-6);
            let (ux, uy) = (vx / l, vy / l);
            if width > 2.5 {
                for (dx, dy) in [
                    (-1.0, -1.0),
                    (0.0, -1.0),
                    (1.0, -1.0),
                    (-1.0, 0.0),
                    (1.0, 0.0),
                    (-1.0, 1.0),
                    (0.0, 1.0),
                    (1.0, 1.0),
                ] {
                    let c = if dx * ux + dy * uy > 0.3 { HEAD } else { DARK };
                    let corner = dx != 0.0 && dy != 0.0;
                    if !corner || dx * ux + dy * uy > 0.0 {
                        p.dot(x + dx, y + dy, c, 1.0);
                    }
                }
                p.dot(x, y, HEAD, 1.0);
                p.dot(x + libm::round(ux), y + libm::round(uy), SHINE, 1.0);
            } else {
                p.dot(x, y, HEAD, 1.0);
                p.dot(x + libm::round(ux), y + libm::round(uy), SHINE, 1.0);
            }
        }
        drop(scene.drain_events());
        shown.push(stage.0.clone());
    }
    let rendered = Rendered {
        w: W,
        h: H,
        fps: 30,
        origin: (0, 0),
        margin: 0,
        frames: shown,
    };
    let path = std::path::Path::new("target/out/blood-spray.gif");
    write_gif(path, &rendered, 4).map_err(|e| e.to_string())?;
    println!("{}", path.display());
    Ok(())
}

struct Stage(Frame);

impl Shell for Stage {
    type Surface = Frame;
    type Error = String;
    fn make(&mut self, frame: &Frame) -> Result<Frame, String> {
        Ok(frame.clone())
    }
    fn refill(&mut self, surface: &mut Frame, frame: &Frame) -> Result<(), String> {
        surface.clone_from(frame);
        Ok(())
    }
    fn show(&mut self, pass: Pass, surface: &Frame) -> Result<(), String> {
        let (w, f) = (surface.w as i64, &mut self.0);
        for (i, (&c, &l)) in surface.color.iter().zip(&surface.light).enumerate() {
            let (x, y) = (pass.rect.x + i as i64 % w, pass.rect.y + i as i64 / w);
            if x < 0 || y < 0 || x >= i64::from(f.w) || y >= i64::from(f.h) {
                continue;
            }
            let j = (y as usize) * f.w as usize + x as usize;
            if c >> 24 != 0
                && let Some(d) = f.color.get_mut(j)
            {
                *d = c;
            }
            if let Some(d) = f.light.get_mut(j) {
                let add = |s: u32| (((*d >> s) & 255) + ((l >> s) & 255)).min(255) << s;
                *d = add(0) | add(8) | add(16);
            }
        }
        Ok(())
    }
    fn free(&mut self, _surface: Frame) -> Result<(), String> {
        Ok(())
    }
}
