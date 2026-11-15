//! The character an effect wraps: the sprite's current frame as pixels, placed in the effect's own space (its
//! origin, the character's feet, at (0, 0)). An effect that wraps its actor ([`crate::Effect::wraps_actor`]) grips
//! its outline, so the game hands it the sprite's frame each time the frame changes.

use std::fmt;

use pixfx_draw::color::Rgba;

/// One frame of the actor's sprite: `w` x `h` colour words (ABGR, as [`pixfx_draw::Frame`] holds them; alpha 0 is
/// empty), its top-left at (`x`, `y`) in the effect's space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    pixels: Vec<u32>,
}

/// The pixels do not fill the size given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorError {
    pub w: u32,
    pub h: u32,
    pub len: usize,
}

impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "an actor of {}x{} needs {} pixels, not {}",
            self.w,
            self.h,
            u64::from(self.w) * u64::from(self.h),
            self.len
        )
    }
}

impl std::error::Error for ActorError {}

impl Actor {
    /// A sprite frame from its colour words, row by row.
    pub fn new(x: i32, y: i32, w: u32, h: u32, pixels: Vec<u32>) -> Result<Self, ActorError> {
        if pixels.len() as u64 != u64::from(w) * u64::from(h) {
            return Err(ActorError {
                w,
                h,
                len: pixels.len(),
            });
        }
        Ok(Self { x, y, w, h, pixels })
    }

    /// A sprite frame from straight-alpha RGBA bytes, row by row (what a canvas's `getImageData` gives).
    pub fn from_rgba(x: i32, y: i32, w: u32, h: u32, rgba: &[u8]) -> Result<Self, ActorError> {
        let (words, rest) = rgba.as_chunks::<4>();
        if !rest.is_empty() {
            return Err(ActorError {
                w,
                h,
                len: rgba.len() / 4,
            });
        }
        Self::new(
            x,
            y,
            w,
            h,
            words.iter().map(|b| u32::from_le_bytes(*b)).collect(),
        )
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn y(&self) -> i32 {
        self.y
    }

    pub fn w(&self) -> u32 {
        self.w
    }

    pub fn h(&self) -> u32 {
        self.h
    }

    /// The colour words, row by row.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// The colour word at (`i`, `j`) of the sprite (0 off it).
    pub fn at(&self, i: u32, j: u32) -> u32 {
        if i >= self.w || j >= self.h {
            return 0;
        }
        self.pixels
            .get(j as usize * self.w as usize + i as usize)
            .copied()
            .unwrap_or(0)
    }

    /// The sprite's opaque pixels (alpha above 0), row by row: each one's cell in the effect's space and its colour.
    /// What an effect reads to tint the body, take its colours, or burst from its pixels.
    pub fn opaque(&self) -> impl Iterator<Item = (i32, i32, Rgba)> + '_ {
        (0..self.h()).flat_map(move |j| {
            (0..self.w()).filter_map(move |i| {
                let word = self.at(i, j);
                (word >> 24 != 0)
                    .then(|| (self.x() + i as i32, self.y() + j as i32, Rgba::unpack(word)))
            })
        })
    }

    /// The same sprite with its top-left at (`x`, `y`).
    pub fn moved_to(self, x: i32, y: i32) -> Self {
        Self { x, y, ..self }
    }

    /// The sprite as the host shows it: stretched to `w` x `h` (nearest neighbour, as pixel art scales) and, with
    /// `flip`, mirrored left-right in its own box. Its top-left stays.
    pub fn shown(&self, w: u32, h: u32, flip: bool) -> Self {
        if w == self.w && h == self.h && !flip {
            return self.clone();
        }
        let mut pixels = Vec::with_capacity(w as usize * h as usize);
        for j in 0..h {
            let sj = (u64::from(j) * u64::from(self.h) / u64::from(h.max(1))) as u32;
            for i in 0..w {
                let i = if flip { w - 1 - i } else { i };
                let si = (u64::from(i) * u64::from(self.w) / u64::from(w.max(1))) as u32;
                pixels.push(self.at(si, sj));
            }
        }
        Self {
            x: self.x,
            y: self.y,
            w,
            h,
            pixels,
        }
    }

    /// The same sprite mirrored left-right about the effect's origin: what an effect drawn flipped sees.
    pub fn mirrored(&self) -> Self {
        let w = self.w as usize;
        let pixels = self
            .pixels
            .chunks(w.max(1))
            .flat_map(|row| row.iter().rev().copied())
            .collect();
        Self {
            x: -self.x - self.w as i32,
            y: self.y,
            w: self.w,
            h: self.h,
            pixels,
        }
    }
}
