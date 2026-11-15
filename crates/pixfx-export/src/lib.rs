//! pixfx export: a rendered effect as sprite sheets plus beanfx's Aseprite-style JSON ([`write_sheet`]), as
//! a gif ([`write_gif`]), a video through ffmpeg ([`write_video`]), or baked for hosts to play without drawing ([`bake`], [`write_bake`], [`decode`]). Layout and JSON values are pure functions ([`Grid`], [`beanfx_json`]); only
//! the `write_*` functions touch files.
#![forbid(unsafe_code)]

pub mod bake;
mod gif;
mod json;
pub mod parity;
mod sheet;
mod video;

use std::fmt;
use std::io;

pub use bake::{bake, decode, write_bake};
pub use gif::write_gif;
pub use json::{FileNames, beanfx_json};
pub use pixfx_draw::RgbaImage;
pub use sheet::{Grid, SheetFiles, decode_png_rgba, pack_cells, write_png_rgba, write_sheet};
pub use video::{video_size, write_video};

/// Why an export failed.
#[derive(Debug)]
pub enum ExportError {
    Io(io::Error),
    Png(png::EncodingError),
    Gif(::gif::EncodingError),
    Json(serde_json::Error),
    /// ffmpeg is missing or failed.
    Ffmpeg(String),
    /// A `w` x `h` image blown up by `scale` does not fit the file format's size field.
    TooBig {
        w: u32,
        h: u32,
        scale: u32,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Png(e) => write!(f, "png: {e}"),
            Self::Gif(e) => write!(f, "gif: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
            Self::Ffmpeg(e) => write!(f, "ffmpeg: {e}"),
            Self::TooBig { w, h, scale } => {
                write!(f, "{w}x{h} at scale {scale} is too big for the file format")
            }
        }
    }
}

impl std::error::Error for ExportError {}

impl From<io::Error> for ExportError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<png::EncodingError> for ExportError {
    fn from(e: png::EncodingError) -> Self {
        Self::Png(e)
    }
}

impl From<::gif::EncodingError> for ExportError {
    fn from(e: ::gif::EncodingError) -> Self {
        Self::Gif(e)
    }
}

impl From<serde_json::Error> for ExportError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
