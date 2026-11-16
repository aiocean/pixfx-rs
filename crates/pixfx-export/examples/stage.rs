//! Play an effect on a stage between two characters, as a game shows it: the effect plays on the caster (left), the
//! target stands facing them, and the sprites are drawn between the effects' back and front layers. For an effect
//! that takes orders, `--order-at` orders it at the target's chest after that many seconds, so its volley flies
//! there and strikes. `--feel` lets the stage answer the effects' cues as a game does (`pixfx_core::Feel`): it
//! freezes on a hit-stop and the whole picture shakes. Writes `<name>.stage.gif`, `<name>` being the id with `/`
//! as `_`.
//!
//!   cargo run -p pixfx-export --example stage -- <id> [--actor PNG] [--target DX,DY] [--order-at SECS]
//!       [--aim converge|pierce] [--feel] [--secs N] [--scale N] [--seed N] [--palette WORD] [--num NAME=V]
//!       [--out DIR]
//!
//! `--actor` is the sprite both stand as (the target mirrored); the effect wraps the caster's. `--target` is where
//! the target's feet stand from the caster's, in pixels (default 160,0). It runs until every effect has ended, or
//! `--secs` (default 4, for a loop) if nothing was ordered. The scene decides everything (`pixfx_scene::Scene`):
//! this script only lays the frames it shows on one stage, as a shell does. `--out` defaults to `target/out`.

mod common;

use std::num::NonZeroU32;
use std::sync::Arc;

use common::{Args, effect, exit, file_name};
use pixfx_core::volley::{Aim, Order};
use pixfx_core::{Actor, Clip, Feel, Layer, Rendered};
use pixfx_draw::Frame;
use pixfx_export::parity::Actors;
use pixfx_export::write_gif;
use pixfx_scene::{Body, Frames, InlinePainter, Pass, Shell};

const USAGE: &str = "usage: stage <id> [--actor PNG] [--target DX,DY] [--order-at SECS] [--aim converge|pierce] [--feel] [--secs N] [--scale N] [--seed N] [--palette WORD] [--num NAME=V] [--out DIR]";

const FPS: u32 = 30;
/// Room round the two on the stage, and how high above its feet the target is struck.
const SIDE: f64 = 70.0;
const TOP: f64 = 110.0;
const BELOW: f64 = 24.0;
const CHEST: f64 = 22.0;
/// Seconds shown after the last effect ends; the longest a run may take.
const TAIL: f64 = 0.3;
const LONGEST: f64 = 12.0;
/// The bodies the two stand as.
const CASTER: u32 = 1;
const TARGET: u32 = 2;

fn main() -> std::process::ExitCode {
    exit(run())
}

fn pair(s: &str) -> Result<(f64, f64), String> {
    let (a, b) = s.split_once(',').ok_or("--target takes DX,DY")?;
    let n = |v: &str| {
        v.trim()
            .parse::<f64>()
            .map_err(|e| format!("--target {v}: {e}"))
    };
    Ok((n(a)?, n(b)?))
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let scale = args.count("--scale", 4)?;
    let sprite = args
        .value("--actor")?
        .map(|p| Actors::read(&[p], 1))
        .transpose()?
        .and_then(|a| a.at(0));
    let (dx, dy) = args
        .value("--target")?
        .map_or(Ok((160.0, 0.0)), |s| pair(&s))?;
    let order_at = args
        .value("--order-at")?
        .map(|s| s.parse::<f64>().map_err(|e| format!("--order-at: {e}")))
        .transpose()?;
    let felt = args.flag("--feel");
    let secs = f64::from(args.count("--secs", 4)?);
    let aim = match args.value("--aim")?.as_deref() {
        None | Some("converge") => Aim::Converge,
        Some("pierce") => Aim::Pierce,
        Some(other) => return Err(format!("--aim {other}: converge or pierce")),
    };
    let opts = args.look()?;
    let out = args.out("target/out")?;
    let id = args.id()?;
    let fx = effect(&id)?;

    let w = libm::ceil(dx.abs() + 2.0 * SIDE) as u32;
    let h = libm::ceil(dy.abs() + TOP + BELOW) as u32;
    let caster = (
        if dx >= 0.0 { SIDE } else { SIDE + dx.abs() },
        TOP + dy.min(0.0).abs(),
    );
    let target = (caster.0 + dx, caster.1 + dy);

    let mut frames: Frames<Frame, InlinePainter> = Frames::default();
    let mut scene = frames.scene(NonZeroU32::MIN);
    if felt {
        scene.set_feel(Some(Feel::new()));
    }
    let mut stage = Stage(Frame::new(w, h, 0, 0));
    let held = scene
        .play(Clip::live(fx, opts), caster.0, caster.1)
        .map_err(|e| e.to_string())?;
    if let Some(a) = &sprite {
        let placed = Actor::clone(a).moved_to(a.x() + caster.0 as i32, a.y() + caster.1 as i32);
        scene
            .set_actor(held, Some(placed))
            .map_err(|e| e.to_string())?;
    }

    // both stand as bodies the volley's pieces may meet: the caster's is spared, the target's is struck
    for (id, at) in [(CASTER, caster), (TARGET, target)] {
        scene.set_body(
            id,
            Some(Body {
                x: at.0 - 8.0,
                y: at.1 - 40.0,
                w: 16.0,
                h: 40.0,
            }),
        );
    }

    let dt = 1.0 / f64::from(FPS);
    let mut shown = Vec::new();
    let (mut t, mut ordered, mut ended_at) = (0.0, false, None::<f64>);
    let last = if order_at.is_some() { LONGEST } else { secs };
    while t < last && ended_at.is_none_or(|e| t < e + TAIL) {
        if !ordered && order_at.is_some_and(|at| t >= at) {
            let at = Order {
                x: target.0,
                y: target.1 - CHEST,
                aim,
                caster: Some(CASTER),
            };
            if !scene.order(held, at).map_err(|e| e.to_string())? {
                return Err(format!("{id} takes no orders"));
            }
            ordered = true;
        }
        // as a host does: the order comes in, the scene steps (launching what is due), then it draws
        scene.update(dt).map_err(|e| e.to_string())?;
        stage.0.clear();
        scene
            .draw_layer(Layer::Back, &mut frames, &mut stage)
            .map_err(|e| e.to_string())?;
        if let Some(a) = &sprite {
            stage.sprite(a, caster, false);
            stage.sprite(a, target, true);
        }
        scene
            .draw_layer(Layer::Front, &mut frames, &mut stage)
            .map_err(|e| e.to_string())?;
        frames.end_frame(&mut stage).map_err(|e| e.to_string())?;
        let (sx, sy) = scene.feel().map_or((0, 0), |f| f.shake(1.0));
        shown.push(shifted(&stage.0, sx, sy));
        drop(scene.drain_events());
        if ended_at.is_none() && scene.handles().next().is_none() {
            ended_at = Some(t);
        }
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
    let path = out.join(format!("{}.stage.gif", file_name(&id)));
    write_gif(&path, &rendered, scale).map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{id}: {}", path.display());
    Ok(true)
}

/// `f` moved by (`dx`, `dy`), as a camera shake moves the whole picture; what leaves the edge is lost.
fn shifted(f: &Frame, dx: i32, dy: i32) -> Frame {
    if (dx, dy) == (0, 0) {
        return f.clone();
    }
    let mut out = Frame::new(f.w, f.h, 0, 0);
    let (w, h) = (i64::from(f.w), i64::from(f.h));
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x - i64::from(dx), y - i64::from(dy));
            if fx < 0 || fy < 0 || fx >= w || fy >= h {
                continue;
            }
            let (to, from) = ((y * w + x) as usize, (fy * w + fx) as usize);
            if let (Some(c), Some(l)) = (f.color.get(from), f.light.get(from)) {
                out.color[to] = *c;
                out.light[to] = *l;
            }
        }
    }
    out
}

/// The stage every surface is laid on, as a shell lays its host objects.
struct Stage(Frame);

impl Stage {
    /// The sprite standing with its feet at `at`, mirrored when `flip`.
    fn sprite(&mut self, a: &Arc<Actor>, at: (f64, f64), flip: bool) {
        let w = a.w() as usize;
        for (i, &c) in a.pixels().iter().enumerate() {
            let x = a.x() + (i % w) as i32;
            let y = a.y() + (i / w) as i32;
            let x = if flip { -x - 1 } else { x };
            self.put(at.0 as i64 + i64::from(x), at.1 as i64 + i64::from(y), c, 0);
        }
    }

    /// Lay colour word `c` over the stage at (`x`, `y`) and add light word `l`.
    fn put(&mut self, x: i64, y: i64, c: u32, l: u32) {
        let f = &mut self.0;
        if x < 0 || y < 0 || x >= i64::from(f.w) || y >= i64::from(f.h) {
            return;
        }
        let i = (y as usize) * f.w as usize + x as usize;
        if let Some(dst) = f.color.get_mut(i) {
            let a = c >> 24;
            if a == 255 || *dst >> 24 == 0 {
                if a > 0 {
                    *dst = c;
                }
            } else if a > 0 {
                let k = f64::from(a) / 255.0;
                let mix = |s: u32| {
                    let (top, under) = (f64::from((c >> s) & 255), f64::from((*dst >> s) & 255));
                    (libm::round(top * k + under * (1.0 - k)) as u32) << s
                };
                *dst = (*dst & 0xff00_0000).max(c & 0xff00_0000) | mix(0) | mix(8) | mix(16);
            }
        }
        if l & 0x00ff_ffff != 0
            && let Some(dst) = f.light.get_mut(i)
        {
            let add = |s: u32| (((*dst >> s) & 255) + ((l >> s) & 255)).min(255) << s;
            *dst = add(0) | add(8) | add(16);
        }
    }
}

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
        let w = surface.w as usize;
        for (i, (&c, &l)) in surface.color.iter().zip(&surface.light).enumerate() {
            let (x, y) = ((i % w) as i64, (i / w) as i64);
            self.put(pass.rect.x + x, pass.rect.y + y, c, l);
        }
        Ok(())
    }

    fn free(&mut self, _surface: Frame) -> Result<(), String> {
        Ok(())
    }
}
