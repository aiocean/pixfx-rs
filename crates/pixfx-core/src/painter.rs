//! The painter: port of pixelfx `src/core/painter.js` (the methods the fire effects use; the rest arrive
//! with the effects that need them).
//!
//! The painter draws into a frame's two buffers: `color` (opaque pixels, faded with ordered dither, never
//! blended) and `light` (additive glow, composited on top). Effects draw in local coordinates around their
//! origin; the painter places, mirrors and rotates them, so an effect never needs to know where it is.
//!
//! Screen coordinates stay `f64` as in pixelfx: a write lands only where JS would index its typed array
//! (in bounds and a whole index), so NaN or fractional coordinates draw nothing, as there.

use crate::color::Rgba;
use crate::frame::Frame;
use crate::math::{clamp, js_max, js_min, js_round, to_int32};

/// 4x4 ordered (Bayer) thresholds: a pixel at alpha a is drawn when a is above its cell's threshold.
const BAYER: [f64; 16] = {
    const V: [u8; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
    let mut out = [0.0; 16];
    let mut i = 0;
    while i < 16 {
        out[i] = (V[i] as f64 + 0.5) / 16.0;
        i += 1;
    }
    out
};

/// A shape parameter that is a constant or a function of one shape coordinate (pixelfx accepts both).
pub trait Param1<T> {
    fn at(&self, u: f64) -> T;
}

/// A shape parameter that is a constant or a function of two shape coordinates.
pub trait Param2<T> {
    fn at(&self, u: f64, v: f64) -> T;
}

impl Param1<Rgba> for Rgba {
    fn at(&self, _: f64) -> Rgba {
        *self
    }
}

impl Param1<f64> for f64 {
    fn at(&self, _: f64) -> f64 {
        *self
    }
}

impl<T, F: Fn(f64) -> T> Param1<T> for F {
    fn at(&self, u: f64) -> T {
        self(u)
    }
}

impl Param2<Rgba> for Rgba {
    fn at(&self, _: f64, _: f64) -> Rgba {
        *self
    }
}

impl Param2<f64> for f64 {
    fn at(&self, _: f64, _: f64) -> f64 {
        *self
    }
}

impl<T, F: Fn(f64, f64) -> T> Param2<T> for F {
    fn at(&self, u: f64, v: f64) -> T {
        self(u, v)
    }
}

/// What a [`Painter::shade`] function sets for its pixel: colour `c` at alpha `a`, light colour `l`
/// (`Rgba::NONE` means "the pixel's colour") and light amount `k`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadeOut {
    pub c: Rgba,
    pub a: f64,
    pub l: Rgba,
    pub k: f64,
}

pub struct Painter<'a> {
    frame: &'a mut Frame,
    w: f64,
    h: f64,
    ox: f64,
    oy: f64,
    tx: f64,
    ty: f64,
    fx: f64,
    cs: f64,
    sn: f64,
    /// Fade 0..1 applied to every alpha and light amount (an instance fading out after `stop`).
    pub fade: f64,
    /// Light strength applied to every light amount.
    pub light_k: f64,
}

impl<'a> Painter<'a> {
    pub fn new(frame: &'a mut Frame) -> Self {
        let mut p = Painter {
            frame,
            w: 0.0,
            h: 0.0,
            ox: 0.0,
            oy: 0.0,
            tx: 0.0,
            ty: 0.0,
            fx: 1.0,
            cs: 1.0,
            sn: 0.0,
            fade: 1.0,
            light_k: 1.0,
        };
        p.reset();
        p
    }

    /// Re-read the frame and clear placement, fade and light strength.
    pub fn reset(&mut self) -> &mut Self {
        self.w = f64::from(self.frame.w);
        self.h = f64::from(self.frame.h);
        self.ox = f64::from(self.frame.ox);
        self.oy = f64::from(self.frame.oy);
        (self.tx, self.ty, self.fx, self.cs, self.sn) = (0.0, 0.0, 1.0, 1.0, 0.0);
        self.fade = 1.0;
        self.light_k = 1.0;
        self
    }

    /// Put the effect's origin at (x, y) in frame space, mirrored if flip, rotated by angle (radians).
    pub fn place(&mut self, x: f64, y: f64, flip: bool, angle: f64) -> &mut Self {
        self.tx = x;
        self.ty = y;
        self.fx = if flip { -1.0 } else { 1.0 };
        self.cs = libm::cos(angle);
        self.sn = libm::sin(angle);
        self
    }

    // ---- transforms: local (effect) space <-> screen (frame) space
    pub fn to_x(&self, x: f64, y: f64) -> f64 {
        self.tx + x * self.fx * self.cs - y * self.sn
    }
    pub fn to_y(&self, x: f64, y: f64) -> f64 {
        self.ty + x * self.fx * self.sn + y * self.cs
    }
    pub fn from_x(&self, sx: f64, sy: f64) -> f64 {
        let (dx, dy) = (sx - self.tx, sy - self.ty);
        (dx * self.cs + dy * self.sn) * self.fx
    }
    pub fn from_y(&self, sx: f64, sy: f64) -> f64 {
        let (dx, dy) = (sx - self.tx, sy - self.ty);
        -dx * self.sn + dy * self.cs
    }
    /// The screen pixel of the local pixel cell holding (x, y): mirrors and rotations land on whole cells.
    pub fn cell_x(&self, x: f64, y: f64) -> f64 {
        libm::floor(self.to_x(libm::floor(x) + 0.5, libm::floor(y) + 0.5))
    }
    pub fn cell_y(&self, x: f64, y: f64) -> f64 {
        libm::floor(self.to_y(libm::floor(x) + 0.5, libm::floor(y) + 0.5))
    }

    /// The buffer index JS would write for screen pixel (sx, sy), if any.
    fn index(&self, sx: f64, sy: f64) -> Option<usize> {
        let (x, y) = (sx - self.ox, sy - self.oy);
        if x < 0.0 || y < 0.0 || x >= self.w || y >= self.h {
            return None;
        }
        let i = y * self.w + x;
        // a typed array ignores NaN and fractional indices
        if i == libm::floor(i) {
            Some(i as usize)
        } else {
            None
        }
    }

    // ---- raw writes, screen space, whole pixels. Colour NONE means "nothing": it never erases what's there.
    pub fn put(&mut self, sx: f64, sy: f64, c: Rgba, a: f64) {
        let c = c.pack();
        if c == 0 {
            return;
        }
        let a = a * self.fade;
        let cell = (((to_int32(sy) & 3) << 2) | (to_int32(sx) & 3)) as usize;
        if a < 1.0 && BAYER.get(cell).is_some_and(|&t| a <= t) {
            return;
        }
        if let Some(px) = self.index(sx, sy).and_then(|i| self.frame.color.get_mut(i)) {
            *px = c;
        }
    }

    /// Add light `c * k` (times fade and light strength) to a screen pixel, each channel capped at 255.
    pub fn add(&mut self, sx: f64, sy: f64, c: Rgba, k: f64) {
        let k = k * self.fade * self.light_k;
        if k <= 0.004 {
            return;
        }
        let Some(px) = self.index(sx, sy).and_then(|i| self.frame.light.get_mut(i)) else {
            return;
        };
        let d = *px;
        let ch = |s: u32, cv: u8| {
            to_int32(js_min(255.0, f64::from((d >> s) & 255) + f64::from(cv) * k)) as u32
        };
        let (r, g, b) = (ch(0, c.0), ch(8, c.1), ch(16, c.2));
        *px = (255 << 24) | (b << 16) | (g << 8) | r;
    }

    /// Blend a colour over what's there at alpha a (0..1), keeping the result's own alpha: soft smoke,
    /// vapour and mist. Use dithered [`Painter::dot`] for hard pixel fades and this for translucent volumes.
    pub fn blend_px(&mut self, sx: f64, sy: f64, c: Rgba, a: f64) {
        let a = a * self.fade;
        let c = c.pack();
        if c == 0 || a <= 0.004 {
            return;
        }
        let Some(px) = self.index(sx, sy).and_then(|i| self.frame.color.get_mut(i)) else {
            return;
        };
        let d = *px;
        let da = f64::from(d >> 24) / 255.0;
        let oa = a + da * (1.0 - a);
        let k = a / oa;
        let ch = |s: u32| {
            let v = f64::from((c >> s) & 255) * k + f64::from((d >> s) & 255) * (1.0 - k);
            (to_int32(v) & 255) as u32
        };
        let alpha = (to_int32(oa * 255.0) & 255) as u32;
        *px = (alpha << 24) | (ch(16) << 16) | (ch(8) << 8) | ch(0);
    }

    /// Translucent pixel at a local point.
    pub fn blend(&mut self, x: f64, y: f64, c: Rgba, a: f64) {
        let (sx, sy) = (self.cell_x(x, y), self.cell_y(x, y));
        self.blend_px(sx, sy, c, a);
    }

    // ---- pixels
    /// One opaque pixel.
    pub fn px(&mut self, x: f64, y: f64, c: Rgba) {
        let (sx, sy) = (self.cell_x(x, y), self.cell_y(x, y));
        self.put(sx, sy, c, 1.0);
    }

    /// One pixel at alpha a (0..1), faded by ordered dither.
    pub fn dot(&mut self, x: f64, y: f64, c: Rgba, a: f64) {
        if a > 0.0 {
            let (sx, sy) = (self.cell_x(x, y), self.cell_y(x, y));
            self.put(sx, sy, c, a);
        }
    }

    /// Visit every screen pixel whose centre falls in the local box [x0, x1] x [y0, y1], with that centre in
    /// local space: `f(painter, lx, ly, sx, sy)`. The basis of every filled shape, so shapes mirror and
    /// rotate exactly.
    pub fn each<F>(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, mut f: F)
    where
        F: FnMut(&mut Self, f64, f64, f64, f64),
    {
        let (mut ax, mut bx, mut ay, mut by) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for k in 0..4 {
            let x = if k & 1 != 0 { x1 } else { x0 };
            let y = if k & 2 != 0 { y1 } else { y0 };
            let (sx, sy) = (self.to_x(x, y), self.to_y(x, y));
            if sx < ax {
                ax = sx;
            }
            if sx > bx {
                bx = sx;
            }
            if sy < ay {
                ay = sy;
            }
            if sy > by {
                by = sy;
            }
        }
        let sx0 = js_max(libm::floor(ax), self.ox);
        let sx1 = js_min(libm::ceil(bx), self.ox + self.w - 1.0);
        let sy0 = js_max(libm::floor(ay), self.oy);
        let sy1 = js_min(libm::ceil(by), self.oy + self.h - 1.0);
        let mut sy = sy0;
        while sy <= sy1 {
            let mut sx = sx0;
            while sx <= sx1 {
                let (cx, cy) = (sx + 0.5, sy + 0.5);
                let (lx, ly) = (self.from_x(cx, cy), self.from_y(cx, cy));
                f(self, lx, ly, sx, sy);
                sx += 1.0;
            }
            sy += 1.0;
        }
    }

    /// A pixel shader over a local box. `f(x, y, out)` returns a colour, or sets `out.c` (colour), `out.a`
    /// (alpha), `out.l` (light colour, defaults to the pixel's colour) and `out.k` (light amount). Return
    /// `None` (or `Rgba::NONE`) to use `out.c`; leave it NONE to leave the pixel alone.
    pub fn shade<F>(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, mut f: F)
    where
        F: FnMut(f64, f64, &mut ShadeOut) -> Option<Rgba>,
    {
        self.each(x0, y0, x1, y1, |p, lx, ly, sx, sy| {
            let mut o = ShadeOut {
                c: Rgba::NONE,
                a: 1.0,
                l: Rgba::NONE,
                k: 0.0,
            };
            let c = match f(lx, ly, &mut o) {
                Some(c) if c != Rgba::NONE => c,
                _ => o.c,
            };
            if c != Rgba::NONE {
                p.put(sx, sy, c, o.a);
            }
            if o.k > 0.0 {
                p.add(sx, sy, if o.l != Rgba::NONE { o.l } else { c }, o.k);
            }
        });
    }

    // ---- shapes. Colours (c) and alphas (a) can be constants or functions of the shape's own coordinates.
    /// 1 px line. c and a may be functions of u (0 at the start, 1 at the end).
    pub fn line(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        c: impl Param1<Rgba>,
        a: impl Param1<f64>,
    ) {
        let (mut sx, mut sy) = (self.cell_x(x0, y0), self.cell_y(x0, y0));
        let (ex, ey) = (self.cell_x(x1, y1), self.cell_y(x1, y1));
        // pixelfx never ends a line with a non-finite end; stop here instead
        if !(sx.is_finite() && sy.is_finite() && ex.is_finite() && ey.is_finite()) {
            return;
        }
        let dx = (ex - sx).abs();
        let dy = -(ey - sy).abs();
        let stepx = if sx < ex { 1.0 } else { -1.0 };
        let stepy = if sy < ey { 1.0 } else { -1.0 };
        let n = js_max(dx, -dy);
        let n = if n == 0.0 { 1.0 } else { n };
        let mut err = dx + dy;
        let mut i = 0.0;
        loop {
            let u = i / n;
            i += 1.0;
            self.put(sx, sy, c.at(u), a.at(u));
            if sx == ex && sy == ey {
                break;
            }
            let e2 = 2.0 * err;
            if e2 >= dy {
                err += dy;
                sx += stepx;
            }
            if e2 <= dx {
                err += dx;
                sy += stepy;
            }
        }
    }

    /// Ellipse. c and a may be functions of (d: 0 centre to 1 rim, angle).
    pub fn ellipse(
        &mut self,
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        c: impl Param2<Rgba>,
        a: impl Param2<f64>,
    ) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        self.shade(cx - rx, cy - ry, cx + rx, cy + ry, |x, y, o| {
            let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
            let d2 = dx * dx + dy * dy;
            if d2 > 1.0 {
                return None;
            }
            let (d, ang) = (libm::sqrt(d2), libm::atan2(dy, dx));
            o.c = c.at(d, ang);
            o.a = a.at(d, ang);
            None
        });
    }

    /// Filled rectangle.
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Rgba, a: f64) {
        if self.sn == 0.0 {
            let (x0, y0, x1, y1) = (js_round(x), js_round(y), js_round(x + w), js_round(y + h));
            let mut yy = y0;
            while yy < y1 {
                let mut xx = x0;
                while xx < x1 {
                    let (sx, sy) = (self.cell_x(xx, yy), self.cell_y(xx, yy));
                    self.put(sx, sy, c, a);
                    xx += 1.0;
                }
                yy += 1.0;
            }
            return;
        }
        self.shade(x, y, x + w, y + h, |lx, ly, o| {
            if lx < x || ly < y || lx >= x + w || ly >= y + h {
                return None;
            }
            o.c = c;
            o.a = a;
            None
        });
    }

    /// Filled polygon from points `[(x, y), ...]` (even-odd rule). c and a may be functions of (x, y).
    pub fn poly(&mut self, pts: &[(f64, f64)], c: impl Param2<Rgba>, a: impl Param2<f64>) {
        let (mut x0, mut y0, mut x1, mut y1) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for &(x, y) in pts {
            if x < x0 {
                x0 = x;
            }
            if x > x1 {
                x1 = x;
            }
            if y < y0 {
                y0 = y;
            }
            if y > y1 {
                y1 = y;
            }
        }
        self.shade(x0, y0, x1, y1, |x, y, o| {
            let mut inside = false;
            let mut j = pts.len().wrapping_sub(1);
            for (i, &(xi, yi)) in pts.iter().enumerate() {
                if let Some(&(xj, yj)) = pts.get(j)
                    && (yi > y) != (yj > y)
                    && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi
                {
                    inside = !inside;
                }
                j = i;
            }
            if !inside {
                return None;
            }
            o.c = c.at(x, y);
            o.a = a.at(x, y);
            None
        });
    }

    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    pub fn tri(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        c: impl Param2<Rgba>,
        a: impl Param2<f64>,
    ) {
        self.poly(&[(x0, y0), (x1, y1), (x2, y2)], c, a);
    }

    // ---- light
    /// Radial glow into the light buffer: k at the centre, falling as (1 - d)^2 to 0 at radius r.
    pub fn glow(&mut self, x: f64, y: f64, r: f64, c: Rgba, k: f64) {
        if r < 1.0 || k <= 0.004 {
            return;
        }
        let (cx, cy) = (self.to_x(x, y), self.to_y(x, y));
        let (x0, x1) = (libm::floor(cx - r), libm::ceil(cx + r));
        let (y0, y1) = (libm::floor(cy - r), libm::ceil(cy + r));
        let mut sy = y0;
        while sy <= y1 {
            let dy = (sy + 0.5 - cy) / r;
            let mut sx = x0;
            while sx <= x1 {
                let dx = (sx + 0.5 - cx) / r;
                let d = dx * dx + dy * dy;
                if d < 1.0 {
                    let f = 1.0 - libm::sqrt(d);
                    self.add(sx, sy, c, f * f * k);
                }
                sx += 1.0;
            }
            sy += 1.0;
        }
    }

    /// Glow around a segment (a soft beam of light).
    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    pub fn glow_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, r: f64, c: Rgba, k: f64) {
        if r < 1.0 || k <= 0.004 {
            return;
        }
        let (ax, ay, bx, by) = (
            self.to_x(x0, y0),
            self.to_y(x0, y0),
            self.to_x(x1, y1),
            self.to_y(x1, y1),
        );
        let (dx, dy) = (bx - ax, by - ay);
        let l2 = dx * dx + dy * dy;
        let l2 = if l2 == 0.0 || l2.is_nan() { 1e-9 } else { l2 };
        let (sx0, sx1) = (
            libm::floor(js_min(ax, bx) - r),
            libm::ceil(js_max(ax, bx) + r),
        );
        let (sy0, sy1) = (
            libm::floor(js_min(ay, by) - r),
            libm::ceil(js_max(ay, by) + r),
        );
        let mut sy = sy0;
        while sy <= sy1 {
            let mut sx = sx0;
            while sx <= sx1 {
                let (px, py) = (sx + 0.5, sy + 0.5);
                let u = clamp(((px - ax) * dx + (py - ay) * dy) / l2, 0.0, 1.0);
                let (ex, ey) = (ax + dx * u - px, ay + dy * u - py);
                let d = libm::sqrt(ex * ex + ey * ey) / r;
                if d < 1.0 {
                    let f = 1.0 - d;
                    self.add(sx, sy, c, f * f * k);
                }
                sx += 1.0;
            }
            sy += 1.0;
        }
    }

    /// Glow hugging a circle of radius `big_r` (a shockwave's light), falling off over width r.
    pub fn glow_ring(&mut self, x: f64, y: f64, big_r: f64, r: f64, c: Rgba, k: f64) {
        if r < 1.0 || k <= 0.004 {
            return;
        }
        let (cx, cy) = (self.to_x(x, y), self.to_y(x, y));
        let e = big_r + r;
        let mut sy = libm::floor(cy - e);
        while sy <= libm::ceil(cy + e) {
            let mut sx = libm::floor(cx - e);
            while sx <= libm::ceil(cx + e) {
                let d = (libm::hypot(sx + 0.5 - cx, sy + 0.5 - cy) - big_r).abs() / r;
                if d < 1.0 {
                    let f = 1.0 - d;
                    self.add(sx, sy, c, f * f * k);
                }
                sx += 1.0;
            }
            sy += 1.0;
        }
    }
}
