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
use crate::traced::{self, Traced};
use core::f64::consts::PI;
use libm::{cos, floor, sin, sqrt};
use pixfx_core::squash::{self, Shape, SquashSpec};
use pixfx_draw::blend::{cover_over, mul_div255, over_premul, straighten};
use pixfx_draw::math::{js_max, js_min, js_round};
use pixfx_draw::{Frame, Rgba, RgbaImage};

/// Sprite width and height, in pixels.
pub const SPRITE: usize = 64;
/// Where the feet are in the sprite: x of the body's middle...
pub const CX: i32 = 32;
/// ...and y of the soles.
pub const FEET: i32 = 60;

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

/// A painted fighter: 64x64 pixels, premultiplied RGBA8, feet at ([`CX`], [`FEET`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    px: Vec<[u8; 4]>,
}

impl Default for Sprite {
    fn default() -> Self {
        Sprite {
            px: vec![[0; 4]; SPRITE * SPRITE],
        }
    }
}

impl Sprite {
    /// The whole sprite as a straight-alpha RGBA image.
    pub fn to_image(&self) -> RgbaImage {
        RgbaImage {
            w: SPRITE as u32,
            h: SPRITE as u32,
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
        if x >= SPRITE {
            return None;
        }
        self.px.get(y * SPRITE + x).map(|&p| straighten(p))
    }

    fn alpha(&self, x: i32, y: i32) -> u8 {
        let inside = (0..SPRITE as i32).contains(&x) && (0..SPRITE as i32).contains(&y);
        if !inside {
            return 0;
        }
        self.px
            .get(y as usize * SPRITE + x as usize)
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
        for row in y0.max(0)..y1.min(SPRITE as i32) {
            let (first, last) = (l >> 8, (r - 1) >> 8);
            for col in first.max(0)..=last.min(SPRITE as i32 - 1) {
                let cover = if first == last {
                    r - l
                } else if col == first {
                    256 - (l & 255)
                } else if col == last {
                    r - (last << 8)
                } else {
                    256
                };
                if let Some(p) = self.px.get_mut(row as usize * SPRITE + col as usize) {
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
    fn px(&mut self, dx: f64, dy: f64, w: f64, h: f64, col: Rgba) {
        let x = if self.face > 0.0 {
            32.0 + dx
        } else {
            32.0 - dx - w + 1.0
        };
        let col = match self.tint {
            Tint::Own => col,
            Tint::Flash => WHITE,
            Tint::Shock => SHOCK,
            Tint::Ice if lum(col) > 0.7 => ICE_LIGHT,
            Tint::Ice if lum(col) > 0.4 => ICE_MID,
            Tint::Ice => ICE_DARK,
        };
        self.sprite.fill_rect(x, 60.0 - dy - h + 1.0, w, h, col);
    }

    /// A traced pose's rects, each in its role's colour.
    fn traced(&mut self, rects: &[Traced]) {
        for r in rects {
            let col = r.role.color(self.c);
            self.px(
                f64::from(r.dx),
                f64::from(r.dy),
                f64::from(r.w),
                f64::from(r.h),
                col,
            );
        }
    }

    fn dot(&mut self, dx: f64, dy: f64, col: Rgba) {
        self.px(dx, dy, 1.0, 1.0, col);
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, col: Rgba) {
        let n = js_max(js_max((x1 - x0).abs(), (y1 - y0).abs()), 1.0);
        let mut i = 0.0;
        while i <= n {
            self.dot(
                js_round(x0 + ((x1 - x0) * i) / n),
                js_round(y0 + ((y1 - y0) * i) / n),
                col,
            );
            i += 1.0;
        }
    }

    /// A sleeve standing out from the robe behind it, edged dark along its back and top.
    fn sleeve(&mut self, dx: f64, dy: f64, w: f64, h: f64, shade: usize) {
        let c = self.c;
        let edge = if shade == 2 { c.outline } else { c.robe[2] };
        let x = dx + self.lean;
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
        let skin = self.c.skin[0];
        self.px(dx + self.lean, dy, 2.0, 2.0, skin);
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

    /// A fighter on its back, `up` pixels off the floor: feet toward the foe, head away, legs raised `legs`
    /// pixels, arms flung up `arms` (0: along its sides), its chest lifting `breath`.
    fn lying(&mut self, up: f64, legs: f64, arms: f64, breath: f64, eye: Eye) {
        let c = self.c;
        let l = up + legs;
        let m = up + js_min(legs, 2.0);
        self.px(12.0, l, 3.0, 4.0, c.boot);
        self.px(-1.0, m, 13.0, 4.0, c.robe[1]);
        self.px(-1.0, m + 3.0, 13.0, 1.0, c.robe[0]);
        self.px(-1.0, m, 13.0, 1.0, c.robe[2]);
        self.px(10.0, m, 2.0, 4.0, c.trim[0]);
        self.px(-11.0, up, 11.0, 5.0 + breath, c.robe[1]);
        self.px(-11.0, up, 11.0, 1.0, c.robe[2]);
        self.px(-9.0, up + 4.0 + breath, 5.0, 1.0, c.inner[0]);
        self.px(-8.0, up + 3.0 + breath, 3.0, 1.0, c.inner[1]);
        self.px(-2.0, up, 1.0, 5.0 + breath, c.trim[0]);
        if arms > 0.0 {
            // flung up and back past the head from the shoulders
            for (x0, len, shade) in [(-10.0, arms, 1), (-8.0, arms - 2.0, 2)] {
                let x1 = x0 - js_round(len * 0.6);
                let mut i = 0.0;
                while i <= len {
                    self.px(
                        js_round(x0 + ((x1 - x0) * i) / len),
                        up + 5.0 + i,
                        2.0,
                        1.0,
                        c.robe[shade],
                    );
                    i += 1.0;
                }
                self.px(x1 - 1.0, up + 6.0 + len, 2.0, 2.0, c.skin[0]);
            }
        } else {
            self.px(-9.0, up + 5.0 + breath, 7.0, 1.0, c.robe[2]);
            self.px(-2.0, up + 5.0 + breath, 2.0, 1.0, c.skin[0]);
        }
        // the head, face up
        self.px(-18.0, up, 7.0, 6.0, c.skin[0]);
        self.px(-18.0, up, 7.0, 1.0, c.skin[1]);
        if eye == Eye::Crossed {
            for (x, y) in [
                (-15.0, 3.0),
                (-14.0, 4.0),
                (-13.0, 3.0),
                (-15.0, 5.0),
                (-13.0, 5.0),
            ] {
                self.px(x, up + y, 1.0, 1.0, c.outline);
            }
        } else {
            self.px(-15.0, up + 4.0, 3.0, 1.0, c.outline);
        }
        match c.hairdo {
            Hairdo::Topknot => {
                self.px(-21.0, up, 3.0, 6.0, c.hair[1]);
                self.px(-23.0, up + 1.0, 2.0, 3.0, c.hair[1]);
                self.px(-22.0, up + 4.0, 1.0, 1.0, c.trim[0]);
                self.px(-25.0, up, 2.0, 1.0, c.ribbon[0]);
            }
            Hairdo::Flowing => {
                self.px(-21.0, up, 3.0, 6.0, c.hair[0]);
                self.px(-29.0, up, 9.0, 2.0, c.hair[0]);
                self.px(-29.0, up, 9.0, 1.0, c.hair[1]);
                self.px(-20.0, up + 5.0, 2.0, 1.0, c.jade);
            }
        }
    }

    /// Neck, face, an eye (open, or screwed shut), a brow and the hair, the neck at `y`.
    fn head(&mut self, y: f64, lean: f64, eye: Eye) {
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
                self.line(-9.0 + lean, by - 9.0, 3.0 + lean, by - 1.0, dull);
                self.line(-9.0 + lean, by - 8.0, 2.0 + lean, by - 1.0, c.trim[1]);
                self.px(3.0 + lean, by - 1.0, 1.0, 2.0, c.trim[0]);
                self.line(4.0 + lean, by, 6.0 + lean, by + 1.0, c.robe[2]);
                self.px(4.0 + lean, by - 3.0, 1.0, 2.0, c.ribbon[0]);
            }
        }
    }
}

/// Paint fighter `a`'s pose into a sprite, not yet outlined. A tumble is painted spread-eagled; [`draw`] turns it.
pub fn paint(look: &Look, a: &Appearance) -> Sprite {
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
    // limbs thrown about while it flies: they swing every few frames
    let flail = if still { 0.0 } else { floor(t * 12.0) % 2.0 };
    let mut pen = Pen {
        sprite: Sprite::default(),
        c: look,
        face: a.face,
        tint,
        t,
        wind,
        lean: 0.0,
    };
    let c = look;
    // a pose traced from drawn art paints from its table
    if let Some(rects) = traced::pose(a.pose, c.hairdo) {
        pen.traced(rects);
        if a.pose == Pose::Down && a.out {
            pen.line(-6.0, -1.0, 10.0, -1.0, c.blade[2]);
        }
        return pen.sprite;
    }
    match a.pose {
        Pose::Down => {
            // flat on its back; a knocked-out fighter's eyes are crossed out and its sword lies dropped beside it
            let breath = if !a.out && !still && floor(t * 2.5) % 2.0 != 0.0 {
                1.0
            } else {
                0.0
            };
            pen.lying(
                0.0,
                0.0,
                0.0,
                breath,
                if a.out { Eye::Crossed } else { Eye::Shut },
            );
            if a.out {
                pen.line(-6.0, -1.0, 10.0, -1.0, c.blade[2]);
                pen.px(11.0, -1.0, 1.0, 1.0, c.trim[0]);
            }
            return pen.sprite;
        }
        // thrown up and back: feet higher than its head, arms flung up past it
        Pose::Launched => {
            pen.lying(6.0, 3.0, 5.0 + flail, 0.0, Eye::Shut);
            return pen.sprite;
        }
        // falling flat on its back, arms and legs trailing up as it drops
        Pose::Fall => {
            pen.lying(6.0, 1.0, 7.0 - flail, 0.0, Eye::Shut);
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
            // dropped low under the blow, body bent level, an arm over the head. The site asks
            // `c.school === "thunder"` of a palette that has no school, so both looks take hair[0] here.
            pen.px(-5.0, 0.0, 4.0, 2.0, c.boot);
            pen.px(3.0, 0.0, 4.0, 2.0, c.boot);
            pen.px(-7.0, 2.0, 15.0, 5.0, c.robe[1]);
            pen.px(-7.0, 2.0, 15.0, 1.0, c.trim[0]);
            pen.px(-7.0, 2.0, 2.0, 5.0, c.robe[2]);
            pen.px(-5.0, 7.0, 11.0, 5.0, c.robe[1]);
            pen.px(-5.0, 11.0, 11.0, 1.0, c.robe[2]);
            pen.px(2.0, 7.0, 3.0, 4.0, c.inner[0]);
            pen.px(6.0, 6.0, 6.0, 6.0, c.skin[0]);
            pen.px(6.0, 6.0, 2.0, 6.0, c.skin[1]);
            pen.px(10.0, 8.0, 1.0, 1.0, c.outline);
            pen.px(5.0, 11.0, 8.0, 3.0, c.hair[0]);
            pen.px(4.0, 12.0, 2.0, 2.0, c.hair[0]);
            pen.px(3.0, 13.0, 3.0, 1.0, c.trim[0]);
            pen.px(6.0, 4.0, 5.0, 3.0, c.robe[1]);
            pen.px(11.0, 4.0, 2.0, 2.0, c.skin[0]);
            pen.line(-10.0, 6.0, -3.0, 3.0, c.blade[2]);
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
            pen.px(8.0, 4.0, 2.0, 2.0, c.skin[0]);
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
    let spread_eagled = pose == Pose::Tumble;
    let p = a.act.and_then(|k| k.windup).map(|w| w.progress);
    let how = a.act.map(|k| k.how);
    // a fresh blow snaps it back; then it reels, bent over the hurt, lower the harder it was struck
    let recoil = (pose == Pose::Hurt && a.hit_t < 0.1) || spread_eagled;
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
    let spread = if spread_eagled {
        3.0
    } else if drop > 0.0 && pose != Pose::Kneel {
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
    while dy <= top {
        let flare = js_max(0.0, js_round((8.0 - dy) / 2.0)) + if drop > 0.0 { 1.0 } else { 0.0 };
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

    // the body: robe crossed over the inner robe, a sash with a jade pendant and a tail that streams back
    let by = 16.0 - bob - drop;
    pen.px(-5.0 + lean, by, 10.0, 9.0, c.robe[1]);
    pen.px(-5.0 + lean, by, 2.0, 9.0, c.robe[2]);
    pen.px(3.0 + lean, by + 1.0, 1.0, 8.0, c.robe[0]);
    pen.line(lean, by + 2.0, 3.0 + lean, by + 8.0, c.inner[0]);
    pen.line(-1.0 + lean, by + 2.0, 2.0 + lean, by + 8.0, c.trim[0]);
    pen.px(-5.0 + lean, by, 10.0, 2.0, c.inner[1]);
    pen.px(-5.0 + lean, by + 1.0, 10.0, 1.0, c.inner[0]);
    pen.px(1.0 + lean, by, 2.0, 2.0, c.trim[0]);
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

/// Traced `rects` painted and outlined as the duel draws a pose of `look` facing right: what `pixfx trace` checks
/// the engine's own painting against.
pub fn paint_traced(look: &Look, rects: &[Traced]) -> Sprite {
    let mut pen = Pen {
        sprite: Sprite::default(),
        c: look,
        face: 1.0,
        tint: Tint::Own,
        t: 0.0,
        wind: 0.0,
        lean: 0.0,
    };
    pen.traced(rects);
    let mut sprite = pen.sprite;
    outline(&mut sprite, look.outline);
    sprite
}

/// Ring every painted pixel with `color`: each empty pixel touching a painted one (alpha over 128) takes it.
pub fn outline(sprite: &mut Sprite, color: Rgba) {
    let n = SPRITE as i32;
    let painted = |s: &Sprite, x: i32, y: i32| s.alpha(x, y) > 128;
    let mut ring = Vec::new();
    for y in 0..n {
        for x in 0..n {
            if sprite.alpha(x, y) > 0 {
                continue;
            }
            if painted(sprite, x - 1, y)
                || painted(sprite, x + 1, y)
                || painted(sprite, x, y - 1)
                || painted(sprite, x, y + 1)
            {
                ring.push(y as usize * SPRITE + x as usize);
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
        Pose::Tumble => Some(18),
        Pose::Roll => Some(5),
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
    let mut out = Sprite::default();
    for y in 0..SPRITE as i32 {
        for x in 0..SPRITE as i32 {
            let (fx, fy) = shape.source(f64::from(x - CX), f64::from(y - FEET));
            if !(fx.is_finite() && fy.is_finite()) {
                continue;
            }
            let (sx, sy) = (CX + js_round(fx) as i32, FEET + js_round(fy) as i32);
            let inside = (0..SPRITE as i32).contains(&sx) && (0..SPRITE as i32).contains(&sy);
            if !inside {
                continue;
            }
            let from = sprite.px.get(sy as usize * SPRITE + sx as usize).copied();
            if let (Some(p), Some(slot)) = (from, out.px.get_mut(y as usize * SPRITE + x as usize))
            {
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
            x: a.x + a.face * 7.0,
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
    let mut sprite = deform(&paint(look, a), shape(a));
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
    let x = js_round(a.x) as i32 - CX + a.jolt[0] + i32::from(tremble);
    let y = js_round(ground_y - a.air) as i32 - FEET + a.jolt[1];
    if let Some(glow) = glow {
        // an aura in two hard-edged rings, the inner one brighter, swelling as the spell builds
        let k = p.unwrap_or(1.0);
        let (cx, cy) = (f64::from(x + CX), f64::from(y + FEET - 20));
        for (rx, ry, base) in [
            (10.0 + 4.0 * k, 22.0 + 3.0 * k, 0.07),
            (7.0 + 2.0 * k, 17.0 + 2.0 * k, 0.09),
        ] {
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
    let pv = pivot(a.pose).map_or(0, |d| FEET - d);
    for sy in 0..SPRITE as i32 {
        for sx in 0..SPRITE as i32 {
            let Some(&p) = sprite.px.get(sy as usize * SPRITE + sx as usize) else {
                continue;
            };
            if p[3] == 0 {
                continue;
            }
            // a pixel's centre turned about the pivot, in whole quarter turns
            let (u, v) = (sx - CX, sy - pv);
            let (dx, dy) = match turn {
                1 => (-v - 1, u),
                2 => (-u - 1, -v - 1),
                3 => (v, -u - 1),
                _ => (u, v),
            };
            let (wx, wy) = (x + CX + dx, y + pv + dy);
            let src = if alpha >= 1.0 {
                p
            } else {
                p.map(|v| mul_div255(u32::from(v), a8) as u8)
            };
            over_premul(frame, wx, wy, src);
        }
    }
}
