//! Every effect beside the fighter at the game's ruler (`pixfx_draw::RULER`), to see which ones are out of scale:
//! the busiest frame of each, played on the fighter's sprite (feet on the effect's origin, drawn between its back and
//! front layers), blown up `--scale` times (default 3), and a page laying them out with each canvas size.
//!
//!   cargo run -p pixfx-fighter --example poses -- thunder --pose idle --states --out target/pixel   # the sprite
//!   cargo run -p pixfx-export --example scale_board -- --actor target/pixel/thunder-idle/r0-m0.png [--scale N]
//!       [--screens] [--out DIR] [--id ID]...
//!
//! With no `--id`, every effect but the full-screen ones (`--screens` keeps them). Writes `<out>/<name>.png` per effect
//! and `<out>/index.html`; `--out` defaults to `target/scale`. The sprite shows only inside the effect's canvas, so a
//! small canvas crops it.

mod common;

use std::fmt::Write as _;

use common::{Args, effect, exit, file_name};
use pixfx_core::{Options, render_frames_on_actor};
use pixfx_export::Actors;
use pixfx_export::board::{busiest, still};
use pixfx_export::write_png_rgba;

const USAGE: &str =
    "usage: scale_board --actor <sprite.png> [--scale N] [--screens] [--out DIR] [--id ID]...";

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args = Args::new(USAGE);
    let scale = args.count("--scale", 3)?;
    let screens = args.flag("--screens");
    let sprite = args.value("--actor")?.ok_or(USAGE)?;
    let actor = Actors::read(&[sprite.as_str()], 1)?;
    let out = args.out("target/scale")?;
    let mut ids = args.values("--id")?;
    if ids.is_empty() {
        ids = pixfx_effects::EFFECTS
            .iter()
            .filter(|e| screens || !e.meta().is_screen())
            .map(|e| e.meta().id.to_string())
            .collect();
    }
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut page = String::from(
        "<!doctype html><meta charset=utf-8><title>scale board</title><style>body{background:#111;color:#ddd;font:13px ui-monospace,monospace}main{display:flex;flex-wrap:wrap;gap:16px;align-items:flex-end}figure{margin:0}img{image-rendering:pixelated;display:block}</style><h1>Every effect beside the fighter (40 px, head 7, eye 1), one game pixel = SCALE screen pixels</h1><main>",
    )
    .replace("SCALE", &scale.to_string());
    for id in &ids {
        let fx = effect(id)?;
        let r = render_frames_on_actor(fx, &Options::default(), |k| actor.at(k));
        let Some(img) = busiest(&r).and_then(|k| still(&r, k, scale)) else {
            println!("{id}: no frames");
            continue;
        };
        let name = format!("{}.png", file_name(id));
        let path = out.join(&name);
        write_png_rgba(&path, img.w, img.h, &img.rgba)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let _ = write!(
            page,
            "<figure><img src=\"{name}\"><figcaption>{id} {}x{}</figcaption></figure>",
            r.w, r.h
        );
        println!("{id}: {}", path.display());
    }
    page.push_str("</main>");
    let index = out.join("index.html");
    std::fs::write(&index, page).map_err(|e| format!("{}: {e}", index.display()))?;
    println!("{}", index.display());
    Ok(true)
}
