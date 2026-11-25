//! Read the JSON index and PNG pages of a rendered clip.

use std::io::Cursor;

use pixfx_core::{ClipKey, FrameId, Layer};
use pixfx_draw::{Frame, RgbaImage};
use serde::{Deserialize, Serialize};

/// The JSON layout version [`decode`] reads.
pub const CLIP_VERSION: u32 = 1;

/// The layers a host shows, in the order rendered.
pub const LAYERS: [(Layer, &str); 2] = [(Layer::Back, "back"), (Layer::Front, "front")];

/// The JSON index of a rendered clip.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ClipIndex {
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
    pub pages: Vec<ClipPage>,
    pub sprites: Vec<ClipSprite>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ClipPage {
    pub file: String,
    pub size: (u32, u32),
}

/// One cropped layer half on a page: `rect` (`[x, y, w, h]` on page `page`) goes at `at` on the frame. A layer
/// half without a sprite shows nothing.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ClipSprite {
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
pub type PageImage = RgbaImage;

/// A decoded frame layer: show `frame` as `layer` of frame `id`.
pub struct ClipLayer {
    pub id: FrameId,
    pub layer: Layer,
    pub frame: Frame,
}

/// The frames of a rendered clip from its JSON index and its pages' PNG bytes, in the index's page order.
pub fn decode(json: &str, pages: &[&[u8]]) -> Result<Vec<ClipLayer>, String> {
    let index: ClipIndex = serde_json::from_str(json).map_err(|e| format!("clip index: {e}"))?;
    if index.version != CLIP_VERSION {
        return Err(format!(
            "clip index version {}, this build reads {CLIP_VERSION}",
            index.version
        ));
    }
    if pages.len() != index.pages.len() {
        return Err(format!(
            "the clip index names {} pages, {} were given",
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
    let density = key.pixel_density();
    let scale = density.factor();
    if w % scale != 0 || h % scale != 0 || ox % scale as i32 != 0 || oy % scale as i32 != 0 {
        return Err("clip raster bounds do not match the pixel density".into());
    }
    let mut layers: Vec<ClipLayer> = (0..index.frames)
        .flat_map(|k| {
            LAYERS.map(|(layer, _)| ClipLayer {
                id: FrameId {
                    key: key.clone(),
                    index: k,
                },
                layer,
                frame: Frame::with_density(
                    w / scale,
                    h / scale,
                    ox / scale as i32,
                    oy / scale as i32,
                    density,
                ),
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
    decode_png_rgba(bytes).map_err(|e| format!("clip page: {e}"))
}

/// Decode a PNG of any 8-bit colour type (grey, grey and alpha, RGB, RGBA; palettes expanded) as RGBA.
pub fn decode_png_rgba(bytes: &[u8]) -> Result<RgbaImage, String> {
    let bad = |e: png::DecodingError| format!("png: {e}");
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(bad)?;
    let mut data = vec![0; reader.output_buffer_size().ok_or("png too big")?];
    let info = reader.next_frame(&mut data).map_err(bad)?;
    data.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => data,
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[g, a]| [g, g, g, a])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("png: palette not expanded".to_string()),
    };
    Ok(RgbaImage {
        w: info.width,
        h: info.height,
        rgba,
    })
}
