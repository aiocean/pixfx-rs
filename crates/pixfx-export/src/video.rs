//! A video of a render (mp4, webm: ffmpeg picks the codec from the file's extension), composited over the same
//! dark backdrop as the gif and blown up with nearest-neighbour scaling. ffmpeg is a tool on the machine, not a
//! dependency: it is run from `PATH`.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use pixfx_core::Rendered;
use pixfx_draw::composite;

use crate::ExportError;
use crate::gif::BACKDROP;

/// The video's size for a `w` x `h` render blown up `scale` times: each side rounded up to even, as `yuv420p`
/// needs. `None` when it overflows.
pub fn video_size(w: u32, h: u32, scale: u32) -> Option<(u32, u32)> {
    let even = |n: u32| {
        n.checked_mul(scale.max(1))
            .and_then(|v| v.checked_add(v % 2))
    };
    Some((even(w)?, even(h)?))
}

/// Write `r` to `path` as a video, each pixel blown up to `scale` x `scale`, the loop played `loops` times
/// (at least once).
pub fn write_video(path: &Path, r: &Rendered, scale: u32, loops: u32) -> Result<(), ExportError> {
    let scale = scale.max(1);
    let (vw, vh) = video_size(r.w, r.h, scale).ok_or(ExportError::TooBig {
        w: r.w,
        h: r.h,
        scale,
    })?;
    let filter = format!(
        "scale=iw*{scale}:ih*{scale}:flags=neighbor,pad={vw}:{vh}:0:0:0x{:02x}{:02x}{:02x}",
        BACKDROP[0], BACKDROP[1], BACKDROP[2]
    );
    let mut ff = Command::new("ffmpeg")
        .args([
            "-y",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
        ])
        .args([
            "-s",
            &format!("{}x{}", r.w, r.h),
            "-r",
            &r.fps.max(1).to_string(),
        ])
        .args(["-i", "-", "-vf", &filter, "-pix_fmt", "yuv420p"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| ExportError::Ffmpeg(format!("cannot run ffmpeg (is it installed?): {e}")))?;
    let n = r.w as usize * r.h as usize;
    let ground = BACKDROP.repeat(n);
    let mut rgba = vec![0; n * 4];
    {
        let Some(stdin) = ff.stdin.as_mut() else {
            return Err(ExportError::Ffmpeg("ffmpeg has no stdin".into()));
        };
        for _ in 0..loops.max(1) {
            for f in &r.frames {
                composite(f, &mut rgba, Some(&ground));
                stdin.write_all(&rgba)?;
            }
        }
    }
    drop(ff.stdin.take());
    let status = ff.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(ExportError::Ffmpeg(format!("ffmpeg failed: {status}")))
    }
}
