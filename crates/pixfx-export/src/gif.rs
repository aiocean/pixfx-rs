//! A looping gif of a render, composited over a dark backdrop (gif has no partial alpha).

use std::fs;
use std::io::BufWriter;
use std::path::Path;

use pixfx_core::Rendered;
use pixfx_draw::composite;

use crate::ExportError;

/// Backdrop colour the effect is composited over.
const BACKDROP: [u8; 4] = [20, 18, 28, 255];

/// Write `r` to `path` as a looping gif, each pixel blown up to `scale` x `scale` (at least 1).
pub fn write_gif(path: &Path, r: &Rendered, scale: u32) -> Result<(), ExportError> {
    let scale = scale.max(1);
    let scaled = |n: u32| {
        n.checked_mul(scale)
            .and_then(|v| u16::try_from(v).ok())
            .ok_or(ExportError::TooBig {
                w: r.w,
                h: r.h,
                scale,
            })
    };
    let (gw16, gh16) = (scaled(r.w)?, scaled(r.h)?);
    let mut enc = gif::Encoder::new(BufWriter::new(fs::File::create(path)?), gw16, gh16, &[])?;
    enc.set_repeat(gif::Repeat::Infinite)?;
    let fps = r.fps.max(1);
    let delay = u16::try_from((100 + fps / 2) / fps).unwrap_or(u16::MAX);
    let n = r.w as usize * r.h as usize;
    let ground = BACKDROP.repeat(n);
    let mut rgba = vec![0; n * 4];
    for f in &r.frames {
        composite(f, &mut rgba, Some(&ground));
        let rgb: Vec<u8> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        let mut frame = gif::Frame::from_rgb_speed(gw16, gh16, &scale_up(&rgb, r.w, scale), 10);
        frame.delay = delay;
        enc.write_frame(&frame)?;
    }
    Ok(())
}

/// An RGB image `w` wide with each pixel repeated `s` x `s` times.
fn scale_up(rgb: &[u8], w: u32, s: u32) -> Vec<u8> {
    let s = s as usize;
    let mut out = Vec::with_capacity(rgb.len() * s * s);
    for row in rgb.chunks(w as usize * 3) {
        let wide: Vec<u8> = row.chunks(3).flat_map(|p| p.repeat(s)).collect();
        for _ in 0..s {
            out.extend_from_slice(&wide);
        }
    }
    out
}
