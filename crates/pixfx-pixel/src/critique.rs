//! Critique: pure checks over a picture (and a row of frames) that an agent cannot judge by eye. Every issue names
//! its cells and a fix the agent can act on; `intended` silences cells placed on purpose (an eye, a spark).
//!
//! Lightness and colour distance are OKLab ([`crate::oklab`]). A cell is painted at alpha `OPAQUE` and above.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::oklab::{Lab, delta_e, lab};
use crate::{OPAQUE, Rgba, RgbaImage};

/// How much an issue matters; issues sort by it, errors first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// One problem: which check found it, where (`cells` as `(x, y)`), what is wrong and what to do.
#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub check: &'static str,
    pub severity: Severity,
    pub cells: Vec<(u32, u32)>,
    pub message: String,
    pub fix: String,
}

/// Every issue, ordered by (severity, check, first cell), and a score: 100 less 30 an error, 12 a warning, 4 a
/// note, never below 0.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub issues: Vec<Issue>,
    pub score: u32,
}

/// Cells an issue lists at most; its message gives the full count.
const MAX_CELLS: usize = 24;
/// Near-duplicate colours: closer than this in OKLab.
const NEAR_DUPLICATE: f64 = 0.02;
/// An orphan differs from every neighbour by more than this.
const ORPHAN_DELTA: f64 = 0.15;
/// Semi-transparent cells allowed, as a share of the cells with any alpha.
const SEMI_SHARE: f64 = 0.05;
/// Noisy cells allowed, as a share of painted cells.
const NOISE_SHARE: f64 = 0.30;
/// Outline: an edge cell is dark below this OKLab lightness; at least this share of edge cells should be.
const DARK_L: f64 = 0.35;
const DARK_SHARE: f64 = 0.5;
/// Stair-step corners allowed.
const JAGGIES: usize = 8;
/// Pillow shading: lightness follows distance from the edge above this, a light direction below this.
const PILLOW_DIST: f64 = 0.55;
const PILLOW_DIR: f64 = 0.25;
/// Frames: painted-cell count may differ from the average by this share; the bottom row may move this many rows.
const VOLUME_DRIFT: f64 = 0.22;
const GROUND_DRIFT: u32 = 2;

const N4: [(i64, i64); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// The picture's checks: colours, alpha, orphans, noise, outline, jaggies and pillow shading.
pub fn critique(img: &RgbaImage, intended: &[(u32, u32)]) -> Report {
    let pic = Pic::new(img);
    let mut issues = Vec::new();
    if pic.painted == 0 {
        issues.push(Issue {
            check: "empty",
            severity: Severity::Error,
            cells: Vec::new(),
            message: format!("the {}x{} picture has no painted cell", img.w, img.h),
            fix: format!("paint the sprite; a cell counts as painted at alpha {OPAQUE} and above"),
        });
        return report(issues);
    }
    let palette = pic.palette();
    issues.extend(colour_budget(&pic, &palette));
    issues.extend(near_duplicate(&palette));
    issues.extend(semi_transparent(&pic));
    issues.extend(orphan(&pic, intended));
    issues.extend(noise(&pic));
    issues.extend(weak_outline(&pic));
    issues.extend(jaggies(&pic));
    issues.extend(pillow_shading(&pic));
    report(issues)
}

/// The checks across a row of frames: empty or repeated frames, drift in volume, ground line and palette.
pub fn critique_frames(frames: &[RgbaImage]) -> Report {
    let pics: Vec<Pic> = frames.iter().map(Pic::new).collect();
    let mut issues = Vec::new();
    for (i, p) in pics.iter().enumerate() {
        if p.painted == 0 {
            issues.push(Issue {
                check: "empty_frame",
                severity: Severity::Warning,
                cells: Vec::new(),
                message: format!("frame {i} has no painted cell"),
                fix: format!("draw frame {i} from its neighbour frame, or drop it from the row"),
            });
        }
    }
    for i in 1..frames.len() {
        if let (Some(a), Some(b)) = (frames.get(i - 1), frames.get(i))
            && pics.get(i).is_some_and(|p| p.painted > 0)
            && a == b
        {
            issues.push(Issue {
                check: "repeated_frame",
                severity: Severity::Note,
                cells: Vec::new(),
                message: format!("frame {i} is identical to frame {}", i - 1),
                fix: format!(
                    "drop frame {i} and hold frame {} for longer, or change what moves in it",
                    i - 1
                ),
            });
        }
    }
    let painted: Vec<(usize, &Pic)> = pics
        .iter()
        .enumerate()
        .filter(|(_, p)| p.painted > 0)
        .collect();
    if painted.len() >= 2 {
        issues.extend(volume_drift(&painted));
        issues.extend(ground_drift(&painted));
        issues.extend(palette_drift(&painted));
    }
    report(issues)
}

fn report(mut issues: Vec<Issue>) -> Report {
    issues.sort_by(|a, b| {
        (a.severity, a.check, a.cells.first().map(|&(x, y)| (y, x))).cmp(&(
            b.severity,
            b.check,
            b.cells.first().map(|&(x, y)| (y, x)),
        ))
    });
    let cost: u32 = issues
        .iter()
        .map(|i| match i.severity {
            Severity::Error => 30,
            Severity::Warning => 12,
            Severity::Note => 4,
        })
        .sum();
    Report {
        issues,
        score: 100u32.saturating_sub(cost),
    }
}

/// A picture as painted cells and alpha, read once.
struct Pic {
    w: u32,
    h: u32,
    /// Colour key `0xRRGGBB` of each painted cell, row by row.
    key: Vec<Option<u32>>,
    alpha: Vec<u8>,
    painted: usize,
}

/// One colour of the picture: how many cells hold it and the first one, row by row.
struct Colour {
    key: u32,
    lab: Lab,
    count: usize,
    first: (u32, u32),
}

impl Pic {
    fn new(img: &RgbaImage) -> Pic {
        let n = img.w as usize * img.h as usize;
        let (mut key, mut alpha) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for y in 0..img.h {
            for x in 0..img.w {
                let [r, g, b, a] = img.get(x, y).unwrap_or([0; 4]);
                alpha.push(a);
                key.push(
                    (a >= OPAQUE).then_some(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)),
                );
            }
        }
        let painted = key.iter().flatten().count();
        Pic {
            w: img.w,
            h: img.h,
            key,
            alpha,
            painted,
        }
    }

    /// The colour key at `(x, y)`, `None` when outside or not painted.
    fn at(&self, x: i64, y: i64) -> Option<u32> {
        if x < 0 || y < 0 || x >= i64::from(self.w) || y >= i64::from(self.h) {
            return None;
        }
        *self.key.get(y as usize * self.w as usize + x as usize)?
    }

    /// Every painted cell with its colour key, row by row.
    fn cells(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        (0..self.h).flat_map(move |y| {
            (0..self.w).filter_map(move |x| self.at(i64::from(x), i64::from(y)).map(|k| (x, y, k)))
        })
    }

    fn is_edge(&self, x: u32, y: u32) -> bool {
        N4.iter()
            .any(|(dx, dy)| self.at(i64::from(x) + dx, i64::from(y) + dy).is_none())
    }

    /// The colours, most used first (ties by key).
    fn palette(&self) -> Vec<Colour> {
        let mut by_key: BTreeMap<u32, (usize, (u32, u32))> = BTreeMap::new();
        for (x, y, k) in self.cells() {
            by_key.entry(k).or_insert((0, (x, y))).0 += 1;
        }
        let mut out: Vec<Colour> = by_key
            .into_iter()
            .map(|(key, (count, first))| Colour {
                key,
                lab: lab(rgba(key)),
                count,
                first,
            })
            .collect();
        out.sort_by(|a, b| b.count.cmp(&a.count).then(a.key.cmp(&b.key)));
        out
    }
}

fn rgba(key: u32) -> Rgba {
    Rgba((key >> 16) as u8, (key >> 8) as u8, key as u8, 255)
}

fn hex(key: u32) -> String {
    format!("#{key:06x}")
}

/// `cells` as text, the first `MAX_CELLS` of them.
fn list(cells: &[(u32, u32)]) -> String {
    let shown: Vec<String> = cells
        .iter()
        .take(MAX_CELLS)
        .map(|(x, y)| format!("({x},{y})"))
        .collect();
    let more = cells.len().saturating_sub(MAX_CELLS);
    if more > 0 {
        format!("{} and {more} more", shown.join(" "))
    } else {
        shown.join(" ")
    }
}

fn capped(mut cells: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    cells.truncate(MAX_CELLS);
    cells
}

fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

fn percent(v: f64) -> f64 {
    libm::floor(v * 100.0 + 0.5)
}

fn colour_budget(pic: &Pic, palette: &[Colour]) -> Vec<Issue> {
    let limit = libm::floor(libm::sqrt(pic.painted as f64) * 1.2 + 0.5).clamp(8.0, 32.0) as usize;
    let Some(extra) = palette.get(limit..).filter(|e| !e.is_empty()) else {
        return Vec::new();
    };
    let cells: Vec<(u32, u32)> = extra.iter().map(|c| c.first).collect();
    let names: Vec<String> = extra.iter().map(|c| hex(c.key)).collect();
    vec![Issue {
        check: "colour_budget",
        severity: Severity::Warning,
        cells: capped(cells),
        message: format!(
            "{} colours for {} painted cells; the budget is {limit}",
            palette.len(),
            pic.painted
        ),
        fix: format!(
            "merge the {} least used colours ({}) into the nearest kept shade, e.g. `cargo run -p pixfx-pixel --example clean -- <sprite.png> --max-colors {limit}`",
            extra.len(),
            names.join(" ")
        ),
    }]
}

fn near_duplicate(palette: &[Colour]) -> Vec<Issue> {
    let mut out = Vec::new();
    for (i, a) in palette.iter().enumerate() {
        for b in palette.iter().skip(i + 1) {
            let d = delta_e(a.lab, b.lab);
            if d < NEAR_DUPLICATE {
                // `b` is the rarer one (palette is most used first): recolour it.
                out.push(Issue {
                    check: "near_duplicate",
                    severity: Severity::Note,
                    cells: vec![b.first],
                    message: format!(
                        "{} ({} cells) and {} ({} cells) look the same (ΔE {d:.3})",
                        hex(b.key),
                        b.count,
                        hex(a.key),
                        a.count
                    ),
                    fix: format!("recolour every {} cell to {}", hex(b.key), hex(a.key)),
                });
            }
        }
    }
    out
}

fn semi_transparent(pic: &Pic) -> Vec<Issue> {
    let mut cells = Vec::new();
    for y in 0..pic.h {
        for x in 0..pic.w {
            let a = pic
                .alpha
                .get(y as usize * pic.w as usize + x as usize)
                .copied()
                .unwrap_or(0);
            if (1..OPAQUE).contains(&a) {
                cells.push((x, y));
            }
        }
    }
    let s = share(cells.len(), cells.len() + pic.painted);
    if s <= SEMI_SHARE {
        return Vec::new();
    }
    vec![Issue {
        check: "semi_transparent",
        severity: Severity::Note,
        message: format!(
            "{} cells ({}%) are semi-transparent: {}",
            cells.len(),
            percent(s),
            list(&cells)
        ),
        fix: format!(
            "make each listed cell fully opaque or fully clear (alpha {OPAQUE} and above counts as painted), unless the glow is meant"
        ),
        cells: capped(cells),
    }]
}

/// One issue for every orphan, so a picture with many specks costs one warning; each cell gets its own recolour.
fn orphan(pic: &Pic, intended: &[(u32, u32)]) -> Vec<Issue> {
    let meant: BTreeSet<(u32, u32)> = intended.iter().copied().collect();
    let mut found: Vec<((u32, u32), u32)> = Vec::new();
    for (x, y, me) in pic.cells() {
        if meant.contains(&(x, y)) {
            continue;
        }
        let around: Vec<u32> = N4
            .iter()
            .filter_map(|(dx, dy)| pic.at(i64::from(x) + dx, i64::from(y) + dy))
            .collect();
        // Fewer than 3 painted neighbours is a silhouette edge or a line, not a speck.
        if around.len() < 3 || around.contains(&me) {
            continue;
        }
        let mine = lab(rgba(me));
        let nearest = around
            .iter()
            .map(|&k| (delta_e(mine, lab(rgba(k))), k))
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        if let Some((d, k)) = nearest
            && d > ORPHAN_DELTA
        {
            found.push(((x, y), k));
        }
    }
    if found.is_empty() {
        return Vec::new();
    }
    let cells: Vec<(u32, u32)> = found.iter().map(|f| f.0).collect();
    let recolour: Vec<String> = found
        .iter()
        .take(MAX_CELLS)
        .map(|((x, y), k)| format!("({x},{y}) to {}", hex(*k)))
        .collect();
    vec![Issue {
        check: "orphan",
        severity: Severity::Warning,
        message: format!(
            "{} single cells with no same-colour neighbour, unlike every neighbour (ΔE > {ORPHAN_DELTA}): {}",
            cells.len(),
            list(&cells)
        ),
        fix: format!(
            "recolour {} (each to its nearest neighbour's colour), or grow each into a 2-3 cell cluster, or list the meant ones in --intended",
            recolour.join(", ")
        ),
        cells: capped(cells),
    }]
}

fn noise(pic: &Pic) -> Vec<Issue> {
    let mut cells = Vec::new();
    for (x, y, me) in pic.cells() {
        let around: Vec<u32> = N4
            .iter()
            .filter_map(|(dx, dy)| pic.at(i64::from(x) + dx, i64::from(y) + dy))
            .collect();
        if around.len() >= 3 && around.iter().filter(|&&k| k != me).count() >= 3 {
            cells.push((x, y));
        }
    }
    let s = share(cells.len(), pic.painted);
    if s <= NOISE_SHARE {
        return Vec::new();
    }
    vec![Issue {
        check: "noise",
        severity: Severity::Warning,
        message: format!(
            "{}% of painted cells differ from 3 or more neighbours, the surface reads as static: {}",
            percent(s),
            list(&cells)
        ),
        fix: "merge the listed cells into clusters of one shade (2-4 cells); keep single cells for edges and highlights"
            .into(),
        cells: capped(cells),
    }]
}

fn weak_outline(pic: &Pic) -> Vec<Issue> {
    let mut edge = 0usize;
    let mut light = Vec::new();
    for (x, y, k) in pic.cells() {
        if pic.is_edge(x, y) {
            edge += 1;
            if lab(rgba(k)).l >= DARK_L {
                light.push((x, y));
            }
        }
    }
    let dark = edge - light.len();
    if edge == 0 || share(dark, edge) >= DARK_SHARE {
        return Vec::new();
    }
    vec![Issue {
        check: "weak_outline",
        severity: Severity::Note,
        message: format!(
            "only {}% of the {edge} silhouette edge cells are dark (OKLab L < {DARK_L}); light edge cells: {}",
            percent(share(dark, edge)),
            list(&light)
        ),
        fix: format!(
            "darken the listed edge cells below lightness {DARK_L} (a dark, hue-shifted shade, not pure black), or add an outer outline"
        ),
        cells: capped(light),
    }]
}

/// An empty cell in the inner corner of an L: painted on two orthogonal sides and on the diagonal between them,
/// empty on the other two sides.
fn jaggies(pic: &Pic) -> Vec<Issue> {
    let p = |x: i64, y: i64| pic.at(x, y).is_some();
    let mut cells = Vec::new();
    for y in 0..i64::from(pic.h) {
        for x in 0..i64::from(pic.w) {
            if p(x, y) {
                continue;
            }
            let (r, l, d, u) = (p(x + 1, y), p(x - 1, y), p(x, y + 1), p(x, y - 1));
            let corner = (r && d && !l && !u && p(x + 1, y + 1))
                || (l && d && !r && !u && p(x - 1, y + 1))
                || (r && u && !l && !d && p(x + 1, y - 1))
                || (l && u && !r && !d && p(x - 1, y - 1));
            if corner {
                cells.push((x as u32, y as u32));
            }
        }
    }
    if cells.len() <= JAGGIES {
        return Vec::new();
    }
    vec![Issue {
        check: "jaggies",
        severity: Severity::Note,
        message: format!("{} stair-step corners along the silhouette: {}", cells.len(), list(&cells)),
        fix: "even out the steps at the listed empty corner cells: keep step lengths regular (1-1-2-2-3) or fill the corner"
            .into(),
        cells: capped(cells),
    }]
}

/// Lightness that rises with distance from the edge, in every direction alike, instead of following one light.
fn pillow_shading(pic: &Pic) -> Vec<Issue> {
    let w = pic.w as usize;
    let mut dist = vec![0u32; pic.key.len()];
    let mut queue = VecDeque::new();
    for (x, y, _) in pic.cells() {
        if pic.is_edge(x, y) {
            if let Some(d) = dist.get_mut(y as usize * w + x as usize) {
                *d = 1;
            }
            queue.push_back((x, y));
        }
    }
    let mut deepest = 0;
    while let Some((x, y)) = queue.pop_front() {
        let here = dist.get(y as usize * w + x as usize).copied().unwrap_or(0);
        for (dx, dy) in N4 {
            let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
            if pic.at(nx, ny).is_none() {
                continue;
            }
            if let Some(d) = dist.get_mut(ny as usize * w + nx as usize)
                && *d == 0
            {
                *d = here + 1;
                deepest = deepest.max(here + 1);
                queue.push_back((nx as u32, ny as u32));
            }
        }
    }
    if deepest < 4 {
        return Vec::new();
    }
    let (mut cx, mut cy) = (0.0, 0.0);
    for (x, y, _) in pic.cells() {
        cx += f64::from(x);
        cy += f64::from(y);
    }
    let n = pic.painted as f64;
    let (cx, cy) = (cx / n, cy / n);
    // (lightness, distance to edge, position along the top-left light direction), outline ring left out.
    let pts: Vec<(f64, f64, f64)> = pic
        .cells()
        .filter_map(|(x, y, k)| {
            let d = dist.get(y as usize * w + x as usize).copied().unwrap_or(0);
            (d >= 2).then(|| {
                (
                    lab(rgba(k)).l,
                    f64::from(d),
                    -((f64::from(x) - cx) + (f64::from(y) - cy)),
                )
            })
        })
        .collect();
    let by_dist = correlation(&pts, |p| p.1);
    let by_dir = correlation(&pts, |p| p.2);
    if by_dist <= PILLOW_DIST || by_dir.abs() >= PILLOW_DIR {
        return Vec::new();
    }
    let cells: Vec<(u32, u32)> = pic
        .cells()
        .filter(|&(x, y, _)| dist.get(y as usize * w + x as usize) == Some(&deepest))
        .map(|(x, y, _)| (x, y))
        .collect();
    vec![Issue {
        check: "pillow_shading",
        severity: Severity::Warning,
        message: format!(
            "lightness follows distance from the edge (r={by_dist:.2}) but not a light direction (r={by_dir:.2}); the listed centre cells are lit like a pillow"
        ),
        fix: "pick one light (top-left): lighten the cells facing top-left, darken the bottom-right side, and stop brightening the centre cells listed"
            .into(),
        cells: capped(cells),
    }]
}

/// Pearson correlation of `f(p)` with each point's lightness; 0 when fewer than 10 points or no spread.
fn correlation(pts: &[(f64, f64, f64)], f: impl Fn(&(f64, f64, f64)) -> f64) -> f64 {
    if pts.len() < 10 {
        return 0.0;
    }
    let n = pts.len() as f64;
    let ma = pts.iter().map(&f).sum::<f64>() / n;
    let mb = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
    for p in pts {
        let (a, b) = (f(p) - ma, p.0 - mb);
        cov += a * b;
        va += a * a;
        vb += b * b;
    }
    if va == 0.0 || vb == 0.0 {
        0.0
    } else {
        cov / libm::sqrt(va * vb)
    }
}

/// The lowest painted cell of a frame, leftmost on its row.
fn bottom(p: &Pic) -> Option<(u32, u32)> {
    p.cells()
        .map(|(x, y, _)| (x, y))
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
}

fn volume_drift(painted: &[(usize, &Pic)]) -> Vec<Issue> {
    let avg = painted.iter().map(|(_, p)| p.painted as f64).sum::<f64>() / painted.len() as f64;
    // The frame furthest from the average (ties: the first); one outlier would push every other frame over too.
    let off = |p: &Pic| (p.painted as f64 - avg).abs();
    let worst = painted
        .iter()
        .fold(None::<(usize, &Pic)>, |best, &(i, p)| match best {
            Some((_, b)) if off(b) >= off(p) => best,
            _ => Some((i, p)),
        });
    worst
        .into_iter()
        .filter(|(_, p)| off(p) / avg > VOLUME_DRIFT)
        .map(|(i, p)| Issue {
            check: "volume_drift",
            severity: Severity::Warning,
            cells: bottom(p).into_iter().collect(),
            message: format!(
                "frame {i} paints {} cells against {:.0} on average: the body changes size",
                p.painted, avg
            ),
            fix: format!(
                "build frame {i} from a base frame and move parts instead of redrawing; bring it within {}% of {avg:.0} cells",
                percent(VOLUME_DRIFT)
            ),
        })
        .collect()
}

fn ground_drift(painted: &[(usize, &Pic)]) -> Vec<Issue> {
    let rows: Vec<(usize, (u32, u32))> = painted
        .iter()
        .filter_map(|&(i, p)| bottom(p).map(|b| (i, b)))
        .collect();
    let (Some(lo), Some(hi)) = (
        rows.iter().map(|r| r.1.1).min(),
        rows.iter().map(|r| r.1.1).max(),
    ) else {
        return Vec::new();
    };
    if hi - lo <= GROUND_DRIFT {
        return Vec::new();
    }
    let list: Vec<String> = rows.iter().map(|(i, b)| format!("{i}:{}", b.1)).collect();
    // The ground is the row most frames stand on (ties: the lowest).
    let mut votes: BTreeMap<u32, usize> = BTreeMap::new();
    for r in &rows {
        *votes.entry(r.1.1).or_insert(0) += 1;
    }
    let ground = votes
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then(a.0.cmp(b.0)))
        .map_or(hi, |(&y, _)| y);
    let off: Vec<String> = rows
        .iter()
        .filter(|r| r.1.1 != ground)
        .map(|r| r.0.to_string())
        .collect();
    vec![Issue {
        check: "ground_drift",
        severity: Severity::Note,
        cells: rows
            .iter()
            .filter(|r| r.1.1 != ground)
            .map(|r| r.1)
            .collect(),
        message: format!(
            "the lowest painted row moves {lo}..{hi} between frames (frame:row {})",
            list.join(" ")
        ),
        fix: format!(
            "put the feet of frames {} back on row {ground}, unless the motion is a jump",
            off.join(", ")
        ),
    }]
}

fn palette_drift(painted: &[(usize, &Pic)]) -> Vec<Issue> {
    let Some(&(first, base)) = painted.first() else {
        return Vec::new();
    };
    let base_keys: BTreeSet<u32> = base.cells().map(|c| c.2).collect();
    let allowed = (base_keys.len() * 3 / 10).max(2);
    let mut out = Vec::new();
    for &(i, p) in painted.iter().skip(1) {
        let mut extra: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        for (x, y, k) in p.cells() {
            if !base_keys.contains(&k) {
                extra.entry(k).or_insert((x, y));
            }
        }
        if extra.len() > allowed {
            let names: Vec<String> = extra.keys().map(|&k| hex(k)).collect();
            let mut cells: Vec<(u32, u32)> = extra.values().copied().collect();
            cells.sort_by_key(|&(x, y)| (y, x));
            out.push(Issue {
                check: "palette_drift",
                severity: Severity::Warning,
                cells: capped(cells),
                message: format!("frame {i} uses {} colours frame {first} does not: {}", extra.len(), names.join(" ")),
                fix: format!("recolour those cells of frame {i} to the nearest colour of frame {first}'s palette"),
            });
        }
    }
    out
}
