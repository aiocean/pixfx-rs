//! The beanfx JSON of a sheet: Aseprite "array" layout with beanfx's fields under `meta.beanfx`
//! (beanfx `docs/output-json.md`). Pure: values in, [`serde_json::Value`] out.

use pixfx_core::math::js_round;
use pixfx_core::{Meta, Rendered};
use serde_json::{Value, json};

use crate::sheet::Grid;

/// File names (not paths) of one sheet's files, as the JSON refers to them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileNames {
    pub sheet: String,
    pub color: String,
    pub light: String,
    pub json: String,
}

impl FileNames {
    pub fn new(name: &str) -> Self {
        Self {
            sheet: format!("{name}.png"),
            color: format!("{name}_color.png"),
            light: format!("{name}_light.png"),
            json: format!("{name}.json"),
        }
    }
}

/// Frame time in ms, `round(1000 / fps)`.
fn frame_ms(fps: u32) -> u32 {
    js_round(1000.0 / f64::from(fps.max(1))) as u32
}

/// The sheet's JSON. `pivot` is `r.origin + r.margin`, y down from the cell's top-left.
pub fn beanfx_json(r: &Rendered, meta: &Meta, grid: &Grid, name: &str, files: &FileNames) -> Value {
    let ms = frame_ms(r.fps);
    let frames: Vec<Value> = (0..grid.frames)
        .map(|i| {
            let (x, y) = grid.cell_origin(i);
            let (w, h) = (grid.cell_w, grid.cell_h);
            json!({
                "filename": format!("{name} {i}"),
                "frame": { "x": x, "y": y, "w": w, "h": h },
                "rotated": false,
                "trimmed": false,
                "spriteSourceSize": { "x": 0, "y": 0, "w": w, "h": h },
                "sourceSize": { "w": w, "h": h },
                "duration": ms,
            })
        })
        .collect();
    let last = grid.frames.saturating_sub(1);
    let margin = r.margin as i32;
    json!({
        "frames": frames,
        "meta": {
            "app": "pixfx",
            "version": "1",
            "image": files.sheet,
            "format": "RGBA8888",
            "size": { "w": grid.width(), "h": grid.height() },
            "scale": "1",
            "frameTags": [{ "name": name, "from": 0, "to": last, "direction": "forward" }],
            "beanfx": {
                "name": name,
                "fps": r.fps,
                "frames": grid.frames,
                "rows": 1,
                "frameSize": [grid.cell_w, grid.cell_h],
                "columns": grid.columns,
                "pivot": [r.origin.0 + margin, r.origin.1 + margin],
                "loop": meta.looping,
                "end": if meta.looping { "loop" } else { "remove" },
                "sheet": files.sheet,
                "color": files.color,
                "light": files.light,
                "blend_light": "add",
                "light_gain": 1.0,
            },
        },
    })
}
