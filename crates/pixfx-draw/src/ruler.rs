//! The ruler every sprite and effect of a game is drawn to: how big its character is, in pixels, part by part.
//!
//! Pixel art keeps one scale across a scene: one pixel is the same size in the world on every sprite and effect, so
//! a thing drawn beside the character keeps the size it has beside a real one. A lens is as wide as the eye, so
//! glasses round a 1-pixel eye are about 2 pixels a lens. The ruler is the character, not centimetres: pixel art
//! enlarges small features (an eye is 1 of a face's 7 columns, where a real one is far less), and measuring
//! against the drawn parts enlarges a new thing by the same amount. Anything under one pixel is drawn at one.
//!
//! [`RULER`] is game-factory's character, the duel's fighter, here at the lowest layer so every crate reaches it: an
//! effect sizes a sword by it, the tools ask an image model for art at it and check what came back against it.
//! `pixfx-fighter`'s `tests/ruler.rs` measures the painted fighter against it, so the two cannot drift. A game with
//! another character fills in a [`Ruler`] of its own the same way.

/// A character's measurements in pixels, as painted at one game pixel per pixel, standing in a side view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ruler {
    /// Side of the square cell a sprite is painted in.
    pub cell: u32,
    /// Where the feet stand in the cell: x of the body's middle and y of the soles.
    pub feet: (i32, i32),
    /// Rows the standing character takes on screen, outline included: crown to the outline under the soles.
    pub height: u32,
    /// Columns it takes standing, outline included.
    pub width: u32,
    /// Width of the outline drawn round it.
    pub outline: u32,
    /// Side of the face, painted as a square (hair may cover its top row): the head as a block.
    pub head: u32,
    /// An open eye: columns, rows.
    pub eye: (u32, u32),
    /// Side of a hand.
    pub hand: u32,
    /// Thickness of an arm or a leg.
    pub limb: u32,
    /// Length of a held blade.
    pub blade: u32,
}

/// The duel's fighter standing, as `pixfx_fighter::paint` paints it: a 104x104 cell with the feet at (52, 96), 63 rows
/// by 31 on screen (61 painted, a 1-pixel outline round it), the face 11 x 11 (`head`), the open eye 2 x 4, a hand
/// 3 x 3 (`hand`), a sleeve 5 thick (`arm`), the blade 26 long (`sword`).
pub const RULER: Ruler = Ruler {
    cell: 104,
    feet: (52, 96),
    height: 63,
    width: 31,
    outline: 1,
    head: 11,
    eye: (2, 4),
    hand: 3,
    limb: 5,
    blade: 26,
};

impl Ruler {
    /// Rows of the painted body, crown to soles, without the outline: what a skeleton guide spans.
    pub const fn figure(&self) -> u32 {
        self.height.saturating_sub(2 * self.outline)
    }

    /// How many heads tall the painted body is: [`Ruler::figure`] over [`Ruler::head`].
    pub fn heads(&self) -> f64 {
        f64::from(self.figure()) / f64::from(self.head.max(1))
    }

    /// Pixels a thing `ratio` times a part `part` pixels long takes, rounded, and never under one: glasses with two
    /// lenses each as wide as a 1-pixel eye are `size(eye.0, 2.0)` = 2.
    pub fn size(part: u32, ratio: f64) -> u32 {
        let px = libm::floor(f64::from(part) * ratio + 0.5);
        if px.is_finite() && px >= 1.0 {
            px as u32
        } else {
            1
        }
    }

    /// The ruler drawn `k` times as big (an image model's sheet whose pixels are `1 / k` engine pixels): every
    /// length times `k`, rounded, and never under one; the feet rounded.
    pub fn scaled(&self, k: f64) -> Ruler {
        let len = |v: u32| Ruler::size(v, k);
        let pos = |v: i32| libm::floor(f64::from(v) * k + 0.5) as i32;
        Ruler {
            cell: len(self.cell),
            feet: (pos(self.feet.0), pos(self.feet.1)),
            height: len(self.height),
            width: len(self.width),
            outline: len(self.outline),
            head: len(self.head),
            eye: (len(self.eye.0), len(self.eye.1)),
            hand: len(self.hand),
            limb: len(self.limb),
            blade: len(self.blade),
        }
    }
}
