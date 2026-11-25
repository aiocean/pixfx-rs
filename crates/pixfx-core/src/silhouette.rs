//! Sprite awareness: what an effect knows of the character it wraps. An effect that wraps its actor
//! ([`crate::Effect::wraps_actor`]) is handed the sprite's current frame ([`crate::EffectState::set_actor`]); a
//! [`Silhouette`] reads it into cells of the effect's own space (origin at the character's feet), so the effect can
//! draw on the body, along its edge or out from it, and pick points on it.
//!
//! Build it once per sprite frame, in `set_actor`, never per drawn frame: reading the sprite and measuring distance
//! costs a pass over the area. Every query is deterministic, so frames stay the same on every target.

use pixfx_draw::math::Rng;

use crate::actor::Actor;

/// A box of effect pixels: its top-left cell at (`x`, `y`), `w` x `h` cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Area {
    /// Whether cell (`x`, `y`) lies in the box.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.index(x, y).is_some()
    }

    /// The cell's place in row-major order, if it lies in the box.
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let (i, j) = (
            i64::from(x) - i64::from(self.x),
            i64::from(y) - i64::from(self.y),
        );
        if i < 0 || j < 0 || i >= i64::from(self.w) || j >= i64::from(self.h) {
            return None;
        }
        Some(j as usize * self.w as usize + i as usize)
    }

    /// The cell at row-major place `k`.
    fn cell(&self, k: usize) -> (i32, i32) {
        let w = self.w.max(1) as usize;
        (self.x + (k % w) as i32, self.y + (k / w) as i32)
    }

    fn len(&self) -> usize {
        self.w as usize * self.h as usize
    }
}

/// Cost of a straight step in [`Silhouette`]'s distances: three a step, four a diagonal (a chamfer distance, close
/// to the true one); a cell's distance in pixels is its count over three.
const STEP: u16 = 3;
const DIAGONAL: u16 = 4;

/// Which cells of an [`Area`] a body covers, and how far every other cell is from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Silhouette {
    area: Area,
    mask: Vec<bool>,
    /// Chamfer distance to the nearest covered cell ([`STEP`]s), at most 255.
    chamfer: Vec<u8>,
}

impl Silhouette {
    /// The cells of `area` under the sprite's opaque pixels (alpha above 0). Empty when the sprite covers none of
    /// it: an effect then draws a stand-in body ([`Silhouette::from_fn`]) or nothing.
    pub fn of(actor: &Actor, area: Area) -> Self {
        let mask = (0..area.len())
            .map(|k| {
                let (x, y) = area.cell(k);
                let (sx, sy) = (
                    i64::from(x) - i64::from(actor.x()),
                    i64::from(y) - i64::from(actor.y()),
                );
                sx >= 0
                    && sy >= 0
                    && sx <= i64::from(u32::MAX)
                    && sy <= i64::from(u32::MAX)
                    && actor.at(sx as u32, sy as u32) >> 24 != 0
            })
            .collect();
        Self::with_mask(area, mask)
    }

    /// The cells of `area` whose centre (`x + 0.5`, `y + 0.5`) `covers` says the body covers: a stand-in body
    /// drawn from shapes, for when the game hands no sprite.
    pub fn from_fn(area: Area, covers: impl Fn(f64, f64) -> bool) -> Self {
        let mask = (0..area.len())
            .map(|k| {
                let (x, y) = area.cell(k);
                covers(f64::from(x) + 0.5, f64::from(y) + 0.5)
            })
            .collect();
        Self::with_mask(area, mask)
    }

    fn with_mask(area: Area, mask: Vec<bool>) -> Self {
        let chamfer = chamfer(area, &mask);
        Self {
            area,
            mask,
            chamfer,
        }
    }

    /// The box it covers cells of.
    pub fn area(&self) -> Area {
        self.area
    }

    /// Whether the body covers no cell.
    pub fn is_empty(&self) -> bool {
        !self.mask.contains(&true)
    }

    /// Whether the body covers cell (`x`, `y`) (false outside the area).
    pub fn covers(&self, x: i32, y: i32) -> bool {
        self.area
            .index(x, y)
            .and_then(|k| self.mask.get(k).copied())
            .unwrap_or(false)
    }

    /// The cells the body covers, row by row.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.mask
            .iter()
            .enumerate()
            .filter(|&(_, &m)| m)
            .map(|(k, _)| self.area.cell(k))
    }

    /// The body's edge: covered cells beside an uncovered one (up, down, left or right) or at the area's border,
    /// row by row. Where an outline, a rim light or sparks off the body go.
    pub fn edge(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.cells().filter(|&(x, y)| {
            [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|&(dx, dy)| !self.covers(x + dx, y + dy))
        })
    }

    /// The covered cells whose neighbour at (`dx`, `dy`) is uncovered or past the area's border, row by row: the
    /// cells with nothing above them (`open(0, -1)`, where snow settles and flames stand), nothing to one side
    /// (`open(-1, 0)`, `open(1, 0)`: where frost creeps in and rime hangs) or nothing below (`open(0, 1)`: where
    /// icicles and drips hang). [`Silhouette::edge`] is the union of the four.
    pub fn open(&self, dx: i32, dy: i32) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.cells()
            .filter(move |&(x, y)| !self.covers(x + dx, y + dy))
    }

    /// The cells off the body at most `max` pixels from it, row by row, each with its distance in pixels (over 0):
    /// where an aura, a glow or a halo goes, fading with the distance. Distances past 85 pixels read as 85.
    pub fn around(&self, max: f64) -> impl Iterator<Item = (i32, i32, f64)> + '_ {
        self.chamfer.iter().enumerate().filter_map(move |(k, &c)| {
            let d = f64::from(c) / f64::from(STEP);
            (d != 0.0 && d <= max).then(|| {
                let (x, y) = self.area.cell(k);
                (x, y, d)
            })
        })
    }

    /// The cells of an outline `width` pixels wide round the body, row by row: off the body and within `width`
    /// steps of it, counting a diagonal as one step with `corners` (a square ring) or two without (a rounded,
    /// pixel-art ring). Only cells in the area: give it room for the ring.
    pub fn ring(&self, width: u32, corners: bool) -> impl Iterator<Item = (i32, i32)> + '_ {
        let reach = i32::try_from(width).unwrap_or(i32::MAX);
        let mut on = vec![false; self.area.len()];
        for (x, y) in self.cells() {
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    if !corners && dx.abs() + dy.abs() > reach {
                        continue;
                    }
                    if let Some(k) = self.area.index(x + dx, y + dy)
                        && let Some(c) = on.get_mut(k)
                    {
                        *c = true;
                    }
                }
            }
        }
        on.into_iter()
            .enumerate()
            .filter(move |&(k, ring)| ring && !self.mask.get(k).copied().unwrap_or(false))
            .map(|(k, _)| self.area.cell(k))
    }

    /// Cell (`x`, `y`)'s distance from the body in pixels: 0 on it, none outside the area.
    pub fn distance(&self, x: i32, y: i32) -> Option<f64> {
        let k = self.area.index(x, y)?;
        self.chamfer.get(k).map(|&c| f64::from(c) / f64::from(STEP))
    }

    /// The smallest box round the body, none when it is empty.
    pub fn bounds(&self) -> Option<Area> {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (x, y) in self.cells() {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        (x0 <= x1).then(|| Area {
            x: x0,
            y: y0,
            w: (x1 - x0 + 1) as u32,
            h: (y1 - y0 + 1) as u32,
        })
    }

    /// The middle of the body: the mean of its cells' centres, none when it is empty.
    pub fn centroid(&self) -> Option<(f64, f64)> {
        let (mut n, mut sx, mut sy) = (0.0, 0.0, 0.0);
        for (x, y) in self.cells() {
            n += 1.0;
            sx += f64::from(x) + 0.5;
            sy += f64::from(y) + 0.5;
        }
        (n > 0.0).then(|| (sx / n, sy / n))
    }

    /// A random cell of the body: up to `tries` cells of the area drawn from `rng` (column, then row), the first the
    /// body covers; none if every try missed. Spawning on the body this way keeps a port's draws in its source's
    /// order.
    pub fn pick(&self, rng: &mut Rng, tries: u32) -> Option<(i32, i32)> {
        let (w, h) = (f64::from(self.area.w), f64::from(self.area.h));
        for _ in 0..tries {
            let i = rng.int(0.0, w - 1.0);
            let j = rng.int(0.0, h - 1.0);
            let (x, y) = (self.area.x + i as i32, self.area.y + j as i32);
            if self.covers(x, y) {
                return Some((x, y));
            }
        }
        None
    }
}

/// Each cell's chamfer distance from the covered ones: a forward pass and a backward pass over the area.
fn chamfer(area: Area, mask: &[bool]) -> Vec<u8> {
    let (w, h) = (area.w as usize, area.h as usize);
    let mut dist: Vec<u8> = mask.iter().map(|&m| if m { 0 } else { 255 }).collect();
    let relax = |dist: &mut [u8], i: usize, j: usize, cost: u16| {
        if let (Some(&dj), Some(&di)) = (dist.get(j), dist.get(i)) {
            let v = u16::from(dj) + cost;
            if v < u16::from(di)
                && let Some(d) = dist.get_mut(i)
            {
                *d = v as u8;
            }
        }
    };
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if x > 0 {
                relax(&mut dist, i, i - 1, STEP);
            }
            if y > 0 {
                relax(&mut dist, i, i - w, STEP);
                if x > 0 {
                    relax(&mut dist, i, i - w - 1, DIAGONAL);
                }
                if x + 1 < w {
                    relax(&mut dist, i, i - w + 1, DIAGONAL);
                }
            }
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if x + 1 < w {
                relax(&mut dist, i, i + 1, STEP);
            }
            if y + 1 < h {
                relax(&mut dist, i, i + w, STEP);
                if x + 1 < w {
                    relax(&mut dist, i, i + w + 1, DIAGONAL);
                }
                if x > 0 {
                    relax(&mut dist, i, i + w - 1, DIAGONAL);
                }
            }
        }
    }
    dist
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3 x 3 body in a 5 x 5 area, its top-left cell at (1, 1).
    fn block() -> Silhouette {
        let area = Area {
            x: 0,
            y: 0,
            w: 5,
            h: 5,
        };
        Silhouette::from_fn(area, |x, y| {
            (1.0..4.0).contains(&x) && (1.0..4.0).contains(&y)
        })
    }

    #[test]
    fn open_gives_the_cells_with_nothing_on_that_side() {
        let s = block();
        let up: Vec<_> = s.open(0, -1).collect();
        assert_eq!(up, vec![(1, 1), (2, 1), (3, 1)]);
        let left: Vec<_> = s.open(-1, 0).collect();
        assert_eq!(left, vec![(1, 1), (1, 2), (1, 3)]);
        let right: Vec<_> = s.open(1, 0).collect();
        assert_eq!(right, vec![(3, 1), (3, 2), (3, 3)]);
        let down: Vec<_> = s.open(0, 1).collect();
        assert_eq!(down, vec![(1, 3), (2, 3), (3, 3)]);
    }

    #[test]
    fn a_cell_at_the_areas_border_is_open_to_it_and_edge_is_the_union() {
        let area = Area {
            x: 0,
            y: 0,
            w: 3,
            h: 3,
        };
        let s = Silhouette::from_fn(area, |_, _| true);
        assert_eq!(
            s.open(0, -1).collect::<Vec<_>>(),
            vec![(0, 0), (1, 0), (2, 0)]
        );
        let mut union: Vec<_> = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .flat_map(|&(dx, dy)| s.open(dx, dy).collect::<Vec<_>>())
            .collect();
        union.sort_unstable();
        union.dedup();
        let mut edge: Vec<_> = s.edge().collect();
        edge.sort_unstable();
        assert_eq!(union, edge);
        assert_eq!(block().open(0, 1).count(), 3);
    }
}
