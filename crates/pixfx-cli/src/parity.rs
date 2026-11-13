//! Parity of a port against pixelfx: how many pixels of its frames differ from pixelfx's ([`compare`]), the
//! reference renders it is checked against ([`read_reference`]) and a gif to see the two by eye
//! ([`write_side_gif`]).

use std::fs;
use std::io;
use std::path::Path;

use pixfx_core::{Frame, Rendered, composite};

/// The largest fraction of colour or light pixels that may differ for a port to pass.
pub const GATE: f64 = 0.005;

/// A light channel may be off by this much without the pixel counting as different (rounding noise).
const LIGHT_TOLERANCE: u32 = 2;

/// How far two renders of an effect are apart, as fractions of all pixels in all frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParityReport {
    /// Pixels whose colour word differs at all.
    pub colour_diff_frac: f64,
    /// Pixels whose light differs by more than [`LIGHT_TOLERANCE`] in some channel (the unused top byte
    /// is ignored).
    pub light_diff_frac: f64,
}

impl ParityReport {
    /// Both fractions within [`GATE`].
    pub fn pass(&self) -> bool {
        self.colour_diff_frac <= GATE && self.light_diff_frac <= GATE
    }
}

/// Compare two renders pixel by pixel. Renders of different shape (size or frame count) differ everywhere.
pub fn compare(a: &Rendered, b: &Rendered) -> ParityReport {
    let px = a.w as usize * a.h as usize;
    let same_shape = a.w == b.w
        && a.h == b.h
        && a.frames.len() == b.frames.len()
        && a.frames
            .iter()
            .chain(&b.frames)
            .all(|f| f.w == a.w && f.h == a.h && f.color.len() == px && f.light.len() == px);
    if !same_shape {
        return ParityReport {
            colour_diff_frac: 1.0,
            light_diff_frac: 1.0,
        };
    }
    let (mut total, mut colour, mut light) = (0usize, 0usize, 0usize);
    for (fa, fb) in a.frames.iter().zip(&b.frames) {
        total += fa.color.len();
        colour += fa
            .color
            .iter()
            .zip(&fb.color)
            .filter(|(x, y)| x != y)
            .count();
        light += fa
            .light
            .iter()
            .zip(&fb.light)
            .filter(|&(&x, &y)| light_differs(x, y))
            .count();
    }
    if total == 0 {
        return ParityReport {
            colour_diff_frac: 0.0,
            light_diff_frac: 0.0,
        };
    }
    ParityReport {
        colour_diff_frac: colour as f64 / total as f64,
        light_diff_frac: light as f64 / total as f64,
    }
}

/// Some channel of two xBGR light words is more than [`LIGHT_TOLERANCE`] apart.
fn light_differs(x: u32, y: u32) -> bool {
    [0, 8, 16]
        .iter()
        .any(|s| ((x >> s) & 255).abs_diff((y >> s) & 255) > LIGHT_TOLERANCE)
}

/// Read a reference render written by `tools/pixelfx-ref.mjs`: `<dir>/<id>.json` `{w, h, n, fps}` and the
/// little-endian colour and light words of every frame in `<dir>/<id>.color.bin` and `<dir>/<id>.light.bin`.
/// pixelfx renders on the canvas the options give, so its origin is `origin` (the port's `Rendered.origin`).
pub fn read_reference(dir: &Path, id: &str, origin: (i32, i32)) -> io::Result<Rendered> {
    let file = |ext: &str| dir.join(format!("{id}.{ext}"));
    let json = fs::read_to_string(file("json"))?;
    let field =
        |key: &str| json_uint(&json, key).ok_or_else(|| bad_data(format!("{id}.json: no `{key}`")));
    let (w, h, n, fps) = (field("w")?, field("h")?, field("n")?, field("fps")?);
    let words = |ext: &str| -> io::Result<Vec<u32>> {
        let bytes = fs::read(file(ext))?;
        let (words, rest) = bytes.as_chunks::<4>();
        if !rest.is_empty() || words.len() != w as usize * h as usize * n as usize {
            return Err(bad_data(format!(
                "{id}.{ext}: not {n} frames of {w}x{h} words"
            )));
        }
        Ok(words.iter().map(|b| u32::from_le_bytes(*b)).collect())
    };
    let (color, light) = (words("color.bin")?, words("light.bin")?);
    let px = w as usize * h as usize;
    let frames = color
        .chunks(px.max(1))
        .zip(light.chunks(px.max(1)))
        .map(|(c, l)| Frame {
            w,
            h,
            ox: 0,
            oy: 0,
            color: c.to_vec(),
            light: l.to_vec(),
        })
        .collect();
    Ok(Rendered {
        w,
        h,
        fps,
        origin,
        margin: 0,
        frames,
    })
}

/// The unsigned integer under `"key"` in a flat JSON object of numbers, as the reference script writes it.
fn json_uint(json: &str, key: &str) -> Option<u32> {
    let body = json.trim().strip_prefix('{')?.strip_suffix('}')?;
    body.split(',').find_map(|kv| {
        let (k, v) = kv.split_once(':')?;
        (k.trim().trim_matches('"') == key).then(|| v.trim().parse().ok())?
    })
}

fn bad_data(msg: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// Pixels between the two renders in a side-by-side gif.
const GAP: u32 = 2;
/// Side-by-side gif scale.
const SCALE: u32 = 4;
/// What the renders are composited over in the side-by-side gif.
const BACKDROP: [u8; 4] = [20, 18, 28, 255];

/// Write `left` and `right` side by side as a looping gif at [`SCALE`]x, each composited over a dark
/// backdrop, at `left`'s fps. A render with fewer frames shows the backdrop once it runs out.
pub fn write_side_gif(path: &Path, left: &Rendered, right: &Rendered) -> io::Result<()> {
    let w = left.w + GAP + right.w;
    let h = left.h.max(right.h);
    let (gw, gh) = (w * SCALE, h * SCALE);
    let too_big = || bad_data(format!("side-by-side gif {gw}x{gh} is too big"));
    let (gw16, gh16) = (
        u16::try_from(gw).map_err(|_| too_big())?,
        u16::try_from(gh).map_err(|_| too_big())?,
    );
    let mut enc = gif::Encoder::new(io::BufWriter::new(fs::File::create(path)?), gw16, gh16, &[])
        .map_err(io::Error::other)?;
    enc.set_repeat(gif::Repeat::Infinite)
        .map_err(io::Error::other)?;
    let delay = (100 + left.fps.max(1) / 2) / left.fps.max(1);
    for i in 0..left.frames.len().max(right.frames.len()) {
        let mut canvas = BACKDROP[..3].repeat(w as usize * h as usize);
        for (r, x0) in [(left, 0), (right, left.w + GAP)] {
            if let Some(f) = r.frames.get(i) {
                blit(&mut canvas, w, x0, f);
            }
        }
        let rgb = scale_up(&canvas, w, h);
        let mut frame = gif::Frame::from_rgb_speed(gw16, gh16, &rgb, 10);
        frame.delay = u16::try_from(delay).unwrap_or(u16::MAX);
        enc.write_frame(&frame).map_err(io::Error::other)?;
    }
    Ok(())
}

/// Composite `f` over [`BACKDROP`] into the RGB `canvas` (`cw` wide) with its left edge at `x0`.
fn blit(canvas: &mut [u8], cw: u32, x0: u32, f: &Frame) {
    let n = f.w as usize * f.h as usize;
    let mut rgba = vec![0; n * 4];
    composite(f, &mut rgba, Some(&BACKDROP.repeat(n)));
    for (i, px) in rgba.as_chunks::<4>().0.iter().enumerate() {
        let (x, y) = (i as u32 % f.w.max(1), i as u32 / f.w.max(1));
        let d = ((y * cw + x0 + x) * 3) as usize;
        if let Some(out) = canvas.get_mut(d..d + 3) {
            out.copy_from_slice(&px[..3]);
        }
    }
}

/// An RGB image `w` x `h`, each pixel blown up to [`SCALE`] x [`SCALE`].
fn scale_up(rgb: &[u8], w: u32, h: u32) -> Vec<u8> {
    let s = SCALE as usize;
    let mut out = Vec::with_capacity(rgb.len() * s * s);
    for row in rgb.chunks(w as usize * 3).take(h as usize) {
        let wide: Vec<u8> = row.chunks(3).flat_map(|p| p.repeat(s)).collect();
        for _ in 0..s {
            out.extend_from_slice(&wide);
        }
    }
    out
}
