//! Colours: [`reduce`] a picture to a few of its own colours, or [`snap`] it onto a given palette.
//!
//! Both judge colours in OKLab and work on the picture's distinct painted colours (alpha at or above
//! [`OPAQUE`](crate)), never on pixel order: every ordering is a total order on integer keys (the packed RGB of a
//! colour, its pixel count, a palette index), so the same picture gives the same colours on every platform. A
//! pixel keeps its alpha; empty pixels are left as they are.

use std::collections::BTreeMap;

use crate::oklab::{Lab, lab};
use crate::{OPAQUE, Rgba, RgbaImage};

/// A colour's RGB packed into one integer key: `0xRRGGBB`.
fn pack(r: u8, g: u8, b: u8) -> u32 {
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

fn unpack(key: u32) -> Rgba {
    Rgba((key >> 16) as u8, (key >> 8) as u8, key as u8, 255)
}

/// Squared OKLab distance (ΔE²): orders the same as ΔE without a square root.
fn d2(x: Lab, y: Lab) -> f64 {
    let (dl, da, db) = (x.l - y.l, x.a - y.a, x.b - y.b);
    dl * dl + da * da + db * db
}

/// Every painted colour of `img` with its pixel count, by key.
fn count(img: &RgbaImage) -> BTreeMap<u32, u64> {
    let mut counts = BTreeMap::new();
    for &[r, g, b, a] in img.rgba.as_chunks::<4>().0 {
        if a >= OPAQUE {
            *counts.entry(pack(r, g, b)).or_insert(0u64) += 1;
        }
    }
    counts
}

/// `img` with every painted pixel's RGB replaced through `table` (key to new colour); alpha kept.
fn recolor(img: &RgbaImage, table: &BTreeMap<u32, Rgba>) -> RgbaImage {
    let mut out = img.clone();
    let mut last: Option<(u32, Rgba)> = None;
    for px in out.rgba.as_chunks_mut::<4>().0 {
        let [r, g, b, a] = *px;
        if a < OPAQUE {
            continue;
        }
        let k = pack(r, g, b);
        let c = match last {
            Some((lk, lc)) if lk == k => Some(lc),
            _ => table.get(&k).copied(),
        };
        if let Some(c) = c {
            last = Some((k, c));
            *px = [c.0, c.1, c.2, a];
        }
    }
    out
}

/// A group of colours merged into one.
struct Cluster {
    /// Indexes into the colours, in the order they joined.
    members: Vec<usize>,
    /// Count-weighted sum of the members' OKLab.
    sum: [f64; 3],
    count: u64,
    alive: bool,
}

impl Cluster {
    fn centre(&self) -> Lab {
        let n = self.count as f64;
        Lab {
            l: self.sum[0] / n,
            a: self.sum[1] / n,
            b: self.sum[2] / n,
        }
    }
}

/// The nearest other living cluster to `i`: (ΔE², index), ties to the lower index; `None` when `i` is alone.
fn nearest_cluster(centres: &[Lab], clusters: &[Cluster], i: usize) -> Option<(f64, usize)> {
    let ci = centres.get(i).copied()?;
    let mut best: Option<(f64, usize)> = None;
    for (j, (c, cl)) in centres.iter().zip(clusters).enumerate() {
        if j == i || !cl.alive {
            continue;
        }
        let d = d2(ci, *c);
        if best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, j));
        }
    }
    best
}

/// `img` on at most `max_colors` of its own colours (at least 1).
///
/// The distinct painted colours, ordered by (count desc, RGB key asc), first join the nearest earlier leader within
/// ΔE `threshold` or become a leader themselves; then the two clusters whose count-weighted OKLab centres are
/// closest merge, ties to the lower cluster indexes, until at most `max_colors` are left. Each cluster is shown by
/// its member nearest its centre (ties to the lower RGB key), so every colour out is one the picture had. The CLI
/// passes 32 and 0.03.
pub fn reduce(img: &RgbaImage, max_colors: usize, threshold: f64) -> RgbaImage {
    let max_colors = max_colors.max(1);
    let mut colours: Vec<(u32, u64)> = count(img).into_iter().collect();
    colours.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
    let labs: Vec<Lab> = colours.iter().map(|&(k, _)| lab(unpack(k))).collect();

    // Leader merge: leaders never move, so a colour joins by its distance to the leader's own colour.
    let limit = threshold * threshold;
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut leaders: Vec<Lab> = Vec::new();
    for (i, (&(_, n), &c)) in colours.iter().zip(&labs).enumerate() {
        let mut best: Option<(f64, usize)> = None;
        for (j, &l) in leaders.iter().enumerate() {
            let d = d2(c, l);
            if d < limit && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, j));
            }
        }
        let w = n as f64;
        let add = [c.l * w, c.a * w, c.b * w];
        match best.and_then(|(_, j)| clusters.get_mut(j)) {
            Some(cl) => {
                cl.members.push(i);
                cl.sum = [cl.sum[0] + add[0], cl.sum[1] + add[1], cl.sum[2] + add[2]];
                cl.count += n;
            }
            None => {
                leaders.push(c);
                clusters.push(Cluster {
                    members: vec![i],
                    sum: add,
                    count: n,
                    alive: true,
                });
            }
        }
    }

    // Closest-pair merge, keeping each cluster's nearest neighbour so a merge costs one pass, not a full search.
    let mut centres: Vec<Lab> = clusters.iter().map(Cluster::centre).collect();
    let mut nn: Vec<Option<(f64, usize)>> = (0..clusters.len())
        .map(|i| nearest_cluster(&centres, &clusters, i))
        .collect();
    let mut alive = clusters.len();
    while alive > max_colors {
        // The closest pair, ordered by (ΔE², lower index, higher index).
        let mut pick: Option<(f64, usize, usize)> = None;
        for (i, (cl, n)) in clusters.iter().zip(&nn).enumerate() {
            if let (true, Some((d, j))) = (cl.alive, *n) {
                let (lo, hi) = (i.min(j), i.max(j));
                let better = pick.is_none_or(|(pd, plo, phi)| {
                    d.total_cmp(&pd)
                        .then(lo.cmp(&plo))
                        .then(hi.cmp(&phi))
                        .is_lt()
                });
                if better {
                    pick = Some((d, lo, hi));
                }
            }
        }
        let Some((_, a, b)) = pick else { break };
        // The lower index (the more frequent leader) absorbs the other.
        let Some(gone) = clusters.get_mut(b) else {
            break;
        };
        gone.alive = false;
        let (members, sum, n) = (std::mem::take(&mut gone.members), gone.sum, gone.count);
        let Some(keep) = clusters.get_mut(a) else {
            break;
        };
        keep.members.extend(members);
        keep.sum = [
            keep.sum[0] + sum[0],
            keep.sum[1] + sum[1],
            keep.sum[2] + sum[2],
        ];
        keep.count += n;
        let centre = keep.centre();
        if let Some(c) = centres.get_mut(a) {
            *c = centre;
        }
        alive -= 1;

        for k in 0..clusters.len() {
            let live = clusters.get(k).is_some_and(|c| c.alive);
            if k == a || !live {
                continue;
            }
            let stale = nn.get(k).copied().flatten();
            let fresh = match stale {
                Some((_, j)) if j == a || j == b => nearest_cluster(&centres, &clusters, k),
                Some((d, j)) => {
                    let da = centres.get(k).map_or(f64::INFINITY, |&ck| d2(ck, centre));
                    if da.total_cmp(&d).then(a.cmp(&j)).is_lt() {
                        Some((da, a))
                    } else {
                        Some((d, j))
                    }
                }
                None => nearest_cluster(&centres, &clusters, k),
            };
            if let Some(slot) = nn.get_mut(k) {
                *slot = fresh;
            }
        }
        let fresh = nearest_cluster(&centres, &clusters, a);
        if let Some(slot) = nn.get_mut(a) {
            *slot = fresh;
        }
    }

    // Each cluster shown by its member nearest the centre.
    let mut table = BTreeMap::new();
    for (cl, &centre) in clusters.iter().zip(&centres) {
        if !cl.alive {
            continue;
        }
        let rep = cl
            .members
            .iter()
            .filter_map(|&m| Some((d2(*labs.get(m)?, centre), colours.get(m)?.0)))
            .min_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)))
            .map(|(_, k)| unpack(k));
        if let Some(rep) = rep {
            for &m in &cl.members {
                if let Some(&(k, _)) = colours.get(m) {
                    table.insert(k, rep);
                }
            }
        }
    }
    recolor(img, &table)
}

/// How [`snap`] matches a picture's colours to a palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapMode {
    /// The palette colour nearest by ΔE.
    Nearest,
    /// The picture's lightness (its 2nd to 98th percentile, weighted by pixel count) stretched onto the palette's
    /// darkest to lightest, then matched on `4·ΔL² + Δa² + Δb²`: a dark picture uses the whole ramp and hue still
    /// counts.
    Lightness,
}

/// The lightness of the `p`-th percentile pixel among `ls` ((L, key, count), sorted by L then key).
fn percentile(ls: &[(f64, u32, u64)], total: u64, p: u64) -> f64 {
    let at = (total * p / 100).min(total.saturating_sub(1));
    let mut seen = 0;
    for &(l, _, n) in ls {
        seen += n;
        if seen > at {
            return l;
        }
    }
    ls.last().map_or(0.0, |&(l, _, _)| l)
}

/// `img` with every painted pixel on its matching colour of `palette` (ties to the lower palette index); alpha
/// kept. An empty palette leaves the picture as it is.
pub fn snap(img: &RgbaImage, palette: &[Rgba], mode: SnapMode) -> RgbaImage {
    if palette.is_empty() {
        return img.clone();
    }
    let pal: Vec<Lab> = palette.iter().map(|&c| lab(c)).collect();
    let counts = count(img);
    let colours: Vec<(u32, Lab)> = counts.keys().map(|&k| (k, lab(unpack(k)))).collect();

    // Lightness: the stretch from the picture's L range onto the palette's.
    let (plo, phi) = pal
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
            (lo.min(p.l), hi.max(p.l))
        });
    let mut ls: Vec<(f64, u32, u64)> = colours
        .iter()
        .filter_map(|&(k, c)| Some((c.l, k, *counts.get(&k)?)))
        .collect();
    ls.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
    let total: u64 = ls.iter().map(|&(_, _, n)| n).sum();
    let (lo, hi) = (percentile(&ls, total, 2), percentile(&ls, total, 98));
    let stretch = |l: f64| {
        if hi - lo <= 1e-9 {
            (plo + phi) / 2.0
        } else {
            (plo + (l - lo) / (hi - lo) * (phi - plo)).clamp(plo, phi)
        }
    };

    let mut table = BTreeMap::new();
    for &(k, c) in &colours {
        let cost = |p: &Lab| match mode {
            SnapMode::Nearest => d2(c, *p),
            SnapMode::Lightness => {
                let (dl, da, db) = (stretch(c.l) - p.l, c.a - p.a, c.b - p.b);
                4.0 * dl * dl + da * da + db * db
            }
        };
        let best = pal
            .iter()
            .enumerate()
            .map(|(i, p)| (cost(p), i))
            .min_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        if let Some(&c) = best.and_then(|(_, i)| palette.get(i)) {
            table.insert(k, Rgba(c.0, c.1, c.2, 255));
        }
    }
    recolor(img, &table)
}
