//! A brush swept along a path: what a camera sees of a bright thing waved through one long exposure. A point swept
//! along a path leaves a line, a short column across it a ribbon (a sword's trail), a long bar that turns while it
//! moves an area (a fan, an arc smear). Every streak in pixfx is drawn here, so they all read alike.
//!
//! The brush is a column of [`Cell`]s, each with its colour and its light, laid square to the way the path goes
//! ([`Turn::Heading`]) or turned with the thing's own angle ([`Turn::Angle`]). Where it passed, a pixel shows the
//! cell that passed it last; how long ago (its age, against [`Look::secs`]) moves its colour along a ramp, thins
//! the brush, waves it and breaks it up by dither. The path is sampled finely enough that no cell moves more than
//! half a pixel between samples, so a fast or turning brush leaves no holes.
//!
//! The brush is the mover's cross-section, so pick it by the mover's shape: a thin mover (a sword's point, a drop, a
//! spark) leaves a narrow ribbon ([`Brush::column`]); a broad body (a mirror, a shield, a disc, a character) leaves
//! a smear as tall as itself, in its own colours, coming off its back edge ([`Brush::edge`]); a blade that turns
//! sweeps its own pixels ([`Brush::from_column`] with [`Turn::Angle`]). A narrow ribbon behind a broad body reads
//! as a beam stuck on it, not as its motion.

use std::collections::BTreeMap;

use core::f64::consts::{PI, TAU};

use crate::color::Rgba;
use crate::frame::Frame;
use crate::painter::Painter;

/// Where the thing was, how it was turned (radians) and how long ago, in the painter's local pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
    pub age: f64,
}

/// One pixel of the brush: `across` pixels from the path (positive to the right of the way it goes, y down: below
/// a brush going +x), its colour, and how much light it adds (0 to 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub across: f64,
    pub color: Rgba,
    pub light: f64,
}

/// How the brush is turned: square to the way the path goes (a ribbon behind a mover), or at the pose's own angle
/// plus a base (a bar fixed to a blade, which sweeps arcs as the blade turns). At angle 0 a brush lies along +y.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Turn {
    Heading,
    Angle(f64),
}

/// A column of cells and how it is turned.
#[derive(Clone, Debug, PartialEq)]
pub struct Brush {
    pub cells: Vec<Cell>,
    pub turn: Turn,
}

impl Brush {
    /// A ribbon `width` pixels across (rounded, at least one): the middle cells in the palette's first colour and
    /// lit, the two edge cells a colour on, unlit. An empty palette gives an empty brush.
    pub fn column(palette: &[Rgba], width: f64) -> Brush {
        let n = crate::math::js_round(width).max(1.0) as usize;
        let half = (n as f64 - 1.0) / 2.0;
        let last = palette.len().saturating_sub(1);
        let cells = (0..n)
            .filter_map(|k| {
                let across = k as f64 - half;
                let edge = n > 1 && (across.abs() - half).abs() < 1e-9;
                let color = *palette.get(usize::from(edge).min(last))?;
                Some(Cell {
                    across,
                    color,
                    light: if edge { 0.0 } else { 1.0 },
                })
            })
            .collect();
        Brush {
            cells,
            turn: Turn::Heading,
        }
    }

    /// The back edge of a thing drawn facing +x, from its `pixels` (`x`, `y`, colour, light 0 to 1, in its local
    /// pixels): on each row the pixel `inset` steps in from its back (0 the backmost; 1 the one inside a sprite's
    /// dark outline, so the smear takes the body's own colour, not its outline's), `across` its row, laid square to
    /// the way it goes (`Turn::Heading`). Swept along its path it smears the thing's own silhouette and colours
    /// behind it, as tall as it is (`arcane/bagua-mirror-flying`: its jade rim; `projectiles/trail` on a character).
    pub fn edge(pixels: impl IntoIterator<Item = (f64, f64, Rgba, f64)>, inset: usize) -> Brush {
        let mut rows: BTreeMap<i64, Vec<(f64, Rgba, f64)>> = BTreeMap::new();
        for (x, y, color, light) in pixels {
            rows.entry(libm::floor(y + 0.5) as i64)
                .or_default()
                .push((x, color, light));
        }
        Brush {
            cells: rows
                .into_iter()
                .filter_map(|(row, mut px)| {
                    px.sort_by(|a, b| a.0.total_cmp(&b.0));
                    let &(_, color, light) = px.get(inset).or(px.last())?;
                    Some(Cell {
                        across: row as f64,
                        color,
                        light,
                    })
                })
                .collect(),
            turn: Turn::Heading,
        }
    }

    /// Column `x` of a painted frame as a brush: its opaque pixels, `across` counted from row `origin_y`, each with
    /// its colour and its light (the light's brightest channel), turned with the thing that sweeps it
    /// (`Turn::Angle(0)`): a sprite's own pixels smeared as it moves. A column outside the frame is empty.
    pub fn from_column(frame: &Frame, x: u32, origin_y: i32) -> Brush {
        let mut cells = Vec::new();
        if x < frame.w {
            for row in 0..frame.h {
                let i = (row * frame.w + x) as usize;
                let Some(&c) = frame.color.get(i) else {
                    continue;
                };
                if c >> 24 == 0 {
                    continue;
                }
                let l = Rgba::unpack(frame.light.get(i).copied().unwrap_or(0));
                cells.push(Cell {
                    across: f64::from(row as i32 - origin_y),
                    color: Rgba::unpack(c),
                    light: f64::from(l.0.max(l.1).max(l.2)) / 255.0,
                });
            }
        }
        Brush {
            cells,
            turn: Turn::Angle(0.0),
        }
    }
}

/// How a sweep changes with age (`u`: the age over `secs`, 0 newest to 1 oldest) and along the path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// How long the streak is, seconds: older passes are not drawn.
    pub secs: f64,
    /// Steps along the ramp a cell's colour moves by `u` = 1 (a cell whose colour is not in the ramp keeps it).
    pub fade: f64,
    /// The brush's width at `u` = 1, as a share of its own (cells beyond it drop out, the middle stays).
    pub taper: f64,
    /// Pixels the path waves across at `u` = 1, growing from none at the head, and the wave's phase (radians).
    pub wave: f64,
    pub phase: f64,
    /// The `u` from which an ordered dither eats the streak, all of it by `u` = 1; and the dither's shift in
    /// pixels (a frame count makes it crawl).
    pub breakup: f64,
    pub shift: f64,
    /// The `u` from which the streak turns translucent, clear by `u` = 1: a soft, blended fade where `breakup`'s
    /// dither is a hard pixel one (1 for none).
    pub soft: f64,
    /// How much a cell's light adds at the head, falling to none at `u` = 1.
    pub light: f64,
    /// Pixels at the head left undrawn (the mover's own body over them).
    pub skip: f64,
}

/// The shortest turn from angle `a` to angle `b`, -PI to PI.
fn turn_to(a: f64, b: f64) -> f64 {
    let d = libm::fmod(b - a + PI, TAU);
    (if d < 0.0 { d + TAU } else { d }) - PI
}

/// A 4 by 4 ordered dither threshold for a pixel, 0 to 1.
fn bayer(x: f64, y: f64) -> f64 {
    const M: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    let xi = (libm::floor(x) as i64).rem_euclid(4) as usize;
    let yi = (libm::floor(y) as i64).rem_euclid(4) as usize;
    (f64::from(M[yi][xi]) + 0.5) / 16.0
}

/// What last passed a pixel: how long ago, how far from the path (the nearer wins a tie), its colour and light.
#[derive(Clone, Copy)]
struct Mark {
    age: f64,
    off: f64,
    color: Rgba,
    light: f64,
}

/// The way the path goes at each pose (radians): along the segments either side of it, the shortest way between
/// them; a pose on a still stretch takes its nearest moving neighbour's. None when the path never moves.
fn headings(poses: &[Pose]) -> Option<Vec<f64>> {
    let seg: Vec<Option<f64>> = poses
        .windows(2)
        .map(|w| {
            let (dx, dy) = (w[0].x - w[1].x, w[0].y - w[1].y);
            (libm::hypot(dx, dy) > 1e-9).then(|| libm::atan2(dy, dx))
        })
        .collect();
    let any = seg.iter().flatten().next().copied()?;
    // fill still segments from the nearest moving one, newer first
    let mut filled = Vec::with_capacity(seg.len());
    let mut last = None;
    for s in &seg {
        last = s.or(last);
        filled.push(last);
    }
    let mut next = None;
    for k in (0..filled.len()).rev() {
        next = seg[k].or(next);
        if filled[k].is_none() {
            filled[k] = next;
        }
    }
    let at = |k: usize| filled.get(k).copied().flatten().unwrap_or(any);
    Some(
        (0..poses.len())
            .map(|i| match (i.checked_sub(1), i < seg.len()) {
                (Some(prev), true) => at(prev) + turn_to(at(prev), at(i)) / 2.0,
                (Some(prev), false) => at(prev),
                (None, _) => at(0),
            })
            .collect(),
    )
}

/// Sweep `brush` along `poses` (newest first), each pixel showing the cell that passed it last, its colour moved
/// along `ramp` with age, as `look` says. Nothing for an empty brush, one pose, or a path that never moves.
pub fn sweep(p: &mut Painter, poses: &[Pose], brush: &Brush, ramp: &[Rgba], look: &Look) {
    if brush.cells.is_empty() || poses.len() < 2 {
        return;
    }
    // a ribbon needs a way to lie across; a bar turned with its thing sweeps even standing still (no wave then)
    let (ways, waving) = match (headings(poses), brush.turn) {
        (Some(w), _) => (w, true),
        (None, Turn::Angle(_)) => (vec![0.0; poses.len()], false),
        (None, Turn::Heading) => return,
    };
    let wave = if waving { look.wave } else { 0.0 };
    let secs = look.secs.max(1e-6);
    let mut cells = brush.cells.clone();
    cells.sort_by(|a, b| a.across.total_cmp(&b.across));
    let reach0 = cells.iter().map(|c| c.across.abs()).fold(0.0, f64::max);
    // every point across the brush half a pixel apart at most, each with the cell nearest it
    let mut across: Vec<(f64, Cell)> = Vec::new();
    for w in cells.windows(2) {
        let (a, b) = (w[0], w[1]);
        let n = libm::ceil((b.across - a.across) / 0.5).max(1.0);
        for k in 0..n as usize {
            let t = k as f64 / n;
            across.push((
                a.across + (b.across - a.across) * t,
                if t < 0.5 { a } else { b },
            ));
        }
    }
    if let Some(&c) = cells.last() {
        across.push((c.across, c));
    }
    let theta = |q: &Pose, way: f64| match brush.turn {
        Turn::Heading => way,
        Turn::Angle(base) => q.angle + base,
    };
    let last = ramp.len().saturating_sub(1);
    let shade = |c: &Cell, u: f64| match ramp.iter().position(|&r| r == c.color) {
        Some(i) if look.fade > 0.0 => ramp
            .get((i + (look.fade * u) as usize).min(last))
            .copied()
            .unwrap_or(c.color),
        _ => c.color,
    };
    let mut marks: BTreeMap<(i64, i64), Mark> = BTreeMap::new();
    let mut dist = 0.0;
    for (i, w) in poses.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let len = libm::hypot(a.x - b.x, a.y - b.y);
        if a.age > secs {
            break;
        }
        let (wa, wb) = (ways[i], ways[i + 1]);
        let (ta, tb) = (theta(&a, wa), theta(&b, wb));
        let (dw, dt) = (turn_to(wa, wb), turn_to(ta, tb));
        let moved = len + dt.abs() * reach0 + wave.abs() * 2.0;
        let n = libm::ceil(moved * 2.0).max(1.0);
        for s in 0..=n as usize {
            let t = s as f64 / n;
            let age = a.age + (b.age - a.age) * t;
            let d = dist + len * t;
            if age > secs || d < look.skip {
                continue;
            }
            let u = (age / secs).clamp(0.0, 1.0);
            let (x, y) = (a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            let way = wa + dw * t;
            let th = ta + dt * t;
            // the wave runs across the path; the brush across its own turn
            let off = wave * u * libm::sin((d - look.skip) * 0.3 - look.phase);
            let (cx, cy) = (x - libm::sin(way) * off, y + libm::cos(way) * off);
            let (nx, ny) = (-libm::sin(th), libm::cos(th));
            let half = ((reach0 + 0.5) * (1.0 - (1.0 - look.taper) * u)).max(0.5);
            let reach = libm::ceil(half - 0.5);
            for &(v, cell) in &across {
                if v.abs() > reach + 1e-9 {
                    continue;
                }
                let key = (
                    libm::floor(cx + nx * v + 0.5) as i64,
                    libm::floor(cy + ny * v + 0.5) as i64,
                );
                let mark = Mark {
                    age,
                    off: v.abs(),
                    color: shade(&cell, u),
                    light: cell.light,
                };
                let newer = marks.get(&key).is_none_or(|m| {
                    age < m.age - 1e-12 || (age <= m.age + 1e-12 && v.abs() < m.off)
                });
                if newer {
                    marks.insert(key, mark);
                }
            }
        }
        dist += len;
    }
    for ((x, y), m) in marks {
        let (x, y) = (x as f64, y as f64);
        let u = (m.age / secs).clamp(0.0, 1.0);
        if u > look.breakup && look.breakup < 1.0 {
            let eaten = (u - look.breakup) / (1.0 - look.breakup);
            if bayer(x + look.shift, y) < eaten {
                continue;
            }
        }
        let clear = if u > look.soft && look.soft < 1.0 {
            (u - look.soft) / (1.0 - look.soft)
        } else {
            0.0
        };
        if clear > 0.0 {
            p.blend(x, y, m.color, 1.0 - clear);
        } else {
            p.dot(x, y, m.color, 1.0);
        }
        let k = look.light * m.light * (1.0 - u) * (1.0 - clear);
        if k > 0.0 {
            p.light(x, y, m.color, k);
        }
    }
}
