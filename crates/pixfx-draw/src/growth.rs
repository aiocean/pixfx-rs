//! Branching growth: trees of nodes grown step by step, for anything that branches as it spreads (veins, lightning,
//! cracks, roots, frost). Two ways to grow one, both deterministic from a seed and walled in by a closure that says
//! which points may be grown into:
//!
//! - [`dbm`]: the dielectric breakdown model (Niemeyer, Pietronero and Wiesmann 1984), with the potential summed over
//!   point charges instead of solved on a grid (Kim and Lin, "Fast animation of lightning using an adaptive mesh",
//!   2007). Each step adds one grid cell next to the tree, picked with probability `potential^eta`: `eta` near 1
//!   branches densely (frost, coral), 2 to 3 cracks, 4 to 6 lightning. `pull` aims it at target points.
//! - [`colonize`]: space colonization (Runions, Fuhrer, Lane et al., "Modeling and visualization of leaf venation
//!   patterns", 2005; Runions, Lane and Prusinkiewicz, "Modeling trees with a space colonization algorithm", 2007).
//!   Attraction points are scattered in the region; each step every tip grows toward the points nearest it, and points
//!   it reaches are removed. It fills a region evenly: veins, roots, trees.
//!
//! A [`Tree`] keeps each node's parent, the step it was born on (play growth by drawing the nodes born so far) and its
//! weight, the number of nodes it leads to counting itself (draw a trunk thicker than a twig).

use crate::math::rand;

/// One node of a tree: where it is, the node it grew from, the step it was born on and its weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub x: f64,
    pub y: f64,
    /// The node it grew from; `None` for a root. Always an earlier index.
    pub parent: Option<usize>,
    /// The step it was added on, 0 for the roots.
    pub born: u32,
    /// How many nodes it leads to, counting itself: 1 for a tip.
    pub weight: u32,
}

/// One pixel a stroke covers ([`Tree::stroke`]): where it is, the node whose piece covers it, and where it sits across
/// the stroke: `depth` 0 on the centre line to 1 at the edge, `normal` the unit direction from the centre line out to
/// it ((0, 0) on the line), `radius` the stroke's radius there. Shade a tube lit from a direction by the normal's dot
/// with it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dab {
    pub x: i32,
    pub y: i32,
    pub node: usize,
    pub depth: f64,
    pub normal: (f64, f64),
    pub radius: f64,
}

/// A grown tree (a forest when it has several roots): its nodes, parents before children, and the last step.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub steps: u32,
}

impl Tree {
    /// A tree of bare roots, to grow from.
    pub fn roots(roots: &[(f64, f64)]) -> Tree {
        Tree {
            nodes: roots
                .iter()
                .map(|&(x, y)| Node {
                    x,
                    y,
                    parent: None,
                    born: 0,
                    weight: 1,
                })
                .collect(),
            steps: 0,
        }
    }

    fn grow(&mut self, x: f64, y: f64, parent: usize, born: u32) {
        self.nodes.push(Node {
            x,
            y,
            parent: Some(parent),
            born,
            weight: 1,
        });
        self.steps = self.steps.max(born);
    }

    /// Lays a set path into the tree, a node every `step` along the polyline `points`: a main vein, a bolt's channel, a
    /// crack's first line, that growth then branches from ([`colonize_from`]). Its first point joins the nearest node
    /// already in the tree when one is within `step` (a fork off another path), else it is a new root. Each node is born
    /// a step after the one before it, so the path grows in from its start.
    pub fn path(&mut self, points: &[(f64, f64)], step: f64) {
        let Some(&first) = points.first() else {
            return;
        };
        let step = step.max(0.5);
        let near = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    i,
                    (n.x - first.0) * (n.x - first.0) + (n.y - first.1) * (n.y - first.1),
                )
            })
            .fold(None, |best: Option<(usize, f64)>, (i, d)| match best {
                Some((_, b)) if b <= d => best,
                _ => Some((i, d)),
            })
            .filter(|&(_, d)| d <= step * step);
        let mut last = match near {
            Some((i, _)) => i,
            None => {
                self.nodes.push(Node {
                    x: first.0,
                    y: first.1,
                    parent: None,
                    born: 0,
                    weight: 1,
                });
                self.nodes.len() - 1
            }
        };
        for pair in points.windows(2) {
            let (Some(&a), Some(&b)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let n = libm::ceil(libm::sqrt(dx * dx + dy * dy) / step).max(1.0) as usize;
            for k in 1..=n {
                let u = k as f64 / n as f64;
                let born = self.nodes.get(last).map_or(0, |p| p.born) + 1;
                self.grow(a.0 + dx * u, a.1 + dy * u, last, born);
                last = self.nodes.len() - 1;
            }
        }
    }

    /// Removes every side branch shorter than `min` nodes (counting all it leads to), so growth leaves no stubs. A
    /// node's first child continues its line and is kept; later children are side branches. Weights are recounted.
    pub fn prune(&mut self, min: u32) {
        self.weigh();
        let mut keep = vec![true; self.nodes.len()];
        let mut first = vec![usize::MAX; self.nodes.len()];
        for i in 0..self.nodes.len() {
            let Some(n) = self.nodes.get(i).copied() else {
                continue;
            };
            let Some(p) = n.parent else {
                continue;
            };
            let parent_kept = keep.get(p).copied().unwrap_or(false);
            let is_first = match first.get_mut(p) {
                Some(f) if *f == usize::MAX => {
                    *f = i;
                    true
                }
                _ => false,
            };
            if let Some(k) = keep.get_mut(i) {
                *k = parent_kept && (is_first || n.weight >= min);
            }
        }
        let mut index = vec![usize::MAX; self.nodes.len()];
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for (i, n) in self.nodes.iter().enumerate() {
            if !keep.get(i).copied().unwrap_or(false) {
                continue;
            }
            if let Some(slot) = index.get_mut(i) {
                *slot = nodes.len();
            }
            let parent = n
                .parent
                .and_then(|p| index.get(p).copied())
                .filter(|&p| p != usize::MAX);
            nodes.push(Node { parent, ..*n });
        }
        self.nodes = nodes;
        self.steps = self.nodes.iter().map(|n| n.born).max().unwrap_or(0);
        self.weigh();
    }

    /// Sets the angle of every fork by Murray's law (Murray 1926: the branching that costs a vessel least work), from
    /// the weights at the fork, read as radii by `weight^(1/3)`: an even split forks in a symmetric Y, both halves turned
    /// out (about 37° each); a thin branch leaves wide and barely moves the line it leaves. Each branch, and all it
    /// leads to, is turned about the fork toward its angle by `strength` (0..1), the angle times `scale` (1 is the
    /// law, under 1 narrower). Nodes before `keep` (paths laid by hand) are never moved, and a turn that would carry a
    /// node out of `inside` is not made. Call it after growing, before [`Tree::prune`] and [`Tree::radii`].
    pub fn murray(
        &mut self,
        strength: f64,
        scale: f64,
        keep: usize,
        inside: impl Fn(f64, f64) -> bool,
    ) {
        self.weigh();
        let n = self.nodes.len();
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, node) in self.nodes.iter().enumerate() {
            if let Some(k) = node.parent.and_then(|p| kids.get_mut(p)) {
                k.push(i);
            }
        }
        let angle = |a: (f64, f64), b: (f64, f64)| libm::atan2(b.1 - a.1, b.0 - a.0);
        for f in 0..n {
            let Some(ks) = kids.get(f).filter(|k| k.len() >= 2).cloned() else {
                continue;
            };
            let Some(fork) = self.nodes.get(f).copied() else {
                continue;
            };
            // the way the line ran into the fork, over the last few nodes
            let mut back = fork;
            for _ in 0..4 {
                match back.parent.and_then(|p| self.nodes.get(p)) {
                    Some(&p) => back = p,
                    None => break,
                }
            }
            if back == fork {
                continue;
            }
            let into = angle((back.x, back.y), (fork.x, fork.y));
            let weight = |i: usize| self.nodes.get(i).map_or(1.0, |m| f64::from(m.weight));
            let Some(&main) = ks.iter().max_by(|&&a, &&b| weight(a).total_cmp(&weight(b))) else {
                continue;
            };
            let rm = libm::cbrt(weight(main));
            let at =
                |i: usize, nodes: &[Node]| nodes.get(i).map_or((fork.x, fork.y), |m| (m.x, m.y));
            let mut turns: Vec<(usize, f64)> = Vec::new();
            let mut widest = (0.0, 1.0);
            for &s in ks.iter().filter(|&&s| s != main) {
                let rs = libm::cbrt(weight(s));
                let r0 = libm::cbrt(rm * rm * rm + rs * rs * rs);
                let (r02, rs2, rm2) = (r0 * r0, rs * rs, rm * rm);
                let theta = libm::acos(
                    ((r02 * r02 + rs2 * rs2 - rm2 * rm2) / (2.0 * r02 * rs2)).clamp(-1.0, 1.0),
                );
                let now = wrap_angle(angle((fork.x, fork.y), at(s, &self.nodes)) - into);
                let side = if now < 0.0 { -1.0 } else { 1.0 };
                turns.push((s, strength * (side * theta * scale - now)));
                if rs > widest.0 {
                    widest = (rs, side);
                }
            }
            if main >= keep {
                let rs = widest.0;
                let r0 = libm::cbrt(rm * rm * rm + rs * rs * rs);
                let (r02, rs2, rm2) = (r0 * r0, rs * rs, rm * rm);
                let theta = libm::acos(
                    ((r02 * r02 + rm2 * rm2 - rs2 * rs2) / (2.0 * r02 * rm2)).clamp(-1.0, 1.0),
                );
                let now = wrap_angle(angle((fork.x, fork.y), at(main, &self.nodes)) - into);
                turns.push((main, strength * (-widest.1 * theta * scale - now)));
            }
            for (child, turn) in turns {
                if child < keep || turn == 0.0 {
                    continue;
                }
                // the branch and everything it leads to
                let mut stack = vec![child];
                let mut sub = Vec::new();
                while let Some(i) = stack.pop() {
                    sub.push(i);
                    if let Some(k) = kids.get(i) {
                        stack.extend(k.iter().copied());
                    }
                }
                let (c, s) = (libm::cos(turn), libm::sin(turn));
                let moved: Vec<(usize, f64, f64)> = sub
                    .iter()
                    .filter_map(|&i| self.nodes.get(i).map(|m| (i, m.x - fork.x, m.y - fork.y)))
                    .map(|(i, dx, dy)| (i, fork.x + dx * c - dy * s, fork.y + dx * s + dy * c))
                    .collect();
                if moved.iter().all(|&(_, x, y)| inside(x, y)) {
                    for (i, x, y) in moved {
                        if let Some(m) = self.nodes.get_mut(i) {
                            m.x = x;
                            m.y = y;
                        }
                    }
                }
            }
        }
    }

    /// Counts each node's weight: children come after their parents, so one pass from the end adds each to its parent.
    pub fn weigh(&mut self) {
        for n in &mut self.nodes {
            n.weight = 1;
        }
        for i in (0..self.nodes.len()).rev() {
            let Some(n) = self.nodes.get(i).copied() else {
                continue;
            };
            if let Some(p) = n.parent.and_then(|p| self.nodes.get_mut(p)) {
                p.weight += n.weight;
            }
        }
    }

    /// Every branch piece, as (the parent, the child).
    pub fn segments(&self) -> impl Iterator<Item = (&Node, &Node)> {
        self.nodes
            .iter()
            .filter_map(|n| n.parent.and_then(|p| self.nodes.get(p)).map(|p| (p, n)))
    }

    /// The heaviest weight, a root's: scale widths by it.
    pub fn heaviest(&self) -> u32 {
        self.nodes.iter().map(|n| n.weight).max().unwrap_or(1)
    }

    /// A stroke radius for `node` by weight alone (use [`Tree::radii`] to draw: it smooths the steps where a big branch
    /// leaves); between between `thin` (a tip) and `thick` (the heaviest root), by the square root of its
    /// weight: a branch's cross-section is the sum of its children's (da Vinci's rule), so widths taper smoothly.
    pub fn radius(&self, node: &Node, thin: f64, thick: f64) -> f64 {
        let k = f64::from(node.weight) / f64::from(self.heaviest().max(1));
        thin + (thick - thin) * libm::sqrt(k)
    }

    /// Every node's stroke radius, by [`Tree::radius`], then smoothed so a line never steps: along each branch (a node
    /// and its heaviest child) the radius falls at most `taper` per node, and a side branch starts no wider than the
    /// node it leaves. Index it by node.
    pub fn radii(&self, thin: f64, thick: f64, taper: f64) -> Vec<f64> {
        let mut main = vec![usize::MAX; self.nodes.len()];
        for (i, n) in self.nodes.iter().enumerate() {
            if let Some(m) = n.parent.and_then(|p| main.get_mut(p)) {
                let heavier = self.nodes.get(*m).is_none_or(|o| n.weight > o.weight);
                if *m == usize::MAX || heavier {
                    *m = i;
                }
            }
        }
        let mut out: Vec<f64> = Vec::with_capacity(self.nodes.len());
        for (i, n) in self.nodes.iter().enumerate() {
            let own = self.radius(n, thin, thick);
            let r = match n.parent.and_then(|p| out.get(p).copied().map(|r| (p, r))) {
                Some((p, rp)) if main.get(p) == Some(&i) => own.max(rp - taper).min(rp),
                Some((_, rp)) => own.min(rp),
                None => own,
            };
            out.push(r);
        }
        out
    }

    /// Paints the tree as tapered strokes: each piece is a capsule whose radius runs from `radii[parent]` to
    /// `radii[child]` (from [`Tree::radii`]), and `f` gets a [`Dab`] for every pixel whose centre it covers, with
    /// where the pixel sits across the stroke, so a caller can shade it as a lit tube, a bevel or a glowing core. A pixel
    /// two pieces cover is passed twice; `f` decides which wins.
    pub fn stroke(&self, radii: &[f64], mut f: impl FnMut(Dab)) {
        let r_of = |i: usize| radii.get(i).copied().unwrap_or(0.5);
        for (i, n) in self.nodes.iter().enumerate() {
            let Some((pi, p)) = n.parent.and_then(|pi| self.nodes.get(pi).map(|p| (pi, p))) else {
                continue;
            };
            let (r0, r1) = (r_of(pi), r_of(i));
            let reach = r0.max(r1);
            let (dx, dy) = (n.x - p.x, n.y - p.y);
            let len2 = (dx * dx + dy * dy).max(1e-9);
            let lo = (
                libm::floor(p.x.min(n.x) - reach) as i32,
                libm::floor(p.y.min(n.y) - reach) as i32,
            );
            let hi = (
                libm::ceil(p.x.max(n.x) + reach) as i32,
                libm::ceil(p.y.max(n.y) + reach) as i32,
            );
            for y in lo.1..=hi.1 {
                for x in lo.0..=hi.0 {
                    let (px, py) = (f64::from(x), f64::from(y));
                    let u = (((px - p.x) * dx + (py - p.y) * dy) / len2).clamp(0.0, 1.0);
                    let (ox, oy) = (px - p.x - dx * u, py - p.y - dy * u);
                    let d = libm::sqrt(ox * ox + oy * oy);
                    let r = r0 + (r1 - r0) * u;
                    if d <= r {
                        let (nx, ny) = if d > 1e-9 {
                            (ox / d, oy / d)
                        } else {
                            (0.0, 0.0)
                        };
                        f(Dab {
                            x,
                            y,
                            node: i,
                            depth: d / r.max(1e-9),
                            normal: (nx, ny),
                            radius: r,
                        });
                    }
                }
            }
        }
    }
}

/// An angle wrapped into -PI..PI.
fn wrap_angle(a: f64) -> f64 {
    let t = core::f64::consts::TAU;
    let a = libm::fmod(a + core::f64::consts::PI, t);
    (if a < 0.0 { a + t } else { a }) - core::f64::consts::PI
}

/// A polyline made jagged by midpoint displacement: each of `depth` rounds puts a point in the middle of every piece,
/// pushed sideways by up to `amount` times the piece's length, from the seed. The classic lightning line, and a way to
/// make a hand-set path ([`Tree::path`]) wander: depth 2 to 4, amount 0.1 (a wobble) to 0.35 (a bolt).
pub fn jag(points: &[(f64, f64)], depth: u32, amount: f64, seed: f64) -> Vec<(f64, f64)> {
    let mut line = points.to_vec();
    for round in 0..depth {
        let mut next = Vec::with_capacity(line.len() * 2);
        for (k, pair) in line.windows(2).enumerate() {
            let (Some(&a), Some(&b)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let push = amount * (2.0 * rand(&[seed, f64::from(round), k as f64]) - 1.0);
            next.push(a);
            next.push(((a.0 + b.0) / 2.0 - dy * push, (a.1 + b.1) / 2.0 + dx * push));
        }
        if let Some(&last) = line.last() {
            next.push(last);
        }
        line = next;
    }
    line
}

/// How [`dbm`] grows. The potential φ is solved on the grid (Laplace's equation, by a few sweeps of successive
/// over-relaxation each step, warm from the step before): grown cells are held at 0, targets at 1, and the walls (the
/// grid's edge and every cell `inside` refuses) at `edge`, or insulating when `edge` is `None`. Each step one cell
/// next to the tree is grown, picked with probability `(φ + drift · position)^eta`, each cell's chance scaled by its
/// own fixed `noise`. Every knob is a field, so one function grows lightning, cracks, frost and coral.
#[derive(Clone, Copy, Debug)]
pub struct Dbm {
    /// How sharply it favours the cells of highest potential: 1 branches like coral (diffusion-limited aggregation),
    /// 2 to 3 cracks, 4 to 6 lightning.
    pub eta: f64,
    /// The walls' potential: `Some(1.0)` grounds them, so growth spreads out toward them (frost, cracks from a point);
    /// `None` insulates them, so only the targets and `drift` draw it (a bolt to a point inside a room).
    pub edge: Option<f64>,
    /// Added to the potential per cell along (x, y): a steady lean. Small: 0.002 to 0.02.
    pub drift: (f64, f64),
    /// 0..1: how much each cell's chance is scaled at random (fixed per cell by the seed); breaks up the grid's straight
    /// and diagonal runs.
    pub noise: f64,
    /// 4: grows only across and down the grid (blockier, cracks); 8: diagonals too.
    pub neighbours: u8,
    /// Relaxation sweeps after each grown cell: more follows the field more exactly, costs more.
    pub relax: u32,
    /// At most this many cells are grown, roots not counted.
    pub cells: usize,
    /// It stops once a cell is grown this close to a target (0: never).
    pub reach: f64,
    /// The grid it grows on, in whole cells: x0, y0, x1, y1 (inclusive).
    pub bounds: (i32, i32, i32, i32),
    pub seed: f64,
}

impl Default for Dbm {
    /// Coral-like growth out to a grounded edge; set the size, the seed and the look.
    fn default() -> Self {
        Dbm {
            eta: 2.0,
            edge: Some(1.0),
            drift: (0.0, 0.0),
            noise: 0.3,
            neighbours: 8,
            relax: 4,
            cells: 300,
            reach: 0.0,
            bounds: (-32, -32, 31, 31),
            seed: 1.0,
        }
    }
}

/// Grows a tree on the cell grid from `roots` toward `targets` by dielectric breakdown (see [`Dbm`]), only into
/// cells where `inside(x, y)`.
pub fn dbm(
    roots: &[(i32, i32)],
    targets: &[(i32, i32)],
    opts: Dbm,
    inside: impl Fn(i32, i32) -> bool,
) -> Tree {
    const FREE: u8 = 0;
    const CAND: u8 = 1;
    const GROWN: u8 = 2;
    const TARGET: u8 = 3;
    const WALL: u8 = 4;
    const OMEGA: f64 = 1.8;
    const STEPS4: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    const STEPS8: [(i32, i32); 8] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (1, -1),
        (-1, 1),
        (-1, -1),
    ];
    let around: &[(i32, i32)] = if opts.neighbours == 4 {
        &STEPS4
    } else {
        &STEPS8
    };
    let (x0, y0, x1, y1) = opts.bounds;
    let w = (x1 - x0 + 1).max(1) as usize;
    let h = (y1 - y0 + 1).max(1) as usize;
    let at = |x: i32, y: i32| -> Option<usize> {
        (x >= x0 && x <= x1 && y >= y0 && y <= y1)
            .then(|| (y - y0) as usize * w + (x - x0) as usize)
    };
    let edge = opts.edge.unwrap_or(0.0);
    let mut kind = vec![FREE; w * h];
    let mut phi = vec![edge.max(0.5); w * h];
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !inside(x, y)
                && let Some(k) = at(x, y)
                && let (Some(c), Some(p)) = (kind.get_mut(k), phi.get_mut(k))
            {
                *c = WALL;
                *p = edge;
            }
        }
    }
    for &(x, y) in targets {
        if let Some(k) = at(x, y)
            && let (Some(c), Some(p)) = (kind.get_mut(k), phi.get_mut(k))
        {
            *c = TARGET;
            *p = 1.0;
        }
    }
    let mut tree = Tree::roots(
        &roots
            .iter()
            .map(|&(x, y)| (f64::from(x), f64::from(y)))
            .collect::<Vec<_>>(),
    );
    for &(x, y) in roots {
        if let Some(k) = at(x, y)
            && let (Some(c), Some(p)) = (kind.get_mut(k), phi.get_mut(k))
        {
            *c = GROWN;
            *p = 0.0;
        }
    }
    // one sweep over the free cells; a neighbour off the grid is a wall
    let sweep = |kind: &[u8], phi: &mut [f64]| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                let Some(k) = at(x, y) else {
                    continue;
                };
                if !matches!(kind.get(k), Some(&FREE) | Some(&CAND)) {
                    continue;
                }
                let (mut sum, mut n) = (0.0, 0.0);
                for (dx, dy) in STEPS4 {
                    match at(x + dx, y + dy) {
                        Some(j) if kind.get(j) != Some(&WALL) || opts.edge.is_some() => {
                            sum += phi.get(j).copied().unwrap_or(edge);
                            n += 1.0;
                        }
                        None if opts.edge.is_some() => {
                            sum += edge;
                            n += 1.0;
                        }
                        _ => {}
                    }
                }
                if n > 0.0
                    && let Some(p) = phi.get_mut(k)
                {
                    *p += OMEGA * (sum / n - *p);
                }
            }
        }
    };
    for _ in 0..(w.max(h) as u32 * 2) {
        sweep(&kind, &mut phi);
    }
    // the cells next to the tree that may grow, each with the node it would grow from
    let mut cands: Vec<(i32, i32, usize)> = Vec::new();
    let open =
        |tree: &Tree, cands: &mut Vec<(i32, i32, usize)>, kind: &mut Vec<u8>, from: usize| {
            let Some(n) = tree.nodes.get(from) else {
                return;
            };
            let (cx, cy) = (libm::floor(n.x) as i32, libm::floor(n.y) as i32);
            for &(dx, dy) in around {
                let (x, y) = (cx + dx, cy + dy);
                if let Some(c) = at(x, y).and_then(|k| kind.get_mut(k))
                    && *c == FREE
                {
                    *c = CAND;
                    cands.push((x, y, from));
                }
            }
        };
    for i in 0..tree.nodes.len() {
        open(&tree, &mut cands, &mut kind, i);
    }
    let target_dist = |x: f64, y: f64| -> f64 {
        targets
            .iter()
            .map(|&(tx, ty)| {
                let (dx, dy) = (f64::from(tx) - x, f64::from(ty) - y);
                libm::sqrt(dx * dx + dy * dy)
            })
            .fold(f64::INFINITY, f64::min)
    };
    for step in 1..=opts.cells as u32 {
        if cands.is_empty() {
            break;
        }
        let chance = |&(x, y, _): &(i32, i32, usize)| -> f64 {
            let p = at(x, y).and_then(|k| phi.get(k)).copied().unwrap_or(0.0);
            let v = (p + opts.drift.0 * f64::from(x) + opts.drift.1 * f64::from(y)).max(0.0);
            let n = 1.0
                + opts.noise * (2.0 * rand(&[opts.seed, 9.0, f64::from(x), f64::from(y)]) - 1.0);
            libm::pow(v, opts.eta) * n
        };
        let total: f64 = cands.iter().map(chance).sum();
        let chosen = if total > 0.0 {
            let mut pick = rand(&[opts.seed, f64::from(step)]) * total;
            let mut chosen = cands.len() - 1;
            for (i, c) in cands.iter().enumerate() {
                pick -= chance(c);
                if pick <= 0.0 {
                    chosen = i;
                    break;
                }
            }
            chosen
        } else {
            (rand(&[opts.seed, f64::from(step)]) * cands.len() as f64) as usize % cands.len()
        };
        let (x, y, from) = cands.swap_remove(chosen);
        if let Some(k) = at(x, y)
            && let (Some(c), Some(p)) = (kind.get_mut(k), phi.get_mut(k))
        {
            *c = GROWN;
            *p = 0.0;
        }
        let (fx, fy) = (f64::from(x), f64::from(y));
        tree.grow(fx, fy, from, step);
        let new = tree.nodes.len() - 1;
        open(&tree, &mut cands, &mut kind, new);
        if opts.reach > 0.0 && target_dist(fx, fy) <= opts.reach {
            break;
        }
        for _ in 0..opts.relax {
            sweep(&kind, &mut phi);
        }
    }
    tree.weigh();
    tree
}

/// How [`colonize`] grows.
#[derive(Clone, Copy, Debug)]
pub struct Colonize {
    /// Attraction points scattered over the region.
    pub attractors: usize,
    /// How far a tip grows each step.
    pub step: f64,
    /// A point pulls the nearest node within this distance.
    pub influence: f64,
    /// A point is removed once a node comes this close.
    pub kill: f64,
    /// At most this many steps.
    pub steps: u32,
    /// Added to every growth direction (x, y), as a fraction of a step: a lean, upward for veins, toward light.
    pub bias: (f64, f64),
    /// How far each step turns at random, in radians either way: 0 grows smooth, 0.4 wiry.
    pub jitter: f64,
    /// How far one step may turn from the step before it, in radians: 0.3 curves smoothly, `PI` follows the points
    /// wherever they are (the classic algorithm).
    pub max_turn: f64,
    /// The angle a new branch leaves the vein it grows from at, in radians, (least, most): (0.35, 0.8) forks in a Y
    /// as veins and trees do, (1.4, 1.7) at right angles.
    pub fork_angle: (f64, f64),
    /// The chance per step that a node which already has a child starts another branch (a tip always grows): 1 sprouts
    /// everywhere (dense, short twigs), 0.05 to 0.15 a few long branches.
    pub branch_chance: f64,
    /// The region the points are scattered over: x0, y0, x1, y1.
    pub bounds: (f64, f64, f64, f64),
    pub seed: f64,
}

/// Grows a tree from `roots` by space colonization over the part of `bounds` where `inside(x, y)`: each step every
/// node pulled by points grows one `step` toward their mean direction, and points it reaches are removed. When no point
/// is near the tree yet, the node nearest a point grows toward it, so a root reaches a region it starts outside of.
pub fn colonize(roots: &[(f64, f64)], opts: Colonize, inside: impl Fn(f64, f64) -> bool) -> Tree {
    colonize_from(Tree::roots(roots), opts, inside)
}

/// [`colonize`] from a tree already started: set paths laid with [`Tree::path`] (the main veins), which it branches
/// from. Its steps follow the tree's, so growth plays after the paths have grown in; points already within `kill` of
/// the tree are spent from the start.
pub fn colonize_from(start: Tree, opts: Colonize, inside: impl Fn(f64, f64) -> bool) -> Tree {
    let (bx0, by0, bx1, by1) = opts.bounds;
    let mut points: Vec<(f64, f64)> = Vec::with_capacity(opts.attractors);
    let mut k = 0.0;
    while points.len() < opts.attractors && k < opts.attractors as f64 * 16.0 {
        let x = bx0 + (bx1 - bx0) * rand(&[opts.seed, 1.0, k]);
        let y = by0 + (by1 - by0) * rand(&[opts.seed, 2.0, k]);
        if inside(x, y) {
            points.push((x, y));
        }
        k += 1.0;
    }
    let mut tree = start;
    let first = tree.steps + 1;
    let kill0 = opts.kill * opts.kill;
    points.retain(|&(x, y)| {
        tree.nodes
            .iter()
            .all(|n| (n.x - x) * (n.x - x) + (n.y - y) * (n.y - y) > kill0)
    });
    let d2 = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1);
    let (inf2, kill2) = (opts.influence * opts.influence, opts.kill * opts.kill);
    for step in first..first + opts.steps {
        if points.is_empty() {
            break;
        }
        // each point pulls its nearest node within reach
        let mut pull = vec![(0.0, 0.0, 0u32); tree.nodes.len()];
        let mut nearest_any: Option<(usize, usize, f64)> = None;
        for (pi, &pt) in points.iter().enumerate() {
            let mut best: Option<(usize, f64)> = None;
            for (ni, n) in tree.nodes.iter().enumerate() {
                let d = d2(pt, (n.x, n.y));
                if best.is_none_or(|(_, b)| d < b) {
                    best = Some((ni, d));
                }
            }
            let Some((ni, d)) = best else {
                continue;
            };
            if nearest_any.is_none_or(|(_, _, b)| d < b) {
                nearest_any = Some((ni, pi, d));
            }
            if d <= inf2 {
                let n = tree.nodes.get(ni).copied();
                if let (Some(n), Some(acc)) = (n, pull.get_mut(ni)) {
                    let l = libm::sqrt(d).max(1e-9);
                    acc.0 += (pt.0 - n.x) / l;
                    acc.1 += (pt.1 - n.y) / l;
                    acc.2 += 1;
                }
            }
        }
        let mut grown = 0;
        let mut kids = vec![false; tree.nodes.len()];
        for n in &tree.nodes {
            if let Some(k) = n.parent.and_then(|p| kids.get_mut(p)) {
                *k = true;
            }
        }
        for (ni, &(dx, dy, count)) in pull.iter().enumerate() {
            if count == 0 {
                continue;
            }
            let has_kid = kids.get(ni).copied().unwrap_or(false);
            if has_kid && rand(&[opts.seed, 4.0, f64::from(step), ni as f64]) >= opts.branch_chance
            {
                continue;
            }
            let l = libm::sqrt(dx * dx + dy * dy);
            // pulled evenly both ways: it would sit still
            if l < 1e-6 {
                continue;
            }
            let Some(n) = tree.nodes.get(ni).copied() else {
                continue;
            };
            let (ux, uy) = (dx / l + opts.bias.0, dy / l + opts.bias.1);
            let want = libm::atan2(uy, ux);
            // the way the vein ran into this node, to turn from
            // (over the last few nodes, so a wobble in the line does not set a fork's angle)
            let along = {
                let mut back = n;
                for _ in 0..4 {
                    match back.parent.and_then(|p| tree.nodes.get(p)) {
                        Some(&p) => back = p,
                        None => break,
                    }
                }
                let (dx, dy) = (n.x - back.x, n.y - back.y);
                (dx * dx + dy * dy > 1e-9).then(|| libm::atan2(dy, dx))
            };
            let a = match along {
                Some(prev) => {
                    let turn = wrap_angle(want - prev);
                    let turn = if has_kid {
                        // a fork: leave at an angle within the fork range, on the side the points pull to
                        let side = if turn < 0.0 { -1.0 } else { 1.0 };
                        side * libm::fabs(turn).clamp(opts.fork_angle.0, opts.fork_angle.1)
                    } else {
                        turn.clamp(-opts.max_turn, opts.max_turn)
                    };
                    prev + turn
                }
                None => want,
            } + opts.jitter
                * (2.0 * rand(&[opts.seed, 3.0, f64::from(step), ni as f64]) - 1.0);
            let (x, y) = (
                n.x + libm::cos(a) * opts.step,
                n.y + libm::sin(a) * opts.step,
            );
            // a node outside the region may still grow toward it; one inside never leaves it
            if inside(n.x, n.y) && !inside(x, y) {
                continue;
            }
            tree.grow(x, y, ni, step);
            grown += 1;
        }
        if grown == 0 {
            let Some((ni, pi, _)) = nearest_any else {
                break;
            };
            let (Some(n), Some(&pt)) = (tree.nodes.get(ni).copied(), points.get(pi)) else {
                break;
            };
            let l = libm::sqrt(d2(pt, (n.x, n.y))).max(1e-9);
            let (x, y) = (
                n.x + (pt.0 - n.x) / l * opts.step,
                n.y + (pt.1 - n.y) / l * opts.step,
            );
            if l <= opts.kill || (inside(n.x, n.y) && !inside(x, y)) {
                points.swap_remove(pi);
                continue;
            }
            tree.grow(x, y, ni, step);
        }
        // points a new node reached are spent
        let first_new = tree.nodes.len()
            - tree
                .nodes
                .iter()
                .rev()
                .take_while(|n| n.born == step)
                .count();
        let fresh = tree.nodes.get(first_new..).unwrap_or(&[]);
        points.retain(|&pt| fresh.iter().all(|n| d2(pt, (n.x, n.y)) > kill2));
    }
    tree.weigh();
    tree
}
