//! Outlines: a ring of one colour round what an effect draws, so it reads over any background. Every effect
//! takes [`Options::outline`]: unset, it looks as designed (no ring, or the outline it draws itself); `Off`, it
//! has none; `On`, it has a ring of that colour and width. An effect that draws its own outline
//! ([`crate::Effect::draws_outline`]) recolours or drops that one ([`Options::outline_color`]) instead of getting a
//! second ring. pixelfx has no such option, so unset renders match it.

use std::fmt;

use crate::color::{Rgba, hex};
use crate::effect::Options;
use crate::frame::Frame;

/// The widest ring, in pixels.
pub const MAX_WIDTH: u32 = 3;

/// A drawn pixel is ringed when at least this many of its 8 neighbours are drawn too. The painter draws a faint
/// colour as a scatter of whole pixels (ordered dither), so smoke, haze and lone motes have few drawn neighbours
/// and get no ghostly halo, while a shape, a 1 px line or a half-faded checkerboard still does.
pub const SHAPE_NEIGHBOURS: u32 = 2;

/// The engine's dark outline (the explosions' rocks, the fighter): a near-black violet that reads over light and
/// dark ground alike.
pub const DARK: Rgba = Rgba::hex24(0x1a1422);

/// A ring round an effect's pixels: its colour, how many pixels wide, and whether it fills the corners (a square
/// ring) or cuts them (a rounded, pixel-art ring).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ring {
    color: Rgba,
    width: u32,
    corners: bool,
}

impl Ring {
    /// A ring, or none when `width` is 0 or wider than [`MAX_WIDTH`].
    pub const fn new(color: Rgba, width: u32, corners: bool) -> Option<Ring> {
        if width == 0 || width > MAX_WIDTH {
            None
        } else {
            Some(Ring {
                color,
                width,
                corners,
            })
        }
    }

    /// A 1 px ring of `color` without corners, the pixel-art default.
    pub const fn thin(color: Rgba) -> Ring {
        Ring {
            color,
            width: 1,
            corners: false,
        }
    }

    pub const fn color(&self) -> Rgba {
        self.color
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn corners(&self) -> bool {
        self.corners
    }
}

/// An effect's outline as the caller sets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outline {
    /// None at all, not even one the effect draws itself.
    Off,
    On(Ring),
}

/// The outlines ready to pick by name: `dark` (the engine's own), `black`, `white` (over dark scenes), and
/// `thick` (dark, 2 px, square).
pub const PRESETS: [(&str, Ring); 4] = [
    ("dark", Ring::thin(DARK)),
    ("black", Ring::thin(Rgba::hex24(0x000000))),
    ("white", Ring::thin(Rgba::hex24(0xffffff))),
    (
        "thick",
        Ring {
            color: DARK,
            width: 2,
            corners: true,
        },
    ),
];

/// A preset's ring by name.
pub fn preset(name: &str) -> Option<Ring> {
    PRESETS.iter().find(|(n, _)| *n == name).map(|&(_, r)| r)
}

/// A one-word outline that is neither `off`, a preset nor a hex colour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineError(pub String);

impl fmt::Display for OutlineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "outline {:?} is not \"off\", a preset (", self.0)?;
        for (i, (name, _)) in PRESETS.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            f.write_str(name)?;
        }
        f.write_str(") or a hex colour")
    }
}

impl std::error::Error for OutlineError {}

impl Outline {
    /// An outline named in one word, as hosts take it: `"off"`, a preset's name, or a hex colour (a 1 px ring of
    /// it).
    pub fn parse(word: &str) -> Result<Outline, OutlineError> {
        if word == "off" {
            return Ok(Outline::Off);
        }
        if let Some(r) = preset(word) {
            return Ok(Outline::On(r));
        }
        hex(word)
            .map(|c| Outline::On(Ring::thin(c)))
            .map_err(|_| OutlineError(word.to_owned()))
    }
}

impl Options {
    /// For an effect that draws its own outline: the colour to draw it in, `default` unless the caller set one,
    /// or none when the caller turned outlines off.
    pub fn outline_color(&self, default: Rgba) -> Option<Rgba> {
        match self.outline {
            None => Some(default),
            Some(Outline::Off) => None,
            Some(Outline::On(r)) => Some(r.color),
        }
    }
}

/// Ring the pixels drawn into `frame` since its colour was `before` (same size): each empty pixel within the ring's
/// width of a drawn one (part of a shape: [`SHAPE_NEIGHBOURS`]) takes the ring's colour, as opaque as the most opaque drawn pixel it
/// rings, so the ring fades with the effect. Light is left alone.
pub(crate) fn ring_round(frame: &mut Frame, before: &[u32], ring: Ring) {
    let (w, h) = (frame.w as usize, frame.h as usize);
    let reach = ring.width as isize;
    let drawn: Vec<bool> = frame
        .color
        .iter()
        .zip(before)
        .map(|(&now, &was)| now != was && now >> 24 != 0)
        .collect();
    let is_drawn = |x: isize, y: isize| {
        x >= 0
            && y >= 0
            && (x as usize) < w
            && (y as usize) < h
            && drawn
                .get(y as usize * w + x as usize)
                .copied()
                .unwrap_or(false)
    };
    let mut alpha = vec![0u8; w * h];
    for (i, &now) in frame.color.iter().enumerate() {
        if !drawn.get(i).copied().unwrap_or(false) {
            continue;
        }
        let (x, y) = ((i % w) as isize, (i / w) as isize);
        let around = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ]
        .into_iter()
        .filter(|&(dx, dy)| is_drawn(x + dx, y + dy))
        .count() as u32;
        if around < SHAPE_NEIGHBOURS {
            continue;
        }
        let a = (now >> 24) as u8;
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                if !ring.corners && dx.abs() + dy.abs() > reach {
                    continue;
                }
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                    continue;
                }
                if let Some(m) = alpha.get_mut(ny as usize * w + nx as usize) {
                    *m = (*m).max(a);
                }
            }
        }
    }
    let rgb = ring.color.pack() & 0x00ff_ffff;
    let ink = u32::from(ring.color.3);
    for (c, &a) in frame.color.iter_mut().zip(&alpha) {
        if a > 0 && *c >> 24 == 0 {
            *c = rgb | (u32::from(a) * ink / 255) << 24;
        }
    }
}
