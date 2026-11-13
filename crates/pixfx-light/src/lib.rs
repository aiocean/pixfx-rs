//! Scene light: the glow a spell, a blow or a lamp throws on the world around it. A [`Light`] is a value that
//! ages ([`step`]); at any instant it shows as a [`Glow`] ([`Light::glow`]); [`draw`] adds glows to a frame's light
//! layer in hard-edged rings, one pixel per game pixel, so the glow keeps a pixel-art look. Nothing here holds
//! state, reads a clock or rolls dice: the caller owns the list of lights and passes time in. How the light
//! swells and fades, and its rings, are data ([`Style`]); which light a game event casts is the game's choice.
//!
//! [`draw`] adds what a Canvas 2D `fillRect` under `globalCompositeOperation = "lighter"` and `globalAlpha` adds
//! (Chrome's raster, 8 bits a channel), so the light layer drawn additively over a scene matches a canvas that
//! drew the same rings itself.
#![forbid(unsafe_code)]

use pixfx_core::math::{js_max, js_min, js_round};
use pixfx_core::{Frame, Rgba};

/// A light in the world: where it is, how far it reaches and how bright it burns at its heart, how long it lives
/// and how long it has lived.
///
/// As data: `{key?, x, y, reach, color: "#rrggbb", strength, life?, age?}`; `key` and `life` absent or null for
/// none, `age` absent for a new light.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(missing_as_null))]
pub struct Light {
    /// The caller's tag, to find the light again (to move it or put it out); `None` for an untagged light.
    #[cfg_attr(feature = "serde", serde(default))]
    pub key: Option<u32>,
    /// Centre, in game pixels (world space, the frame's offset applies).
    pub x: f64,
    pub y: f64,
    /// Radius of the outermost ring at full brightness, in game pixels.
    pub reach: f64,
    /// Its colour (alpha ignored).
    pub color: Rgba,
    /// Brightness at its heart at full swell, 0 to 1.
    pub strength: f64,
    /// Seconds it lives; `None` until it is put out ([`Light::put_out`]).
    #[cfg_attr(feature = "serde", serde(default))]
    pub life: Option<f64>,
    /// Seconds it has lived.
    #[cfg_attr(feature = "serde", serde(default))]
    pub age: f64,
}

/// A light as it shows at one instant: what [`draw`] draws.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct Glow {
    pub x: f64,
    pub y: f64,
    pub reach: f64,
    pub color: Rgba,
    pub strength: f64,
}

/// One ring of a glow: out to `out` of its reach, `bright` of its strength.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    pub out: f64,
    pub bright: f64,
}

/// How lights swell, fade and are drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    /// Rings from the heart outwards: `out` rising, `bright` falling.
    pub rings: Vec<Ring>,
    /// A light with a life swells to full in `1 / rise` seconds, then fades linearly to nothing at the end of it.
    pub rise: f64,
    /// A light living until put out swells to full in `1 / rise_held` seconds and holds.
    pub rise_held: f64,
    /// Its reach at swell `k` (0 to 1) is `reach * (reach_base + reach_swell * k)`.
    pub reach_base: f64,
    pub reach_swell: f64,
    /// A glow this faint or fainter is not drawn.
    pub faint: f64,
}

impl Default for Style {
    /// The duel's light: in fast, out slow, in four rings.
    fn default() -> Self {
        Self {
            rings: vec![
                Ring {
                    out: 0.3,
                    bright: 1.0,
                },
                Ring {
                    out: 0.55,
                    bright: 0.6,
                },
                Ring {
                    out: 0.8,
                    bright: 0.3,
                },
                Ring {
                    out: 1.0,
                    bright: 0.12,
                },
            ],
            rise: 12.0,
            rise_held: 8.0,
            reach_base: 0.8,
            reach_swell: 0.2,
            faint: 0.01,
        }
    }
}

impl Light {
    /// The light `dt` seconds older; `None` once it has gone out (its age reached its life).
    pub fn aged(self, dt: f64) -> Option<Light> {
        let age = self.age + dt;
        let lit = self.life.is_none_or(|life| age < life);
        lit.then_some(Light { age, ..self })
    }

    /// The light set to go out at most `within` seconds from now.
    pub fn put_out(self, within: f64) -> Light {
        let end = self.age + within;
        let life = self.life.map_or(end, |life| js_min(life, end));
        Light {
            life: Some(life),
            ..self
        }
    }

    /// How it shows now: swelling in fast, then fading out over its life; a light without a life holds once
    /// swelled.
    pub fn glow(&self, style: &Style) -> Glow {
        let k = match self.life {
            None => js_min(1.0, self.age * style.rise_held),
            Some(life) => js_min(1.0, self.age * style.rise) * (1.0 - self.age / life),
        };
        Glow {
            x: self.x,
            y: self.y,
            reach: self.reach * (style.reach_base + style.reach_swell * k),
            color: self.color,
            strength: self.strength * k,
        }
    }
}

/// Let `dt` seconds pass: every light ages, those whose life is over are gone.
pub fn step(lights: Vec<Light>, dt: f64) -> Vec<Light> {
    lights.into_iter().filter_map(|l| l.aged(dt)).collect()
}

/// Add each glow to `frame`'s light layer in `style`'s rings: each ring a disc of whole pixels around the glow's
/// centre (rounded to a whole pixel), the widest first, each inner ring adding its share on top. Channels
/// saturate at 255.
pub fn draw(frame: &mut Frame, glows: &[Glow], style: &Style) {
    for g in glows {
        glow(frame, g, &style.rings, style.faint);
    }
}

fn glow(frame: &mut Frame, g: &Glow, rings: &[Ring], faint: f64) {
    let finite = [g.x, g.y, g.reach, g.strength]
        .iter()
        .all(|v| v.is_finite());
    if !finite || g.strength <= faint || g.reach < 1.0 {
        return;
    }
    let (cx, cy) = (js_round(g.x) as i64, js_round(g.y) as i64);
    for (i, ring) in rings.iter().enumerate().rev() {
        let next = rings.get(i + 1).map_or(0.0, |r| r.bright);
        let alpha = g.strength * (ring.bright - next);
        disc(
            frame,
            cx,
            cy,
            js_round(g.reach * ring.out) as i64,
            g.color,
            alpha,
        );
    }
}

/// A filled disc of radius `r`, one row at a time (the canvas `fillRect` per row the light was designed with).
fn disc(frame: &mut Frame, cx: i64, cy: i64, r: i64, c: Rgba, alpha: f64) {
    let add = [c.0, c.1, c.2].map(|v| lighter(v, alpha));
    if add == [0; 3] {
        return;
    }
    for y in -r..=r {
        let half = js_round(libm::sqrt((r * r - y * y) as f64)) as i64;
        for x in cx - half..=cx + half {
            add_px(frame, x, cy + y, add);
        }
    }
}

/// What a canvas `fillRect` of channel `v` under `globalAlpha = alpha` adds in `"lighter"` mode: the colour
/// premultiplied and stored to 8 bits in single precision, as Chrome's software raster does (checked against
/// headless Chrome on every channel value at 171 alphas, 40 pairs of fills on one pixel, and 446 half-way cases
/// where f64 rounding gives another byte). An alpha outside 0 to 1 is clamped; a canvas ignores such a
/// `globalAlpha` and keeps the previous one.
fn lighter(v: u8, alpha: f64) -> u8 {
    let a = js_min(1.0, js_max(0.0, alpha)) as f32;
    let premul = (f32::from(v) / 255.0) * a;
    // 0 <= premul * 255 + 0.5 < 256: truncation is floor
    (premul * 255.0 + 0.5) as u8
}

fn add_px(frame: &mut Frame, x: i64, y: i64, add: [u8; 3]) {
    let (lx, ly) = (x - i64::from(frame.ox), y - i64::from(frame.oy));
    if lx < 0 || ly < 0 || lx >= i64::from(frame.w) || ly >= i64::from(frame.h) {
        return;
    }
    let Some(px) = frame.light.get_mut((ly * i64::from(frame.w) + lx) as usize) else {
        return;
    };
    let w = *px;
    let ch = |s: u32, a: u8| (((w >> s) & 255) + u32::from(a)).min(255) << s;
    *px = ch(0, add[0]) | ch(8, add[1]) | ch(16, add[2]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spark(life: Option<f64>) -> Light {
        Light {
            key: Some(1),
            x: 10.0,
            y: 10.0,
            reach: 30.0,
            color: Rgba(255, 128, 0, 255),
            strength: 0.42,
            life,
            age: 0.0,
        }
    }

    #[test]
    fn a_flash_swells_fades_and_goes_out_while_a_held_light_lasts_until_put_out() {
        let style = Style::default();
        let flash = step(vec![spark(Some(0.5))], 0.25);
        // halfway through its life: fully swelled, half faded
        assert_eq!(flash[0].glow(&style).strength, 0.42 * 0.5);
        assert!(step(flash, 0.25).is_empty());

        let held = step(vec![spark(None)], 100.0);
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].glow(&style).strength, 0.42);
        let out = held.into_iter().map(|l| l.put_out(0.2)).collect();
        assert!(step(out, 0.2).is_empty());
    }

    #[test]
    fn rings_add_more_light_towards_the_heart_and_nothing_past_the_reach() {
        let mut frame = Frame::new(40, 1, -20, 0);
        let glow = Glow {
            x: 0.0,
            y: 0.0,
            reach: 10.0,
            color: Rgba(255, 255, 255, 255),
            strength: 1.0,
        };
        draw(&mut frame, &[glow], &Style::default());
        let red = |x: i32| frame.light[(x + 20) as usize] & 255;
        // heart: every ring adds, 0.12 + 0.18 + 0.3 + 0.4 of 255; the outer ring alone 0.12; past it nothing
        assert_eq!([red(0), red(9), red(10), red(11)], [255, 31, 31, 0]);
        assert!(red(0) > red(4) && red(4) > red(7) && red(7) > red(9));
    }
}
