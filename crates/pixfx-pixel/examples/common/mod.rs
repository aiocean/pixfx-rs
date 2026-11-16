//! What the step scripts in `examples/` share: their arguments, and pictures and text in and out. Each script uses
//! part of it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use pixfx_draw::RgbaImage;
use pixfx_export::{decode_png_rgba, write_png_rgba};
use pixfx_fighter::look::Look;
use pixfx_pixel::tool::Palette;

/// A script's arguments, taken flag by flag; what is left are its files.
pub struct Args {
    words: Vec<String>,
    usage: &'static str,
}

impl Args {
    /// The arguments the script was run with; `usage` is shown with every mistake.
    pub fn new(usage: &'static str) -> Self {
        Self {
            words: std::env::args().skip(1).collect(),
            usage,
        }
    }

    /// Whether `--name` was given.
    pub fn flag(&mut self, name: &str) -> bool {
        match self.words.iter().position(|w| w == name) {
            Some(i) => {
                self.words.remove(i);
                true
            }
            None => false,
        }
    }

    /// The word after `--name`, when it was given.
    pub fn value(&mut self, name: &str) -> Result<Option<String>, String> {
        let Some(i) = self.words.iter().position(|w| w == name) else {
            return Ok(None);
        };
        self.words.remove(i);
        if i < self.words.len() {
            Ok(Some(self.words.remove(i)))
        } else {
            Err(self.error(&format!("{name} needs a value")))
        }
    }

    /// The number after `--name`, else `default`.
    pub fn number<T: FromStr>(&mut self, name: &str, default: T) -> Result<T, String> {
        match self.value(name)? {
            Some(v) => v
                .parse()
                .map_err(|_| self.error(&format!("{name} expects a whole number: {v}"))),
            None => Ok(default),
        }
    }

    /// `--name X,Y,W,H`.
    pub fn rect(&mut self, name: &str) -> Result<Option<[u32; 4]>, String> {
        let Some(v) = self.value(name)? else {
            return Ok(None);
        };
        let n: Option<Vec<u32>> = v.split(',').map(|p| p.trim().parse().ok()).collect();
        n.and_then(|n| n.try_into().ok())
            .map(Some)
            .ok_or_else(|| self.error(&format!("{name} expects X,Y,W,H: {v}")))
    }

    /// `--name WxH`, both above 0.
    pub fn pair(&mut self, name: &str) -> Result<Option<(u32, u32)>, String> {
        let Some(v) = self.value(name)? else {
            return Ok(None);
        };
        v.split_once('x')
            .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
            .filter(|&(w, h)| w > 0 && h > 0)
            .map(Some)
            .ok_or_else(|| self.error(&format!("{name} expects WxH: {v}")))
    }

    /// `--palette`: a fighter look's name (its colours come named), a palette file (`.gpl` with names, as the `ramp`
    /// script writes, or `.hex`), or colours as `#rrggbb` split by commas.
    pub fn palette(&mut self) -> Result<Option<Palette>, String> {
        self.value("--palette")?
            .map(|v| match Look::preset(&v) {
                Some(look) => Ok(Palette::named(look.colors())),
                None if Path::new(&v).is_file() => {
                    Palette::parse_file(&read_text(Path::new(&v))?).map_err(|e| format!("{v}: {e}"))
                }
                None => Palette::parse_hex(&v),
            })
            .transpose()
    }

    /// `--out DIR`, made if missing; `target/pixel` when left out.
    pub fn out(&mut self) -> Result<PathBuf, String> {
        let out = PathBuf::from(
            self.value("--out")?
                .unwrap_or_else(|| "target/pixel".to_string()),
        );
        std::fs::create_dir_all(&out).map_err(|e| format!("creating {}: {e}", out.display()))?;
        Ok(out)
    }

    /// The files left once every flag was taken; a flag nobody took is a mistake.
    pub fn files(self) -> Result<Vec<PathBuf>, String> {
        if let Some(flag) = self.words.iter().find(|w| w.starts_with("--")) {
            return Err(self.error(&format!("unknown flag {flag}")));
        }
        Ok(self.words.iter().map(PathBuf::from).collect())
    }

    /// Exactly `N` files, else the usage.
    pub fn exactly<const N: usize>(self) -> Result<[PathBuf; N], String> {
        let usage = self.usage;
        <[PathBuf; N]>::try_from(self.files()?).map_err(|_| usage.to_string())
    }

    fn error(&self, what: &str) -> String {
        format!("{what}\n{}", self.usage)
    }
}

/// The exit code of a script's result: 0 when it ran and found nothing wrong, 1 when it found something, 2 when it
/// could not run.
pub fn exit(result: Result<bool, String>) -> ExitCode {
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

pub fn read_image(path: &Path) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    decode_png_rgba(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
}

/// Write a PNG and print its path.
pub fn write_image(path: &Path, img: &RgbaImage) -> Result<(), String> {
    write_png_rgba(path, img.w, img.h, &img.rgba)
        .map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

/// Write a text file and print its path.
pub fn write_text(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("writing {}: {e}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

/// A file's name without its extension: what the files written from it are named after.
pub fn stem(path: &Path) -> String {
    path.file_stem()
        .map_or("picture".into(), |s| s.to_string_lossy().into_owned())
}
