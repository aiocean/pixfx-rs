//! Step 4: one page with every effect's sheet, linking to each review page.
//!
//!   cargo run -p pixfx-pixel --example gallery -- [--out FILE] [--title TEXT] [<review dir>...]
//!
//! With no directories, every `target/ref/*/review` that `extract` wrote. `--out` defaults to
//! `target/ref/index.html`, where the review pages' back links point by default.

mod common;

use std::path::{Path, PathBuf};

use common::{Args, exit_reference, read_text, write_reference_text};
use pixfx_pixel::reference::review::{Entry, gallery};
use serde_json::Value;

const USAGE: &str = "usage: gallery [--out FILE] [--title TEXT] [<review dir>...]";
const INTRO: &str = "Each sheet came back from the image model with an alpha channel, shown here over black. Click a \
sheet to zoom; the link under it opens every frame split into pixfx's colour and light layers.";

fn main() -> std::process::ExitCode {
    exit_reference("gallery", run())
}

/// `p` relative to directory `base` when it is inside it, else as given.
fn relative(p: &Path, base: &Path) -> String {
    p.strip_prefix(base).unwrap_or(p).display().to_string()
}

fn run() -> Result<(), String> {
    let mut args = Args::new(USAGE);
    let out = PathBuf::from(
        args.value("--out")?
            .unwrap_or_else(|| "target/ref/index.html".into()),
    );
    let title = args
        .value("--title")?
        .unwrap_or_else(|| "Effect references".into());
    let mut dirs: Vec<PathBuf> = args
        .rest(0..=usize::MAX)?
        .into_iter()
        .map(PathBuf::from)
        .collect();
    if dirs.is_empty() {
        let root = Path::new("target/ref");
        let list =
            std::fs::read_dir(root).map_err(|e| format!("reading {}: {e}", root.display()))?;
        dirs = list
            .flatten()
            .map(|e| e.path().join("review"))
            .filter(|p| p.join("entry.json").exists())
            .collect();
        dirs.sort();
    }
    let base = out.parent().unwrap_or(Path::new("."));
    let mut entries = Vec::new();
    for d in &dirs {
        let v: Value = serde_json::from_str(&read_text(&d.join("entry.json"))?)
            .map_err(|e| format!("{}: {e}", d.join("entry.json").display()))?;
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        entries.push(Entry {
            key: s("key"),
            id: s("id"),
            name: s("name"),
            look: s("look"),
            sheet: relative(&d.join("sheet-black.png"), base),
            page: relative(&d.join("index.html"), base),
            pitch: v["pitch"].as_f64().unwrap_or(0.0),
            mean_err: v["mean_err"].as_f64().unwrap_or(0.0),
        });
    }
    write_reference_text(&out, &gallery(&title, INTRO, &entries))?;
    println!("{} effects: {}", entries.len(), out.display());
    Ok(())
}
