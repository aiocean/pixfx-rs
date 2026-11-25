//! Rendered clips: every frame of an effect, drawn as live hosts draw it ([`Player::live`], back and front layer),
//! cropped to what shows and packed onto PNG pages, with a JSON index keyed by the clip's [`ClipKey`]. A host
//! loads them into its frame store ([`decode`]) and plays the effect without drawing it.
//!
//! Pages hold two kinds of sprite: `color`, the colour layer as straight RGBA, and `light`, the light layer as
//! `r, g, b, 255` (unlit pixels `0, 0, 0, 0`), which adds as is. Both are exact: decoding gives back the frames.
//! Rendering, packing and decoding are pure; only [`write_clip`] touches files.

use std::fs;

use std::path::{Path, PathBuf};

use pixfx_core::{ClipKey, Effect, Layer, Options, Player};
use pixfx_draw::Frame;
use pixfx_scene::clip::LAYERS;
pub use pixfx_scene::clip::{
    CLIP_VERSION, ClipIndex, ClipLayer, ClipPage, ClipSprite, PageImage, decode,
};

use crate::ExportError;

/// Widest a page grows, unless one sprite is wider; also how tall before the next page starts.
const PAGE: u32 = 2048;

/// Every frame of one effect with one set of options, drawn by layer.
pub struct Clip {
    pub id: &'static str,
    pub key: ClipKey,
    pub fps: u32,
    pub looping: bool,
    /// Per frame index, the back and the front layer.
    pub frames: Vec<[Frame; 2]>,
}

/// `effect` with `opts` drawn frame by frame as a live host plays it: frame k is what [`Player::live`] shows
/// with its clock on frame k.
pub fn render_clip(effect: &'static dyn Effect, opts: Options) -> Result<Clip, String> {
    let meta = effect.meta();
    let mut player = Player::live(effect, opts);
    let place = player.placement();
    let key = player
        .shown(place)
        .ok_or("a new player shows no frame")?
        .key;
    let step = 1.0 / f64::from(meta.fps);
    let mut frames = Vec::new();
    for k in 0..meta.frames() {
        if k > 0 {
            player.update(step);
        }
        let shown = player.shown(place).map(|id| id.index);
        if shown != Some(k) {
            return Err(format!(
                "{}: stepping to frame {k} shows {shown:?}",
                meta.id
            ));
        }
        let mut layer = |l| {
            player.draw(l, place);
            player.frame().clone()
        };
        frames.push([layer(Layer::Back), layer(Layer::Front)]);
    }
    Ok(Clip {
        id: meta.id,
        key,
        fps: meta.fps,
        looping: meta.looping,
        frames,
    })
}

/// A frame layer half as RGBA bytes, and whether a pixel shows.
type Half = (&'static str, fn(&Frame, usize) -> [u8; 4]);

const HALVES: [Half; 2] = [("color", color_px), ("light", light_px)];

fn color_px(f: &Frame, i: usize) -> [u8; 4] {
    f.color.get(i).map_or([0; 4], |c| c.to_le_bytes())
}

fn light_px(f: &Frame, i: usize) -> [u8; 4] {
    match f.light.get(i).map(|l| l.to_le_bytes()) {
        Some([0, 0, 0, _]) | None => [0; 4],
        Some([r, g, b, _]) => [r, g, b, 255],
    }
}

/// The smallest rectangle of `f` where `px` shows (alpha above 0), `(x, y, w, h)`.
fn crop(f: &Frame, px: fn(&Frame, usize) -> [u8; 4]) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..f.h {
        for x in 0..f.w {
            if px(f, (y * f.w + x) as usize)[3] != 0 {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    (x0 != u32::MAX).then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// Lay `clip` out on pages: the index (page files `<name>.clip.<n>.png`) and the page images.
pub fn pack(clip: &Clip, name: &str) -> (ClipIndex, Vec<PageImage>) {
    // every sprite, tallest first, onto shelves left to right
    let mut cut = Vec::new();
    for (k, layers) in (0..).zip(&clip.frames) {
        for (frame, (_, layer)) in layers.iter().zip(LAYERS) {
            for (kind, px) in HALVES {
                if let Some(rect) = crop(frame, px) {
                    cut.push((k, layer, kind, px, frame, rect));
                }
            }
        }
    }
    cut.sort_by_key(|c| std::cmp::Reverse(c.5.3));
    let page_w = cut.iter().map(|c| c.5.2).max().unwrap_or(1).max(PAGE);
    let mut sprites = Vec::new();
    let mut sizes: Vec<(u32, u32)> = vec![(page_w, 0)];
    let (mut x, mut shelf_y, mut shelf_h) = (0, 0, 0);
    for &(k, layer, kind, _, _, (rx, ry, rw, rh)) in &cut {
        if x + rw > page_w {
            (x, shelf_y, shelf_h) = (0, shelf_y + shelf_h, 0);
        }
        if shelf_y > 0 && shelf_y + rh > PAGE {
            sizes.push((page_w, 0));
            (x, shelf_y, shelf_h) = (0, 0, 0);
        }
        let page = sizes.len() - 1;
        if let Some(size) = sizes.last_mut() {
            size.1 = size.1.max(shelf_y + rh);
        }
        sprites.push(ClipSprite {
            frame: k,
            layer: layer.to_string(),
            kind: kind.to_string(),
            page,
            rect: (x, shelf_y, rw, rh),
            at: (rx, ry),
        });
        x += rw;
        shelf_h = shelf_h.max(rh);
    }
    let mut pages: Vec<PageImage> = sizes
        .iter()
        .map(|&(w, h)| PageImage {
            w,
            h,
            rgba: vec![0; w as usize * h as usize * 4],
        })
        .collect();
    for (s, &(_, _, _, px, frame, _)) in sprites.iter().zip(&cut) {
        let Some(page) = pages.get_mut(s.page) else {
            continue;
        };
        let (px0, py0, w, h) = s.rect;
        for y in 0..h {
            for x in 0..w {
                let src = ((s.at.1 + y) * frame.w + s.at.0 + x) as usize;
                let dst = (((py0 + y) * page.w + px0 + x) * 4) as usize;
                if let Some(out) = page.rgba.get_mut(dst..dst + 4) {
                    out.copy_from_slice(&px(frame, src));
                }
            }
        }
    }
    let (size, corner) = clip
        .frames
        .first()
        .map_or(((0, 0), (0, 0)), |[f, _]| ((f.w, f.h), (f.ox, f.oy)));
    let index = ClipIndex {
        version: CLIP_VERSION,
        id: clip.id.to_string(),
        key: clip.key.to_string(),
        fps: clip.fps,
        looping: clip.looping,
        size,
        corner,
        frames: clip.frames.len() as u64,
        pages: (0..)
            .zip(&sizes)
            .map(|(n, &size)| ClipPage {
                file: format!("{name}.clip.{n}.png"),
                size,
            })
            .collect(),
        sprites,
    };
    (index, pages)
}

/// Paths of the files [`write_clip`] wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipFiles {
    pub json: PathBuf,
    pub pages: Vec<PathBuf>,
}

/// Write `<name>.clip.json` and its pages into `out_dir`, creating it.
pub fn write_clip(clip: &Clip, out_dir: &Path, name: &str) -> Result<ClipFiles, ExportError> {
    fs::create_dir_all(out_dir)?;
    let (index, pages) = pack(clip, name);
    let mut files = ClipFiles {
        json: out_dir.join(format!("{name}.clip.json")),
        pages: Vec::new(),
    };
    for (page, image) in index.pages.iter().zip(&pages) {
        let path = out_dir.join(&page.file);
        crate::write_png_rgba(&path, image.w, image.h, &image.rgba)?;
        files.pages.push(path);
    }
    fs::write(&files.json, serde_json::to_vec(&index)?)?;
    Ok(files)
}
