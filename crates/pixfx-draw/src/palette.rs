//! Palette presets: one word recolours any effect. Every effect's `palette` runs from its darkest colour to its
//! lightest, and a palette of another length is resampled to the effect's own (`Options::palette_or` in pixfx-core),
//! so one 8-step ramp per preset fits them all.

use std::fmt;

use crate::color::Rgba;

const fn ramp(hex: [u32; 8]) -> [Rgba; 8] {
    [
        Rgba::hex24(hex[0]),
        Rgba::hex24(hex[1]),
        Rgba::hex24(hex[2]),
        Rgba::hex24(hex[3]),
        Rgba::hex24(hex[4]),
        Rgba::hex24(hex[5]),
        Rgba::hex24(hex[6]),
        Rgba::hex24(hex[7]),
    ]
}

/// The presets by name, each a ramp from dark to light.
pub const PRESETS: [(&str, [Rgba; 8]); 10] = [
    (
        "fire",
        ramp([
            0x1a0a06, 0x4a1208, 0x8a240c, 0xc8441a, 0xee7a22, 0xfbb03b, 0xffe07a, 0xfffbe0,
        ]),
    ),
    (
        "ice",
        ramp([
            0x0a1428, 0x13305a, 0x1e5a8c, 0x3a8cc0, 0x6cc0e6, 0xa6e2f6, 0xd6f4fc, 0xffffff,
        ]),
    ),
    (
        "poison",
        ramp([
            0x0e1a0a, 0x1f3a10, 0x37601a, 0x58901e, 0x86c02a, 0xb6e046, 0xdcf68a, 0xf6ffd6,
        ]),
    ),
    (
        "holy",
        ramp([
            0x2a1e0a, 0x5a4416, 0x9a7a26, 0xd0ae3c, 0xf2d660, 0xfbec9a, 0xfff8d0, 0xffffff,
        ]),
    ),
    (
        "shadow",
        ramp([
            0x07040c, 0x160c22, 0x2a163e, 0x44225e, 0x663686, 0x8e56aa, 0xb88cd0, 0xe6d2f2,
        ]),
    ),
    (
        "arcane",
        ramp([
            0x0e0a2a, 0x22175c, 0x3a2a96, 0x5a44c8, 0x8068ec, 0xa896f8, 0xd0c4fe, 0xf4f0ff,
        ]),
    ),
    (
        "blood",
        ramp([
            0x140406, 0x34080c, 0x5c0e14, 0x8c1820, 0xb82a2a, 0xde4a3a, 0xf6806a, 0xffc8b8,
        ]),
    ),
    (
        "electric",
        ramp([
            0x06101e, 0x0e2a4a, 0x145080, 0x1c80b8, 0x30b4e8, 0x74dcff, 0xc0f2ff, 0xffffff,
        ]),
    ),
    (
        "nature",
        ramp([
            0x0c1408, 0x1c2e10, 0x2e4a18, 0x466a20, 0x64902c, 0x8cb83c, 0xbede6a, 0xeaf8b8,
        ]),
    ),
    (
        "gold",
        ramp([
            0x1e1206, 0x4a2c08, 0x86520c, 0xc48412, 0xf0b424, 0xffe066, 0xfff6c4, 0xffffff,
        ]),
    ),
];

/// A preset's ramp by name.
pub fn preset(name: &str) -> Option<&'static [Rgba]> {
    PRESETS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, r)| r.as_slice())
}

/// A palette word that names no preset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteError(pub String);

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "palette {:?} is not a preset (", self.0)?;
        for (i, (name, _)) in PRESETS.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
        }
        f.write_str("); give a preset's name or a list of hex colours")
    }
}

impl std::error::Error for PaletteError {}

/// A preset's ramp, or the error naming every preset.
pub fn parse(word: &str) -> Result<Vec<Rgba>, PaletteError> {
    preset(word)
        .map(<[Rgba]>::to_vec)
        .ok_or_else(|| PaletteError(word.to_owned()))
}
