//! A painted glow, the way pixel artists draw one: not a soft additive blur but solid steps of colour round every
//! stroke (white core, pale, mid, dark, a dithered edge). Strokes are stamped with a heat, the heat spreads out a
//! fixed amount a pixel, and the caller paints each pixel with the colour of its heat's step.
//!
//! A stroke also carries a `kind`, a tag of the caller's own (a rune drawn white, a ribbon pixel that may hold a
//! speck): only the stamped pixel keeps it, so a stroke can be drawn in its own colour while its heat only says how
//! far its halo reaches. Halo pixels have kind 0.
//!
//! Only the box the strokes cover, grown by the halo, is spread and walked, so a small glow on a large canvas costs
//! little. Everything is in the caller's local pixels: `(0, 0)` is the grid's origin.

/// A grid of heats over a `w` by `h` canvas whose origin sits `(ox, oy)` pixels from its top left.
pub struct Halo {
    w: usize,
    h: usize,
    ox: f64,
    oy: f64,
    heat: Vec<f32>,
    kind: Vec<u8>,
    /// The box holding every hot pixel, inclusive; empty while `x0 > x1`.
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
}

impl Halo {
    /// An empty grid. A stroke on the outermost row or column is dropped, so every halo fits inside.
    pub fn new(w: usize, h: usize, ox: f64, oy: f64) -> Self {
        Halo {
            w,
            h,
            ox,
            oy,
            heat: vec![0.0; w * h],
            kind: vec![0; w * h],
            x0: w,
            y0: h,
            x1: 0,
            y1: 0,
        }
    }

    /// A grid that takes no strokes and paints nothing: for a depth that is not drawn this time.
    pub fn none() -> Self {
        Self::new(0, 0, 0.0, 0.0)
    }

    /// Stamp a stroke of heat `heat` at local `(x, y)`, keeping the hotter of it and what is there.
    pub fn stamp(&mut self, x: f64, y: f64, heat: f32, kind: u8) {
        if heat <= 0.0 || self.w < 3 || self.h < 3 {
            return;
        }
        let (gx, gy) = (libm::floor(x + self.ox), libm::floor(y + self.oy));
        if gx < 1.0 || gy < 1.0 || gx >= (self.w - 1) as f64 || gy >= (self.h - 1) as f64 {
            return;
        }
        let (gx, gy) = (gx as usize, gy as usize);
        let i = gy * self.w + gx;
        if let (Some(cur), Some(k)) = (self.heat.get_mut(i), self.kind.get_mut(i))
            && heat > *cur
        {
            *cur = heat;
            *k = kind.max(*k);
            self.x0 = self.x0.min(gx);
            self.y0 = self.y0.min(gy);
            self.x1 = self.x1.max(gx);
            self.y1 = self.y1.max(gy);
        }
    }

    /// Spread the heat out, losing `fall` a pixel, with four neighbours and eight in turn so a halo comes out as a
    /// rounded octagon, not a square or a diamond.
    pub fn spread(&mut self, fall: f32) {
        if self.x0 > self.x1 || fall <= 0.0 {
            return;
        }
        let w = self.w;
        let mut top = 0.0_f32;
        for y in self.y0..=self.y1 {
            for v in self
                .heat
                .get(y * w + self.x0..=y * w + self.x1)
                .unwrap_or(&[])
            {
                top = top.max(*v);
            }
        }
        let steps = libm::ceil(f64::from(top / fall)) as usize;
        let mut next = self.heat.clone();
        for s in 0..steps {
            // the box grows a pixel a step, never onto the canvas edge
            self.x0 = self.x0.saturating_sub(1).max(1);
            self.y0 = self.y0.saturating_sub(1).max(1);
            self.x1 = (self.x1 + 1).min(w - 2);
            self.y1 = (self.y1 + 1).min(self.h - 2);
            let near: &[usize] = if s % 2 == 1 {
                &[1, w - 1, w, w + 1]
            } else {
                &[1, w]
            };
            for y in self.y0..=self.y1 {
                for x in self.x0..=self.x1 {
                    let i = y * w + x;
                    let mut best = self.heat.get(i).copied().unwrap_or(0.0);
                    for &d in near {
                        let a = self.heat.get(i - d).copied().unwrap_or(0.0);
                        let b = self.heat.get(i + d).copied().unwrap_or(0.0);
                        best = best.max(a.max(b) - fall);
                    }
                    if let Some(v) = next.get_mut(i) {
                        *v = best;
                    }
                }
            }
            core::mem::swap(&mut self.heat, &mut next);
            next.clone_from(&self.heat);
        }
    }

    /// Every hot pixel, row by row: its local `x`, `y`, its heat and its kind.
    pub fn each(&self) -> impl Iterator<Item = (f64, f64, f32, u8)> + '_ {
        let (x0, x1) = (self.x0, self.x1);
        let rows = if self.x0 > self.x1 {
            1..1
        } else {
            self.y0..self.y1 + 1
        };
        rows.flat_map(move |y| (x0..=x1).map(move |x| (x, y)))
            .filter_map(move |(x, y)| {
                let i = y * self.w + x;
                let (&h, &k) = (self.heat.get(i)?, self.kind.get(i)?);
                (h > 0.0).then_some((x as f64 - self.ox, y as f64 - self.oy, h, k))
            })
    }
}
