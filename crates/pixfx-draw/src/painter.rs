//! The painter: port of pixelfx `src/core/painter.js`, every method, plus [`Painter::save`], [`Painter::restore`]
//! and [`Painter::nest`] for an effect that draws other effects inside its own space (`pixfx_core::chain`).
//!
//! The painter draws into a frame's two buffers: `color` (opaque pixels, faded with ordered dither, never
//! blended) and `light` (additive glow, composited on top). Effects draw in local coordinates around their
//! origin; the painter places, mirrors and rotates them, so an effect never needs to know where it is.
//!
//! Screen coordinates stay `f64` as in pixelfx: a write lands only where JS would index its typed array
//! (in bounds and a whole index), so NaN or fractional coordinates draw nothing, as there.

use crate::color::Rgba;
use crate::font::{Font, MINI};
use crate::frame::Frame;
use crate::mask::Mask;
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

/// A local box `[x0, x1) x [y0, y1)` and how a write is tested against it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Clip {
    /// The write's cell centre, in local space, is inside the box (pixelfx transitions `clipTo`).
    Cells([f64; 4]),
    /// pixelfx weather `clip`: for an unrotated painter the box's screen rectangle, edges rounded; otherwise
    /// as `Cells`.
    Canvas([f64; 4]),
}

/// Everything [`Painter::place`], [`Painter::nest`], the fade, the light strength and the clip set: what
/// [`Painter::save`] keeps and [`Painter::restore`] puts back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saved {
    tx: f64,
    ty: f64,
    fx: f64,
    cs: f64,
    sn: f64,
    fade: f64,
    light_k: f64,
    clip: Option<Clip>,
}

/// Where [`Painter::text`] puts `x`: the left edge, the centre or the right edge of the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// pixelfx `text` options: the font, font pixels drawn `scale` px square, the alignment, a 1 px `outline` and a
/// `shadow` 1 px down-right (none by default), alpha `a`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextOpts {
    pub font: Font,
    pub scale: f64,
    pub align: Align,
    pub outline: Option<Rgba>,
    pub shadow: Option<Rgba>,
    pub a: f64,
}

impl Default for TextOpts {
    fn default() -> Self {
        Self {
            font: MINI,
            scale: 1.0,
            align: Align::Left,
            outline: None,
            shadow: None,
            a: 1.0,
        }
    }
}

/// pixelfx `sprite` options for a sprite drawn from strings ([`Painter::sprite_rows`]): each character `scale` px
/// square, alpha `a`, mirrored left-right (`flip`) or upside down (`flip_y`), every pixel in one colour (`tint`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteOpts {
    pub scale: f64,
    pub a: f64,
    pub flip: bool,
    pub flip_y: bool,
    pub tint: Option<Rgba>,
}

impl Default for SpriteOpts {
    fn default() -> Self {
        Self {
            scale: 1.0,
            a: 1.0,
            flip: false,
            flip_y: false,
            tint: None,
        }
    }
}

pub struct Painter<'a> {
    frame: &'a mut Frame,
    w: f64,
    h: f64,
    ox: f64,
    oy: f64,
    density: f64,
    texel: f64,
    tx: f64,
    ty: f64,
    fx: f64,
    cs: f64,
    sn: f64,
    /// Fade 0..1 applied to every alpha and light amount (an instance fading out after `stop`).
    pub fade: f64,
    /// Light strength applied to every light amount.
    pub light_k: f64,
    /// The box writes are kept inside, if any (see [`Painter::clip_local`], [`Painter::clip_box`]).
    clip: Option<Clip>,
}

impl<'a> Painter<'a> {
    pub fn new(frame: &'a mut Frame) -> Self {
        let mut p = Painter {
            frame,
            w: 0.0,
            h: 0.0,
            ox: 0.0,
            oy: 0.0,
            density: 1.0,
            texel: 1.0,
            tx: 0.0,
            ty: 0.0,
            fx: 1.0,
            cs: 1.0,
            sn: 0.0,
            fade: 1.0,
            light_k: 1.0,
            clip: None,
        };
        p.reset();
        p
    }

    /// Re-read the frame and clear placement, fade, light strength and clip.
    pub fn reset(&mut self) -> &mut Self {
        self.w = f64::from(self.frame.w);
        self.h = f64::from(self.frame.h);
        self.ox = f64::from(self.frame.ox);
        self.oy = f64::from(self.frame.oy);
        self.density = f64::from(self.frame.density().factor());
        self.texel = 1.0 / self.density;
        (self.tx, self.ty, self.fx, self.cs, self.sn) = (0.0, 0.0, 1.0, 1.0, 0.0);
        self.fade = 1.0;
        self.light_k = 1.0;
        self.clip = None;
        self
    }

    /// Put the effect's origin at (x, y) in frame space, mirrored if flip, rotated by angle (radians).
    pub fn place(&mut self, x: f64, y: f64, flip: bool, angle: f64) -> &mut Self {
        self.tx = x * self.density;
        self.ty = y * self.density;
        self.fx = if flip { -1.0 } else { 1.0 };
        self.cs = libm::cos(angle);
        self.sn = libm::sin(angle);
        self
    }

    /// The placement, fade, light strength and clip as they are now, for [`Painter::restore`].
    pub fn save(&self) -> Saved {
        Saved {
            tx: self.tx,
            ty: self.ty,
            fx: self.fx,
            cs: self.cs,
            sn: self.sn,
            fade: self.fade,
            light_k: self.light_k,
            clip: self.clip,
        }
    }

    /// Put back what [`Painter::save`] kept (pixelfx chain's `Object.assign(p, { tx, ty, fx, cs, sn, fade, lightK })`).
    pub fn restore(&mut self, s: Saved) -> &mut Self {
        (self.tx, self.ty, self.fx, self.cs, self.sn) = (s.tx, s.ty, s.fx, s.cs, s.sn);
        self.fade = s.fade;
        self.light_k = s.light_k;
        self.clip = s.clip;
        self
    }

    /// Move the origin to the local point (`x`, `y`), mirrored once more if `flip`, keeping the rotation: what is
    /// drawn next sits inside the current effect's space. Unrotated, this is pixelfx chain's placement of a cue
    /// (`x = tx + fx * x`, `y = ty + y`, `flip = fx < 0 !== flip`), bit for bit.
    pub fn nest(&mut self, x: f64, y: f64, flip: bool) -> &mut Self {
        let (tx, ty) = (self.to_x(x, y), self.to_y(x, y));
        self.tx = tx;
        self.ty = ty;
        if flip {
            self.fx = -self.fx;
        }
        self
    }

    /// Drop every write whose local pixel cell lies outside `[0, w) x [0, h)`: a full-screen effect's sparks,
    /// smoke and light stop at its edge instead of spilling past it (pixelfx transitions `clipTo`, a painter
    /// whose `put`, `add` and `blendPx` check `inside` first). Follows later placement, and lasts until
    /// [`Painter::reset`] or the end of this painter's use.
    pub fn clip_local(&mut self, w: f64, h: f64) -> &mut Self {
        self.clip = Some(Clip::Cells([0.0, 0.0, w, h]));
        self
    }

    /// Pixelfx weather `clip`: keep writes inside the local box `[x0, x1) x [y0, y1)`, the canvas around the
    /// effect's origin (`x0 = -origin_x`). Unlike [`Painter::clip_local`], an unrotated painter keeps the
    /// screen columns `round(tx + x0) .. round(tx + x1)` (and rows alike), which at a half-pixel placement
    /// is not the cells whose centres are inside the box; a rotated one keeps cell centres.
    pub fn clip_box(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) -> &mut Self {
        self.clip = Some(Clip::Canvas([x0, y0, x1, y1]));
        self
    }

    /// Whether the clip drops a write to screen pixel (sx, sy). A NaN compares false and is dropped, as in
    /// pixelfx.
    fn clipped(&self, sx: f64, sy: f64) -> bool {
        let inside = |[x0, y0, x1, y1]: [f64; 4]| {
            let (lx, ly) = (
                self.from_x(sx + 0.5, sy + 0.5),
                self.from_y(sx + 0.5, sy + 0.5),
            );
            lx >= x0 && lx < x1 && ly >= y0 && ly < y1
        };
        match self.clip {
            None => false,
            Some(Clip::Cells(b)) => !inside(b),
            Some(Clip::Canvas(b)) if self.sn != 0.0 => !inside(b),
            Some(Clip::Canvas([x0, y0, x1, y1])) => {
                let (a, b) = (self.to_x(x0, 0.0), self.to_x(x1, 0.0));
                let (sx0, sx1) = (js_round(js_min(a, b)), js_round(js_max(a, b)));
                let (sy0, sy1) = (js_round(self.to_y(0.0, y0)), js_round(self.to_y(0.0, y1)));
                !(sx >= sx0 && sx < sx1 && sy >= sy0 && sy < sy1)
            }
        }
    }

    // ---- transforms: local (effect) space <-> screen (frame) space
    pub fn to_x(&self, x: f64, y: f64) -> f64 {
        self.tx + (x * self.density) * self.fx * self.cs - (y * self.density) * self.sn
    }
    pub fn to_y(&self, x: f64, y: f64) -> f64 {
        self.ty + (x * self.density) * self.fx * self.sn + (y * self.density) * self.cs
    }
    pub fn from_x(&self, sx: f64, sy: f64) -> f64 {
        let (dx, dy) = (sx - self.tx, sy - self.ty);
        (dx * self.cs + dy * self.sn) * self.fx * self.texel
    }
    pub fn from_y(&self, sx: f64, sy: f64) -> f64 {
        let (dx, dy) = (sx - self.tx, sy - self.ty);
        (-dx * self.sn + dy * self.cs) * self.texel
    }
    /// The farthest a local point can be from the effect's origin and still land on the frame: the distance
    /// from the placed origin to the frame's farthest corner. Placement only mirrors and rotates, which keep
    /// distances, so a shape whose every local point is farther than this draws nothing; an effect may skip
    /// it (a tall flame's rows past the frame) without changing a pixel. NaN when the placement is NaN.
    pub fn reach(&self) -> f64 {
        let far = |lo: f64, size: f64, at: f64| js_max((at - lo).abs(), (lo + size - at).abs());
        libm::hypot(far(self.ox, self.w, self.tx), far(self.oy, self.h, self.ty)) * self.texel
    }

    /// The screen pixel of the local pixel cell holding (x, y): mirrors and rotations land on whole cells.
    pub fn cell_x(&self, x: f64, y: f64) -> f64 {
        libm::floor(
            self.tx + self.cell_center(x) * self.fx * self.cs - self.cell_center(y) * self.sn,
        )
    }
    pub fn cell_y(&self, x: f64, y: f64) -> f64 {
        libm::floor(
            self.ty + self.cell_center(x) * self.fx * self.sn + self.cell_center(y) * self.cs,
        )
    }

    fn cell_center(&self, value: f64) -> f64 {
        libm::floor(value * self.density) + 0.5
    }

    /// Visit the raster texels in the game cell containing `(x, y)`.
    pub fn each_game_cell(&mut self, x: f64, y: f64, mut f: impl FnMut(&mut Self, f64, f64)) {
        if self.density == 1.0 {
            f(self, self.cell_x(x, y), self.cell_y(x, y));
            return;
        }
        let (x, y) = (libm::floor(x), libm::floor(y));
        self.each(x, y, x + 1.0, y + 1.0, |p, lx, ly, sx, sy| {
            if (x..x + 1.0).contains(&lx) && (y..y + 1.0).contains(&ly) {
                f(p, sx, sy);
            }
        });
    }

    /// Draw one whole game cell from a pre-existing pixel mask.
    pub fn paint_cell(&mut self, x: f64, y: f64, c: Rgba, a: f64) {
        self.each_game_cell(x, y, |p, sx, sy| p.put(sx, sy, c, a));
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
        if self.clipped(sx, sy) {
            return;
        }
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
        if self.clipped(sx, sy) {
            return;
        }
        let k = k * self.fade * self.light_k;
        if k <= 0.004 {
            return;
        }
        if let Some(i) = self.index(sx, sy) {
            self.add_at(i, c, k);
        }
    }

    /// [`Painter::add`] at a light index, with `k` already scaled by fade and light strength.
    fn add_at(&mut self, i: usize, c: Rgba, k: f64) {
        let Some(px) = self.frame.light.get_mut(i) else {
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
        if self.clipped(sx, sy) {
            return;
        }
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

    /// Read back the packed colour word under a local point (0 if empty or out of bounds).
    pub fn get(&self, x: f64, y: f64) -> u32 {
        let (sx, sy) = (self.cell_x(x, y), self.cell_y(x, y));
        self.index(sx, sy)
            .and_then(|i| self.frame.color.get(i).copied())
            .unwrap_or(0)
    }

    /// Cluster cleanup on the frame's colour buffer: any opaque pixel whose 4-neighbours are all opaque and none
    /// matches its own colour (plus, when `diagonals` is true, all 4 diagonal neighbours are also non-empty) takes
    /// its 4-neighbours' most common colour. Reads from a copy per pass so scan order never matters.
    pub fn clean_flecks(&mut self, passes: u32, diagonals: bool) {
        let (w, h) = (self.frame.w as usize, self.frame.h as usize);
        if w < 3 || h < 3 {
            return;
        }
        for _ in 0..passes {
            let src = self.frame.color.clone();
            let mut changed = false;
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let i = y * w + x;
                    let c = src.get(i).copied().unwrap_or(0);
                    if c >> 24 != 255 {
                        continue;
                    }
                    let n4 = [
                        src.get(i - 1).copied().unwrap_or(0),
                        src.get(i + 1).copied().unwrap_or(0),
                        src.get(i - w).copied().unwrap_or(0),
                        src.get(i + w).copied().unwrap_or(0),
                    ];
                    if n4.iter().any(|&v| v >> 24 != 255 || v == c) {
                        continue;
                    }
                    if diagonals {
                        let d4 = [
                            src.get(i - w - 1).copied().unwrap_or(0),
                            src.get(i - w + 1).copied().unwrap_or(0),
                            src.get(i + w - 1).copied().unwrap_or(0),
                            src.get(i + w + 1).copied().unwrap_or(0),
                        ];
                        if d4.contains(&0) {
                            continue;
                        }
                    }
                    let mut best = n4[0];
                    let mut best_n = 0u32;
                    for &a in &n4 {
                        let count = n4.iter().filter(|&&b| b == a).count() as u32;
                        if count > best_n || (!diagonals && count == best_n && a < best) {
                            best = a;
                            best_n = count;
                        }
                    }
                    if let Some(dst) = self.frame.color.get_mut(i) {
                        *dst = best;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
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

    /// Stamp a straight-alpha RGBA8 sprite at its local top-left corner.
    /// Samples nearest neighbours and applies the painter's transform, clip and ordered-dither fade.
    /// A buffer whose length does not match the dimensions is ignored.
    pub fn sprite(&mut self, x: f64, y: f64, w: u32, h: u32, rgba: &[u8]) {
        let pixels = rgba.as_chunks::<4>();
        if !pixels.1.is_empty() || (w as usize).checked_mul(h as usize) != Some(pixels.0.len()) {
            return;
        }
        let (width, height) = (f64::from(w), f64::from(h));
        self.each(x, y, x + width, y + height, |p, lx, ly, sx, sy| {
            let (dx, dy) = (lx - x, ly - y);
            if (0.0..width).contains(&dx)
                && (0.0..height).contains(&dy)
                && let Some(&[r, g, b, a]) = pixels.0.get(dy as usize * w as usize + dx as usize)
                && a != 0
            {
                p.put(sx, sy, Rgba(r, g, b, 255), f64::from(a) / 255.0);
            }
        });
    }

    /// Stamp a smooth RGBA sprite. `pitch` is the game-space width of one source texel.
    /// Premultiplied interpolation preserves coverage without pulling colour out of transparent texels.
    pub fn sprite_smooth(&mut self, x: f64, y: f64, w: u32, h: u32, pitch: f64, rgba: &[u8]) {
        let pixels = rgba.as_chunks::<4>();
        if !(pitch > 0.0 && pitch.is_finite())
            || !pixels.1.is_empty()
            || (w as usize).checked_mul(h as usize) != Some(pixels.0.len())
        {
            return;
        }
        let (width, height) = (f64::from(w) * pitch, f64::from(h) * pitch);
        let get = |col: i64, row: i64| {
            if (0..i64::from(w)).contains(&col) && (0..i64::from(h)).contains(&row) {
                pixels.0[row as usize * w as usize + col as usize]
            } else {
                [0; 4]
            }
        };
        self.each(x, y, x + width, y + height, |p, lx, ly, sx, sy| {
            let (dx, dy) = (lx - x, ly - y);
            if !(0.0..width).contains(&dx) || !(0.0..height).contains(&dy) {
                return;
            }
            let (u, v) = (dx / pitch - 0.5, dy / pitch - 0.5);
            let (col, row) = (libm::floor(u), libm::floor(v));
            let (fx, fy) = (u - col, v - row);
            let (col, row) = (col as i64, row as i64);
            let (mut alpha, mut rgb) = (0.0, [0.0; 3]);
            for (sample, weight) in [
                (get(col, row), (1.0 - fx) * (1.0 - fy)),
                (get(col + 1, row), fx * (1.0 - fy)),
                (get(col, row + 1), (1.0 - fx) * fy),
                (get(col + 1, row + 1), fx * fy),
            ] {
                let coverage = f64::from(sample[3]) * weight;
                alpha += coverage;
                for (out, channel) in rgb.iter_mut().zip(sample) {
                    *out += f64::from(channel) * coverage;
                }
            }
            if alpha > 0.0 {
                let [r, g, b] = rgb.map(|channel| js_round(channel / alpha) as u8);
                p.blend_px(sx, sy, Rgba(r, g, b, 255), alpha / 255.0);
            }
        });
    }

    /// Width of a raster texel in game units.
    pub fn texel(&self) -> f64 {
        self.texel
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
        self.line_by(x0, y0, x1, y1, &c, &a);
    }

    /// [`Painter::line`] with its colour and alpha borrowed, so [`Painter::polyline`] reuses them per segment.
    fn line_by<C: Param1<Rgba>, A: Param1<f64>>(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        c: &C,
        a: &A,
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

    /// A translucent disc: puffs of smoke, vapour, dust. c and a may be functions of (d: 0 centre to 1 rim,
    /// angle).
    pub fn disc(&mut self, cx: f64, cy: f64, r: f64, c: impl Param2<Rgba>, a: impl Param2<f64>) {
        if r * self.density < 0.5 {
            return self.blend(cx, cy, c.at(0.0, 0.0), a.at(0.0, 0.0));
        }
        self.each(cx - r, cy - r, cx + r, cy + r, |p, x, y, sx, sy| {
            let (dx, dy) = (x - cx, y - cy);
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                return;
            }
            let (d, ang) = (libm::sqrt(d2) / r, libm::atan2(dy, dx));
            p.blend_px(sx, sy, c.at(d, ang), a.at(d, ang));
        });
    }

    /// Filled disc. c and a may be functions of (d: 0 centre to 1 rim, angle).
    pub fn circle(&mut self, cx: f64, cy: f64, r: f64, c: impl Param2<Rgba>, a: impl Param2<f64>) {
        if r * self.density < 0.6 {
            return self.dot(cx, cy, c.at(0.0, 0.0), a.at(0.0, 0.0));
        }
        self.shade(cx - r, cy - r, cx + r, cy + r, |x, y, o| {
            let (dx, dy) = (x - cx, y - cy);
            let d2 = dx * dx + dy * dy;
            if d2 > r * r {
                return None;
            }
            let (d, ang) = (libm::sqrt(d2) / r, libm::atan2(dy, dx));
            o.c = c.at(d, ang);
            o.a = a.at(d, ang);
            None
        });
    }

    /// Ring from radius r - w to r. c and a may be functions of (v: 0 inner to 1 outer, angle).
    pub fn ring(
        &mut self,
        cx: f64,
        cy: f64,
        r: f64,
        w: f64,
        c: impl Param2<Rgba>,
        a: impl Param2<f64>,
    ) {
        let r0 = js_max(0.0, r - w);
        self.shade(cx - r, cy - r, cx + r, cy + r, |x, y, o| {
            let (dx, dy) = (x - cx, y - cy);
            let d2 = dx * dx + dy * dy;
            if d2 > r * r || d2 < r0 * r0 {
                return None;
            }
            let v = if w > 0.0 {
                // JS `r - r0 || 1`: a zero (or NaN) span counts as 1
                let span = r - r0;
                (libm::sqrt(d2) - r0)
                    / if span == 0.0 || span.is_nan() {
                        1.0
                    } else {
                        span
                    }
            } else {
                1.0
            };
            let ang = libm::atan2(dy, dx);
            o.c = c.at(v, ang);
            o.a = a.at(v, ang);
            None
        });
    }

    /// Add light to one pixel at a local point.
    pub fn light(&mut self, x: f64, y: f64, c: Rgba, k: f64) {
        let (sx, sy) = (self.cell_x(x, y), self.cell_y(x, y));
        self.add(sx, sy, c, k);
    }

    /// Keep only a share `keep` (0..1) of the light already on a screen pixel (zero light stays zero): the pixel
    /// primitive of pixelfx shadow `unlight`, which writes the light buffer directly (no clip, fade or strength).
    pub fn dim_light(&mut self, sx: f64, sy: f64, keep: f64) {
        let Some(px) = self.index(sx, sy).and_then(|i| self.frame.light.get_mut(i)) else {
            return;
        };
        let d = *px;
        if d == 0 {
            return;
        }
        if keep <= 0.0 {
            *px = 0;
            return;
        }
        let ch = |s: u32| to_int32(f64::from((d >> s) & 255) * keep) as u32;
        *px = (255 << 24) | (ch(16) << 16) | (ch(8) << 8) | ch(0);
    }

    /// Filled rectangle.
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Rgba, a: f64) {
        if self.sn == 0.0 && self.density == 1.0 {
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

    /// Lines through a list of points (pixelfx `polyline`). c and a may be functions of u along each segment.
    pub fn polyline(&mut self, pts: &[(f64, f64)], c: impl Param1<Rgba>, a: impl Param1<f64>) {
        for w in pts.windows(2) {
            let ((x0, y0), (x1, y1)) = (w[0], w[1]);
            self.line_by(x0, y0, x1, y1, &c, &a);
        }
    }

    /// Thick segment with round ends. c and a may be functions of (u along, v across: 0 centre, 1 edge).
    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    pub fn beam(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        width: f64,
        c: impl Param2<Rgba>,
        a: impl Param2<f64>,
    ) {
        let r = width / 2.0;
        let (dx, dy) = (x1 - x0, y1 - y0);
        let l2 = dx * dx + dy * dy;
        // JS `|| 1e-9`
        let l2 = if l2 == 0.0 || l2.is_nan() { 1e-9 } else { l2 };
        self.shade(
            js_min(x0, x1) - r,
            js_min(y0, y1) - r,
            js_max(x0, x1) + r,
            js_max(y0, y1) + r,
            |x, y, o| {
                let u = clamp(((x - x0) * dx + (y - y0) * dy) / l2, 0.0, 1.0);
                let (px, py) = (x0 + dx * u - x, y0 + dy * u - y);
                let d2 = px * px + py * py;
                if d2 > r * r {
                    return None;
                }
                let v = libm::sqrt(d2) / r;
                o.c = c.at(u, v);
                o.a = a.at(u, v);
                None
            },
        );
    }

    /// Band of a ring from angle `a0` to `a1` (radians, a1 > a0), radius r - w to r: the slash and smear shape.
    /// c and a may be functions of (u: 0 at a0 to 1 at a1, v: 0 inner to 1 outer). w may be a function of u for a
    /// crescent that tapers at both ends.
    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    pub fn arc(
        &mut self,
        cx: f64,
        cy: f64,
        r: f64,
        w: impl Param1<f64>,
        a0: f64,
        a1: f64,
        c: impl Param2<Rgba>,
        a: impl Param2<f64>,
    ) {
        let span = a1 - a0;
        let pi = core::f64::consts::PI;
        self.shade(cx - r, cy - r, cx + r, cy + r, |x, y, o| {
            let (dx, dy) = (x - cx, y - cy);
            let d = libm::sqrt(dx * dx + dy * dy);
            if d > r {
                return None;
            }
            let mut ang = libm::atan2(dy, dx) - a0;
            ang -= libm::floor(ang / (pi * 2.0)) * pi * 2.0;
            if ang > span {
                return None;
            }
            let u = ang / span;
            let ww = w.at(u);
            if ww <= 0.0 || d < r - ww {
                return None;
            }
            let v = (d - (r - ww)) / ww;
            o.c = c.at(u, v);
            o.a = a.at(u, v);
            None
        });
    }

    // ---- sprites, text, masks
    /// A little sprite from strings (pixelfx `sprite`): rows like `["..##..", ".#oo#."]`, `pal` maps characters to
    /// colours (a character it lacks, or one mapped to `Rgba::NONE`, is empty; so are `.` and ` `). Top-left at
    /// (x, y). Not [`Painter::sprite`], which stamps RGBA bytes.
    pub fn sprite_rows(
        &mut self,
        rows: &[&str],
        pal: &[(char, Rgba)],
        x: f64,
        y: f64,
        o: &SpriteOpts,
    ) {
        let (k, a) = (o.scale, o.a);
        let w = rows.first().map_or(0, |r| r.chars().count()) as f64;
        let h = rows.len() as f64;
        for (r, row) in rows.iter().enumerate() {
            for (q, ch) in row.chars().enumerate() {
                if ch == '.' || ch == ' ' {
                    continue;
                }
                let c = match o.tint {
                    Some(c) => c,
                    None => pal
                        .iter()
                        .find(|(k, _)| *k == ch)
                        .map_or(Rgba::NONE, |&(_, c)| c),
                };
                if c.pack() == 0 {
                    continue;
                }
                let col = if o.flip { w - 1.0 - q as f64 } else { q as f64 };
                let row = if o.flip_y {
                    h - 1.0 - r as f64
                } else {
                    r as f64
                };
                if k == 1.0 {
                    self.dot(x + col, y + row, c, a);
                } else {
                    self.rect(x + col * k, y + row * k, k, k, c, a);
                }
            }
        }
    }

    /// Pixel text (pixelfx `text`): (x, y) is the top of the text, at its left, centre or right ([`TextOpts`]).
    /// c may be a function of the font pixel (gx, gy).
    pub fn text(&mut self, s: &str, x: f64, y: f64, c: impl Param2<Rgba>, o: &TextOpts) {
        let (font, k, a) = (o.font, o.scale, o.a);
        let w = f64::from(font.width(s)) * k;
        let left = js_round(match o.align {
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
            Align::Left => x,
        });
        let top = js_round(y);
        if let Some(ol) = o.outline {
            font.raster(s, |gx, gy| {
                let (gx, gy) = (f64::from(gx), f64::from(gy));
                self.rect(
                    left + gx * k - 1.0,
                    top + gy * k - 1.0,
                    k + 2.0,
                    k + 2.0,
                    ol,
                    a,
                );
            });
        }
        if let Some(sh) = o.shadow {
            font.raster(s, |gx, gy| {
                let (gx, gy) = (f64::from(gx), f64::from(gy));
                self.rect(left + gx * k + 1.0, top + gy * k + 1.0, k, k, sh, a);
            });
        }
        font.raster(s, |gx, gy| {
            let (gx, gy) = (f64::from(gx), f64::from(gy));
            self.rect(left + gx * k, top + gy * k, k, k, c.at(gx, gy), a);
        });
    }

    /// Size of text in pixels, `(w, h)` (pixelfx `measure`).
    pub fn measure(s: &str, font: Font, scale: f64) -> (f64, f64) {
        (f64::from(font.width(s)) * scale, f64::from(font.h) * scale)
    }

    /// Fill a mask with its top-left at (x, y) (pixelfx `mask`). c and a may be functions of the cell (mx, my).
    pub fn mask(
        &mut self,
        m: &impl Mask,
        x: f64,
        y: f64,
        c: impl Param2<Rgba>,
        a: impl Param2<f64>,
    ) {
        for my in 0..m.h() {
            for mx in 0..m.w() {
                if m.solid(mx, my) {
                    let (fx, fy) = (f64::from(mx), f64::from(my));
                    self.dot(x + fx, y + fy, c.at(fx, fy), a.at(fx, fy));
                }
            }
        }
    }

    /// The 1 px outline just outside a mask with its top-left at (x, y) (pixelfx `outline`). c and a may be
    /// functions of the outline cell (mx, my), which runs from -1 to the mask's w and h.
    pub fn outline(
        &mut self,
        m: &impl Mask,
        x: f64,
        y: f64,
        c: impl Param2<Rgba>,
        a: impl Param2<f64>,
    ) {
        let (w, h) = (i64::from(m.w()), i64::from(m.h()));
        let at = |mx: i64, my: i64| {
            mx >= 0 && my >= 0 && mx < w && my < h && m.solid(mx as u32, my as u32)
        };
        for my in -1..=h {
            for mx in -1..=w {
                if !at(mx, my)
                    && (at(mx - 1, my) || at(mx + 1, my) || at(mx, my - 1) || at(mx, my + 1))
                {
                    let (fx, fy) = (mx as f64, my as f64);
                    self.dot(x + fx, y + fy, c.at(fx, fy), a.at(fx, fy));
                }
            }
        }
    }

    // ---- light
    /// Radial glow into the light buffer: k at the centre, falling as (1 - d)^2 to 0 at radius r.
    pub fn glow(&mut self, x: f64, y: f64, r: f64, c: Rgba, k: f64) {
        let r = r * self.density;
        if r < 1.0 || k <= 0.004 {
            return;
        }
        let (cx, cy) = (self.to_x(x, y), self.to_y(x, y));
        // Only pixels that can change the frame are visited: those inside the frame, and those near enough
        // the centre that `add` keeps their strength. Every visited pixel still runs the exact test and `add`,
        // and a skipped pixel is one `add` would drop with no effect, so the frame is the same as visiting all.
        let reach = self.glow_reach(k);
        if reach <= 0.0 {
            return;
        }
        let (x0, x1) = (
            js_max(libm::floor(cx - r), self.ox),
            js_min(libm::ceil(cx + r), self.ox + self.w - 1.0),
        );
        let (y0, y1) = (
            js_max(libm::floor(cy - r), self.oy),
            js_min(libm::ceil(cy + r), self.oy + self.h - 1.0),
        );
        let reach2 = reach * reach;
        let mut sy = y0;
        while sy <= y1 {
            let dy = (sy + 0.5 - cy) / r;
            let room = reach2 - dy * dy;
            if room > 0.0 {
                // pixel centres within `half` of cx on this row; one pixel of slack each side for rounding
                let half = libm::sqrt(room) * r;
                let mut sx = js_max(x0, libm::floor(cx - half - 0.5) - 1.0);
                let end = js_min(x1, libm::ceil(cx + half - 0.5) + 1.0);
                // sx and sy are whole and inside the frame here, so `index` is this row's start plus the column
                let row = (sy - self.oy) * self.w - self.ox;
                while sx <= end {
                    let dx = (sx + 0.5 - cx) / r;
                    let d = dx * dx + dy * dy;
                    if d < 1.0 {
                        let f = 1.0 - libm::sqrt(d);
                        // `add`, with the index known (a NaN strength is written, as `add` writes it)
                        let kk = f * f * k * self.fade * self.light_k;
                        if (kk > 0.004 || kk.is_nan()) && !self.clipped(sx, sy) {
                            self.add_at((row + sx) as usize, c, kk);
                        }
                    }
                    sx += 1.0;
                }
            }
            sy += 1.0;
        }
    }

    /// For a glow of strength `k`: the distance (as a fraction of its radius) beyond which `add` drops every
    /// step, widened a little so rounding never cuts a pixel `add` would keep. `add` keeps a step when
    /// `k·f²·fade·light_k > 0.004` with `f = 1 - distance`, so the cut is `1 - sqrt(0.004 / (k·fade·light_k))`.
    /// 0 or less: the glow adds nothing anywhere. NaN or infinite strengths visit the whole radius (1).
    fn glow_reach(&self, k: f64) -> f64 {
        let kt = k * self.fade * self.light_k;
        if !kt.is_finite() {
            return 1.0;
        }
        if kt <= 0.004 {
            return 0.0;
        }
        js_min(1.0, 1.0 - 0.999_999 * libm::sqrt(0.004 / kt))
    }

    /// Glow around a segment (a soft beam of light).
    #[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
    pub fn glow_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, r: f64, c: Rgba, k: f64) {
        let r = r * self.density;
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
        let (big_r, r) = (big_r * self.density, r * self.density);
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
