//! The character a parity check plays an effect on (`--actor`): sprite frames read from PNGs, each standing with its
//! feet (the middle of its bottom row) on the effect's origin, shown in turn for `every` render frames each. Both
//! sides of the check see the same sprite on the same frame: the port through
//! [`pixfx_core::render_frames_with_actor`], the reference script through the JSON [`Actors::to_json`] writes.

use std::path::Path;
use std::sync::Arc;

use crate::decode_png_rgba;
use pixfx_core::Actor;

/// Sprite frames, shown in turn.
#[derive(Clone, Debug, PartialEq)]
pub struct Actors {
    frames: Vec<Arc<Actor>>,
    every: u64,
}

impl Actors {
    /// The frames of the PNGs at `paths`, in order, each held for `every` render frames (at least 1).
    pub fn read(paths: &[impl AsRef<Path>], every: u64) -> Result<Self, String> {
        let frames = paths
            .iter()
            .map(|p| {
                let p = p.as_ref();
                let bytes =
                    std::fs::read(p).map_err(|e| format!("reading {}: {e}", p.display()))?;
                let img = decode_png_rgba(&bytes).map_err(|e| format!("{}: {e}", p.display()))?;
                let (w, h) = (img.w, img.h);
                Actor::from_rgba(-((w / 2) as i32), -(h as i32), w, h, &img.rgba)
                    .map(Arc::new)
                    .map_err(|e| format!("{}: {e}", p.display()))
            })
            .collect::<Result<Vec<_>, String>>()?;
        if frames.is_empty() {
            return Err("--actor needs at least one PNG".to_string());
        }
        Ok(Self {
            frames,
            every: every.max(1),
        })
    }

    /// The sprite frame shown on render frame `k`.
    pub fn at(&self, k: u64) -> Option<Arc<Actor>> {
        let n = self.frames.len() as u64;
        let i = (k / self.every) % n.max(1);
        self.frames.get(i as usize).cloned()
    }

    /// What `tools/spritefusion-ref.mjs --actor` reads: `{"every": n, "frames": [{x, y, w, h, data: [word, ...]}]}`,
    /// each word an ABGR colour.
    pub fn to_json(&self) -> String {
        let frames: Vec<String> = self
            .frames
            .iter()
            .map(|a| {
                let data: Vec<String> = a.pixels().iter().map(u32::to_string).collect();
                format!(
                    r#"{{"x":{},"y":{},"w":{},"h":{},"data":[{}]}}"#,
                    a.x(),
                    a.y(),
                    a.w(),
                    a.h(),
                    data.join(",")
                )
            })
            .collect();
        format!(
            r#"{{"every":{},"frames":[{}]}}"#,
            self.every,
            frames.join(",")
        )
    }
}
