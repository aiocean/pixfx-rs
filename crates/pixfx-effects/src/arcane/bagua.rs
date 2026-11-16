//! The bagua mirror every bagua effect shows (`arcane/bagua-mirror`, the shield behind the character's back, and
//! `arcane/bagua-mirror-strike` with its flight and burst): one octagonal bronze mirror ringed with the
//! eight trigrams round a jade face, painted at any radius and turn, with its glow, the reflected beam, shards and
//! sparks. Made here from the parts sheets of `pixfx-ref`'s `examples/specs/shield/bagua-mirror.json`;
//! the colours are the exact values of the drawn MAIN (`parts/review/matrix.json`).
//!
//! The mirror's rings, from the edge in, follow the drawn MAIN: a dark line, a bronze rim lit from the top left, a
//! dark band holding the trigram bars, a gold ring, a deep teal ring and the pale jade face with its swirl. Below
//! `SMALL` pixels of radius each trigram is one tick. The glow is painted in steps
//! (`pixfx_draw::halo`), as on the sheet: a one-pixel jade edge and a dither, growing when the mirror flashes.
//!
//! Both effects wrap their actor: [`Body`] reads the sprite once each time it changes (its outline, its chest), and
//! [`outline`] runs a jade line along it, washing down, pulsing, flaring under a blow and fading.

// each effect built alone uses only some of these
#![cfg_attr(
    not(all(
        feature = "arcane-bagua-mirror",
        feature = "arcane-bagua-mirror-strike"
    )),
    allow(dead_code)
)]

use core::f64::consts::{FRAC_PI_4, PI, TAU};

use pixfx_core::{Actor, Area, Silhouette};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::halo::Halo;
use pixfx_draw::math::{clamp, rand};

/// The drawn MAIN's colours (`matrix.json` letters in brackets).
pub const DARK: Rgba = Rgba::hex24(0x1d1e16); // b
pub const BAND: Rgba = Rgba::hex24(0x2e3124); // c
pub const GOLD_HI: Rgba = Rgba::hex24(0xf2d769); // d
pub const GOLD: Rgba = Rgba::hex24(0xe3b440); // h
pub const BRONZE: Rgba = Rgba::hex24(0xbd8b2d); // j
pub const BRONZE_LO: Rgba = Rgba::hex24(0xa97522); // m
pub const SIDE: Rgba = Rgba::hex24(0x6d572a); // a
pub const BAR: Rgba = Rgba::hex24(0xf9eeaf); // e
pub const BAR_LO: Rgba = Rgba::hex24(0xd4cb99); // i
pub const BAR_GAP: Rgba = Rgba::hex24(0x8d6a2a); // f
pub const TEAL: Rgba = Rgba::hex24(0x295f57); // v
pub const TEAL_HI: Rgba = Rgba::hex24(0x1b7171); // r
pub const JADE_LO: Rgba = Rgba::hex24(0x3ba8ad); // n
pub const JADE: Rgba = Rgba::hex24(0x71e7dc); // k
pub const JADE_HI: Rgba = Rgba::hex24(0xaef8f3); // w
pub const WHITE: Rgba = Rgba::hex24(0xf6fdf7); // l

/// The glow's steps, from its dithered edge to white: each step of heat is one.
pub const RAMP: [Rgba; 4] = [Rgba::hex24(0x21878b), JADE, JADE_HI, WHITE];
/// The light the jade adds over the scene.
pub const GLOW: Rgba = Rgba::hex24(0x4fd8cc);
/// Heat lost a pixel out from a stroke.
pub const FALL: f32 = 1.0;
/// A radius under this draws each trigram as one tick; under `TINY` the mirror is a rim round its face.
const SMALL: f64 = 12.0;
const TINY: f64 = 6.0;
/// The radius the rings' widths are laid out for; a bigger mirror widens them all alike.
const LAYOUT: f64 = 18.0;

/// The eight trigrams round the mirror, clockwise from the top (the Earlier Heaven order): a bit per line, the
/// lowest the innermost, set for an unbroken line.
const TRIGRAMS: [u8; 8] = [0b111, 0b011, 0b010, 0b001, 0b000, 0b100, 0b101, 0b110];

/// How the mirror looks at one moment.
#[derive(Clone, Copy)]
pub struct Look {
    /// Its centre in local pixels and its radius across the flats.
    pub x: f64,
    pub y: f64,
    pub r: f64,
    /// Its turn about the upright axis, 0 facing the viewer.
    pub turn: f64,
    /// 0 to 1: the face flashing white.
    pub flash: f64,
    /// The trigram lit (0 to 7, clockwise from the top), and how brightly, 0 to 1.
    pub lit: usize,
    pub lit_k: f64,
    /// The swirl's turn, radians.
    pub swirl: f64,
}

impl Look {
    pub fn new(x: f64, y: f64, r: f64) -> Self {
        Look {
            x,
            y,
            r,
            turn: 0.0,
            flash: 0.0,
            lit: 0,
            lit_k: 0.0,
            swirl: 0.0,
        }
    }
}

/// The mirror's pixels (local x, y, colour), and its edge stamped into `halo` so its glow can be painted under it.
pub fn mirror(halo: &mut Halo, m: &Look) -> Vec<(f64, f64, Rgba)> {
    let mut out = Vec::new();
    if m.r < 0.6 {
        return out;
    }
    let (cx, cy) = (libm::floor(m.x), libm::floor(m.y));
    let u = libm::cos(m.turn);
    let side = libm::sin(m.turn);
    let heat = (2.2 + 1.4 * m.flash) as f32;
    let reach = libm::ceil(m.r) + 3.0;
    let mut y = -reach;
    while y <= reach {
        let mut x = -reach;
        while x <= reach {
            if let Some(c) = pixel(m, x, y, u, side) {
                {
                    out.push((cx + x, cy + y, c));
                    if edge(m, x, y, u) {
                        halo.stamp(cx + x, cy + y, heat, 1);
                    }
                }
            }
            x += 1.0;
        }
        y += 1.0;
    }
    out
}

/// The octagon's distance of a point (unsquashed, from the centre), flat sides up, down and to the sides.
fn d8(x: f64, y: f64) -> f64 {
    libm::fabs(x)
        .max(libm::fabs(y))
        .max((libm::fabs(x) + libm::fabs(y)) * core::f64::consts::FRAC_1_SQRT_2)
}

/// Whether pixel (`x`, `y`) lies on the mirror's outer line, where its glow starts.
fn edge(m: &Look, x: f64, y: f64, u: f64) -> bool {
    if libm::fabs(u) < 0.22 {
        return true;
    }
    let (lx, ly) = ((x + 0.5) / libm::fabs(u), y + 0.5);
    m.r - d8(lx, ly) < 1.0
}

/// The colour of pixel (`x`, `y`) from the mirror's centre, if the mirror covers it.
fn pixel(m: &Look, x: f64, y: f64, u: f64, side: f64) -> Option<Rgba> {
    let r = m.r;
    let au = libm::fabs(u);
    if au < 0.22 {
        // edge-on: a bronze bar three pixels thick, lit on its left
        let h = libm::floor(r);
        if y < -h || y >= h || !(-1.0..=1.0).contains(&x) {
            return None;
        }
        return Some(match x as i32 {
            -1 => GOLD_HI,
            0 => GOLD,
            _ => BRONZE_LO,
        });
    }
    let (lx, ly) = ((x + 0.5) / au, y + 0.5);
    let d = d8(lx, ly);
    if d > r {
        // the rim's thickness shows on the side turned away
        let thick = libm::floor(libm::fabs(side) * r * 0.16 + 0.5);
        let dir = if side * u > 0.0 { 1.0 } else { -1.0 };
        for k in 1..=thick as i32 {
            let sx = (x - dir * f64::from(k) + 0.5) / au;
            if d8(sx, ly) <= r {
                return Some(if ly < 0.0 { BRONZE } else { SIDE });
            }
        }
        return None;
    }
    // top left faces the light
    let lit = lx + ly < 0.0;
    let e = libm::hypot(lx, ly);
    if r < TINY {
        // a dark line, a gold rim and the jade face
        return Some(match libm::floor(r - d) as i32 {
            0 => DARK,
            1 => {
                if lit {
                    GOLD_HI
                } else {
                    GOLD
                }
            }
            _ => face_px(m, lx, ly, e / (r - 1.5).max(1.0), x, y),
        });
    }
    let small = r < SMALL;
    // a big mirror keeps the rings of one of `LAYOUT` pixels, each ring as wide as its share
    let unit = if small { 1.0 } else { (r / LAYOUT).max(1.0) };
    let depth = libm::floor((r - d) / unit);
    // the face and its rings, round
    let face = if small { r - 5.0 } else { r - 11.5 * unit };
    if e < face + 2.0 * unit {
        if e >= face + unit {
            return Some(if lit { GOLD_HI } else { GOLD });
        }
        if e >= face && !small {
            return Some(if lit { TEAL_HI } else { TEAL });
        }
        return Some(face_px(m, lx, ly, e / face.max(1.0), x, y));
    }
    let c = match depth as i32 {
        0 => DARK,
        1 => {
            if lit {
                GOLD_HI
            } else {
                BRONZE
            }
        }
        2 if !small => {
            if lit {
                GOLD
            } else {
                BRONZE_LO
            }
        }
        _ => {
            let band = if small { 2.0 } else { 3.0 };
            bar(m, lx, ly, depth - band, small).unwrap_or(BAND)
        }
    };
    Some(c)
}

/// A trigram bar's pixel at `step` pixels in from the band's outer edge, if one lies there.
fn bar(m: &Look, lx: f64, ly: f64, step: f64, small: bool) -> Option<Rgba> {
    // the sector, 0 at the top, clockwise
    let a = libm::atan2(ly, lx) + PI / 2.0;
    let k = (libm::floor(a / FRAC_PI_4 + 0.5) as i32).rem_euclid(8) as usize;
    let na = k as f64 * FRAC_PI_4 - PI / 2.0;
    // along the sector's side
    let along = -lx * libm::sin(na) + ly * libm::cos(na);
    let len = if small { 1.6 } else { m.r * 0.19 };
    if libm::fabs(along) > len {
        return None;
    }
    let line = if small {
        if step as i32 != 0 {
            return None;
        }
        0
    } else {
        match step as i32 {
            1 => 2,
            3 => 1,
            5 => 0,
            // between a trigram's lines, a bronze plate
            2 | 4 => return Some(BAR_GAP),
            _ => return None,
        }
    };
    let whole = TRIGRAMS.get(k).copied().unwrap_or(0) >> line & 1 == 1;
    if !small && !whole && libm::fabs(along) < 1.0 {
        return None;
    }
    if k == m.lit && m.lit_k > 0.5 {
        return Some(WHITE);
    }
    Some(if lx + ly < m.r * 0.6 { BAR } else { BAR_LO })
}

/// The jade face: a pale core, a three-armed swirl, a darker rim to the bottom right, white under a flash.
fn face_px(m: &Look, lx: f64, ly: f64, f: f64, x: f64, y: f64) -> Rgba {
    if m.flash > 0.6 || (m.flash > 0.3 && (x + y).rem_euclid(2.0) == 0.0) {
        return WHITE;
    }
    let lit = lx + ly < 0.0;
    if f > 0.82 {
        return if lit { JADE } else { JADE_LO };
    }
    if f < 0.3 {
        return WHITE;
    }
    let s = libm::sin(3.0 * libm::atan2(ly, lx) + 5.0 * f + m.swirl);
    if s > 0.45 {
        JADE_HI
    } else if lit {
        WHITE
    } else {
        JADE
    }
}

/// Paint a halo's heats as steps of the ramp, under what is drawn next: kind 0 only (the strokes paint themselves),
/// the faint edge dithered, each hot step blooming a little.
pub fn paint_halo(p: &mut Painter, halo: &Halo) {
    for (x, y, h, kind) in halo.each() {
        if kind != 0 {
            continue;
        }
        let step = libm::floor(f64::from(h)) as usize;
        if step == 0 {
            if (x + y).rem_euclid(2.0) == 0.0 {
                p.dot(x, y, RAMP[0], 0.8);
            }
            continue;
        }
        p.dot(x, y, RAMP[step.min(RAMP.len() - 1)], 1.0);
        p.light(x, y, GLOW, 0.12 * step as f64);
    }
}

/// Paint pixels, opaque.
pub fn put(p: &mut Painter, px: &[(f64, f64, Rgba)]) {
    for &(x, y, c) in px {
        p.px(x, y, c);
    }
}

/// The blow thrown back: a beam three pixels thick leaving (`x0`, `y0`) toward side `dir` (1 right, -1 left)
/// at `speed` pixels a second, `len` long, `since` seconds after the blow; its core white, its edges pale jade.
pub fn beam(
    halo: &mut Halo,
    x0: f64,
    y0: f64,
    dir: f64,
    since: f64,
    speed: f64,
    len: f64,
) -> Vec<(f64, f64, Rgba)> {
    let mut out = Vec::new();
    if !(0.0..0.6).contains(&since) {
        return out;
    }
    let head = since * speed;
    let tail = (head - len).max(0.0);
    let thick = if since < 0.12 { 1.0 } else { 0.0 };
    let (x0, y0) = (libm::floor(x0), libm::floor(y0));
    let mut d = libm::floor(tail);
    while d <= head {
        let x = x0 + dir * d;
        out.push((x, y0, WHITE));
        halo.stamp(x, y0, 2.6, 1);
        for e in [-1.0, 1.0] {
            // the edges thin out toward the tail
            if thick > 0.0 || d > tail + len * 0.4 {
                out.push((x, y0 + e, JADE_HI));
                halo.stamp(x, y0 + e, 1.9, 1);
            }
        }
        d += 1.0;
    }
    out
}

/// A four-pointed spark at (`x`, `y`), `size` pixels from its middle to each point.
pub fn star(p: &mut Painter, x: f64, y: f64, size: i32, k: f64) {
    if k <= 0.0 {
        return;
    }
    let (x, y) = (libm::floor(x), libm::floor(y));
    p.dot(x, y, WHITE, k);
    for i in 1..=size {
        let c = if i == size { JADE } else { JADE_HI };
        let f = f64::from(i);
        for (dx, dy) in [(f, 0.0), (-f, 0.0), (0.0, f), (0.0, -f)] {
            p.dot(x + dx, y + dy, c, k);
        }
    }
    p.light(x, y, GLOW, 0.5 * k);
}

/// Bronze shards and jade sparks flung out from (`x0`, `y0`), `since` seconds after they leave, falling: `n`
/// of them, seeded by `seed`, gone after `life` seconds. `dir` biases them to a side (0 for all round).
#[allow(clippy::too_many_arguments)]
pub fn shards(
    p: &mut Painter,
    x0: f64,
    y0: f64,
    since: f64,
    n: usize,
    seed: f64,
    life: f64,
    dir: f64,
) {
    if since < 0.0 || since > life {
        return;
    }
    for i in 0..n {
        let fi = i as f64;
        let own = life * (0.6 + 0.4 * rand(&[fi, seed, 41.0]));
        if since > own {
            continue;
        }
        let ang = if dir == 0.0 {
            rand(&[fi, seed, 43.0]) * TAU
        } else {
            let base = if dir > 0.0 { 0.0 } else { PI };
            base + (rand(&[fi, seed, 43.0]) - 0.5) * 2.4
        };
        let speed = 30.0 + 45.0 * rand(&[fi, seed, 45.0]);
        let x = x0 + libm::cos(ang) * speed * since;
        let y = y0 + libm::sin(ang) * speed * since + 60.0 * since * since;
        let k = 1.0 - since / own;
        if i % 3 == 0 {
            star(p, x, y, 1, k);
        } else {
            let (x, y) = (libm::floor(x), libm::floor(y));
            p.dot(
                x,
                y,
                if i % 2 == 0 { GOLD_HI } else { GOLD },
                clamp(k * 1.5, 0.0, 1.0),
            );
            p.dot(x + 1.0, y, BRONZE_LO, clamp(k * 1.5, 0.0, 1.0));
            p.dot(x, y + 1.0, SIDE, clamp(k * 1.5, 0.0, 1.0));
        }
    }
}

/// A flat ripple ring under the mirror (the shield form's), `k` 0 to 1 through its life: it widens and fades.
pub fn ripple(p: &mut Painter, x: f64, y: f64, k: f64, max: f64) {
    if !(0.0..1.0).contains(&k) {
        return;
    }
    let rx = 4.0 + (max - 4.0) * k;
    let ry = (rx * 0.22).max(1.0);
    let n = (rx * 4.0) as i32;
    let a = 1.0 - k;
    for i in 0..n {
        let t = f64::from(i) / f64::from(n) * TAU;
        let (px, py) = (
            libm::floor(x + rx * libm::cos(t)),
            libm::floor(y + ry * libm::sin(t)),
        );
        // the far side of the ring is fainter
        let far = libm::sin(t) < 0.0;
        p.dot(
            px,
            py,
            if far { JADE } else { JADE_HI },
            a * if far { 0.5 } else { 1.0 },
        );
        p.light(px, py, GLOW, 0.2 * a);
    }
}

/// What a bagua effect reads of the sprite it guards: its outline cells, its top and bottom, its middle and chest.
pub struct Body {
    pub ring: Vec<(i32, i32)>,
    pub top: f64,
    pub bottom: f64,
    /// The chest: the body's middle across, halfway down.
    pub chest: (f64, f64),
}

/// The box a stand-in body is read in when the game gives no sprite.
const BOX: Area = Area {
    x: -16,
    y: -48,
    w: 32,
    h: 50,
};

impl Body {
    /// The body of `actor`, read once each time the sprite changes; a stand-in when there is none.
    pub fn of(actor: Option<&Actor>) -> Self {
        let sil = match actor {
            Some(a) => Silhouette::of(
                a,
                Area {
                    x: a.x() - 1,
                    y: a.y() - 1,
                    w: a.w() + 2,
                    h: a.h() + 2,
                },
            ),
            None => stand_in(),
        };
        let sil = if sil.is_empty() { stand_in() } else { sil };
        let ring: Vec<(i32, i32)> = sil.ring(1, false).collect();
        let (top, bottom, chest) = match (sil.bounds(), sil.centroid()) {
            (Some(b), Some((cx, _))) => {
                let (top, h) = (f64::from(b.y), f64::from(b.h));
                (top, top + h, (libm::floor(cx), libm::floor(top + h * 0.5)))
            }
            _ => (-42.0, 0.0, (0.0, -21.0)),
        };
        Body {
            ring,
            top,
            bottom,
            chest,
        }
    }

    /// How tall the body is, in pixels.
    pub fn height(&self) -> f64 {
        (self.bottom - self.top).max(1.0)
    }
}

/// A body to guard when the game gives no sprite: a head, a trunk and two legs, as `feedback/power-up`'s.
fn stand_in() -> Silhouette {
    Silhouette::from_fn(BOX, |px, py| {
        let head = libm::hypot(px, py + 38.0) < 4.5;
        let trunk = libm::fabs(px) < 4.0 && py > -34.0 && py < -14.0;
        let legs =
            (-14.0..0.0).contains(&py) && libm::fabs(libm::fabs(px) - (py + 14.0) * 0.28) < 1.6;
        head || trunk || legs
    })
}

/// How the jade line round the body looks at one moment.
pub struct Rim {
    /// 0 to 1: how far it has washed down from the head; while under 1 its front burns bright.
    pub wash: f64,
    /// 0 to 1: where a pulse running down it is, once it has washed down; `None` for no pulse.
    pub pulse: Option<f64>,
    /// 0 to 1: the blow's flare round the whole body.
    pub flare: f64,
    /// 1 while it stands, down to 0 as it fades.
    pub fade: f64,
    /// Heats: resting, in the pulse, under the flare (each step of heat is one step of the ramp).
    pub rest: f64,
    pub pulsed: f64,
    pub flared: f64,
}

/// The front of the wash and the pulse, in pixels of height.
const FRONT: f64 = 3.0;

/// The jade line round the body (the sprite's own outline), its glow painted in steps round it. It lies off the
/// body: drawn behind the sprite, which hides none of it.
pub fn outline(p: &mut Painter, body: &Body, rim: &Rim, size: (usize, usize), origin: (f64, f64)) {
    if rim.fade <= 0.0 || rim.wash <= 0.0 {
        return;
    }
    let height = (body.bottom - body.top).max(1.0);
    let span = |k: f64| body.top - FRONT + (height + 2.0 * FRONT) * k;
    let wash = span(rim.wash);
    let washing = rim.wash < 1.0;
    let pulse = rim.pulse.map(span);
    let mut halo = Halo::new(size.0, size.1, origin.0, origin.1);
    let mut cells = Vec::with_capacity(body.ring.len());
    for &(x, y) in &body.ring {
        let fy = f64::from(y);
        if fy > wash {
            continue;
        }
        let front = if washing {
            clamp(1.0 - (wash - fy) / FRONT, 0.0, 1.0)
        } else {
            0.0
        };
        let pulsed = pulse.map_or(0.0, |py| clamp(1.0 - libm::fabs(fy - py) / FRONT, 0.0, 1.0));
        let heat = (rim.rest + rim.pulsed * pulsed + rim.flared * front.max(rim.flare)) * rim.fade;
        if heat < 0.3 {
            continue;
        }
        halo.stamp(f64::from(x), fy, heat as f32, 2);
        cells.push((f64::from(x), fy, heat));
    }
    halo.spread(FALL);
    paint_halo(p, &halo);
    for (x, y, heat) in cells {
        let step = libm::floor(heat) as usize;
        p.dot(x, y, RAMP[step.min(RAMP.len() - 1)], clamp(heat, 0.0, 1.0));
        p.light(x, y, GLOW, 0.1 * heat);
    }
    // a soft bloom over the body as it flares
    if rim.flare > 0.0 {
        p.glow(
            body.chest.0,
            (body.top + body.bottom) * 0.5,
            height * 0.7,
            GLOW,
            0.35 * rim.flare * rim.fade,
        );
    }
}
