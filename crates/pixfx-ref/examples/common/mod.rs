//! What the scripts in `examples/` share: arguments, files in and out, and where things go under `target/ref`.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pixfx_export::{decode_png_rgba, write_png_rgba};
use pixfx_ref::{RgbaImage, Spec};

/// Where a spec's files go by default: `target/ref/<slug>`.
pub fn home(spec: &Spec) -> PathBuf {
    PathBuf::from("target/ref").join(spec.slug())
}

/// The sheet a script works on and its folder: the parts sheet (`target/ref/<slug>/parts`), one moment's sheet
/// (`target/ref/<slug>/<moment>`), or the whole spec on one sheet (`target/ref/<slug>`). A spec with layers is drawn
/// as a parts sheet, then a sheet per moment built from it.
pub fn sheet_of(spec: Spec, parts: bool, moment: Option<&str>) -> Result<(Spec, PathBuf), String> {
    match (parts, moment) {
        (true, Some(_)) => Err("give --parts-sheet or --moment, not both".into()),
        (true, None) => Ok((spec.parts_sheet(), home(&spec).join("parts"))),
        (false, Some(m)) => Ok((spec.moment(m)?, home(&spec).join(m.to_lowercase()))),
        (false, None) => {
            let dir = home(&spec);
            Ok((spec, dir))
        }
    }
}

/// The id's last part, a page anchor: `shield/golden-bell` is `golden-bell`.
pub fn key(spec: &Spec) -> String {
    spec.id.rsplit('/').next().unwrap_or(&spec.id).to_string()
}

/// A script's arguments: `--name value` flags taken out one by one, the rest left as positional words.
pub struct Args {
    words: Vec<String>,
    usage: &'static str,
}

impl Args {
    pub fn new(usage: &'static str) -> Self {
        Self {
            words: std::env::args().skip(1).collect(),
            usage,
        }
    }

    pub fn flag(&mut self, name: &str) -> bool {
        let at = self.words.iter().position(|w| w == name);
        at.map(|i| self.words.remove(i)).is_some()
    }

    pub fn value(&mut self, name: &str) -> Result<Option<String>, String> {
        let Some(i) = self.words.iter().position(|w| w == name) else {
            return Ok(None);
        };
        self.words.remove(i);
        if i < self.words.len() {
            Ok(Some(self.words.remove(i)))
        } else {
            Err(format!("{name} needs a value\n{}", self.usage))
        }
    }

    /// The positional words left, failing on a leftover `--flag` or a count other than `n`.
    pub fn rest(self, n: std::ops::RangeInclusive<usize>) -> Result<Vec<String>, String> {
        if let Some(w) = self.words.iter().find(|w| w.starts_with("--")) {
            return Err(format!("unknown flag {w}\n{}", self.usage));
        }
        if !n.contains(&self.words.len()) {
            return Err(self.usage.to_string());
        }
        Ok(self.words)
    }
}

pub fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
}

pub fn write_text(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("writing {}: {e}", path.display()))
}

pub fn read_spec(path: &Path) -> Result<Spec, String> {
    Spec::parse(&read_text(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn read_png(path: &Path) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    decode_png_rgba(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn write_png(path: &Path, img: &RgbaImage) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    write_png_rgba(path, img.w, img.h, &img.rgba)
        .map_err(|e| format!("writing {}: {e}", path.display()))
}

/// `run`'s error printed with the script's name, as the exit code.
pub fn exit(name: &str, r: Result<(), String>) -> ExitCode {
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{name}: {e}");
            ExitCode::FAILURE
        }
    }
}
