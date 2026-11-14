//! An image as plain bytes, 8 bits a channel RGBA with straight alpha: what a painted sprite becomes when it
//! leaves the engine, and what a picture read from a file comes in as.

/// An image, 8 bits a channel RGBA, straight alpha, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl RgbaImage {
    /// A `w` x `h` image, all transparent.
    pub fn new(w: u32, h: u32) -> Self {
        RgbaImage {
            w,
            h,
            rgba: vec![0; w as usize * h as usize * 4],
        }
    }

    fn at(&self, x: u32, y: u32) -> Option<usize> {
        (x < self.w && y < self.h).then(|| (y as usize * self.w as usize + x as usize) * 4)
    }

    /// The pixel at (x, y); `None` outside the image.
    pub fn get(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let i = self.at(x, y)?;
        let p = self.rgba.get(i..i + 4)?;
        Some([p[0], p[1], p[2], p[3]])
    }

    /// Set the pixel at (x, y); nothing outside the image.
    pub fn set(&mut self, x: u32, y: u32, p: [u8; 4]) {
        if let Some(i) = self.at(x, y)
            && let Some(dst) = self.rgba.get_mut(i..i + 4)
        {
            dst.copy_from_slice(&p);
        }
    }

    /// The image mirrored left to right.
    pub fn flipped(&self) -> RgbaImage {
        let mut out = RgbaImage::new(self.w, self.h);
        for y in 0..self.h {
            for x in 0..self.w {
                if let Some(p) = self.get(x, y) {
                    out.set(self.w - 1 - x, y, p);
                }
            }
        }
        out
    }

    /// The `w` x `h` part of the image whose top-left is (x, y), clipped to the image.
    pub fn crop(&self, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        let (w, h) = (
            w.min(self.w.saturating_sub(x)),
            h.min(self.h.saturating_sub(y)),
        );
        let mut out = RgbaImage::new(w, h);
        for oy in 0..h {
            for ox in 0..w {
                if let Some(p) = self.get(x + ox, y + oy) {
                    out.set(ox, oy, p);
                }
            }
        }
        out
    }
}
