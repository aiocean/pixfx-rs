//! The drawn fighter (site `duel-art.js` paint, blit, outline): its pose painted into a 64x64 sprite at one
//! pixel per game pixel, outlined there (dark, or in its glow while it gathers a spell), then put into a
//! [`Frame`] in world space with its gathering aura, a dodge's afterimage, a held spell's tremble and the
//! quarter turns of a tumble or roll.
//!
//! The sprite is kept premultiplied, 8 bits a channel, as a browser canvas keeps it: a fighter blown along by
//! a blow streams its cloth by a fraction of a pixel, and those rects are filled with the coverage Chrome's
//! raster gives their edge pixels, so the picture is the site's to the pixel.

use crate::look::{Hairdo, Look};
use crate::pose::{Gather, Pose};
use core::f64::consts::PI;
use libm::{cos, floor, sin, sqrt};
use pixfx_core::squash::{self, Shape, SquashSpec};
use pixfx_draw::blend::{cover_over, mul_div255, over_premul, straighten};
use pixfx_draw::color::mix;
use pixfx_draw::math::{js_max, js_min, js_round};
use pixfx_draw::{Frame, PixelDensity, Rgba, RgbaImage};

/// How many sprite pixels a pixel of the poses' drawing grid takes: the poses are laid out on a 40-row grid and
/// every part lands on the sprite's own pixels through this one factor, so the standing fighter is 64 rows tall.
pub const SCALE: f64 = 1.6;
/// Sprite width and height, in pixels.
pub const SPRITE: usize = 104;
/// Where the feet are in the sprite: x of the body's middle...
pub const CX: i32 = 52;
/// ...and y of the soles.
pub const FEET: i32 = 96;

/// A length on the poses' drawing grid, in sprite pixels.
fn sized(grid: f64) -> f64 {
    js_round(grid * SCALE)
}

const WHITE: Rgba = Rgba::hex24(0xffffff);
/// A frozen fighter is drawn in ice; a shocked one flickers with lightning.
const ICE_LIGHT: Rgba = Rgba::hex24(0xe8fbff);
const ICE_MID: Rgba = Rgba::hex24(0xa6ecff);
const ICE_DARK: Rgba = Rgba::hex24(0x5fa8d8);
const SHOCK: Rgba = Rgba::hex24(0xfff6a0);

/// A spell or blow being gathered: how far, and whether the wind-up is a long one (0.3 s or more), whose last
/// fifth trembles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Windup {
    /// 0 to 1.
    pub progress: f64,
    pub long: bool,
}

/// What a fighter is doing with its hands, as the caller's rules resolve it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "ActWire"))]
pub struct Act {
    /// How it gathers; the site gives blade blows `Palm`.
    pub how: Gather,
    /// A blade blow from the air, drawn point down to plunge.
    pub dive: bool,
    /// Set while it is still winding up.
    pub windup: Option<Windup>,
}

/// [`Act`] as JS passes it: the windup flattened, `progress` null once it is no longer winding up.
#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(tsify::Tsify),
    tsify(rename = "Act", missing_as_null)
)]
#[serde(deny_unknown_fields)]
struct ActWire {
    how: Gather,
    dive: bool,
    /// 0 to 1 while winding up, else null.
    progress: Option<f64>,
    long: bool,
}

#[cfg(feature = "serde")]
impl From<ActWire> for Act {
    fn from(w: ActWire) -> Self {
        Act {
            how: w.how,
            dive: w.dive,
            windup: w.progress.map(|progress| Windup {
                progress,
                long: w.long,
            }),
        }
    }
}

/// Everything the painter reads off a fighter.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(
    feature = "serde",
    serde(rename_all = "camelCase", deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", tsify(missing_as_null))]
pub struct Appearance {
    pub pose: Pose,
    /// Seconds since the pose began.
    pub pose_t: f64,
    /// 1 facing right, -1 facing left.
    pub face: f64,
    /// Feet, in world pixels: x across, `air` up from the ground.
    pub x: f64,
    pub air: f64,
    /// Horizontal speed, pixels a second: cloth streams back from it, a tumble turns its way.
    pub vx: f64,
    /// Vertical speed, pixels a second: a body rising or falling fast stretches along it.
    pub vy: f64,
    /// Knocked out: crossed-out eyes, sword dropped, no afterimage.
    pub out: bool,
    /// Drawn white for a hit's flash.
    pub flash: bool,
    /// Nothing can touch it (on the floor, its grace after getting up, a spent juggle): it blinks, so the player
    /// sees the protection the body's rules give it.
    pub untouchable: bool,
    /// Drawn in ice, and still.
    pub frozen: bool,
    /// Flickering with lightning.
    pub shocked: bool,
    /// Seconds since it was last struck, and how hard (a reel is lower from 9 up).
    pub hit_t: f64,
    pub hit_power: f64,
    /// Seconds of a dodge's afterimage left (fades from 0.3).
    pub dodged: f64,
    pub act: Option<Act>,
    /// Whole pixels the screen's answer to a blow jolts it off its place.
    pub jolt: [i32; 2],
}

/// A painted fighter, premultiplied RGBA8, with feet at ([`CX`], [`FEET`]) in game pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    px: Vec<[u8; 4]>,
    density: PixelDensity,
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new(PixelDensity::One)
    }
}

impl Sprite {
    fn new(density: PixelDensity) -> Self {
        let side = SPRITE * density.factor() as usize;
        Self {
            px: vec![[0; 4]; side * side],
            density,
        }
    }

    fn side(&self) -> usize {
        SPRITE * self.density.factor() as usize
    }

    /// The whole sprite as a straight-alpha RGBA image.
    pub fn to_image(&self) -> RgbaImage {
        RgbaImage {
            w: self.side() as u32,
            h: self.side() as u32,
            rgba: self
                .px
                .iter()
                .flat_map(|&p| {
                    let c = straighten(p);
                    [c.0, c.1, c.2, c.3]
                })
                .collect(),
        }
    }

    /// The pixel at (x, y) as a straight (not premultiplied) colour; `None` outside the sprite.
    pub fn pixel(&self, x: usize, y: usize) -> Option<Rgba> {
        let side = self.side();
        if x >= side || y >= side {
            return None;
        }
        self.px.get(y * side + x).map(|&p| straighten(p))
    }

    fn alpha(&self, x: i32, y: i32) -> u8 {
        let side = self.side();
        let inside = (0..side as i32).contains(&x) && (0..side as i32).contains(&y);
        if !inside {
            return 0;
        }
        self.px
            .get(y as usize * side + x as usize)
            .map_or(0, |p| p[3])
    }

    /// Fill a rect as a canvas `fillRect` with an opaque colour does on the CPU: whole pixels where it covers
    /// them, and source-over at Skia's 8-bit coverage where an edge falls inside a pixel. Edges fall across x
    /// only (fractional cloth); y is taken in whole pixels.
    fn fill_rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Rgba) {
        if !(w > 0.0 && h > 0.0) {
            return;
        }
        // Skia: float rect -> 16.16 fixed -> 24.8 "dot8"; an edge pixel's coverage is the 1/256ths it covers
        let dot8 = |v: f32| ((v * 65536.0) as i32 + 0x80) >> 8;
        let (l, r) = (dot8(x as f32), dot8(x as f32 + w as f32));
        let (y0, y1) = (floor(y) as i32, floor(y + h) as i32);
        let side = self.side();
        for row in y0.max(0)..y1.min(side as i32) {
            let (first, last) = (l >> 8, (r - 1) >> 8);
            for col in first.max(0)..=last.min(side as i32 - 1) {
                let cover = if first == last {
                    r - l
                } else if col == first {
                    256 - (l & 255)
                } else if col == last {
                    r - (last << 8)
                } else {
                    256
                };
                if let Some(p) = self.px.get_mut(row as usize * side + col as usize) {
                    *p = cover_over(*p, c, cover);
                }
            }
        }
    }
}

/// JS `Math.sign(v) || fallback`.
fn sign_or(v: f64, fallback: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        fallback
    }
}

/// Relative lightness of a colour, 0 to 1 (site `lum`).
fn lum(c: Rgba) -> f64 {
    (0.3 * f64::from(c.0) + 0.59 * f64::from(c.1) + 0.11 * f64::from(c.2)) / 255.0
}

/// What every part is painted in: its own colour, white in a hit's flash, ice when frozen, lightning when shocked.
#[derive(Clone, Copy)]
enum Tint {
    Own,
    Flash,
    Ice,
    Shock,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Eye {
    Open,
    Shut,
    Crossed,
}

/// Where the sword is held.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Held {
    Hip,
    Back,
    Raised,
    Thrust,
    Upright,
    Planted,
}

/// Paints in the fighter's own frame: dx forward (toward where it faces), dy up from its feet.
struct Pen<'a> {
    sprite: Sprite,
    c: &'a Look,
    face: f64,
    tint: Tint,
    t: f64,
    wind: f64,
    lean: f64,
}

impl Pen<'_> {
    fn detailed(&self) -> bool {
        self.sprite.density == PixelDensity::Two
    }

    fn density(&self) -> f64 {
        f64::from(self.sprite.density.factor())
    }

    fn grid(&self, value: f64) -> f64 {
        js_round(value * self.density()) / self.density()
    }

    /// `col` as this pose is tinted: itself, white in a hit's flash, ice when frozen, lightning when shocked.
    fn tinted(&self, col: Rgba) -> Rgba {
        match self.tint {
            Tint::Own => col,
            Tint::Flash => WHITE,
            Tint::Shock => SHOCK,
            Tint::Ice if lum(col) > 0.7 => ICE_LIGHT,
            Tint::Ice if lum(col) > 0.4 => ICE_MID,
            Tint::Ice => ICE_DARK,
        }
    }

    fn px(&mut self, dx: f64, dy: f64, w: f64, h: f64, col: Rgba) {
        // a rect's edges land on whole sprite pixels and its neighbours' edges land on the same ones, so parts that
        // touch on the grid still touch
        let d = self.density();
        let scale = SCALE * d;
        let (left, right) = (js_round(dx * scale), js_round((dx + w) * scale));
        let (low, high) = (js_round(dy * scale), js_round((dy + h) * scale));
        let (w, h) = (js_max(right - left, 1.0), js_max(high - low, 1.0));
        let (dx, dy) = (left, low);
        let x = if self.face > 0.0 {
            f64::from(CX) * d + dx
        } else {
            f64::from(CX) * d - dx - w + d
        };
        let col = self.tinted(col);
        self.sprite
            .fill_rect(x, f64::from(FEET) * d - dy - h + d, w, h, col);
    }

    /// A part that keeps its size wherever it lands (a hand, always as wide as it is tall), at `w` x `h` grid pixels
    /// whose corner is at (dx, dy).
    fn block(&mut self, dx: f64, dy: f64, w: f64, h: f64, col: Rgba) {
        let d = self.density();
        let scale = SCALE * d;
        let (cw, ch) = (js_round(w * scale), js_round(h * scale));
        let x = js_round(dx * scale);
        let y = js_round(dy * scale);
        let left = if self.face > 0.0 {
            f64::from(CX) * d + x
        } else {
            f64::from(CX) * d - x - cw + d
        };
        let col = self.tinted(col);
        self.sprite
            .fill_rect(left, f64::from(FEET) * d - y - ch + d, cw, ch, col);
    }

    fn dot(&mut self, dx: f64, dy: f64, col: Rgba) {
        self.px(dx, dy, 1.0, 1.0, col);
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, col: Rgba) {
        let n = js_max(js_max((x1 - x0).abs(), (y1 - y0).abs()), 1.0);
        let step = 1.0 / self.density();
        let mut i = 0.0;
        while i <= n {
            self.dot(
                self.grid(x0 + ((x1 - x0) * i) / n),
                self.grid(y0 + ((y1 - y0) * i) / n),
                col,
            );
            i += step;
        }
    }

    /// A line whose pixels touch by an edge: a diagonal step fills its corner too, so a band one pixel wide (a hem,
    /// a collar's edge, a scabbard) reads as one stroke instead of loose dots.
    fn line4(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, col: Rgba) {
        let n = js_max(js_max((x1 - x0).abs(), (y1 - y0).abs()), 1.0);
        let mut last: Option<(f64, f64)> = None;
        let step = 1.0 / self.density();
        let mut i = 0.0;
        while i <= n {
            let x = self.grid(x0 + ((x1 - x0) * i) / n);
            let y = self.grid(y0 + ((y1 - y0) * i) / n);
            if let Some((lx, ly)) = last
                && lx != x
                && ly != y
            {
                self.dot(x, ly, col);
            }
            self.dot(x, y, col);
            last = Some((x, y));
            i += step;
        }
    }

    /// A sleeve standing out from the robe behind it, edged dark along its back and top.
    fn sleeve(&mut self, dx: f64, dy: f64, w: f64, h: f64, shade: usize) {
        let c = self.c;
        let edge = if shade == 2 { c.outline } else { c.robe[2] };
        let x = dx + self.lean;
        if self.detailed() {
            self.quad(
                &[
                    (x, dy + 0.5),
                    (x + w - 0.5, dy),
                    (x + w, dy + 0.45),
                    (x + w - 0.25, dy + h - 0.45),
                    (x + 0.65, dy + h),
                ],
                c.robe[shade],
            );
            self.seam(
                (x + 0.3, dy + h - 0.65),
                (x + w - 0.75, dy + h - 0.9),
                0.35,
                mix(c.robe[shade], c.robe[0], 0.5),
            );
            self.seam((x + 0.5, dy + 0.7), (x + w - 0.55, dy + 0.35), 0.4, edge);
            self.seam(
                (x + 0.55, dy + 0.25),
                (x + w - 0.75, dy + 0.15),
                0.38,
                c.trim[0],
            );
            return;
        }
        self.px(x, dy, w, h, c.robe[shade]);
        if w > 2.0 {
            self.px(x, dy, 1.0, h, edge);
        }
        if h > 2.0 {
            self.px(x, dy + h - 1.0, w, 1.0, edge);
        }
        self.px(x, dy, w, 1.0, c.trim[0]);
    }

    fn hand(&mut self, dx: f64, dy: f64) {
        self.palm(dx + self.lean, dy);
    }

    fn palm(&mut self, x: f64, y: f64) {
        let [skin, shade] = self.c.skin;
        if !self.detailed() {
            self.block(x, y, 2.0, 2.0, skin);
            return;
        }
        self.quad(
            &[
                (x, y + 0.35),
                (x + 1.35, y),
                (x + 2.0, y + 0.65),
                (x + 1.9, y + 1.65),
                (x + 0.85, y + 2.0),
                (x, y + 1.4),
            ],
            shade,
        );
        self.px(x + 0.35, y + 0.75, 1.45, 0.7, skin);
        self.px(x + 0.65, y + 1.45, 0.8, 0.45, mix(skin, WHITE, 0.28));
        self.px(x + 1.05, y + 0.3, 0.75, 0.32, skin);
    }

    fn seam(&mut self, a: (f64, f64), b: (f64, f64), width: f64, col: Rgba) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let steps = libm::ceil(js_max(dx.abs(), dy.abs()) * SCALE * self.density()).max(1.0) as u32;
        for i in 0..=steps {
            let k = f64::from(i) / f64::from(steps);
            self.px(a.0 + dx * k, a.1 + dy * k, width, width, col);
        }
    }

    /// Qi drawn in from round about to a point, closer as the spell gathers.
    fn motes(&mut self, cx: f64, cy: f64, k: f64) {
        let r = 2.0 + js_round((1.0 - k) * 11.0);
        for i in 0..4 {
            let a = self.t * 7.0 + (f64::from(i) * PI) / 2.0;
            let col = if i % 2 == 1 { self.c.glow } else { WHITE };
            self.dot(
                js_round(cx + self.lean + cos(a) * r),
                js_round(cy + sin(a) * r * 0.7),
                col,
            );
        }
    }

    fn orb(&mut self, cx: f64, cy: f64, k: f64) {
        let r = 1.0 + js_round(2.0 * k);
        let glow = self.c.glow;
        self.px(
            cx + self.lean - floor(r / 2.0),
            cy - floor(r / 2.0),
            r,
            r,
            glow,
        );
        if k > 0.5 {
            self.dot(cx + self.lean, cy, WHITE);
        }
    }

    /// A `w` x `h` block stamped at every step from (x0, y0) to (x1, y1): a limb, or an edge of a shape.
    fn bar(&mut self, (x0, y0): (f64, f64), (x1, y1): (f64, f64), w: f64, h: f64, col: Rgba) {
        let n = js_max(js_max((x1 - x0).abs(), (y1 - y0).abs()), 1.0);
        let step = 1.0 / self.density();
        let mut i = 0.0;
        while i <= n {
            self.px(
                self.grid(x0 + ((x1 - x0) * i) / n),
                self.grid(y0 + ((y1 - y0) * i) / n),
                w,
                h,
                col,
            );
            i += step;
        }
    }

    /// The convex shape with corners `p` (any number, in order round it) filled, a row at a time: a robe's skirt, a body on a slant.
    fn quad(&mut self, p: &[(f64, f64)], col: Rgba) {
        let lo = p.iter().fold(f64::INFINITY, |m, q| js_min(m, q.1));
        let hi = p.iter().fold(f64::NEG_INFINITY, |m, q| js_max(m, q.1));
        let raster = if self.detailed() {
            self.density() * SCALE
        } else {
            1.0
        };
        let step = 1.0 / raster;
        let mut y = floor(lo * raster) / raster;
        while y <= hi {
            // where the row's middle crosses each edge
            let mid = y + step * 0.5;
            let (mut left, mut right) = (f64::INFINITY, f64::NEG_INFINITY);
            for (&a, &b) in p.iter().zip(p.iter().cycle().skip(1)) {
                let (y0, y1) = (js_min(a.1, b.1), js_max(a.1, b.1));
                if mid < y0 || mid > y1 || y0 == y1 {
                    continue;
                }
                let x = a.0 + ((b.0 - a.0) * (mid - a.1)) / (b.1 - a.1);
                left = js_min(left, x);
                right = js_max(right, x);
            }
            if left <= right {
                let x0 = if self.detailed() {
                    left
                } else {
                    self.grid(left)
                };
                let x1 = if self.detailed() {
                    right
                } else {
                    self.grid(right) + step
                };
                self.px(x0, y, x1 - x0, step, col);
            }
            y += step;
        }
    }

    /// An arm in its sleeve from the shoulder at (x0, y0) to the hand at (x1, y1): the sleeve in robe shade
    /// `shade`, gold at the cuff, the hand bare.
    fn arm(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, shade: usize) {
        let c = self.c;
        let (cx, cy) = (x1 - (x1 - x0).signum(), y1 - (y1 - y0).signum());
        self.bar((x0, y0), (cx, cy), 3.0, 3.0, c.robe[shade]);
        self.bar((cx, cy), (cx, cy), 3.0, 3.0, c.trim[0]);
        self.palm(x1, y1);
    }

    /// A head lying face up, its crown toward -dx, at (x, y) to (x + 6, y + 6). The hair streams back from the
    /// crown, `hang` pixels up (down when negative) at its end: spread on the floor at 0.
    fn face_up(&mut self, x: f64, y: f64, eye: Eye, hang: f64) {
        let (c, t) = (self.c, self.t);
        self.px(x, y, 7.0, 7.0, c.skin[0]);
        self.px(x, y, 7.0, 2.0, c.skin[1]);
        // the nose and chin up
        self.px(x + 3.0, y + 7.0, 1.0, 1.0, c.skin[0]);
        match eye {
            Eye::Crossed => {
                for (ex, ey) in [(1.0, 4.0), (2.0, 5.0), (3.0, 4.0), (1.0, 6.0), (3.0, 6.0)] {
                    self.px(x + ex, y + ey, 1.0, 1.0, c.outline);
                }
            }
            Eye::Open => self.px(x + 1.0, y + 5.0, 2.0, 1.0, c.outline),
            Eye::Shut => self.px(x + 1.0, y + 5.0, 3.0, 1.0, c.outline),
        }
        // the hair over the brow and round the back of the head
        self.px(x - 3.0, y, 4.0, 7.0, c.hair[1]);
        self.px(x - 1.0, y + 5.0, 2.0, 2.0, c.hair[1]);
        match c.hairdo {
            Hairdo::Topknot => {
                self.px(x - 2.0, y + 1.0, 1.0, 5.0, c.hair[0]);
                if self.detailed() {
                    self.quad(
                        &[
                            (x - 3.0, y + 1.3),
                            (x - 5.7, y + 1.9),
                            (x - 6.4, y + 3.15),
                            (x - 5.65, y + 4.45),
                            (x - 3.0, y + 4.65),
                        ],
                        c.hair[1],
                    );
                    self.seam((x - 5.65, y + 3.7), (x - 3.8, y + 4.1), 0.36, c.hair[0]);
                    return;
                }
                // the knot under its gold crown, and the ribbon streaming from it
                self.px(x - 6.0, y + 2.0, 3.0, 3.0, c.hair[1]);
                self.px(x - 7.0, y + 1.0, 1.0, 5.0, c.trim[0]);
                for i in 0..7 {
                    let fi = f64::from(i);
                    let sway = js_round(sin(t * 6.0 + fi) * fi / 4.0);
                    let ry = y + 3.0 + js_round((hang * fi) / 6.0) + sway;
                    self.px(x - 8.0 - fi, ry, 1.0, 2.0, c.ribbon[usize::from(i >= 4)]);
                }
            }
            Hairdo::Flowing => {
                self.px(x - 3.0, y, 4.0, 7.0, c.hair[0]);
                self.px(x - 3.0, y, 4.0, 1.0, c.hair[1]);
                self.px(x - 1.0, y + 6.0, 2.0, 1.0, c.jade);
                // the long hair, thick at the head and thinning to its end
                for i in 0..13 {
                    let fi = f64::from(i);
                    let sway = js_round(sin(t * 4.0 - fi * 0.5) * fi / 8.0);
                    let ty = y + 1.0 + js_round((hang * fi) / 12.0) + sway;
                    let thick = 5.0 - floor(fi / 4.0);
                    self.px(x - 4.0 - fi, ty, 1.0, thick, c.hair[0]);
                    self.px(x - 4.0 - fi, ty, 1.0, 1.0, c.hair[1]);
                }
            }
        }
    }

    /// A swish of motion round a spinning body: a stroke along the circle of radius `r` about (cx, cy), from angle `a0`
    /// to `a1` (radians, counter-clockwise from forward).
    fn swish(&mut self, cx: f64, cy: f64, r: f64, a0: f64, a1: f64, col: Rgba) {
        let n = js_max(1.0, js_round((a1 - a0).abs() * r));
        let mut i = 0.0;
        while i <= n {
            let a = a0 + ((a1 - a0) * i) / n;
            self.px(
                js_round(cx + cos(a) * r),
                js_round(cy + sin(a) * r),
                2.0,
                2.0,
                col,
            );
            i += 1.0;
        }
    }

    /// Neck, face, an eye (open, or screwed shut), a brow and the hair, the neck at `y`.
    fn head(&mut self, y: f64, lean: f64, eye: Eye) {
        if self.detailed() {
            self.profile(lean, y, eye);
            return;
        }
        let (c, t, wind) = (self.c, self.t, self.wind);
        self.px(-1.0 + lean, y, 2.0, 1.0, c.skin[1]);
        self.px(-3.0 + lean, y + 1.0, 7.0, 7.0, c.skin[0]);
        self.px(-3.0 + lean, y + 1.0, 2.0, 7.0, c.skin[1]);
        if eye == Eye::Open {
            self.px(2.0 + lean, y + 4.0, 1.0, 2.0, c.outline);
        } else {
            self.px(1.0 + lean, y + 4.0, 3.0, 1.0, c.outline);
            self.px(2.0 + lean, y + 2.0, 2.0, 1.0, c.skin[1]);
        }
        self.px(2.0 + lean, y + 6.0, 2.0, 1.0, c.hair[1]);
        self.px(3.0 + lean, y + 2.0, 1.0, 1.0, c.skin[1]);
        match c.hairdo {
            Hairdo::Topknot => {
                // swept up into a topknot under a gold crown, a ribbon streaming from it
                self.px(-4.0 + lean, y + 6.0, 8.0, 3.0, c.hair[1]);
                self.px(-4.0 + lean, y + 2.0, 3.0, 5.0, c.hair[1]);
                self.px(-3.0 + lean, y + 8.0, 5.0, 1.0, c.hair[0]);
                self.px(-1.0 + lean, y + 9.0, 3.0, 3.0, c.hair[1]);
                self.px(-2.0 + lean, y + 10.0, 5.0, 1.0, c.trim[0]);
                self.px(lean, y + 11.0, 1.0, 2.0, c.trim[0]);
                self.px(-2.0 + lean, y + 10.0, 1.0, 1.0, c.trim[1]);
                for i in 0..6 {
                    let fi = f64::from(i);
                    let sway = js_round(sin(t * 5.0 + fi * 0.9) * (fi / 3.0));
                    let x = -3.0 + lean - fi + js_min(0.0, wind) * js_round(fi / 3.0);
                    let col = c.ribbon[usize::from(i >= 3)];
                    self.px(x, y + 10.0 - js_round(fi / 2.0) + sway, 1.0, 1.0, col);
                }
            }
            Hairdo::Flowing => {
                // long hair falling down the back and lifting in the wind, a jade pin
                self.px(-4.0 + lean, y + 6.0, 8.0, 3.0, c.hair[0]);
                self.px(-5.0 + lean, y + 3.0, 3.0, 5.0, c.hair[0]);
                self.px(2.0 + lean, y + 6.0, 2.0, 2.0, c.hair[0]);
                self.px(-4.0 + lean, y + 8.0, 6.0, 1.0, c.hair[1]);
                for i in 0..14 {
                    let fi = f64::from(i);
                    let sway = js_round(sin(t * 3.0 - fi * 0.45) * (fi / 7.0))
                        + js_round((js_min(0.0, wind) * fi) / 6.0);
                    let col = if i % 4 == 0 { c.hair[1] } else { c.hair[0] };
                    self.px(
                        -5.0 + lean - js_round(fi / 5.0) + sway,
                        y + 3.0 - fi,
                        2.0,
                        1.0,
                        col,
                    );
                }
                self.px(-1.0 + lean, y + 9.0, 3.0, 1.0, c.jade);
                self.px(-2.0 + lean, y + 9.0, 1.0, 1.0, c.trim[1]);
            }
        }
    }

    fn profile(&mut self, x: f64, y: f64, eye: Eye) {
        let c = self.c;
        let [skin, shadow] = c.skin;
        let lit = mix(skin, WHITE, 0.24);
        self.px(x - 1.15, y, 2.35, 2.7, shadow);
        self.quad(
            &[
                (x - 3.3, y + 2.3),
                (x - 0.8, y + 1.05),
                (x + 2.05, y + 1.1),
                (x + 3.5, y + 2.2),
                (x + 3.4, y + 3.25),
                (x + 4.1, y + 3.85),
                (x + 3.5, y + 4.85),
                (x + 3.5, y + 6.8),
                (x + 1.1, y + 8.05),
                (x - 2.3, y + 7.35),
            ],
            skin,
        );
        self.quad(
            &[
                (x - 2.9, y + 6.8),
                (x - 0.65, y + 6.4),
                (x - 0.2, y + 3.0),
                (x + 2.3, y + 1.3),
                (x - 0.8, y + 1.05),
                (x - 3.15, y + 2.3),
            ],
            shadow,
        );
        self.seam((x + 2.8, y + 4.2), (x + 3.55, y + 3.65), 0.35, lit);
        self.seam((x + 2.1, y + 2.6), (x + 3.0, y + 2.7), 0.28, shadow);
        self.seam((x + 0.9, y + 5.65), (x + 2.9, y + 5.9), 0.35, c.hair[1]);
        let lid = if eye == Eye::Open { 0.62 } else { 0.28 };
        self.px(x + 1.5, y + 4.72, 1.4, lid, c.outline);
        if eye == Eye::Open {
            self.px(x + 1.5, y + 5.02, 0.38, 0.32, lit);
        }
        self.px(x - 0.5, y + 3.52, 0.9, 1.45, shadow);
        self.px(x - 0.36, y + 4.25, 0.5, 0.64, lit);

        let [base, shade] = match c.hairdo {
            Hairdo::Topknot => [c.hair[1], c.hair[0]],
            Hairdo::Flowing => c.hair,
        };
        self.quad(
            &[
                (x - 4.2, y + 3.5),
                (x - 3.7, y + 7.4),
                (x - 2.3, y + 9.0),
                (x + 1.8, y + 8.9),
                (x + 3.5, y + 7.85),
                (x + 3.9, y + 6.55),
                (x + 1.9, y + 6.85),
                (x + 0.2, y + 6.1),
                (x - 1.5, y + 3.4),
            ],
            base,
        );
        self.seam((x - 3.35, y + 6.6), (x - 1.15, y + 8.15), 0.55, shade);
        self.seam((x - 0.75, y + 8.1), (x + 2.5, y + 7.65), 0.35, shade);
        match c.hairdo {
            Hairdo::Topknot => {
                self.quad(
                    &[
                        (x - 1.6, y + 8.7),
                        (x - 1.9, y + 10.3),
                        (x - 1.2, y + 12.4),
                        (x - 0.36, y + 13.0),
                        (x + 0.7, y + 13.0),
                        (x + 1.64, y + 12.15),
                        (x + 1.85, y + 10.5),
                        (x + 1.36, y + 9.0),
                    ],
                    base,
                );
                self.seam((x - 1.1, y + 11.37), (x - 0.05, y + 12.38), 0.43, shade);
                self.seam((x - 1.3, y + 9.65), (x + 1.12, y + 9.96), 0.34, c.ribbon[1]);
            }
            Hairdo::Flowing => {
                let tip = x - 8.0 + self.wind.min(0.0) + sin(self.t * 3.0);
                self.quad(
                    &[
                        (x - 3.15, y + 8.1),
                        (x - 5.0, y + 6.9),
                        (tip - 0.9, y - 8.2),
                        (tip + 0.85, y - 6.7),
                        (x - 1.65, y + 6.7),
                    ],
                    shade,
                );
                self.seam((x - 4.25, y + 5.5), (tip, y - 6.6), 0.55, base);
                self.seam((x - 3.28, y + 6.1), (tip + 1.25, y - 5.15), 0.32, base);
                self.seam((x - 2.5, y + 7.95), (x - 1.3, y + 8.29), 0.35, c.trim[1]);
                self.px(x - 1.82, y + 8.19, 0.4, 0.35, c.jade);
            }
        }
    }

    /// The sword where the pose holds it, the body's top at `by`.
    fn sword(&mut self, held: Held, by: f64) {
        let (c, lean) = (self.c, self.lean);
        let [edge, blade, dull] = c.blade;
        match held {
            Held::Thrust => {
                let y = by + 6.0;
                self.line(11.0 + lean, y, 26.0 + lean, y, blade);
                self.line(11.0 + lean, y + 1.0, 24.0 + lean, y + 1.0, edge);
                self.px(27.0 + lean, y, 1.0, 1.0, WHITE);
                self.px(10.0 + lean, y - 2.0, 1.0, 5.0, c.trim[0]);
                // the arc the blade just cut
                self.px(14.0 + lean, y + 3.0, 10.0, 1.0, c.glow);
                self.px(18.0 + lean, y + 4.0, 5.0, 1.0, c.glow);
            }
            Held::Upright => {
                self.line(8.0 + lean, by + 4.0, 8.0 + lean, by + 18.0, blade);
                self.line(9.0 + lean, by + 5.0, 9.0 + lean, by + 17.0, edge);
                self.px(8.0 + lean, by + 19.0, 1.0, 1.0, WHITE);
                self.px(6.0 + lean, by + 3.0, 5.0, 1.0, c.trim[0]);
            }
            Held::Raised => {
                self.line(-8.0 + lean, by + 10.0, -16.0 + lean, by + 22.0, blade);
                self.line(-7.0 + lean, by + 10.0, -15.0 + lean, by + 22.0, edge);
                self.px(-9.0 + lean, by + 8.0, 3.0, 1.0, c.trim[0]);
            }
            Held::Planted => {
                self.line(2.0 + lean, by - 4.0, 2.0 + lean, by + 6.0, blade);
                self.px(lean, by + 7.0, 5.0, 1.0, c.trim[0]);
                self.px(2.0 + lean, by + 8.0, 1.0, 2.0, c.trim[1]);
            }
            Held::Back => {
                self.line(-5.0 + lean, by - 2.0, -9.0 + lean, by + 12.0, dull);
                self.px(-10.0 + lean, by + 12.0, 3.0, 1.0, c.trim[0]);
                self.px(-10.0 + lean, by + 13.0, 1.0, 2.0, c.ribbon[0]);
            }
            Held::Hip => {
                // in its scabbard at the hip, hilt forward, a tassel hanging
                if self.detailed() {
                    let x = lean;
                    self.quad(
                        &[
                            (x - 9.3, by - 8.9),
                            (x - 8.2, by - 9.0),
                            (x + 3.7, by - 1.45),
                            (x + 2.9, by - 0.35),
                        ],
                        c.trim[1],
                    );
                    self.seam((x - 9.0, by - 8.75), (x + 3.15, by - 0.95), 0.45, dull);
                    self.seam((x - 8.8, by - 8.23), (x + 3.2, by - 0.55), 0.33, c.trim[0]);
                    self.seam((x + 2.6, by - 0.9), (x + 3.75, by + 0.62), 0.5, c.trim[0]);
                    self.seam((x + 3.7, by - 0.23), (x + 6.45, by + 1.05), 0.65, c.robe[2]);
                    self.px(x + 6.32, by + 0.9, 0.5, 0.7, c.trim[1]);
                    self.seam((x + 3.5, by - 1.3), (x + 4.1, by - 3.2), 0.32, c.ribbon[1]);
                    return;
                }
                self.line(-9.0 + lean, by - 9.0, 3.0 + lean, by - 1.0, dull);
                self.line(-9.0 + lean, by - 8.0, 2.0 + lean, by - 1.0, c.trim[1]);
                self.px(3.0 + lean, by - 1.0, 1.0, 2.0, c.trim[0]);
                self.line(4.0 + lean, by, 6.0 + lean, by + 1.0, c.robe[2]);
                self.px(4.0 + lean, by - 3.0, 1.0, 2.0, c.ribbon[0]);
            }
        }
    }
}

/// Paint fighter `a`'s pose into a sprite, not yet outlined. A tumble is painted curled up; [`draw`] turns it.
pub fn paint(look: &Look, a: &Appearance) -> Sprite {
    paint_at_density(look, a, PixelDensity::One)
}

/// Paint the pose on the requested raster without changing its game-space placement.
pub fn paint_at_density(look: &Look, a: &Appearance, density: PixelDensity) -> Sprite {
    let t = a.pose_t;
    let still = a.frozen;
    let tint = if a.flash {
        Tint::Flash
    } else if a.frozen {
        Tint::Ice
    } else if a.shocked && sin(t * 70.0) > 0.0 {
        Tint::Shock
    } else {
        Tint::Own
    };
    // cloth streams back from the way it moves, and sways a little when it does not
    let wind = if still {
        0.0
    } else {
        js_max(-3.0, js_min(3.0, -a.vx * a.face * 0.03)) - 1.0 + js_round(sin(t * 4.0))
    };
    let mut pen = Pen {
        sprite: Sprite::new(density),
        c: look,
        face: a.face,
        tint,
        t,
        wind,
        lean: 0.0,
    };
    let c = look;
    match a.pose {
        Pose::Down => {
            // flat on its back, head away from the foe: the near knee bent up, the far foot standing up at the
            // end of its leg, the near hand on its chest, the hair spread on the floor. A knocked-out fighter's
            // eyes are crossed out and its sword lies dropped beside it.
            let breath = if !a.out && !still && floor(t * 2.5) % 2.0 != 0.0 {
                1.0
            } else {
                0.0
            };
            // the far leg straight along the floor, its boot toe-up
            pen.px(4.0, 0.0, 17.0, 4.0, c.robe[2]);
            pen.px(4.0, 3.0, 17.0, 1.0, c.robe[1]);
            pen.px(19.0, 0.0, 4.0, 6.0, c.boot);
            // the near knee bent up under the robe, its foot flat
            pen.px(13.0, 0.0, 5.0, 2.0, c.boot);
            pen.quad(
                &[(2.0, 0.0), (9.0, 15.0), (10.0, 15.0), (16.0, 1.0)],
                c.robe[1],
            );
            pen.bar((10.0, 14.0), (16.0, 1.0), 1.0, 1.0, c.robe[2]);
            pen.bar((11.0, 12.0), (15.0, 2.0), 1.0, 1.0, c.robe[2]);
            pen.bar((3.0, 1.0), (9.0, 14.0), 1.0, 1.0, c.robe[0]);
            pen.bar((13.0, 1.0), (16.0, 1.0), 1.0, 2.0, c.trim[0]);
            // the body, chest up: shadowed along its back, the inner robe crossing the chest, the sash
            let h = 7.0 + breath;
            pen.px(-9.0, 0.0, 14.0, h, c.robe[1]);
            pen.px(-9.0, 0.0, 14.0, 1.0, c.robe[2]);
            pen.px(-9.0, h - 1.0, 14.0, 1.0, c.robe[0]);
            pen.line(-8.0, h - 2.0, 0.0, h - 4.0, c.inner[0]);
            pen.px(-9.0, h - 2.0, 3.0, 2.0, c.inner[1]);
            pen.px(2.0, 0.0, 2.0, h, c.inner[0]);
            pen.px(2.0, h - 2.0, 2.0, 1.0, c.trim[0]);
            pen.px(4.0, 1.0, 1.0, 3.0, c.inner[1]);
            // the far arm along its side, the near forearm across its chest
            pen.arm(-7.0, 0.0, 1.0, 0.0, 2);
            pen.arm(-7.0, h - 3.0, -1.0, h - 1.0, 1);
            pen.face_up(
                -16.0,
                0.0,
                if a.out { Eye::Crossed } else { Eye::Shut },
                0.0,
            );
            if a.out {
                pen.line(-6.0, -1.0, 10.0, -1.0, c.blade[2]);
                pen.px(11.0, -1.0, 1.0, 1.0, c.trim[0]);
            }
            return pen.sprite;
        }
        Pose::Launched => {
            // just thrown up and back by a heavy blow, spread like an X: head back and chest up on a 45° slant, the near
            // arm flung up, the far one out behind, the near leg kicked up toward the foe, the far leg and the robe's skirt
            // flung down, the sash tails and the scabbard trailing
            // the far arm out behind, in the shade, gold at the cuff
            pen.quad(
                &[(-4.0, 32.0), (-2.0, 28.0), (-13.0, 23.0), (-15.0, 27.0)],
                c.robe[2],
            );
            pen.line4(-15.0, 27.0, -13.0, 23.0, c.trim[0]);
            pen.px(-18.0, 23.0, 3.0, 3.0, c.skin[1]);
            // the far leg and the skirt flung down: shadowed along its back, the inner robe down its middle, the hem gold
            pen.bar((9.0, 17.0), (12.0, 12.0), 3.0, 3.0, c.robe[2]);
            pen.px(11.0, 9.0, 4.0, 5.0, c.boot);
            pen.quad(
                &[(0.0, 27.0), (9.0, 29.0), (19.0, 18.0), (5.0, 11.0)],
                c.robe[1],
            );
            pen.line4(0.0, 27.0, 5.0, 11.0, c.robe[2]);
            pen.line4(9.0, 29.0, 19.0, 18.0, c.robe[0]);
            pen.line4(7.0, 25.0, 10.0, 14.0, c.inner[0]);
            pen.line4(5.0, 11.0, 19.0, 18.0, c.trim[0]);
            pen.dot(8.0, 13.0, c.trim[1]);
            pen.dot(15.0, 16.0, c.trim[1]);
            // the near leg kicked up toward the foe
            pen.bar((9.0, 23.0), (18.0, 26.0), 3.0, 3.0, c.robe[1]);
            pen.line4(9.0, 23.0, 17.0, 26.0, c.robe[2]);
            pen.px(18.0, 28.0, 4.0, 5.0, c.boot);
            // the sword in its scabbard, trailing from the hip
            pen.line4(2.0, 24.0, -6.0, 13.0, c.blade[2]);
            pen.px(2.0, 25.0, 2.0, 2.0, c.trim[0]);
            pen.px(4.0, 23.0, 1.0, 2.0, c.ribbon[0]);
            // the body, chest up: the crossed collar's gold edge over the red inner robe
            pen.quad(
                &[(-6.0, 34.0), (0.0, 37.0), (8.0, 29.0), (0.0, 25.0)],
                c.robe[1],
            );
            pen.line4(-6.0, 33.0, 0.0, 25.0, c.robe[2]);
            pen.line4(0.0, 37.0, 7.0, 30.0, c.robe[0]);
            pen.line4(-3.0, 34.0, 2.0, 29.0, c.trim[0]);
            pen.line4(-1.0, 34.0, 3.0, 30.0, c.inner[0]);
            // the sash across the waist, its buckle, the jade pendant hanging from it, the two tails flying
            pen.bar((-1.0, 25.0), (5.0, 28.0), 2.0, 2.0, c.inner[0]);
            pen.px(3.0, 27.0, 2.0, 2.0, c.trim[0]);
            pen.px(5.0, 25.0, 1.0, 2.0, c.ribbon[1]);
            pen.dot(5.0, 24.0, c.jade);
            for i in 0..7 {
                let fi = f64::from(i);
                let flutter = if i > 3 {
                    js_round(sin(t * 6.0 + fi))
                } else {
                    0.0
                };
                pen.dot(
                    -2.0 - fi,
                    24.0 - js_round(fi * 0.8) - flutter,
                    c.inner[usize::from(i >= 3)],
                );
            }
            for i in 0..6 {
                let fi = f64::from(i);
                let flutter = if i > 4 {
                    js_round(sin(t * 6.0 + fi + 2.0))
                } else {
                    0.0
                };
                pen.px(
                    -3.0 - fi,
                    22.0 - fi - flutter,
                    1.0,
                    2.0,
                    c.inner[usize::from(i >= 3)],
                );
            }
            pen.face_up(-12.0, 31.0, Eye::Shut, 3.0);
            // the near arm flung up, gold at the cuff
            pen.quad(
                &[(-1.0, 33.0), (3.0, 34.0), (6.0, 40.0), (2.0, 41.0)],
                c.robe[0],
            );
            pen.line4(-1.0, 33.0, 2.0, 41.0, c.robe[1]);
            pen.px(2.0, 40.0, 5.0, 2.0, c.trim[0]);
            pen.px(2.0, 40.0, 5.0, 1.0, c.trim[1]);
            pen.px(3.0, 42.0, 3.0, 4.0, c.skin[0]);
            pen.dot(3.0, 45.0, c.skin[1]);
            return pen.sprite;
        }
        Pose::Fall => {
            // dropping back-first, head-down: the body on a slant with the head low and the hair hanging below it, the
            // legs trailing up, the robe's skirt flaring up off them, the near arm trailing up
            // the legs trailing up
            pen.bar((9.0, 18.0), (12.0, 24.0), 3.0, 3.0, c.robe[2]);
            pen.px(13.0, 27.0, 4.0, 7.0, c.boot);
            pen.bar((14.0, 15.0), (17.0, 18.0), 3.0, 3.0, c.robe[2]);
            pen.px(18.0, 21.0, 4.0, 5.0, c.boot);
            // the skirt flaring up off the legs: the inner robe down its middle, the hem gold
            pen.quad(
                &[
                    (6.0, 11.0),
                    (6.0, 24.0),
                    (10.0, 30.0),
                    (20.0, 19.0),
                    (13.0, 11.0),
                ],
                c.robe[1],
            );
            pen.line4(12.0, 19.0, 15.0, 15.0, c.robe[2]);
            pen.px(8.0, 13.0, 3.0, 11.0, c.robe[0]);
            pen.line4(6.0, 24.0, 6.0, 11.0, c.robe[2]);
            pen.px(7.0, 12.0, 2.0, 9.0, c.inner[0]);
            pen.px(7.0, 17.0, 1.0, 4.0, c.inner[1]);
            pen.line4(10.0, 30.0, 20.0, 20.0, c.trim[0]);
            // the sword in its scabbard at the hip
            pen.line4(9.0, 11.0, 14.0, 16.0, c.blade[2]);
            pen.px(8.0, 10.0, 1.0, 2.0, c.trim[0]);
            pen.px(10.0, 9.0, 2.0, 1.0, c.ribbon[0]);
            // the body, head-down: the crossed collar's gold edge over the red inner robe
            pen.quad(
                &[(-6.0, 11.0), (5.0, 17.0), (7.0, 8.0), (-3.0, 4.0)],
                c.robe[1],
            );
            pen.line4(-3.0, 4.0, 7.0, 8.0, c.robe[2]);
            pen.line4(-6.0, 11.0, 4.0, 17.0, c.robe[0]);
            pen.line4(-2.0, 8.0, 2.0, 12.0, c.trim[0]);
            pen.line4(0.0, 8.0, 4.0, 12.0, c.inner[0]);
            // the sash across the waist, its buckle, the jade pendant, the tail streaming
            pen.px(4.0, 8.0, 2.0, 9.0, c.inner[0]);
            pen.px(4.0, 16.0, 2.0, 1.0, c.inner[1]);
            pen.px(4.0, 12.0, 2.0, 2.0, c.trim[0]);
            pen.px(3.0, 8.0, 1.0, 2.0, c.ribbon[1]);
            pen.dot(3.0, 7.0, c.jade);
            for i in 0..7 {
                let fi = f64::from(i);
                let flutter = if i > 3 {
                    js_round(sin(t * 6.0 + fi))
                } else {
                    0.0
                };
                pen.px(
                    2.0 - fi,
                    5.0 - js_round(fi / 3.0) - flutter,
                    1.0,
                    2.0,
                    c.inner[usize::from(i >= 3)],
                );
            }
            // the hair hanging below the head
            pen.px(-16.0, 4.0, 5.0, 4.0, c.hair[1]);
            pen.px(-15.0, 2.0, 2.0, 2.0, c.hair[1]);
            pen.px(-12.0, 5.0, 1.0, 3.0, c.hair[0]);
            pen.face_up(-11.0, 8.0, Eye::Shut, 4.0);
            // the near arm trailing up, gold at the cuff
            pen.quad(
                &[(-5.0, 8.0), (-1.0, 7.0), (0.0, 16.0), (-5.0, 16.0)],
                c.robe[0],
            );
            pen.line4(0.0, 16.0, -1.0, 7.0, c.robe[2]);
            pen.px(-5.0, 17.0, 5.0, 2.0, c.trim[0]);
            pen.px(-5.0, 17.0, 5.0, 1.0, c.trim[1]);
            pen.px(-4.0, 19.0, 3.0, 4.0, c.skin[0]);
            pen.dot(-4.0, 22.0, c.skin[1]);
            return pen.sprite;
        }
        Pose::Tumble => {
            // spinning helplessly, curled into a ball: the head tucked down, the knees hugged to the chest, the feet folded
            // under, the far arm flung out, swishes of motion round it; draw turns it a quarter at a time
            pen.swish(6.0, 14.0, 14.0, 2.6, 3.7, c.blade[1]);
            pen.swish(6.0, 14.0, 14.0, -0.4, 0.6, c.blade[1]);
            // the far arm flung out, gold at the cuff
            pen.quad(
                &[(1.0, 25.0), (2.0, 18.0), (-8.0, 20.0), (-9.0, 26.0)],
                c.robe[0],
            );
            pen.line4(-7.0, 20.0, 1.0, 18.0, c.robe[2]);
            pen.px(-9.0, 21.0, 2.0, 6.0, c.trim[0]);
            pen.px(-9.0, 21.0, 1.0, 6.0, c.trim[1]);
            pen.px(-13.0, 24.0, 3.0, 3.0, c.skin[0]);
            // the feet folded under
            pen.px(4.0, 4.0, 4.0, 4.0, c.boot);
            pen.px(8.0, 6.0, 3.0, 3.0, c.boot);
            // the back curled round and shadowed, the knees lit, the hem gold at the shins, the inner robe between them
            pen.quad(
                &[
                    (-1.0, 21.0),
                    (8.0, 22.0),
                    (14.0, 15.0),
                    (13.0, 8.0),
                    (3.0, 4.0),
                    (-2.0, 10.0),
                ],
                c.robe[1],
            );
            pen.line4(-1.0, 20.0, -2.0, 11.0, c.robe[2]);
            pen.line4(-2.0, 10.0, 3.0, 4.0, c.robe[2]);
            pen.px(7.0, 12.0, 5.0, 4.0, c.robe[0]);
            pen.line4(12.0, 14.0, 12.0, 9.0, c.robe[2]);
            pen.line4(3.0, 4.0, 12.0, 8.0, c.trim[0]);
            pen.bar((5.0, 11.0), (7.0, 6.0), 2.0, 2.0, c.inner[0]);
            // the crossed collar's gold edge over the red inner robe
            pen.line4(4.0, 20.0, 1.0, 15.0, c.trim[0]);
            pen.line4(6.0, 20.0, 3.0, 15.0, c.inner[0]);
            // the sword in its scabbard across the back
            pen.line4(2.0, 11.0, -5.0, 5.0, c.blade[2]);
            pen.px(2.0, 12.0, 1.0, 2.0, c.trim[0]);
            pen.px(-7.0, 5.0, 1.0, 2.0, c.ribbon[0]);
            // the sash, its buckle, the jade pendant, the tail streaming
            pen.bar((-2.0, 11.0), (3.0, 14.0), 2.0, 2.0, c.inner[0]);
            pen.px(1.0, 14.0, 2.0, 2.0, c.trim[0]);
            pen.px(0.0, 11.0, 1.0, 2.0, c.ribbon[1]);
            pen.dot(0.0, 10.0, c.jade);
            for i in 0..7 {
                let fi = f64::from(i);
                let flutter = if i > 3 {
                    js_round(sin(t * 6.0 + fi))
                } else {
                    0.0
                };
                pen.px(
                    -3.0 - fi,
                    12.0 - js_round(fi / 3.0) - flutter,
                    1.0,
                    2.0,
                    c.inner[usize::from(i >= 3)],
                );
            }
            pen.head(18.0, 8.0, Eye::Shut);
            // the near arm hugging the knees
            pen.quad(
                &[(5.0, 20.0), (9.0, 20.0), (13.0, 16.0), (10.0, 13.0)],
                c.robe[0],
            );
            pen.line4(5.0, 19.0, 9.0, 14.0, c.robe[1]);
            pen.line4(11.0, 16.0, 11.0, 13.0, c.trim[0]);
            pen.palm(12.0, 14.0);
            return pen.sprite;
        }
        Pose::Roll => {
            // tucked into a ball for a quick rise; draw turns it a quarter at a time
            pen.px(-6.0, 1.0, 12.0, 9.0, c.robe[1]);
            pen.px(-5.0, 0.0, 10.0, 11.0, c.robe[1]);
            pen.px(-5.0, 0.0, 10.0, 1.0, c.robe[2]);
            pen.px(-6.0, 4.0, 12.0, 1.0, c.trim[0]);
            pen.px(1.0, 5.0, 5.0, 5.0, c.skin[0]);
            pen.px(1.0, 9.0, 5.0, 2.0, c.hair[1]);
            pen.px(-4.0, 1.0, 3.0, 2.0, c.boot);
            return pen.sprite;
        }
        Pose::Sit => {
            // sitting up from the floor: legs out in front, leaning back on one hand, still dazed
            pen.px(10.0, 0.0, 3.0, 4.0, c.boot);
            pen.px(-2.0, 0.0, 12.0, 4.0, c.robe[1]);
            pen.px(-2.0, 3.0, 12.0, 1.0, c.robe[0]);
            pen.px(8.0, 0.0, 2.0, 4.0, c.trim[0]);
            pen.px(-7.0, 3.0, 9.0, 9.0, c.robe[1]);
            pen.px(-7.0, 3.0, 2.0, 9.0, c.robe[2]);
            pen.px(-7.0, 3.0, 9.0, 1.0, c.trim[0]);
            pen.line(-3.0, 5.0, 0.0, 11.0, c.inner[0]);
            // the arm behind, propping it up from the floor
            for i in 0..=7 {
                let fi = f64::from(i);
                pen.px(-6.0 - js_round(fi * 0.75), 9.0 - fi, 2.0, 1.0, c.robe[2]);
            }
            pen.px(-6.0, 9.0, 2.0, 1.0, c.trim[0]);
            pen.px(-13.0, 0.0, 3.0, 2.0, c.skin[0]);
            pen.head(12.0, -3.0, Eye::Shut);
            return pen.sprite;
        }
        Pose::Duck => {
            // ducking under a blow: dropped into a deep squat, the body bent level and the head low, the near arm raised
            // over the head to shield it, the sash tail streaming back
            let bob = if !still && sin(t * 3.0) > 0.6 {
                1.0
            } else {
                0.0
            };
            // the far sleeve hanging behind, the boots wide apart
            pen.sleeve(-8.0, 8.0, 3.0, 5.0, 2);
            pen.px(-12.0, 0.0, 4.0, 2.0, c.boot);
            pen.px(3.0, 0.0, 4.0, 2.0, c.boot);
            // the lower robe over the folded legs: shadowed behind, lit in front, the inner robe down its middle, a fold
            // at the knee, the hem gold
            let mut dy = 2.0;
            while dy <= 8.0 {
                let back = -9.0 - if dy < 4.0 { 1.0 } else { 0.0 };
                let front = 6.0 - if dy > 6.0 { 1.0 } else { 0.0 };
                pen.px(back, dy, front - back + 1.0, 1.0, c.robe[1]);
                pen.px(back, dy, 2.0, 1.0, c.robe[2]);
                pen.px(front - 1.0, dy, 1.0, 1.0, c.robe[0]);
                pen.px(-2.0, dy, 2.0, 1.0, c.inner[usize::from(dy < 5.0)]);
                pen.px(1.0, dy, 1.0, 1.0, c.robe[2]);
                dy += 1.0;
            }
            pen.px(-10.0, 1.0, 12.0, 1.0, c.trim[0]);
            pen.px(2.0, 2.0, 4.0, 1.0, c.trim[0]);
            // the sword in its scabbard, swung back and down from the hip
            pen.line4(-15.0, 3.0, -4.0, 8.0, c.blade[2]);
            pen.px(-3.0, 8.0, 1.0, 2.0, c.trim[0]);
            pen.px(-4.0, 5.0, 1.0, 2.0, c.ribbon[0]);
            // the body bent level, the crossed collar's gold edge over the red inner robe
            let ty = 9.0 - bob;
            pen.quad(
                &[(-6.0, ty), (7.0, ty), (5.0, ty + 5.0), (-3.0, ty + 5.0)],
                c.robe[1],
            );
            pen.px(-6.0, ty, 2.0, 3.0, c.robe[2]);
            pen.px(-4.0, ty + 3.0, 2.0, 2.0, c.robe[2]);
            pen.px(5.0, ty + 1.0, 1.0, 3.0, c.robe[0]);
            pen.line4(2.0, ty + 4.0, 1.0, ty + 1.0, c.trim[0]);
            pen.px(3.0, ty + 2.0, 2.0, 3.0, c.inner[0]);
            // the sash, its buckle, the jade pendant hanging from it, the tail streaming back
            pen.px(-7.0, ty, 13.0, 2.0, c.inner[1]);
            pen.px(-7.0, ty + 1.0, 13.0, 1.0, c.inner[0]);
            pen.px(1.0, ty, 2.0, 2.0, c.trim[0]);
            pen.px(2.0, ty - 3.0, 1.0, 3.0, c.ribbon[1]);
            pen.dot(2.0, ty - 4.0, c.jade);
            for i in 0..6 {
                let fi = f64::from(i);
                let flutter = if i > 3 {
                    js_round(sin(t * 6.0 + fi))
                } else {
                    0.0
                };
                pen.px(
                    -8.0 - fi + js_round(wind * fi * 0.3),
                    ty + js_round(fi / 3.0) + flutter,
                    1.0,
                    2.0,
                    c.inner[usize::from(i >= 3)],
                );
            }
            pen.head(ty + 4.0, 7.0, Eye::Shut);
            // the near arm raised over the head
            pen.arm(6.0, ty + 5.0, 12.0, ty + 13.0, 0);
            return pen.sprite;
        }
        Pose::Meditate => {
            // seated cross-legged, seen from the side: thighs out in front, a foot turned up on top, the robe
            // spread round the seat, back straight, hands on the knees; it breathes
            let breath = if still { 0.0 } else { floor(t * 1.5) % 2.0 };
            pen.px(-8.0, 0.0, 20.0, 1.0, c.robe[2]);
            pen.px(-7.0, 1.0, 6.0, 3.0, c.robe[1]);
            pen.px(-7.0, 1.0, 2.0, 3.0, c.robe[2]);
            pen.px(-2.0, 1.0, 13.0, 3.0, c.robe[1]);
            pen.px(-2.0, 3.0, 12.0, 1.0, c.robe[0]);
            pen.px(11.0, 1.0, 1.0, 2.0, c.robe[1]);
            pen.px(1.0, 1.0, 9.0, 1.0, c.robe[2]);
            pen.px(5.0, 4.0, 3.0, 1.0, c.boot);
            pen.px(-8.0, 1.0, 1.0, 1.0, c.trim[0]);
            pen.px(-8.0, 0.0, 20.0, 1.0, c.trim[0]);
            // the body, upright on the seat
            let by = 4.0 + breath;
            pen.px(-5.0, by, 9.0, 9.0, c.robe[1]);
            pen.px(-5.0, by, 2.0, 9.0, c.robe[2]);
            pen.px(3.0, by + 1.0, 1.0, 8.0, c.robe[0]);
            pen.line(0.0, by + 2.0, 3.0, by + 8.0, c.inner[0]);
            pen.line(-1.0, by + 2.0, 2.0, by + 8.0, c.trim[0]);
            pen.px(-5.0, by, 9.0, 1.0, c.trim[0]);
            // forearms along the thighs, hands on the knees, thumb and finger touching
            pen.px(1.0, by - 1.0, 7.0, 2.0, c.robe[1]);
            pen.px(1.0, by, 7.0, 1.0, c.trim[0]);
            pen.palm(8.0, 4.0);
            pen.px(9.0, 6.0, 1.0, 1.0, c.skin[1]);
            pen.head(by + 9.0, 0.0, Eye::Shut);
            return pen.sprite;
        }
        _ => {}
    }
    standing(pen, a)
}

/// The upright poses: feet, lower robe, body, head, then arms and sword.
fn standing(mut pen: Pen<'_>, a: &Appearance) -> Sprite {
    let c = pen.c;
    let (t, wind) = (pen.t, pen.wind);
    let pose = a.pose;
    let bob = if !a.frozen && sin(t * 3.0) > 0.6 {
        1.0
    } else {
        0.0
    };
    let p = a.act.and_then(|k| k.windup).map(|w| w.progress);
    let how = a.act.map(|k| k.how);
    // a fresh blow snaps it back; then it reels, bent over the hurt, lower the harder it was struck
    let recoil = pose == Pose::Hurt && a.hit_t < 0.1;
    let reel = pose == Pose::Hurt && !recoil;
    // how far the body sinks: a crouch, a landing, a kneel, a reel, a stance braced to gather or to strike
    let braced = (pose == Pose::Channel && matches!(how, Some(Gather::Summon | Gather::Palm)))
        || (pose == Pose::Draw && a.air == 0.0);
    let lunge = pose == Pose::Channel && how == Some(Gather::Lunge);
    let drop = match pose {
        Pose::Crouch => 4.0,
        Pose::Land => 3.0,
        Pose::Kneel => 7.0,
        _ if reel => {
            if a.hit_power >= 9.0 {
                5.0
            } else {
                3.0
            }
        }
        _ if lunge => 4.0,
        _ if braced => 2.0,
        _ => 0.0,
    };
    pen.lean = if recoil {
        -3.0
    } else if reel {
        2.0
    } else {
        match pose {
            Pose::Shocked => -2.0,
            Pose::Slash => 2.0,
            Pose::Cast => 1.0,
            Pose::Channel if how == Some(Gather::Lunge) => 2.0,
            Pose::Channel if how == Some(Gather::Palm) => -1.0,
            Pose::Draw => -1.0,
            _ => 0.0,
        }
    };
    let lean = pen.lean;
    // knees drawn up in a jump
    let tuck = if pose == Pose::Jump { 4.0 } else { 0.0 };

    if pose == Pose::Fly {
        // riding its sword: the blade flat under its feet, a trail of light behind
        pen.line(-11.0, -2.0, 11.0, -2.0, c.blade[1]);
        pen.line(-11.0, -1.0, 9.0, -1.0, c.blade[0]);
        pen.px(12.0, -2.0, 1.0, 1.0, WHITE);
        pen.px(-13.0, -3.0, 2.0, 3.0, c.trim[0]);
        pen.px(-15.0, -2.0, 2.0, 1.0, c.trim[1]);
        if sin(t * 20.0) > -0.3 {
            pen.line(
                -16.0 - (js_round(t * 30.0) % 3.0),
                -2.0,
                -14.0,
                -2.0,
                c.glow,
            );
        }
    }

    // feet: apart in a stance, staggering back from a blow, one knee on the floor in a kneel
    let stride = if pose == Pose::Walk {
        if sin(t * 12.0) > 0.0 { 2.0 } else { -2.0 }
    } else {
        0.0
    };
    let spread = if drop > 0.0 && pose != Pose::Kneel {
        2.0
    } else {
        0.0
    };
    if pose == Pose::Kneel {
        pen.px(4.0, 0.0, 4.0, 2.0, c.boot);
        pen.px(-7.0, 0.0, 5.0, 2.0, c.robe[2]);
        pen.px(-10.0, 0.0, 3.0, 1.0, c.boot);
        pen.px(3.0, 2.0, 6.0, 3.0, c.robe[1]);
        pen.px(3.0, 4.0, 6.0, 1.0, c.robe[0]);
    } else {
        pen.px(
            -4.0 + stride - spread - if reel { 2.0 } else { 0.0 },
            0.0,
            4.0,
            2.0,
            c.boot,
        );
        pen.px(1.0 - stride + spread, 0.0, 4.0, 2.0, c.boot);
    }
    // the lower robe, flaring to the hem, open at the front over the inner robe; shorter as the knees bend
    let top = 15.0 - drop;
    let mut dy = 2.0 + tuck;
    if pen.detailed() {
        let wind = js_min(0.0, wind);
        let flare = if drop > 0.0 { 1.0 } else { 0.0 };
        let back = -7.8 - flare + wind;
        let front = 6.15 + flare;
        pen.quad(
            &[(back, dy), (front, dy), (4.4, top + 1.0), (-4.9, top + 1.0)],
            c.robe[1],
        );
        pen.quad(
            &[
                (back, dy),
                (back + 2.25, dy),
                (-3.5, top),
                (-4.9, top + 1.0),
            ],
            c.robe[2],
        );
        pen.quad(
            &[
                (back + 2.7, dy + 0.8),
                (back + 4.9, dy + 0.55),
                (-1.85, top),
            ],
            mix(c.robe[1], c.robe[0], 0.62),
        );
        pen.quad(
            &[(0.85, dy), (3.25, dy), (2.25, top + 0.8), (0.9, top)],
            c.inner[1],
        );
        pen.seam((1.65, dy + 1.0), (1.23, top), 0.62, c.inner[0]);
        pen.quad(
            &[(4.15, dy), (front, dy), (4.25, top + 1.0), (3.0, top)],
            c.robe[0],
        );
        pen.seam(
            (back + 0.35, dy + 0.4),
            (front - 0.45, dy + 0.4),
            0.46,
            c.trim[1],
        );
        pen.seam(
            (back + 0.5, dy + 0.85),
            (front - 0.6, dy + 0.85),
            0.38,
            c.trim[0],
        );
        for x in [back + 2.0, back + 5.35, 3.45] {
            pen.seam(
                (x, dy + 1.45),
                (x * 0.66, top - 2.0),
                0.31,
                mix(c.robe[1], c.robe[2], 0.55),
            );
        }
    } else {
        while dy <= top {
            let flare =
                js_max(0.0, js_round((8.0 - dy) / 2.0)) + if drop > 0.0 { 1.0 } else { 0.0 };
            let back = -5.0 - flare + if dy < 6.0 { js_min(0.0, wind) } else { 0.0 };
            let front = 4.0 + js_max(0.0, flare - 1.0);
            pen.px(back, dy, front - back + 1.0, 1.0, c.robe[1]);
            pen.px(back, dy, 2.0, 1.0, c.robe[2]);
            pen.px(front - 1.0, dy, 1.0, 1.0, c.robe[0]);
            pen.px(1.0, dy, 2.0, 1.0, c.inner[usize::from(dy < 8.0)]);
            dy += 1.0;
        }
        pen.px(
            -7.0 + js_min(0.0, wind),
            2.0 + tuck,
            13.0 - js_min(0.0, wind),
            1.0,
            c.trim[0],
        );
        pen.px(-6.0, 3.0 + tuck, 1.0, 1.0, c.trim[1]);
    }

    // the body: robe crossed over the inner robe, a sash with a jade pendant and a tail that streams back
    let by = 16.0 - bob - drop;
    if pen.detailed() {
        let x = lean;
        pen.quad(
            &[
                (x - 5.15, by),
                (x + 4.5, by),
                (x + 4.8, by + 6.7),
                (x + 2.9, by + 8.9),
                (x - 2.4, by + 9.25),
                (x - 4.8, by + 7.0),
            ],
            c.robe[1],
        );
        pen.quad(
            &[
                (x - 5.05, by + 1.3),
                (x - 3.2, by + 1.4),
                (x - 1.7, by + 8.8),
                (x - 3.7, by + 8.1),
                (x - 4.65, by + 6.7),
            ],
            c.robe[2],
        );
        pen.quad(
            &[
                (x - 2.7, by + 8.8),
                (x + 0.4, by + 8.85),
                (x + 3.8, by + 5.25),
                (x + 2.6, by + 3.4),
            ],
            c.robe[0],
        );
        pen.seam((x - 1.56, by + 8.7), (x + 3.6, by + 5.05), 0.48, c.inner[0]);
        pen.seam((x + 3.1, by + 8.5), (x - 0.85, by + 2.1), 0.68, c.trim[1]);
        pen.seam((x + 2.9, by + 8.45), (x - 1.08, by + 2.35), 0.35, c.trim[0]);
        pen.seam(
            (x - 3.35, by + 5.75),
            (x - 2.7, by + 3.2),
            0.31,
            mix(c.robe[1], c.robe[0], 0.6),
        );
    } else {
        pen.px(-5.0 + lean, by, 10.0, 9.0, c.robe[1]);
        pen.px(-5.0 + lean, by, 2.0, 9.0, c.robe[2]);
        pen.px(3.0 + lean, by + 1.0, 1.0, 8.0, c.robe[0]);
        pen.line(lean, by + 2.0, 3.0 + lean, by + 8.0, c.inner[0]);
        pen.line(-1.0 + lean, by + 2.0, 2.0 + lean, by + 8.0, c.trim[0]);
    }
    pen.px(-5.0 + lean, by, 10.0, 2.0, c.inner[1]);
    pen.px(-5.0 + lean, by + 1.0, 10.0, 1.0, c.inner[0]);
    pen.px(1.0 + lean, by, 2.0, 2.0, c.trim[0]);
    if pen.detailed() {
        pen.px(
            -4.45 + lean,
            by + 1.62,
            8.6,
            0.35,
            mix(c.inner[0], c.trim[0], 0.28),
        );
        pen.px(1.4 + lean, by + 0.4, 1.05, 0.75, c.trim[1]);
        pen.px(
            1.55 + lean,
            by + 1.25,
            0.72,
            0.35,
            mix(c.trim[0], WHITE, 0.45),
        );
    }
    pen.px(2.0 + lean, by - 3.0, 1.0, 3.0, c.ribbon[1]);
    pen.px(2.0 + lean, by - 4.0, 1.0, 1.0, c.jade);
    for i in 0..6 {
        let fi = f64::from(i);
        let flutter = if i > 3 {
            js_round(sin(t * 6.0 + fi))
        } else {
            0.0
        };
        let x = -6.0 + lean - fi + js_round(wind * fi * 0.3);
        pen.dot(
            x,
            by + 1.0 - js_round(fi / 2.0) + flutter,
            c.inner[usize::from(i >= 3)],
        );
    }
    // the head snaps back further than the body from a fresh blow, and hangs forward as it reels
    let eye = if recoil
        || reel
        || pose == Pose::Shocked
        || (pose == Pose::Channel && how == Some(Gather::Prayer))
    {
        Eye::Shut
    } else {
        Eye::Open
    };
    let nod = if recoil {
        -2.0
    } else if reel {
        1.0
    } else {
        0.0
    };
    pen.head(by + 9.0, lean + nod, eye);

    // arms and sword
    let sh = by + 7.0;
    match pose {
        Pose::Channel => {
            // a wind-up's progress; JS arithmetic takes a missing one as 0
            let k = p.unwrap_or(0.0);
            match how {
                Some(Gather::Summon) => {
                    // both arms rise out to the sides and up, a ball of qi swelling over the head
                    let up = js_round(2.0 + 9.0 * k);
                    let out = js_round(6.0 - 2.0 * k);
                    for (x0, side, shade) in [(-4.0, -1.0, 2), (3.0, 1.0, 1)] {
                        let x1 = x0 + side * out;
                        pen.line(x0 + lean, sh - 1.0, x1 + lean, sh - 1.0 + up, c.robe[shade]);
                        pen.line(
                            x0 + lean + side,
                            sh - 1.0,
                            x1 + lean + side,
                            sh - 1.0 + up,
                            c.robe[shade],
                        );
                        pen.dot(x0 + lean, sh - 1.0, c.trim[0]);
                        pen.hand(x1 - if side < 0.0 { 1.0 } else { 0.0 }, sh + up);
                    }
                    pen.orb(0.0, sh + up + 4.0, k);
                    pen.motes(0.0, sh + up + 4.0, k);
                    pen.sword(Held::Back, by);
                }
                Some(Gather::Palm | Gather::Lunge) => {
                    // the near palm drawn back to the hip, gathering; the far hand out, two fingers pointing
                    pen.sleeve(-7.0, sh - 4.0, 5.0, 3.0, 2);
                    pen.hand(-9.0, sh - 4.0);
                    pen.orb(-9.0, sh - 3.0, k);
                    pen.motes(-9.0, sh - 3.0, k);
                    pen.sleeve(3.0, sh - 1.0, 4.0, 2.0, 0);
                    pen.hand(7.0, sh - 1.0);
                    pen.px(9.0 + lean, sh, 2.0, 1.0, c.skin[0]);
                    pen.sword(
                        if how == Some(Gather::Lunge) {
                            Held::Hip
                        } else {
                            Held::Back
                        },
                        by,
                    );
                }
                Some(Gather::Prayer) => {
                    // the hands joined before the chest, eyes shut, qi rising round it
                    pen.sleeve(-2.0, sh - 4.0, 4.0, 3.0, 2);
                    pen.sleeve(1.0, sh - 4.0, 4.0, 3.0, 0);
                    pen.hand(4.0, sh - 3.0);
                    pen.px(5.0 + lean, sh - 1.0, 1.0, 3.0, c.skin[0]);
                    for i in 0..3 {
                        let fi = f64::from(i);
                        pen.dot(
                            -6.0 + fi * 6.0,
                            ((floor(t * 20.0) + fi * 5.0) % 14.0) + by,
                            c.glow,
                        );
                    }
                    pen.sword(Held::Back, by);
                }
                Some(Gather::Cross) | None => {
                    // arms crossed before the chest, braced
                    pen.line(-3.0 + lean, sh - 4.0, 6.0 + lean, sh, c.robe[2]);
                    pen.line(-3.0 + lean, sh, 6.0 + lean, sh - 4.0, c.robe[1]);
                    pen.hand(6.0, sh - 1.0);
                    pen.hand(6.0, sh - 5.0);
                    pen.sword(Held::Hip, by);
                }
            }
        }
        Pose::Cast => {
            if matches!(how, Some(Gather::Prayer | Gather::Cross)) {
                // arms flung wide as the spell goes out
                pen.sleeve(-9.0, sh - 1.0, 4.0, 3.0, 2);
                pen.sleeve(5.0, sh - 1.0, 4.0, 3.0, 0);
                pen.hand(-10.0, sh);
                pen.hand(9.0, sh);
            } else {
                // the near arm thrust out, palm open, the spell leaving it
                pen.sleeve(3.0, sh - 3.0, 8.0, 4.0, 0);
                pen.px(3.0 + lean, sh - 3.0, 2.0, 4.0, c.robe[2]);
                pen.hand(11.0, sh - 2.0);
                pen.px(13.0 + lean, sh - 2.0, 3.0, 3.0, c.glow);
                pen.dot(15.0 + lean, sh + 1.0, WHITE);
                pen.px(-1.0 + lean, sh - 2.0, 2.0, 2.0, c.skin[1]);
            }
            pen.sword(Held::Back, by);
        }
        Pose::Draw => {
            // the blade drawn back over the shoulder, higher the nearer the blow; from the air, point down
            if a.act.is_some_and(|k| k.dive) {
                pen.sleeve(1.0, sh - 6.0, 3.0, 6.0, 0);
                pen.hand(2.0, sh - 7.0);
                pen.line(3.0 + lean, sh - 8.0, 3.0 + lean, sh - 24.0, c.blade[1]);
                pen.line(4.0 + lean, sh - 8.0, 4.0 + lean, sh - 23.0, c.blade[0]);
                pen.px(1.0 + lean, sh - 7.0, 5.0, 1.0, c.trim[0]);
            } else {
                pen.sleeve(-7.0, sh - 1.0, 4.0, 4.0, 2);
                pen.hand(-8.0, sh + 2.0);
                pen.sword(Held::Raised, by + js_round(2.0 * p.unwrap_or(1.0)));
            }
        }
        Pose::Slash => {
            pen.sleeve(3.0, sh - 2.0, 6.0, 3.0, 0);
            pen.hand(9.0, sh - 2.0);
            pen.sword(Held::Thrust, by);
        }
        Pose::Guard => {
            pen.sleeve(3.0, sh - 2.0, 4.0, 4.0, 0);
            pen.hand(7.0, sh - 2.0);
            pen.sword(Held::Upright, by);
        }
        Pose::Win => {
            pen.sleeve(2.0, sh - 6.0, 3.0, 6.0, 0);
            pen.hand(2.0, sh - 7.0);
            pen.sword(Held::Planted, by);
        }
        Pose::Fly => {
            // arms out for balance, the sleeves streaming
            pen.sleeve(-9.0, sh - 1.0, 4.0, 3.0, 2);
            pen.sleeve(5.0, sh - 1.0, 4.0, 3.0, 0);
            pen.hand(-10.0, sh - 1.0);
            pen.hand(9.0, sh - 1.0);
        }
        _ if recoil => {
            // struck: arms thrown out behind and up
            pen.sleeve(-9.0, sh - 1.0, 4.0, 3.0, 2);
            pen.hand(-10.0, sh + 1.0);
            pen.sleeve(3.0, sh, 3.0, 4.0, 0);
            pen.hand(4.0, sh + 4.0);
            pen.sword(Held::Hip, by);
        }
        _ if reel => {
            // clutching the chest where it was struck, the other arm hanging
            pen.sleeve(-7.0, sh - 8.0, 3.0, 9.0, 2);
            pen.sleeve(1.0, sh - 3.0, 4.0, 3.0, 0);
            pen.hand(3.0, sh - 2.0);
            pen.sword(Held::Hip, by);
        }
        Pose::Shocked => {
            // locked rigid, arms jerking out
            let j = if sin(t * 60.0) > 0.0 { 1.0 } else { 0.0 };
            pen.sleeve(-9.0, sh - 1.0 + j, 4.0, 2.0, 2);
            pen.sleeve(5.0, sh - j, 4.0, 2.0, 0);
            pen.hand(-10.0, sh - 1.0 + j);
            pen.hand(9.0, sh - j);
            pen.sword(Held::Hip, by);
        }
        Pose::Kneel => {
            // a hand on the raised knee, pushing up
            pen.sleeve(-7.0, sh - 8.0, 3.0, 9.0, 2);
            pen.sleeve(3.0, sh - 5.0, 3.0, 5.0, 0);
            pen.hand(4.0, sh - 6.0);
            pen.sword(Held::Hip, by);
        }
        _ => {
            // idle, walk, jump, land, crouch, frozen: wide sleeves hanging, the sword at the hip
            pen.sleeve(-7.0, sh - 8.0, 3.0, 9.0, 2);
            pen.sleeve(4.0, sh - 8.0, 3.0, 9.0, 0);
            pen.px(4.0 + lean, sh - 8.0, 3.0, 1.0, c.trim[1]);
            pen.hand(5.0, sh - 9.0);
            pen.sword(Held::Hip, by);
        }
    }
    pen.sprite
}

/// Ring every painted pixel with `color`: each empty pixel touching a painted one (alpha over 128) takes it.
pub fn outline(sprite: &mut Sprite, color: Rgba) {
    let n = sprite.side() as i32;
    let radius = sprite.density.factor() as i32;
    let painted = |x: i32, y: i32| sprite.alpha(x, y) > 128;
    let mut ring = Vec::new();
    for y in 0..n {
        for x in 0..n {
            if sprite.alpha(x, y) > 0 {
                continue;
            }
            let touch = (-radius..=radius).any(|dy| {
                let width = radius - dy.abs();
                (-width..=width).any(|dx| painted(x + dx, y + dy))
            });
            if touch {
                ring.push((y * n + x) as usize);
            }
        }
    }
    for i in ring {
        if let Some(p) = sprite.px.get_mut(i) {
            *p = [color.0, color.1, color.2, 255];
        }
    }
}

/// Where a turning pose turns about, in pixels above the feet: a tumble round its middle, a roll round its ball.
fn pivot(pose: Pose) -> Option<i32> {
    match pose {
        Pose::Tumble => Some(sized(18.0) as i32),
        Pose::Roll => Some(sized(5.0) as i32),
        _ => None,
    }
}

/// The body's shape by the engine's squash and stretch ([`pixfx_core::squash`]), its area kept: stretched along
/// its flight (rising, falling, or both and sideways at once) while it moves fast, squashed upright as it lands
/// and easing back, itself otherwise. Frozen bodies and tumbling ones (turned, so their height is not up) keep
/// their shape.
fn shape(a: &Appearance) -> Shape {
    if a.frozen {
        return Shape::IDENTITY;
    }
    let spec = SquashSpec::default();
    match a.pose {
        // `vy` counts up and the sprite's rows count down
        Pose::Jump | Pose::Launched | Pose::Fall => {
            Shape::along(a.vx, -a.vy, squash::stretch(&spec, libm::hypot(a.vx, a.vy)))
        }
        Pose::Land => Shape::upright(squash::squash(&spec, a.pose_t, 1.0)),
        _ => Shape::IDENTITY,
    }
}

/// `sprite` drawn through `shape` about the feet so it stays on the floor, in whole pixels (each drawn pixel takes
/// the nearest painted one).
fn deform(sprite: &Sprite, shape: Shape) -> Sprite {
    if shape == Shape::IDENTITY {
        return sprite.clone();
    }
    let mut out = Sprite::new(sprite.density);
    let side = sprite.side();
    let d = sprite.density.factor() as i32;
    let (cx, feet) = (CX * d, FEET * d);
    for y in 0..side as i32 {
        for x in 0..side as i32 {
            let (fx, fy) = shape.source(f64::from(x - cx), f64::from(y - feet));
            if !(fx.is_finite() && fy.is_finite()) {
                continue;
            }
            let (sx, sy) = (cx + js_round(fx) as i32, feet + js_round(fy) as i32);
            let inside = (0..side as i32).contains(&sx) && (0..side as i32).contains(&sy);
            if !inside {
                continue;
            }
            let from = sprite.px.get(sy as usize * side + sx as usize).copied();
            if let (Some(p), Some(slot)) = (from, out.px.get_mut(y as usize * side + x as usize)) {
                *slot = p;
            }
        }
    }
    out
}

/// Quarter turns a tumbling or rolling fighter has made: one every four frames, the way it is moving.
fn quarter_turns(a: &Appearance) -> i32 {
    if pivot(a.pose).is_none() {
        return 0;
    }
    let way = sign_or(a.vx, -a.face);
    ((floor(a.pose_t * 15.0) % 4.0) * way) as i32
}

/// Draw fighter `a` into `frame` (world space; the ground at `ground_y`): a dodge's afterimage where the blow
/// was aimed, then the fighter with its gathering aura (site `drawFighter` without its marks).
pub fn draw(frame: &mut Frame, look: &Look, a: &Appearance, ground_y: f64) {
    if a.dodged > 0.0 && !a.out {
        let ghost = Appearance {
            x: a.x + a.face * sized(7.0),
            flash: true,
            jolt: [0, 0],
            ..*a
        };
        blit(frame, look, &ghost, ground_y, a.dodged * 1.5);
    }
    blit(frame, look, a, ground_y, untouchable_alpha(a));
}

/// How strongly a fighter is drawn: whole, or blinking faint at 10 Hz ([`squash::blink`]) while nothing can touch
/// it. Faint, not gone, so the pose still reads.
fn untouchable_alpha(a: &Appearance) -> f64 {
    if a.untouchable && !a.out {
        let spec = SquashSpec {
            blink_rate: 10.0,
            ..SquashSpec::default()
        };
        squash::blink(&spec, a.pose_t)
    } else {
        1.0
    }
}

fn blit(frame: &mut Frame, look: &Look, a: &Appearance, ground_y: f64, alpha: f64) {
    let t = a.pose_t;
    let density = frame.density();
    let d = density.factor() as i32;
    let factor = f64::from(d);
    let (pivot_x, feet) = (CX * d, FEET * d);
    let mut sprite = deform(&paint_at_density(look, a, density), shape(a));
    let windup = a.act.and_then(|k| k.windup);
    let p = windup.map(|w| w.progress);
    let glow = matches!(a.pose, Pose::Channel | Pose::Cast).then_some(look.glow);
    let ring = match glow {
        Some(g) if sin(t * 18.0) > -0.4 => g,
        _ => look.outline,
    };
    outline(&mut sprite, ring);
    // the last fifth of a long wind-up trembles with the power held in
    let tremble =
        windup.is_some_and(|w| w.progress > 0.8 && w.long) && floor(t * 30.0) % 2.0 != 0.0;
    let x = js_round(a.x * factor) as i32 - pivot_x + (a.jolt[0] + i32::from(tremble)) * d;
    let y = js_round((ground_y - a.air) * factor) as i32 - feet + a.jolt[1] * d;
    if let Some(glow) = glow {
        // an aura in two hard-edged rings, the inner one brighter, swelling as the spell builds
        let k = p.unwrap_or(1.0);
        let (cx, cy) = (
            f64::from(x + pivot_x),
            f64::from(y + feet) - sized(20.0) * factor,
        );
        for (rx, ry, base) in [
            (sized(10.0 + 4.0 * k), sized(22.0 + 3.0 * k), 0.07),
            (sized(7.0 + 2.0 * k), sized(17.0 + 2.0 * k), 0.09),
        ] {
            let (rx, ry) = (rx * factor, ry * factor);
            let ga = alpha * (base + 0.12 * k) * (0.85 + 0.15 * sin(t * 9.0));
            // the canvas takes a fill's alpha to 8 bits, then premultiplies the colour by it
            let a8 = js_round(ga * 255.0) as u32;
            let ch = |v: u8| mul_div255(u32::from(v), a8) as u8;
            let src = [ch(glow.0), ch(glow.1), ch(glow.2), a8 as u8];
            let mut row = -js_round(ry);
            while row <= js_round(ry) {
                let half = js_round(rx * sqrt(1.0 - (row * row) / (ry * ry)));
                if half.is_finite() {
                    let mut i = -half;
                    while i <= half {
                        over_premul(frame, (cx + i) as i32, (cy + row) as i32, src);
                        i += 1.0;
                    }
                }
                row += 1.0;
            }
        }
    }
    let a8 = js_round(alpha * 255.0) as u32;
    let turn = quarter_turns(a).rem_euclid(4);
    let pv = pivot(a.pose).map_or(0, |offset| feet - offset * d);
    let side = sprite.side();
    for sy in 0..side as i32 {
        for sx in 0..side as i32 {
            let Some(&p) = sprite.px.get(sy as usize * side + sx as usize) else {
                continue;
            };
            if p[3] == 0 {
                continue;
            }
            // a pixel's centre turned about the pivot, in whole quarter turns
            let (u, v) = (sx - pivot_x, sy - pv);
            let (dx, dy) = match turn {
                1 => (-v - 1, u),
                2 => (-u - 1, -v - 1),
                3 => (v, -u - 1),
                _ => (u, v),
            };
            let (wx, wy) = (x + pivot_x + dx, y + pv + dy);
            let src = if alpha >= 1.0 {
                p
            } else {
                p.map(|v| mul_div255(u32::from(v), a8) as u8)
            };
            over_premul(frame, wx, wy, src);
        }
    }
}
