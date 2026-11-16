//! Shattering: a shape struck by bolts of cracks, one after another, breaks into the pieces the cracks wall in.
//!
//! Each strike is a crack grown by dielectric breakdown ([`crate::growth::dbm`], at a crack's `eta`) from a point on the
//! shape's rim or on a crack already there, drawn to cracks already there (or, the first, to the far rim), and stopping
//! where it meets one. A crack that meets another closes a loop; the cells the loop walls in are then one piece apart
//! from the rest. No loop is searched for: after each strike the cells that are not crack are split into groups that
//! touch across a side (a 4-connected flood fill, so a diagonal crack seals), and a group is a piece.
//!
//! Which pieces fall is [`Hold`]'s call: the ones the rim does not hold (glass in a frame: only a piece walled in by
//! cracks alone falls), or every one but the piece holding a point (a wall held at its base). A piece that falls is
//! gone from the shape: later strikes run only through what still stands.
//!
//! Pure and deterministic from the seed. It gives the cells; the effect decides how they are drawn and how they fall.

use crate::growth::{Dbm, Tree, dbm};
use crate::math::rand;

/// What holds the shape up, so which pieces fall once cut off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hold {
    /// The rim holds: a piece touching the shape's outline stays; a piece walled in by cracks alone falls.
    Rim,
    /// The piece holding this cell stays (the one nearest it, once the cell itself is cracked); every other falls.
    At(i32, i32),
}

/// How [`shatter`] strikes.
#[derive(Clone, Copy, Debug)]
pub struct Cracks {
    /// How many bolts strike, one after another. A bolt that cannot reach a crack in about twice the straight distance
    /// stops short: a dead-end crack.
    pub strikes: usize,
    /// The crack's `eta`: 2 to 3 branches like a crack spreading, 3 to 5 runs as one jagged line.
    pub eta: f64,
    /// 0..1: the grid's straight runs broken up (see [`Dbm::noise`]).
    pub noise: f64,
    /// How far, in cells, a strike that starts on a crack reaches for another: it is drawn to crack cells between
    /// `reach.0` and `reach.1` away from where it starts. Short closes small pieces; long, big ones.
    pub reach: (f64, f64),
    /// The share of strikes that start on a crack already there (the rest start on the rim): high breaks it up
    /// inside, low cuts it in from the edge.
    pub inner: f64,
    pub hold: Hold,
    /// A piece of fewer cells than this does not fall alone: it stays with the cracks round it.
    pub min_piece: usize,
    /// The grid, in whole cells: x0, y0, x1, y1 (inclusive).
    pub bounds: (i32, i32, i32, i32),
    pub seed: f64,
}

impl Default for Cracks {
    fn default() -> Self {
        Cracks {
            strikes: 8,
            eta: 3.5,
            noise: 0.35,
            reach: (6.0, 18.0),
            inner: 0.75,
            hold: Hold::Rim,
            min_piece: 6,
            bounds: (-32, -32, 31, 31),
            seed: 1.0,
        }
    }
}

/// One bolt: its crack as a tree (its nodes' `born` steps play it growing) and the cells it cracked.
#[derive(Clone, Debug, PartialEq)]
pub struct Strike {
    pub tree: Tree,
    pub cells: Vec<(i32, i32)>,
}

/// A piece that broke away: its cells, the strike that cut it off, and its middle.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub cells: Vec<(i32, i32)>,
    pub strike: usize,
    pub centre: (f64, f64),
}

/// A shattered shape: the strikes in order, the pieces that fell, in the order they fell, and what was left standing.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Shatter {
    pub strikes: Vec<Strike>,
    pub pieces: Vec<Piece>,
    /// What still stands after the last strike, in the blocks its cracks split it into (each with the cracks along its
    /// sides), `strike` set to the number of strikes: for an effect that breaks the rest away too.
    pub rest: Vec<Piece>,
}

const STEPS4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// The cell grid of the bounds: index of a cell, or none off it.
struct Grid {
    x0: i32,
    y0: i32,
    w: usize,
    h: usize,
}

impl Grid {
    fn at(&self, x: i32, y: i32) -> Option<usize> {
        let (dx, dy) = (x - self.x0, y - self.y0);
        (dx >= 0 && dy >= 0 && (dx as usize) < self.w && (dy as usize) < self.h)
            .then(|| dy as usize * self.w + dx as usize)
    }
    fn cell(&self, k: usize) -> (i32, i32) {
        (self.x0 + (k % self.w) as i32, self.y0 + (k / self.w) as i32)
    }
}

/// What a cell of the shape is now.
#[derive(Clone, Copy, PartialEq)]
enum State {
    Out,
    Whole,
    Crack,
    Fallen,
}

/// Strikes the shape (the cells of `bounds` where `inside`) with `opts.strikes` cracks and breaks it (see the module).
pub fn shatter(opts: Cracks, inside: impl Fn(i32, i32) -> bool) -> Shatter {
    let (x0, y0, x1, y1) = opts.bounds;
    let grid = Grid {
        x0,
        y0,
        w: (x1 - x0 + 1).max(1) as usize,
        h: (y1 - y0 + 1).max(1) as usize,
    };
    let mut state: Vec<State> = (0..grid.w * grid.h)
        .map(|k| {
            let (x, y) = grid.cell(k);
            if inside(x, y) {
                State::Whole
            } else {
                State::Out
            }
        })
        .collect();
    let is = |state: &[State], x: i32, y: i32, s: State| {
        grid.at(x, y).and_then(|k| state.get(k)) == Some(&s)
    };
    // the rim: shape cells with a side off the shape
    let rim: Vec<bool> = (0..state.len())
        .map(|k| {
            let (x, y) = grid.cell(k);
            state.get(k) == Some(&State::Whole)
                && STEPS4.iter().any(|&(dx, dy)| {
                    grid.at(x + dx, y + dy).and_then(|j| state.get(j)) != Some(&State::Whole)
                })
        })
        .collect();
    let mut out = Shatter::default();
    for s in 0..opts.strikes {
        let seed = opts.seed * 7.31 + s as f64 * 13.7;
        let standing = |st: &State| matches!(st, State::Whole | State::Crack);
        let rim_cells: Vec<usize> = (0..state.len())
            .filter(|&k| rim.get(k) == Some(&true) && state.get(k).is_some_and(standing))
            .collect();
        let crack_cells: Vec<usize> = (0..state.len())
            .filter(|&k| state.get(k) == Some(&State::Crack))
            .collect();
        let from_crack = !crack_cells.is_empty() && rand(&[seed, 1.0]) < opts.inner;
        let pick = |cells: &[usize], salt: f64| -> Option<usize> {
            let n = cells.len();
            (n > 0)
                .then(|| {
                    cells
                        .get((rand(&[seed, salt]) * n as f64) as usize % n)
                        .copied()
                })
                .flatten()
        };
        let Some(root) = (if from_crack {
            pick(&crack_cells, 2.0)
        } else {
            pick(&rim_cells, 3.0)
        }) else {
            break;
        };
        let (rx, ry) = grid.cell(root);
        let dist = |k: usize| {
            let (x, y) = grid.cell(k);
            libm::hypot(f64::from(x - rx), f64::from(y - ry))
        };
        // drawn to cracks at the reach; with none there yet, to the rim across from where it starts
        let mut targets: Vec<usize> = crack_cells
            .iter()
            .copied()
            .filter(|&k| (opts.reach.0..=opts.reach.1).contains(&dist(k)))
            .collect();
        if targets.is_empty() {
            let far = rim_cells.iter().map(|&k| dist(k)).fold(0.0, f64::max);
            targets = rim_cells
                .iter()
                .copied()
                .filter(|&k| dist(k) >= far * 0.7)
                .collect();
        }
        if targets.is_empty() {
            continue;
        }
        let near = targets
            .iter()
            .map(|&k| dist(k))
            .fold(f64::INFINITY, f64::min);
        let target_xy: Vec<(i32, i32)> = targets.iter().map(|&k| grid.cell(k)).collect();
        // the field is solved only round the root and its targets, a few cells past them: the bolt cannot stray further
        // than its cell budget lets it anyway, and the solve costs by the area
        const MARGIN: i32 = 6;
        let window = target_xy.iter().fold((rx, ry, rx, ry), |b, &(x, y)| {
            (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y))
        });
        let window = (
            (window.0 - MARGIN).max(x0),
            (window.1 - MARGIN).max(y0),
            (window.2 + MARGIN).min(x1),
            (window.3 + MARGIN).min(y1),
        );
        // it grows through whole cells only: every crack not a target is a wall, so it cannot run along its own
        let snapshot = state.clone();
        let tree = dbm(
            &[(rx, ry)],
            &target_xy,
            Dbm {
                eta: opts.eta,
                edge: None,
                drift: (0.0, 0.0),
                noise: opts.noise,
                neighbours: 8,
                relax: 3,
                // a crack runs about straight: one that cannot reach stops short, a dead end
                cells: (near * 2.5) as usize + 6,
                reach: 1.5,
                bounds: window,
                seed,
            },
            |x, y| is(&snapshot, x, y, State::Whole) || (x, y) == (rx, ry),
        );
        let mut cells = Vec::new();
        for n in &tree.nodes {
            let (x, y) = (libm::floor(n.x + 0.5) as i32, libm::floor(n.y + 0.5) as i32);
            if let Some(c) = grid.at(x, y).and_then(|k| state.get_mut(k))
                && *c == State::Whole
            {
                *c = State::Crack;
                cells.push((x, y));
            }
        }
        out.strikes.push(Strike { tree, cells });
        let (group, groups) = group_whole(&grid, &state);
        let held: Vec<bool> = match opts.hold {
            Hold::Rim => groups
                .iter()
                .map(|g| g.iter().any(|&k| rim.get(k) == Some(&true)))
                .collect(),
            Hold::At(hx, hy) => {
                // the group nearest the holding cell
                let near = |g: &Vec<usize>| {
                    g.iter()
                        .map(|&k| {
                            let (x, y) = grid.cell(k);
                            (x - hx) * (x - hx) + (y - hy) * (y - hy)
                        })
                        .min()
                        .unwrap_or(i32::MAX)
                };
                let best = groups
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, g)| near(g))
                    .map(|(i, _)| i);
                (0..groups.len()).map(|i| Some(i) == best).collect()
            }
        };
        for (g, members) in groups.iter().enumerate() {
            if held.get(g) == Some(&true) || members.len() < opts.min_piece {
                continue;
            }
            // the cracks that wall it in and touch nothing still standing go with it, so it falls whole
            let mut piece: Vec<usize> = members.clone();
            for &k in members {
                let (x, y) = grid.cell(k);
                for (dx, dy) in STEPS4 {
                    let Some(j) = grid.at(x + dx, y + dy) else {
                        continue;
                    };
                    if state.get(j) != Some(&State::Crack) || piece.contains(&j) {
                        continue;
                    }
                    let (cx, cy) = grid.cell(j);
                    let touches_standing = STEPS4.iter().any(|&(ex, ey)| {
                        grid.at(cx + ex, cy + ey).is_some_and(|m| {
                            state.get(m) == Some(&State::Whole)
                                && group
                                    .get(m)
                                    .is_some_and(|&o| o != g && held.get(o) == Some(&true))
                        })
                    });
                    if !touches_standing {
                        piece.push(j);
                    }
                }
            }
            out.pieces.push(take(&grid, &mut state, &piece, s));
        }
    }
    // what still stands, in the blocks the cracks split it into; each takes the cracks along its sides not yet taken
    let (_, groups) = group_whole(&grid, &state);
    for members in &groups {
        let mut piece = members.clone();
        for &k in members {
            let (x, y) = grid.cell(k);
            for (dx, dy) in STEPS4 {
                if let Some(j) = grid.at(x + dx, y + dy)
                    && state.get(j) == Some(&State::Crack)
                    && !piece.contains(&j)
                {
                    piece.push(j);
                }
            }
        }
        out.rest.push(take(&grid, &mut state, &piece, opts.strikes));
    }
    out
}

/// The whole cells grouped by the sides they share: each cell's group (or `usize::MAX`), and each group's cells.
fn group_whole(grid: &Grid, state: &[State]) -> (Vec<usize>, Vec<Vec<usize>>) {
    let mut group = vec![usize::MAX; state.len()];
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for start in 0..state.len() {
        if state.get(start) != Some(&State::Whole) || group.get(start) != Some(&usize::MAX) {
            continue;
        }
        let id = groups.len();
        let mut members = vec![start];
        if let Some(g) = group.get_mut(start) {
            *g = id;
        }
        let mut i = 0;
        while let Some(&k) = members.get(i) {
            i += 1;
            let (x, y) = grid.cell(k);
            for (dx, dy) in STEPS4 {
                if let Some(j) = grid.at(x + dx, y + dy)
                    && state.get(j) == Some(&State::Whole)
                    && let Some(g) = group.get_mut(j)
                    && *g == usize::MAX
                {
                    *g = id;
                    members.push(j);
                }
            }
        }
        groups.push(members);
    }
    (group, groups)
}

/// Takes `cells` off the shape as one piece cut off by strike `strike`.
fn take(grid: &Grid, state: &mut [State], cells: &[usize], strike: usize) -> Piece {
    let mut sum = (0.0, 0.0);
    let cells: Vec<(i32, i32)> = cells
        .iter()
        .map(|&k| {
            if let Some(c) = state.get_mut(k) {
                *c = State::Fallen;
            }
            let (x, y) = grid.cell(k);
            sum.0 += f64::from(x);
            sum.1 += f64::from(y);
            (x, y)
        })
        .collect();
    let n = cells.len().max(1) as f64;
    Piece {
        cells,
        strike,
        centre: (sum.0 / n, sum.1 / n),
    }
}
