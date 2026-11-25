//! Convert a looping GIF to animated WebP with libwebp's gif2webp on PATH.

use std::path::Path;
use std::process::Command;

/// Write an animated WebP from a GIF, preserving its frame timing and loop count.
pub fn write_webp_from_gif(gif: &Path, webp: &Path) -> std::io::Result<()> {
    let output = Command::new("gif2webp")
        .arg(gif)
        .arg("-o")
        .arg(webp)
        .output()
        .map_err(|err| std::io::Error::new(err.kind(), format!("running gif2webp: {err}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "gif2webp exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
