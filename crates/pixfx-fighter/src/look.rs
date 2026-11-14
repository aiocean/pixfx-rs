//! The palette a fighter is drawn in: each part's colours, darker tones after the first where a part is shaded. A
//! look is plain data: build any palette, or start from a preset ([`Look::preset`]) and change what you like.

use pixfx_draw::Rgba;

/// How a fighter wears its hair, which also decides its head's shape when lying down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Hairdo {
    /// Swept up into a topknot under a crown, a ribbon streaming from it.
    Topknot,
    /// Long hair falling down the back and lifting in the wind, held by a jade pin.
    Flowing,
}

/// A fighter's colours (site `duel-art.js` LOOKS). Every field is public: a game builds its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Look {
    pub robe: [Rgba; 3],
    pub inner: [Rgba; 2],
    pub trim: [Rgba; 2],
    pub hair: [Rgba; 2],
    pub skin: [Rgba; 2],
    pub boot: Rgba,
    pub blade: [Rgba; 3],
    pub jade: Rgba,
    pub ribbon: [Rgba; 2],
    /// The colour of its qi: a gathering spell's aura, outline and motes.
    pub glow: Rgba,
    pub outline: Rgba,
    pub hairdo: Hairdo,
}

const fn c(rgb: u32) -> Rgba {
    Rgba::hex24(rgb)
}

impl Look {
    /// The stock looks by name: `"thunder"`, `"frost"` or `"alchemist"`.
    pub fn preset(name: &str) -> Option<Look> {
        match name {
            "thunder" => Some(Look::thunder()),
            "frost" => Some(Look::frost()),
            "alchemist" => Some(Look::alchemist()),
            _ => None,
        }
    }

    /// The Thunder school: a white robe over red, gold trim, black hair in a topknot.
    pub const fn thunder() -> Look {
        Look {
            robe: [c(0xf4efe2), c(0xd6ccb6), c(0xa39680)],
            inner: [c(0xb3261c), c(0x7a1912)],
            trim: [c(0xf0bc3a), c(0xa8741a)],
            hair: [c(0x3a3048), c(0x1a1420)],
            skin: [c(0xecc9a2), c(0xc99f7a)],
            boot: c(0x2a1a14),
            blade: [c(0xffffff), c(0xfff6c4), c(0xc8b880)],
            jade: c(0x5fd08a),
            ribbon: [c(0xe0443a), c(0xa8241c)],
            glow: c(0xffe066),
            outline: c(0x1a1220),
            hairdo: Hairdo::Topknot,
        }
    }

    /// The Frost school: a blue robe over white, ice trim, long white hair.
    pub const fn frost() -> Look {
        Look {
            robe: [c(0x4a7ed0), c(0x2f5fae), c(0x1e3f80)],
            inner: [c(0xe6f0ff), c(0xa8c0e4)],
            trim: [c(0xbff2ff), c(0x5fa8d8)],
            hair: [c(0xffffff), c(0xc4d2ee)],
            skin: [c(0xf2dcc2), c(0xd2b294)],
            boot: c(0x141a33),
            blade: [c(0xffffff), c(0xdcfbff), c(0x8fc8e8)],
            jade: c(0x9ff0e0),
            ribbon: [c(0xbff2ff), c(0x7ab8e0)],
            glow: c(0xa6ecff),
            outline: c(0x0c1024),
            hairdo: Hairdo::Flowing,
        }
    }

    /// An alchemist: a white dress shaded lavender over a blue under-robe, silver-blue
    /// trim and sash ribbons, long black hair held by a teal jade pin. Colours read from a reference drawing with
    /// `pixfx pixel` (`clean`, then `read`).
    pub const fn alchemist() -> Look {
        Look {
            robe: [c(0xf4f0fc), c(0xcec3ec), c(0x9d92ba)],
            inner: [c(0x9dbaed), c(0x5062a7)],
            trim: [c(0xe6dffc), c(0x758ecb)],
            hair: [c(0x322837), c(0x1d1023)],
            skin: [c(0xfbe2cb), c(0xcaa999)],
            boot: c(0xd1d2d7),
            blade: [c(0xfcfcfd), c(0xcfcdd2), c(0x635b68)],
            jade: c(0x5d91a6),
            ribbon: [c(0xacc9fa), c(0x6b83c4)],
            glow: c(0xc6d8ff),
            outline: c(0x1d1023),
            hairdo: Hairdo::Flowing,
        }
    }
}
