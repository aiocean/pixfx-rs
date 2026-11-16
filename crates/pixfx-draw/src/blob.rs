//! Liquid as a pixel artist paints it: points splatted into a field, the field cut at a level into one solid shape,
//! and that shape painted in solid steps instead of soft dots: a lit rim along its top, a dark outline round the rest,
//! the body, and a deeper tone where it is thick. Points close together merge into one body; a point alone is a
//! round drop. Whatever moves the points (a fluid solver, particles, a formula) is the caller's.

use crate::Painter;
use crate::color::Rgba;

/// The colours and levels a [`Blob`] is painted with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlobLook {
    /// The outline: a pixel of the shape next to an empty pixel to its side or below.
    pub edge: Rgba,
    /// The body.
    pub body: Rgba,
    /// Where the field is at least [`BlobLook::deep_at`]: thick liquid.
    pub deep: Rgba,
    /// A pixel of the shape with an empty pixel above it: light catching the top surface.
    pub rim: Rgba,
    /// The level that is inside the shape.
    pub iso: f64,
    /// The level painted [`BlobLook::deep`].
    pub deep_at: f64,
    /// Additive light put on each rim pixel (0 for none), in the rim's colour.
    pub shine: f64,
}

/// A field over a local box of whole pixels.
#[derive(Clone, Debug)]
pub struct Blob {
    x0: i32,
    y0: i32,
    w: usize,
    h: usize,
    field: Vec<f64>,
}

impl Blob {
    /// An empty field over the local box from `(x0, y0)` to `(x1, y1)`, held to 1024 pixels a side.
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        let (ax, ay) = (libm::floor(x0), libm::floor(y0));
        let w = libm::ceil(x1 - ax).clamp(0.0, 1024.0) as usize;
        let h = libm::ceil(y1 - ay).clamp(0.0, 1024.0) as usize;
        Self {
            x0: ax as i32,
            y0: ay as i32,
            w,
            h,
            field: vec![0.0; w * h],
        }
    }

    /// Adds a round bump of height `k` and radius `r` at `(x, y)`: `k (1 - d²/r²)²` at distance `d` from it.
    pub fn splat(&mut self, x: f64, y: f64, r: f64, k: f64) {
        if !(r > 0.0 && x.is_finite() && y.is_finite()) {
            return;
        }
        let r2 = r * r;
        let lx = (libm::floor(x - r) as i32 - self.x0).max(0);
        let hx = (libm::ceil(x + r) as i32 - self.x0).min(self.w as i32 - 1);
        let ly = (libm::floor(y - r) as i32 - self.y0).max(0);
        let hy = (libm::ceil(y + r) as i32 - self.y0).min(self.h as i32 - 1);
        for cy in ly..=hy {
            let dy = f64::from(cy + self.y0) + 0.5 - y;
            for cx in lx..=hx {
                let dx = f64::from(cx + self.x0) + 0.5 - x;
                let d2 = dx * dx + dy * dy;
                if d2 < r2 {
                    let u = 1.0 - d2 / r2;
                    if let Some(f) = self.field.get_mut(cy as usize * self.w + cx as usize) {
                        *f += k * u * u;
                    }
                }
            }
        }
    }

    /// The field at the local pixel `(x, y)`, 0 outside the box.
    pub fn at(&self, x: i32, y: i32) -> f64 {
        let (cx, cy) = (x - self.x0, y - self.y0);
        if cx < 0 || cy < 0 || cx >= self.w as i32 || cy >= self.h as i32 {
            return 0.0;
        }
        self.field
            .get(cy as usize * self.w + cx as usize)
            .copied()
            .unwrap_or(0.0)
    }

    /// Paints the shape: rim, outline, deep and body, each pixel one solid colour.
    pub fn paint(&self, p: &mut Painter, look: &BlobLook) {
        let iso = look.iso;
        for cy in 0..self.h as i32 {
            for cx in 0..self.w as i32 {
                let (x, y) = (cx + self.x0, cy + self.y0);
                let v = self.at(x, y);
                if v < iso {
                    continue;
                }
                let (lx, ly) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                if self.at(x, y - 1) < iso {
                    p.px(lx, ly, look.rim);
                    if look.shine > 0.0 {
                        p.light(lx, ly, look.rim, look.shine);
                    }
                } else if self.at(x - 1, y) < iso
                    || self.at(x + 1, y) < iso
                    || self.at(x, y + 1) < iso
                {
                    p.px(lx, ly, look.edge);
                } else if v >= look.deep_at {
                    p.px(lx, ly, look.deep);
                } else {
                    p.px(lx, ly, look.body);
                }
            }
        }
    }
}
