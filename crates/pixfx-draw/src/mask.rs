//! A grid of solid cells: pixelfx's mask object (`{ w, h, data }`), what [`crate::Painter::mask`] fills and
//! [`crate::Painter::outline`] rings. A character's sprite is one (`pixfx_core::Actor`, solid where its alpha is
//! above 0); [`Cells`] holds one an effect builds itself.

/// `w` x `h` cells, each solid or empty. (`mx`, `my`) counts from the top-left cell.
pub trait Mask {
    fn w(&self) -> u32;
    fn h(&self) -> u32;
    /// Whether cell (`mx`, `my`) is solid; false off the grid.
    fn solid(&self, mx: u32, my: u32) -> bool;
}

/// A mask an effect fills in itself: `data` row by row, non-zero is solid (pixelfx `{ w, h, data }`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cells {
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

impl Cells {
    /// An empty `w` x `h` mask.
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            w,
            h,
            data: vec![0; w as usize * h as usize],
        }
    }

    /// Make cell (`mx`, `my`) solid or empty; a cell off the grid is ignored.
    pub fn set(&mut self, mx: u32, my: u32, solid: bool) {
        if mx < self.w
            && my < self.h
            && let Some(c) = self
                .data
                .get_mut(my as usize * self.w as usize + mx as usize)
        {
            *c = u8::from(solid);
        }
    }
}

impl Mask for Cells {
    fn w(&self) -> u32 {
        self.w
    }
    fn h(&self) -> u32 {
        self.h
    }
    fn solid(&self, mx: u32, my: u32) -> bool {
        mx < self.w
            && my < self.h
            && self
                .data
                .get(my as usize * self.w as usize + mx as usize)
                .is_some_and(|&c| c != 0)
    }
}
