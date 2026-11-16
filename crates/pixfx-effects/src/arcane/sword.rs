//! The spirit sword the sword effects share (`arcane/sword-formation`, `arcane/flying-sword`,
//! `arcane/held-sword`, `arcane/sword-slash`): its sprite and palette, read off the parts sheet of `pixfx-ref`'s
//! `examples/specs/arcane/sword-formation.json`, and the painter that turns it to any angle.

use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

/// The sword pointing up, tip first, a row a pixel (from the SWORD part: a white edge, a cyan edge darkening toward
/// the gold guard, a slate grip). The guard's middle (`PIVOT_ROW`, column 2) is where it turns.
pub(super) const SPRITE: [&[u8; 5]; 18] = [
    b"..W..", b"..Wm.", b".bWm.", b".bWz.", b".bWz.", b".bWb.", b".bWb.", b".bWc.", b".bWc.",
    b".bWp.", b".bWp.", b"rhrhr", b"jhrhj", b"k.j.k", b"..d..", b"..e..", b"..d..", b".jhj.",
];
pub(super) const PIVOT_ROW: f64 = 11.0;
/// From the pivot to the tip, and to the far side of the sprite.
pub(super) const TIP: f64 = 11.0;
pub(super) const REACH: f64 = 13.0;
/// Where a flying or held sword's guard is, behind its origin; its tip is at `GUARD + TIP`.
#[cfg(any(
    feature = "arcane-flying-sword",
    feature = "arcane-held-sword",
    feature = "arcane-sword-formation"
))]
pub(super) const GUARD: f64 = -4.0;

/// The palette, from the parts sheet's exact colours.
pub(super) const WHITE: Rgba = Rgba::hex24(0xf2fdfd);
pub(super) const ICE: Rgba = Rgba::hex24(0xa4f7fd);
pub(super) const AQUA: Rgba = Rgba::hex24(0x63eefd);
pub(super) const CYAN: Rgba = Rgba::hex24(0x56cef8);
pub(super) const SKY: Rgba = Rgba::hex24(0x56b6e2);
pub(super) const STEEL: Rgba = Rgba::hex24(0x3d85b0);
pub(super) const DUSK: Rgba = Rgba::hex24(0x396b8a);
pub(super) const CREAM: Rgba = Rgba::hex24(0xf8f6c3);
pub(super) const GOLD: Rgba = Rgba::hex24(0xeccf54);
pub(super) const AMBER: Rgba = Rgba::hex24(0xd9ae31);
pub(super) const BRONZE: Rgba = Rgba::hex24(0xb5922e);
pub(super) const UMBER: Rgba = Rgba::hex24(0x8b7121);
pub(super) const GRIP: Rgba = Rgba::hex24(0x2d5672);
/// The additive glow round blades, trails and flashes.
pub(super) const GLOW: Rgba = Rgba::hex24(0x44b2c1);

/// Where a sword is and how it is drawn at a moment.
#[derive(Clone, Copy)]
pub(super) struct Pose {
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) angle: f64,
    /// How much of the blade is drawn, from the guard out (0 to 1), while it forms.
    pub(super) grown: f64,
    /// Whiteness of a flash over it (0 to 1).
    pub(super) hot: f64,
    /// Opacity, faded by dither.
    pub(super) alpha: f64,
}

/// A 4 by 4 ordered dither threshold for a pixel, 0 to 1.
pub(super) fn bayer(x: f64, y: f64) -> f64 {
    const M: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    let xi = (libm::floor(x) as i64).rem_euclid(4) as usize;
    let yi = (libm::floor(y) as i64).rem_euclid(4) as usize;
    (f64::from(M[yi][xi]) + 0.5) / 16.0
}

/// A colour a step darker, for a sword scattering.
pub(super) fn dim(c: Rgba) -> Rgba {
    match c {
        WHITE => ICE,
        ICE => AQUA,
        AQUA | CYAN => SKY,
        SKY => STEEL,
        CREAM => GOLD,
        GOLD => AMBER,
        AMBER => BRONZE,
        other => other,
    }
}

/// The colour of a sprite letter.
pub(super) fn colour_of(ch: u8) -> Option<Rgba> {
    Some(match ch {
        b'W' => WHITE,
        b'm' => ICE,
        b'z' => AQUA,
        b'b' => CYAN,
        b'c' => SKY,
        b'p' => STEEL,
        b'd' => DUSK,
        b'r' => CREAM,
        b'h' => GOLD,
        b'j' => AMBER,
        b'i' => BRONZE,
        b'k' => UMBER,
        b'e' => GRIP,
        _ => return None,
    })
}

/// One sword at a pose: the sprite turned to its angle by nearest pixel (each pixel of the box mapped back into the
/// sprite), only the rows out to how far it has grown, whitened by its flash, faded by dither; a soft glow along the
/// blade.
#[cfg(any(
    feature = "arcane-flying-sword",
    feature = "arcane-held-sword",
    feature = "arcane-sword-formation"
))]
pub(super) fn sword(p: &mut Painter, s: &Pose) {
    sword_at(p, s, 1.0);
}

/// [`sword`] drawn `scale` times its size (each engine pixel mapped back into the sprite), for a blade that must
/// read at a character's size (`arcane/sword-slash`).
pub(super) fn sword_at(p: &mut Painter, s: &Pose, scale: f64) {
    let (ca, sa) = (libm::cos(s.angle), libm::sin(s.angle));
    let reach = REACH * scale;
    let (x0, y0) = (libm::floor(s.x - reach), libm::floor(s.y - reach));
    let span = (reach * 2.0) as i32 + 1;
    // rows from the tip that show while the blade grows out of the guard
    let shown = PIVOT_ROW - TIP * s.grown;
    for j in 0..span {
        for i in 0..span {
            let (px, py) = (x0 + f64::from(i), y0 + f64::from(j));
            let (dx, dy) = (px + 0.5 - s.x, py + 0.5 - s.y);
            let along = (dx * ca + dy * sa) / scale;
            let across = (-dx * sa + dy * ca) / scale;
            let row = libm::floor(PIVOT_ROW - along + 0.5);
            let col = libm::floor(across + 2.5);
            if !(0.0..SPRITE.len() as f64).contains(&row) || !(0.0..5.0).contains(&col) {
                continue;
            }
            if row < libm::floor(shown) {
                continue;
            }
            let Some(&ch) = SPRITE.get(row as usize).and_then(|r| r.get(col as usize)) else {
                continue;
            };
            let Some(mut c) = colour_of(ch) else {
                continue;
            };
            if s.hot > 0.0 && row < PIVOT_ROW && bayer(px, py) < s.hot {
                c = WHITE;
            }
            if s.alpha < 1.0 {
                // scattering: it crumbles from the tip, dimmed a step
                if row < libm::floor((1.0 - s.alpha) * SPRITE.len() as f64) {
                    continue;
                }
                c = dim(c);
            }
            p.dot(px, py, c, 1.0);
        }
    }
    let k = s.alpha * (0.35 + 0.5 * s.hot);
    let reach = TIP * scale * s.grown;
    p.glow_line(
        s.x,
        s.y,
        s.x + ca * reach,
        s.y + sa * reach,
        3.0 * scale,
        GLOW,
        k,
    );
}
