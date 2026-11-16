//! Carry an effect back and forth, as a game carries a torch or a lantern: it plays with `carried` 1 and is moved
//! along a cosine (`--reach` pixels each way, `--period` seconds), so an effect that reads its motion shows it.
//! Writes `<name>.carry.gif`, `<name>` being the id with `/` as `_`.
//!
//!   cargo run -p pixfx-export --example carry -- <id> [--reach N] [--period N] [--secs N] [--scale N] [--out DIR]

mod common;

use std::num::NonZeroU32;

use common::{Args, effect, exit, file_name};
use pixfx_core::{Clip, Layer, OptValue, Rendered};
use pixfx_draw::Frame;
use pixfx_export::write_gif;
use pixfx_scene::{Frames, InlinePainter, Pass, Shell};

const USAGE: &str = "usage: carry <id> [--reach N] [--period N] [--secs N] [--scale N] [--out DIR]";
const FPS: u32 = 30;

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let scale = args.count("--scale", 4)?;
    let reach = f64::from(args.count("--reach", 40)?);
    let period = f64::from(args.count("--period", 2)?);
    let secs = f64::from(args.count("--secs", 4)?);
    let mut opts = args.look()?;
    let out = args.out("target/out")?;
    let id = args.id()?;
    let fx = effect(&id)?;
    opts.extra.insert("carried".to_string(), OptValue::Num(1.0));

    let (w, h) = (
        fx.meta().size.0 + 2 * reach as u32 + 40,
        fx.meta().size.1 + 20,
    );
    let home = (f64::from(w) / 2.0, f64::from(fx.meta().origin.1) + 10.0);
    let mut frames: Frames<Frame, InlinePainter> = Frames::default();
    let mut scene = frames.scene(NonZeroU32::MIN);
    let mut stage = Stage(Frame::new(w, h, 0, 0));
    let held = scene
        .play(Clip::live(fx, opts), home.0 - reach, home.1)
        .map_err(|e| e.to_string())?;
    let dt = 1.0 / f64::from(FPS);
    let mut shown = Vec::new();
    let mut t = 0.0;
    while t < secs {
        let x = home.0 - reach * libm::cos(t / period * core::f64::consts::TAU);
        scene.move_to(held, x, home.1).map_err(|e| e.to_string())?;
        scene.update(dt).map_err(|e| e.to_string())?;
        stage.0.clear();
        for layer in [Layer::Back, Layer::Front] {
            scene
                .draw_layer(layer, &mut frames, &mut stage)
                .map_err(|e| e.to_string())?;
        }
        frames.end_frame(&mut stage).map_err(|e| e.to_string())?;
        drop(scene.drain_events());
        shown.push(stage.0.clone());
        t += dt;
    }
    let rendered = Rendered {
        w,
        h,
        fps: FPS,
        origin: (0, 0),
        margin: 0,
        frames: shown,
    };
    let path = out.join(format!("{}.carry.gif", file_name(&id)));
    write_gif(&path, &rendered, scale).map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{id}: {}", path.display());
    Ok(true)
}

/// The stage every surface is laid on: colour over, light added.
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
        let (w, f) = (i64::from(surface.w), &mut self.0);
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
