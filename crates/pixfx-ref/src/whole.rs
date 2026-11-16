//! The whole effect from a reference drawn a sheet per moment: every moment's whole frames in order, on one canvas,
//! so it plays start to end and can be looked at as one animation. Each frame stands on the canvas's bottom edge,
//! centred across (the feet line is the cell's bottom), so moments drawn at slightly different pixel sizes line up.

use crate::RgbaImage;

/// How long a frame shows, in hundredths of a second: a moment's frames at `step`, its last one held for `hold`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    pub step: u16,
    pub hold: u16,
}

/// One moment's whole frames and how they are timed.
pub struct Moment {
    pub name: String,
    pub frames: Vec<RgbaImage>,
    pub timing: Timing,
}

/// Every frame of `moments` in order on a canvas as big as the biggest (each standing on its bottom edge, centred
/// across, black round it), with its delay.
pub fn sequence(moments: &[Moment]) -> Vec<(RgbaImage, u16)> {
    let all = || moments.iter().flat_map(|m| m.frames.iter());
    let w = all().map(|f| f.w).max().unwrap_or(0);
    let h = all().map(|f| f.h).max().unwrap_or(0);
    let mut out = Vec::new();
    for m in moments {
        let n = m.frames.len();
        for (k, f) in m.frames.iter().enumerate() {
            let mut c = RgbaImage::new(w, h);
            for p in c.rgba.as_chunks_mut::<4>().0 {
                *p = [0, 0, 0, 255];
            }
            let (x0, y0) = ((w - f.w) / 2, h - f.h);
            for y in 0..f.h {
                for x in 0..f.w {
                    if let Some(p) = f.get(x, y) {
                        c.set(x0 + x, y0 + y, [p[0], p[1], p[2], 255]);
                    }
                }
            }
            let delay = if k + 1 == n {
                m.timing.hold
            } else {
                m.timing.step
            };
            out.push((c, delay));
        }
    }
    out
}

/// The frames as a contact sheet: a row per moment, each frame drawn `scale` times bigger, `gap` dark pixels between.
pub fn strip(moments: &[Moment], frames: &[(RgbaImage, u16)], scale: u32, gap: u32) -> RgbaImage {
    let (w, h) = frames
        .first()
        .map_or((0, 0), |f| (f.0.w * scale, f.0.h * scale));
    let cols = moments.iter().map(|m| m.frames.len()).max().unwrap_or(0) as u32;
    let rows = moments.len() as u32;
    let mut out = RgbaImage::new(cols * (w + gap) + gap, rows * (h + gap) + gap);
    for p in out.rgba.as_chunks_mut::<4>().0 {
        *p = [30, 30, 38, 255];
    }
    let mut at = frames.iter();
    for (r, m) in moments.iter().enumerate() {
        for c in 0..m.frames.len() {
            let Some((f, _)) = at.next() else { break };
            let (x0, y0) = (gap + c as u32 * (w + gap), gap + r as u32 * (h + gap));
            for y in 0..h {
                for x in 0..w {
                    out.set(
                        x0 + x,
                        y0 + y,
                        f.get(x / scale, y / scale).unwrap_or([0, 0, 0, 255]),
                    );
                }
            }
        }
    }
    out
}
