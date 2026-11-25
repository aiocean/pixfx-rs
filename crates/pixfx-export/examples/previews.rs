//! Write the catalog's looping GIFs, animated WebPs and still PNGs at game resolution.
//! Usage: previews <dense-thunder-poses.png> [--actor-density 1|2] [--out DIR]

mod common;

use std::sync::Arc;

use common::{Args, exit, file_name};
use pixfx_core::render::render_frames_on_actor_with;
use pixfx_core::{Actor, Options, render_frames_with_actor};
use pixfx_draw::RULER;
use pixfx_export::{
    board::{PreviewCanvas, busiest, still},
    decode_png_rgba, write_gif, write_png_rgba, write_webp_from_gif,
};

fn main() -> std::process::ExitCode {
    exit(run())
}

fn run() -> Result<bool, String> {
    let mut args =
        Args::new("usage: previews <dense-thunder-poses.png> [--actor-density 1|2] [--out DIR]");
    let density = args.count("--actor-density", 2)?;
    if density > 2 {
        return Err("--actor-density must be 1 or 2".into());
    }
    let out = args.out("target/previews")?;
    let sheet_path = args.id()?;
    let bytes = std::fs::read(&sheet_path).map_err(|err| format!("{sheet_path}: {err}"))?;
    let sheet = decode_png_rgba(&bytes)?;
    // Same first (idle) cell and foot pivot as the site's fighter show.
    let cell = RULER.cell;
    let side = cell * density;
    let (cx, feet) = RULER.feet;
    if sheet.w < side || sheet.h < side {
        return Err(format!("{sheet_path}: missing the idle cell"));
    }
    let pixels: Vec<_> = sheet
        .rgba
        .chunks_exact(sheet.w as usize * 4)
        .take(side as usize)
        .flat_map(|row| row[..side as usize * 4].iter().copied())
        .collect();
    let sprite = Arc::new(
        Actor::from_rgba(-cx, -feet, side, side, &pixels)
            .map_err(|err| err.to_string())?
            .shown(cell, cell, false),
    );

    let preview_canvas = PreviewCanvas::for_effects(pixfx_effects::EFFECTS);
    for &effect in pixfx_effects::EFFECTS {
        let meta = effect.meta();
        let mut opts = Options::default();
        if meta.id == "feedback/outline" {
            // Its default dark ink blends into the file export's backdrop.
            opts.palette = Some(vec![pixfx_draw::color::WHITE]);
        }
        let actor = |_| effect.wraps_actor().then(|| Arc::clone(&sprite));
        // Entrances and exits paint the sprite themselves while the game hides it.
        let draws_sprite = effect.replaces_actor()
            || meta.id.starts_with("appear/")
            || meta.id.starts_with("disappear/");
        let mut rendered = if effect.wraps_actor() && !draws_sprite {
            render_frames_on_actor_with(effect, &opts, actor, |p, at| {
                p.sprite_smooth(
                    f64::from(at.x()),
                    f64::from(at.y()),
                    side,
                    side,
                    1.0 / f64::from(density),
                    &pixels,
                );
            })
        } else {
            render_frames_with_actor(effect, &opts, 0, actor)
        };
        if !meta.is_screen() {
            preview_canvas.pad(&mut rendered);
        }
        let poster = busiest(&rendered)
            .and_then(|k| still(&rendered, k, 1))
            .ok_or_else(|| format!("{}: no frames", meta.id))?;
        let name = file_name(meta.id);
        let gif = out.join(format!("{name}.gif"));
        write_gif(&gif, &rendered, 1).map_err(|err| format!("{}: {err}", gif.display()))?;
        write_webp_from_gif(&gif, &out.join(format!("{name}.webp")))
            .map_err(|err| format!("{}: {err}", meta.id))?;
        write_png_rgba(
            &out.join(format!("{name}.png")),
            poster.w,
            poster.h,
            &poster.rgba,
        )
        .map_err(|err| format!("{}: {err}", meta.id))?;
        println!("preview: {}", meta.id);
    }
    Ok(true)
}
