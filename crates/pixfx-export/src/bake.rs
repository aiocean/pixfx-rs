//! Baked clips: every frame of an effect, drawn as live hosts draw it ([`Player::live`], back and front layer),
//! cropped to what shows and packed onto PNG pages, with a JSON index keyed by the clip's [`ClipKey`]. A host
//! loads them into its frame store ([`decode`]) and plays the effect without drawing it.
//!
//! Pages hold two kinds of sprite: `color`, the colour layer as straight RGBA, and `light`, the light layer as
//! `r, g, b, 255` (unlit pixels `0, 0, 0, 0`), which adds as is. Both are exact: decoding gives back the frames.
//! Rendering, packing and decoding are pure; only [`write_bake`] touches files.

use std::fs;
use std::io::{BufWriter, Cursor};
use std::path::{Path, PathBuf};

use pixfx_core::{ClipKey, Effect, Frame, FrameId, Layer, Options, Player};
use serde::{Deserialize, Serialize};

use crate::ExportError;

/// The JSON layout version [`decode`] reads.
pub const BAKE_VERSION: u32 = 1;

/// Widest a page grows, unless one sprite is wider; also how tall before the next page starts.
const PAGE: u32 = 2048;

/// The layers a host shows, in the order baked: back, then front.
const LAYERS: [(Layer, &str); 2] = [(Layer::Back, "back"), (Layer::Front, "front")];

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
pub fn bake(effect: &'static dyn Effect, opts: Options) -> Result<Clip, String> {
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

/// The JSON index of a baked clip.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct BakeIndex {
    pub version: u32,
    pub id: String,
    /// The [`ClipKey`] the frames are shown under, as hex.
    pub key: String,
    pub fps: u32,
    #[serde(rename = "loop")]
    pub looping: bool,
    /// Frame size, `[w, h]`, and where the frame's top-left sits in effect space, `[x, y]`.
    pub size: (u32, u32),
    pub corner: (i32, i32),
    pub frames: u64,
    pub pages: Vec<BakePage>,
    pub sprites: Vec<BakeSprite>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct BakePage {
    pub file: String,
    pub size: (u32, u32),
}

/// One cropped layer half on a page: `rect` (`[x, y, w, h]` on page `page`) goes at `at` on the frame. A layer
/// half without a sprite shows nothing.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct BakeSprite {
    pub frame: u64,
    /// `"back"` or `"front"`.
    pub layer: String,
    /// `"color"` or `"light"`.
    pub kind: String,
    pub page: usize,
    pub rect: (u32, u32, u32, u32),
    pub at: (u32, u32),
}

/// A page's pixels, RGBA8.
pub struct PageImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
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

/// Lay `clip` out on pages: the index (page files `<name>.bake.<n>.png`) and the page images.
pub fn pack(clip: &Clip, name: &str) -> (BakeIndex, Vec<PageImage>) {
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
        sprites.push(BakeSprite {
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
    let index = BakeIndex {
        version: BAKE_VERSION,
        id: clip.id.to_string(),
        key: clip.key.to_string(),
        fps: clip.fps,
        looping: clip.looping,
        size,
        corner,
        frames: clip.frames.len() as u64,
        pages: (0..)
            .zip(&sizes)
            .map(|(n, &size)| BakePage {
                file: format!("{name}.bake.{n}.png"),
                size,
            })
            .collect(),
        sprites,
    };
    (index, pages)
}

/// Paths of the files [`write_bake`] wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BakeFiles {
    pub json: PathBuf,
    pub pages: Vec<PathBuf>,
}

/// Write `<name>.bake.json` and its pages into `out_dir`, creating it.
pub fn write_bake(clip: &Clip, out_dir: &Path, name: &str) -> Result<BakeFiles, ExportError> {
    fs::create_dir_all(out_dir)?;
    let (index, pages) = pack(clip, name);
    let mut files = BakeFiles {
        json: out_dir.join(format!("{name}.bake.json")),
        pages: Vec::new(),
    };
    for (page, image) in index.pages.iter().zip(&pages) {
        let path = out_dir.join(&page.file);
        let mut enc = png::Encoder::new(BufWriter::new(fs::File::create(&path)?), image.w, image.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()?.write_image_data(&image.rgba)?;
        files.pages.push(path);
    }
    fs::write(&files.json, serde_json::to_vec(&index)?)?;
    Ok(files)
}

/// A decoded frame layer: show `frame` as `layer` of frame `id`.
pub struct BakedLayer {
    pub id: FrameId,
    pub layer: Layer,
    pub frame: Frame,
}

/// The frames of a baked clip from its JSON index and its pages' PNG bytes, in the index's page order.
pub fn decode(json: &str, pages: &[&[u8]]) -> Result<Vec<BakedLayer>, String> {
    let index: BakeIndex = serde_json::from_str(json).map_err(|e| format!("bake index: {e}"))?;
    if index.version != BAKE_VERSION {
        return Err(format!(
            "bake index version {}, this build reads {BAKE_VERSION}",
            index.version
        ));
    }
    if pages.len() != index.pages.len() {
        return Err(format!(
            "the bake index names {} pages, {} were given",
            index.pages.len(),
            pages.len()
        ));
    }
    let key: ClipKey = index.key.parse()?;
    let images = pages
        .iter()
        .map(|bytes| read_png(bytes))
        .collect::<Result<Vec<_>, _>>()?;
    let (w, h) = index.size;
    let (ox, oy) = index.corner;
    let mut layers: Vec<BakedLayer> = (0..index.frames)
        .flat_map(|k| {
            LAYERS.map(|(layer, _)| BakedLayer {
                id: FrameId {
                    key: key.clone(),
                    index: k,
                },
                layer,
                frame: Frame::new(w, h, ox, oy),
            })
        })
        .collect();
    for s in &index.sprites {
        let side = LAYERS
            .iter()
            .position(|(_, name)| *name == s.layer)
            .ok_or_else(|| format!("sprite layer {:?}", s.layer))?;
        let target = usize::try_from(s.frame)
            .ok()
            .and_then(|k| layers.get_mut(k * 2 + side))
            .ok_or_else(|| format!("sprite of frame {} past the clip", s.frame))?;
        let page = images
            .get(s.page)
            .ok_or_else(|| format!("sprite on page {} past the pages", s.page))?;
        let (px0, py0, rw, rh) = s.rect;
        if px0 + rw > page.w || py0 + rh > page.h || s.at.0 + rw > w || s.at.1 + rh > h {
            return Err(format!("sprite {:?} out of bounds", s.rect));
        }
        let words = match s.kind.as_str() {
            "color" => &mut target.frame.color,
            "light" => &mut target.frame.light,
            other => return Err(format!("sprite kind {other:?}")),
        };
        let light = s.kind == "light";
        for y in 0..rh {
            for x in 0..rw {
                let src = (((py0 + y) * page.w + px0 + x) * 4) as usize;
                let Some(&[r, g, b, a]) = page.rgba.get(src..src + 4) else {
                    continue;
                };
                let word = if light {
                    u32::from_le_bytes([r, g, b, 0])
                } else {
                    u32::from_le_bytes([r, g, b, a])
                };
                if let Some(dst) = words.get_mut(((s.at.1 + y) * w + s.at.0 + x) as usize) {
                    *dst = word;
                }
            }
        }
    }
    Ok(layers)
}

fn read_png(bytes: &[u8]) -> Result<PageImage, String> {
    let bad = |e: png::DecodingError| format!("bake page: {e}");
    let mut reader = png::Decoder::new(Cursor::new(bytes))
        .read_info()
        .map_err(bad)?;
    if reader.output_color_type() != (png::ColorType::Rgba, png::BitDepth::Eight) {
        return Err("bake page is not RGBA8".to_string());
    }
    let mut rgba = vec![0; reader.output_buffer_size().ok_or("bake page too big")?];
    let info = reader.next_frame(&mut rgba).map_err(bad)?;
    rgba.truncate(info.buffer_size());
    Ok(PageImage {
        w: info.width,
        h: info.height,
        rgba,
    })
}
