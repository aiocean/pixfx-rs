//! Aseprite array JSON for the composite atlas, extended with playback and layer metadata for pixfx.

use pixfx_core::{Meta, Rendered};
use pixfx_draw::math::js_round;
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
pub fn sheet_json(r: &Rendered, meta: &Meta, grid: &Grid, name: &str, files: &FileNames) -> Value {
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
            "version": "2",
            "image": files.sheet,
            "format": "RGBA8888",
            "size": { "w": grid.width(), "h": grid.height() },
            "scale": "1",
            "frameTags": [{ "name": name, "from": 0, "to": last, "direction": "forward" }],
            "pixfx": {
                "pivot": [r.origin.0 + margin, r.origin.1 + margin],
                "fps": r.fps,
                "loop": meta.looping,
                "layers": [
                    { "image": files.color, "blend": "normal" },
                    { "image": files.light, "blend": "add" },
                ],
            },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixfx_draw::Frame;

    #[test]
    fn pivot_is_the_rendered_origin_plus_the_margin_not_the_meta_origin() {
        // a weather canvas sized 12 x 12 by its options: origin (6, 2), where its own 320 x 180 has (160, 170)
        let meta = Meta {
            id: "test/sized",
            name: "Sized",
            desc: "A canvas its options size.",
            tags: &["test"],
            size: (320, 180),
            origin: (160, 170),
            duration: 1.0,
            looping: true,
            fps: 30,
            seed: 1,
            state: None,
            options: &[],
        };
        let r = Rendered {
            w: 16,
            h: 16,
            fps: 30,
            origin: (6, 2),
            margin: 2,
            frames: vec![Frame::new(16, 16, 0, 0)],
        };
        let json = sheet_json(
            &r,
            &meta,
            &Grid::new(1, 16, 16),
            "sized",
            &FileNames::new("sized"),
        );
        assert_eq!(json["frames"][0]["sourceSize"], json!({ "w": 16, "h": 16 }));
        assert_eq!(json["frames"][0]["duration"], json!(33));
        assert_eq!(json["meta"]["image"], json!("sized.png"));
        assert_eq!(json["meta"]["version"], json!("2"));
        assert_eq!(
            json["meta"]["pixfx"],
            json!({
                "pivot": [8, 4],
                "fps": 30,
                "loop": true,
                "layers": [
                    { "image": "sized_color.png", "blend": "normal" },
                    { "image": "sized_light.png", "blend": "add" },
                ],
            }),
        );
        assert!(json["meta"].get("beanfx").is_none());
    }
}
