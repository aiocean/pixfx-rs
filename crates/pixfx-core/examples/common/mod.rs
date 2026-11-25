//! What both scripts share: writing one clip's sheet, GIF and JSON into a directory.

use std::path::Path;

use pixfx_core::body::{Body, Clip, Layers, Look, View, clip_sheet, sprite};
use pixfx_core::render::Rendered;
use pixfx_export::{write_gif, write_png_rgba};

pub fn view(name: &str) -> Result<View, String> {
    match name {
        "side" => Ok(View::Side),
        "iso" => Ok(View::Iso),
        "topdown" => Ok(View::Topdown),
        _ => Err(format!("--camera {name}: side, iso or topdown")),
    }
}

/// `composite.png`, `hull.png`, `bones.png`, `parts.png`, `<clip>.gif` (the first direction, 4x) and `<clip>.json`.
pub fn write_clip(body: &Body, clip: &Clip, view: View, out: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let look = Look::mannequin();
    let s = clip_sheet(body, clip, view, &look);
    for (name, img) in [
        ("composite", &s.composite),
        ("hull", &s.hull),
        ("bones", &s.bones),
        ("parts", &s.parts),
    ] {
        let p = out.join(format!("{name}.png"));
        write_png_rgba(&p, img.w, img.h, &img.rgba)
            .map_err(|e| format!("{}: {e:?}", p.display()))?;
    }
    let Some((_, cam)) = view.rows().first().copied() else {
        return Err("a view with no direction".into());
    };
    let cell = pixfx_core::body::Cell {
        w: s.meta.cell[0],
        h: s.meta.cell[1],
        pivot: (s.meta.pivot[0], s.meta.pivot[1]),
    };
    let frames = (0..clip.frames.max(1))
        .map(|f| sprite(body, clip, f, cam, cell, &look, Layers::ALL).frame)
        .collect();
    let gif = Rendered {
        w: cell.w,
        h: cell.h,
        fps: clip.fps,
        origin: (cell.pivot.0 as i32, cell.pivot.1 as i32),
        margin: 0,
        frames,
    };
    let file = clip.name.replace('/', "_");
    let p = out.join(format!("{file}.gif"));
    write_gif(&p, &gif, 4).map_err(|e| format!("{}: {e:?}", p.display()))?;
    let json = serde_json::to_string_pretty(&s.meta).map_err(|e| e.to_string())?;
    let p = out.join(format!("{file}.json"));
    std::fs::write(&p, json).map_err(|e| format!("{}: {e}", p.display()))
}
