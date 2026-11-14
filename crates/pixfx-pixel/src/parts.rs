//! Splitting a sprite into parts an effect can move on its own: a head cut off, limbs flying, a body blown apart.
//!
//! The labels are the agent's call. It reads the sprite, writes a spec (each part a name, a parent and a rule), and
//! [`split`] checks the spec: every painted cell lands in exactly one part, every parent exists and touches its part.
//! [`components`] finds the connected regions an agent starts from.
//!
//! Boxes are `[x, y, w, h]` in cells. A cell `(x, y)` covers `x..x + 1` across and `y..y + 1` down, so its centre is
//! `(x + 0.5, y + 0.5)` and the border between two cells sits on a whole number.
//!
//! Spec text, one part per line, blank lines and lines starting with `#` skipped:
//!
//! ```text
//! <name> <parent>|- box X,Y,W,H
//! <name> <parent>|- colors a,b[,..] [in X,Y,W,H]
//! <name> <parent>|- rest
//! ```
//!
//! `-` means no parent; colours are the legend letters of [`crate::text`] (`a` is palette index 0).
use std::fmt;

use crate::{LEGEND, Map};

/// A connected region of painted cells (8-connected, any colours).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    /// How many cells it holds.
    pub cells: usize,
    /// Its box, `[x, y, w, h]`.
    pub bbox: [u32; 4],
    /// The palette indices it uses, ascending.
    pub colors: Vec<usize>,
}

/// The 8-connected regions of painted cells, ordered by each region's first cell, row by row.
pub fn components(map: &Map) -> Vec<Component> {
    let (w, h) = (i64::from(map.w), i64::from(map.h));
    let mut seen = vec![false; map.cells.len()];
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let start = index(map, x, y);
            if map.get(x, y).is_none() || seen.get(start).copied().unwrap_or(true) {
                continue;
            }
            mark(&mut seen, start);
            let mut stack = vec![(x, y)];
            let mut cells = 0;
            let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
            let mut colors = Vec::new();
            while let Some((cx, cy)) = stack.pop() {
                cells += 1;
                (x0, y0, x1, y1) = (x0.min(cx), y0.min(cy), x1.max(cx), y1.max(cy));
                if let Some(c) = map.get(cx, cy) {
                    colors.push(c);
                }
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if map.get(nx, ny).is_none() {
                            continue;
                        }
                        let i = index(map, nx, ny);
                        if !seen.get(i).copied().unwrap_or(true) {
                            mark(&mut seen, i);
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            colors.sort_unstable();
            colors.dedup();
            out.push(Component {
                cells,
                bbox: bbox_of(x0, y0, x1, y1),
                colors,
            });
        }
    }
    out
}

/// Which cells a part takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Every painted cell inside the box `[x, y, w, h]`.
    Box([u32; 4]),
    /// Every cell of these palette indices, only inside the box when one is given.
    Colors {
        colors: Vec<usize>,
        within: Option<[u32; 4]>,
    },
    /// Every painted cell no other rule takes, wherever it stands in the spec.
    Rest,
}

/// One part as the agent names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartSpec {
    pub name: String,
    /// The part it hangs from (a hand from an arm); `None` for the root.
    pub parent: Option<String>,
    pub rule: Rule,
}

/// One part of a split sprite.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub name: String,
    pub parent: Option<String>,
    /// The source map's size, holding only this part's cells.
    pub map: Map,
    /// The box of its cells, `[x, y, w, h]`.
    pub bbox: [u32; 4],
    /// The middle of the border it shares with its parent, `(x, y)` in cells: the mean of the midpoints of every
    /// cell edge between a part cell and a 4-neighbouring parent cell. A border between rows 3 and 4 lies at
    /// `y = 4.0`; a cell's centre is at `.5`. `None` when the part has no parent.
    pub pivot: Option<(f64, f64)>,
}

/// Why a spec does not split a map, in words that name the parts at fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitError(pub String);

impl fmt::Display for SplitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SplitError {}

/// The map split into the spec's parts, in spec order.
///
/// Each painted cell goes to the first `Box` or `Colors` rule that takes it; cells none takes go to the `Rest` part.
/// Refuses a spec with a repeated name, two `Rest` parts, a part with no cells, an unknown parent, a parent loop, a
/// parent that does not touch its part (4-neighbour), or painted cells left over with no `Rest` part.
pub fn split(map: &Map, spec: &[PartSpec]) -> Result<Vec<Part>, SplitError> {
    check_names(spec)?;
    let rest = spec.iter().position(|s| s.rule == Rule::Rest);

    let mut owner: Vec<Option<usize>> = vec![None; map.cells.len()];
    let mut left = 0usize;
    for y in 0..map.h {
        for x in 0..map.w {
            let Some(color) = map.get(i64::from(x), i64::from(y)) else {
                continue;
            };
            let taken = spec
                .iter()
                .position(|s| takes(&s.rule, x, y, color))
                .or(rest);
            match taken {
                Some(p) => {
                    if let Some(o) = owner.get_mut(index(map, i64::from(x), i64::from(y))) {
                        *o = Some(p);
                    }
                }
                None => left += 1,
            }
        }
    }
    if left > 0 {
        return Err(SplitError(format!(
            "{left} painted cells match no rule and the spec has no rest part"
        )));
    }

    let mut parts = Vec::with_capacity(spec.len());
    for (p, s) in spec.iter().enumerate() {
        let mut part_map = Map::empty(map.w, map.h);
        let mut corners: Option<(i64, i64, i64, i64)> = None;
        for (i, (cell, o)) in map.cells.iter().zip(&owner).enumerate() {
            if *o != Some(p) {
                continue;
            }
            if let Some(slot) = part_map.cells.get_mut(i) {
                *slot = *cell;
            }
            let (x, y) = position(map, i);
            corners = Some(match corners {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
        let Some((x0, y0, x1, y1)) = corners else {
            return Err(SplitError(format!("part {} has no cells", s.name)));
        };
        let pivot = match &s.parent {
            None => None,
            Some(parent) => Some(pivot(map, &owner, p, &s.name, spec, parent)?),
        };
        parts.push(Part {
            name: s.name.clone(),
            parent: s.parent.clone(),
            map: part_map,
            bbox: bbox_of(x0, y0, x1, y1),
            pivot,
        });
    }
    Ok(parts)
}

/// The spec in its text form, one line per part; [`parse_spec`] reads it back.
pub fn spec_text(spec: &[PartSpec]) -> String {
    let mut out = String::new();
    for s in spec {
        let rule = match &s.rule {
            Rule::Box(b) => format!("box {}", box_text(*b)),
            Rule::Colors { colors, within } => {
                let letters: Vec<String> = colors
                    .iter()
                    .map(|&c| {
                        LEGEND
                            .chars()
                            .nth(c)
                            .map_or_else(|| format!("?{c}"), String::from)
                    })
                    .collect();
                match within {
                    Some(b) => format!("colors {} in {}", letters.join(","), box_text(*b)),
                    None => format!("colors {}", letters.join(",")),
                }
            }
            Rule::Rest => "rest".to_string(),
        };
        let parent = s.parent.as_deref().unwrap_or("-");
        out.push_str(&format!("{} {parent} {rule}\n", s.name));
    }
    out
}

/// A spec from its text form (see the module docs); errors name the line.
pub fn parse_spec(text: &str) -> Result<Vec<PartSpec>, SplitError> {
    let mut spec = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let err = |m: &str| SplitError(format!("line {}: {m}: {line}", n + 1));
        let words: Vec<&str> = line.split_whitespace().collect();
        let (name, parent, rest) = match words.as_slice() {
            [name, parent, rest @ ..] => (*name, *parent, rest),
            _ => return Err(err("expected `name parent rule`")),
        };
        if name == "-" {
            return Err(err("`-` is not a part name"));
        }
        let rule = match rest {
            ["box", b] => Rule::Box(parse_box(b).ok_or_else(|| err("box must be X,Y,W,H"))?),
            ["colors", c] => Rule::Colors {
                colors: parse_colors(c).map_err(|m| err(&m))?,
                within: None,
            },
            ["colors", c, "in", b] => Rule::Colors {
                colors: parse_colors(c).map_err(|m| err(&m))?,
                within: Some(parse_box(b).ok_or_else(|| err("box must be X,Y,W,H"))?),
            },
            ["rest"] => Rule::Rest,
            _ => {
                return Err(err(
                    "rule must be `box X,Y,W,H`, `colors a,b [in X,Y,W,H]` or `rest`",
                ));
            }
        };
        spec.push(PartSpec {
            name: name.to_string(),
            parent: (parent != "-").then(|| parent.to_string()),
            rule,
        });
    }
    Ok(spec)
}

fn check_names(spec: &[PartSpec]) -> Result<(), SplitError> {
    for (i, s) in spec.iter().enumerate() {
        if spec.iter().take(i).any(|t| t.name == s.name) {
            return Err(SplitError(format!("part {} is named twice", s.name)));
        }
    }
    if spec.iter().filter(|s| s.rule == Rule::Rest).count() > 1 {
        return Err(SplitError(
            "the spec has more than one rest part".to_string(),
        ));
    }
    for s in spec {
        let mut at = s;
        for _ in 0..=spec.len() {
            let Some(parent) = &at.parent else {
                break;
            };
            at = spec.iter().find(|t| &t.name == parent).ok_or_else(|| {
                SplitError(format!(
                    "part {}: parent {parent} is not a part of the spec",
                    at.name
                ))
            })?;
            if at.name == s.name {
                return Err(SplitError(format!(
                    "part {} is its own ancestor through its parents",
                    s.name
                )));
            }
        }
    }
    Ok(())
}

/// The middle of the border between part `p` and its parent; an error when they do not touch.
fn pivot(
    map: &Map,
    owner: &[Option<usize>],
    p: usize,
    name: &str,
    spec: &[PartSpec],
    parent: &str,
) -> Result<(f64, f64), SplitError> {
    let parent_index = spec.iter().position(|s| s.name == parent);
    let owned_by = |x: i64, y: i64, q: Option<usize>| {
        x >= 0
            && y >= 0
            && x < i64::from(map.w)
            && y < i64::from(map.h)
            && q.is_some()
            && owner.get(index(map, x, y)).copied().flatten() == q
    };
    // Midpoints doubled, so every sum stays a whole number.
    let (mut sx, mut sy, mut count) = (0i64, 0i64, 0i64);
    for (i, o) in owner.iter().enumerate() {
        if *o != Some(p) {
            continue;
        }
        let (x, y) = position(map, i);
        for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1)] {
            if owned_by(x + dx, y + dy, parent_index) {
                sx += 2 * x + 1 + dx;
                sy += 2 * y + 1 + dy;
                count += 1;
            }
        }
    }
    if count == 0 {
        return Err(SplitError(format!(
            "part {name}: parent {parent} does not touch it"
        )));
    }
    let d = (2 * count) as f64;
    Ok((sx as f64 / d, sy as f64 / d))
}

fn takes(rule: &Rule, x: u32, y: u32, color: usize) -> bool {
    match rule {
        Rule::Box(b) => inside(*b, x, y),
        Rule::Colors { colors, within } => {
            colors.contains(&color) && within.is_none_or(|b| inside(b, x, y))
        }
        Rule::Rest => false,
    }
}

fn inside([bx, by, bw, bh]: [u32; 4], x: u32, y: u32) -> bool {
    let (x, y) = (u64::from(x), u64::from(y));
    x >= u64::from(bx)
        && y >= u64::from(by)
        && x < u64::from(bx) + u64::from(bw)
        && y < u64::from(by) + u64::from(bh)
}

fn parse_box(text: &str) -> Option<[u32; 4]> {
    let n: Vec<u32> = text
        .split(',')
        .map(|v| v.trim().parse().ok())
        .collect::<Option<_>>()?;
    n.try_into().ok()
}

fn parse_colors(text: &str) -> Result<Vec<usize>, String> {
    text.split(',')
        .map(|letter| {
            let mut chars = letter.trim().chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => LEGEND
                    .chars()
                    .position(|l| l == c)
                    .ok_or_else(|| format!("{c} is not a legend letter")),
                _ => Err(format!("colour `{letter}` must be one legend letter")),
            }
        })
        .collect()
}

fn box_text([x, y, w, h]: [u32; 4]) -> String {
    format!("{x},{y},{w},{h}")
}

fn bbox_of(x0: i64, y0: i64, x1: i64, y1: i64) -> [u32; 4] {
    let u = |v: i64| u32::try_from(v).unwrap_or(0);
    [u(x0), u(y0), u(x1 - x0 + 1), u(y1 - y0 + 1)]
}

fn index(map: &Map, x: i64, y: i64) -> usize {
    y as usize * map.w as usize + x as usize
}

fn position(map: &Map, i: usize) -> (i64, i64) {
    let w = (map.w as usize).max(1);
    ((i % w) as i64, (i / w) as i64)
}

fn mark(seen: &mut [bool], i: usize) {
    if let Some(s) = seen.get_mut(i) {
        *s = true;
    }
}
