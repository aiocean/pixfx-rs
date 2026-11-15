//! A video is the render's frames, blown up, at even sizes `yuv420p` takes.

use pixfx_core::{Options, render_frames};
use pixfx_export::{video_size, write_video};

#[test]
fn sizes_are_scaled_and_even() {
    assert_eq!(video_size(10, 7, 4), Some((40, 28)));
    assert_eq!(video_size(5, 3, 1), Some((6, 4)));
    assert_eq!(video_size(5, 3, 0), Some((6, 4)));
    assert_eq!(video_size(u32::MAX, 3, 2), None);
}

/// Needs ffmpeg and ffprobe on PATH: `cargo test -p pixfx-export --test video -- --ignored`.
#[test]
#[ignore = "runs ffmpeg"]
fn mp4_has_every_frame() -> Result<(), String> {
    let fx = pixfx_effects::effect("fire/campfire").ok_or("no fire/campfire")?;
    let r = render_frames(fx, &Options::default(), 0);
    let path = std::env::temp_dir().join(format!("pixfx-video-{}.mp4", std::process::id()));
    write_video(&path, &r, 3, 2).map_err(|e| e.to_string())?;
    let probe = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-count_frames"])
        .args([
            "-show_entries",
            "stream=width,height,nb_read_frames",
            "-of",
            "csv=p=0",
        ])
        .arg(&path)
        .output()
        .map_err(|e| e.to_string())?;
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    let got = String::from_utf8_lossy(&probe.stdout).trim().to_string();
    let (w, h) = video_size(r.w, r.h, 3).ok_or("too big")?;
    assert_eq!(got, format!("{w},{h},{}", r.frames.len() * 2));
    Ok(())
}
