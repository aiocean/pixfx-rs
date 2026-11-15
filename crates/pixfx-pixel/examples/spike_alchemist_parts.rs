//! Spike: pose a finished sprite by cutting it into parts and turning each part about its joint, no new colours.
//!
//!   cargo run -p pixfx-pixel --example spike_alchemist_parts -- <sprite.png> <spec.txt> <out dir>
//!
//! Each destination cell takes the most common source cell under a 4x4 grid of sample points mapped back through
//! the part's transform (an empty sample votes too), so a turned part keeps the sprite's own colours.

use std::collections::BTreeMap;
use std::path::Path;

use pixfx_pixel::parts::{Part, parse_spec, split};
use pixfx_pixel::{Map, Rgba, colors, nearest, shrink};

const CANVAS: (u32, u32) = (72, 64);
/// Where the sprite's (0, 0) lands on the canvas in the idle pose.
const PLACE: (f64, f64) = (12.0, 6.0);
/// Sample points per cell side.
const SUB: u32 = 4;

/// A 2D affine map `p -> (a*x + b*y + c, d*x + e*y + f)`.
#[derive(Clone, Copy, Debug)]
struct Affine([f64; 6]);

impl Affine {
    fn translate(x: f64, y: f64) -> Self {
        Affine([1.0, 0.0, x, 0.0, 1.0, y])
    }

    /// A turn by `deg` degrees about `(px, py)`; positive turns clockwise on screen (y grows down).
    fn turn(deg: f64, (px, py): (f64, f64)) -> Self {
        let r = deg.to_radians();
        let (s, c) = (libm::sin(r), libm::cos(r));
        Affine([c, -s, px - c * px + s * py, s, c, py - s * px - c * py])
    }

    /// `self` after `other`: `p -> self(other(p))`.
    fn then_after(self, other: Affine) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [g, h, i, j, k, l] = other.0;
        Affine([
            a * g + b * j,
            a * h + b * k,
            a * i + b * l + c,
            d * g + e * j,
            d * h + e * k,
            d * i + e * l + f,
        ])
    }

    fn apply(self, (x, y): (f64, f64)) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.0;
        (a * x + b * y + c, d * x + e * y + f)
    }

    fn inverse(self) -> Option<Self> {
        let [a, b, c, d, e, f] = self.0;
        let det = a * e - b * d;
        if det.abs() < 1e-12 {
            return None;
        }
        let (ia, ib, id, ie) = (e / det, -b / det, -d / det, a / det);
        Some(Affine([
            ia,
            ib,
            -(ia * c + ib * f),
            id,
            ie,
            -(id * c + ie * f),
        ]))
    }
}

/// How one part moves in a pose, relative to its parent: a turn about its joint, then a shift.
#[derive(Clone, Copy, Debug, Default)]
struct Move {
    deg: f64,
    dx: f64,
    dy: f64,
}

fn mv(deg: f64, dx: f64, dy: f64) -> Move {
    Move { deg, dx, dy }
}

/// The joint each part turns about, in sprite cells: the `at` of its spec line (the split's pivot).
fn joint(part: &Part) -> (f64, f64) {
    part.pivot.unwrap_or((0.0, 0.0))
}

/// Cells of a part within this radius of its joint are also drawn unturned under the turned part, so a turn
/// leaves no notch at the joint.
fn cap_radius(name: &str) -> f64 {
    match name {
        "near_arm" | "far_arm" => 3.5,
        "head" | "hair" => 2.5,
        _ => 0.0,
    }
}

/// Back to front.
const ORDER: [&str; 7] = [
    "hair", "ribbons", "far_arm", "body", "head", "sword", "near_arm",
];

/// Each part's sprite-to-canvas map for a pose: its parent's map after its own move.
fn transforms(parts: &[Part], pose: &BTreeMap<&str, Move>) -> BTreeMap<String, Affine> {
    let mut out: BTreeMap<String, Affine> = BTreeMap::new();
    // Parents come before children once every part whose parent is placed has been placed.
    while out.len() < parts.len() {
        let before = out.len();
        for p in parts {
            if out.contains_key(&p.name) {
                continue;
            }
            let base = match &p.parent {
                None => Some(Affine::translate(PLACE.0, PLACE.1)),
                Some(parent) => out.get(parent).copied(),
            };
            let Some(base) = base else { continue };
            let m = pose.get(p.name.as_str()).copied().unwrap_or_default();
            let local = Affine::translate(m.dx, m.dy).then_after(Affine::turn(m.deg, joint(p)));
            out.insert(p.name.clone(), base.then_after(local));
        }
        if out.len() == before {
            break;
        }
    }
    out
}

/// Draw `map` through `to_canvas` onto `canvas`, by majority vote of the sub-samples in each cell.
fn stamp(canvas: &mut Map, map: &Map, to_canvas: Affine) {
    let Some(back) = to_canvas.inverse() else {
        return;
    };
    for y in 0..canvas.h {
        for x in 0..canvas.w {
            let mut votes: BTreeMap<Option<usize>, u32> = BTreeMap::new();
            for sy in 0..SUB {
                for sx in 0..SUB {
                    let p = (
                        f64::from(x) + (f64::from(sx) + 0.5) / f64::from(SUB),
                        f64::from(y) + (f64::from(sy) + 0.5) / f64::from(SUB),
                    );
                    let (u, v) = back.apply(p);
                    let cell = map.get(libm::floor(u) as i64, libm::floor(v) as i64);
                    *votes.entry(cell).or_insert(0) += 1;
                }
            }
            // The most votes wins; a tie goes to a painted cell over empty, then the lower index.
            let winner = votes
                .iter()
                .max_by_key(|(c, n)| (**n, c.is_some(), std::cmp::Reverse(**c)))
                .and_then(|(c, _)| *c);
            if let Some(c) = winner {
                let i = (y * canvas.w + x) as usize;
                if let Some(slot) = canvas.cells.get_mut(i) {
                    *slot = Some(c);
                }
            }
        }
    }
}

/// The part's cells within `r` of `(px, py)`.
fn cap(part: &Part, (px, py): (f64, f64), r: f64) -> Map {
    let mut out = Map::empty(part.map.w, part.map.h);
    for y in 0..part.map.h {
        for x in 0..part.map.w {
            let (dx, dy) = (f64::from(x) + 0.5 - px, f64::from(y) + 0.5 - py);
            if dx * dx + dy * dy <= r * r {
                let i = (y * part.map.w + x) as usize;
                if let (Some(slot), Some(c)) = (out.cells.get_mut(i), part.map.cells.get(i)) {
                    *slot = *c;
                }
            }
        }
    }
    out
}

fn compose(parts: &[Part], pose: &BTreeMap<&str, Move>) -> Map {
    let at = transforms(parts, pose);
    let mut canvas = Map::empty(CANVAS.0, CANVAS.1);
    for name in ORDER {
        let Some(part) = parts.iter().find(|p| p.name == name) else {
            continue;
        };
        let Some(&t) = at.get(name) else { continue };
        let m = pose.get(name).copied().unwrap_or_default();
        // A part thrown clear of its joint (shifted, not only turned) leaves no cap behind.
        let attached = m.dx == 0.0 && m.dy == 0.0;
        let r = cap_radius(name);
        if r > 0.0 && attached {
            // The joint cap rides with the parent, unturned by this part's own move.
            let parent_t = part
                .parent
                .as_ref()
                .and_then(|p| at.get(p).copied())
                .unwrap_or(t);
            stamp(&mut canvas, &cap(part, joint(part), r), parent_t);
        }
        stamp(&mut canvas, &part.map, t);
    }
    canvas
}

fn poses() -> Vec<(&'static str, BTreeMap<&'static str, Move>)> {
    let p = |moves: &[(&'static str, Move)]| moves.iter().copied().collect::<BTreeMap<_, _>>();
    vec![
        ("idle", p(&[])),
        (
            "guard",
            p(&[
                ("near_arm", mv(-75.0, 0.0, 0.0)),
                ("sword", mv(-20.0, 0.0, 0.0)),
                ("far_arm", mv(12.0, 0.0, 0.0)),
            ]),
        ),
        (
            "hurt",
            p(&[
                // Struck from the front (she faces right): the top recoils back, to the left.
                ("body", mv(8.0, 0.0, 0.0)),
                ("head", mv(12.0, 0.0, 0.0)),
                ("near_arm", mv(20.0, 0.0, 0.0)),
                ("far_arm", mv(-10.0, 0.0, 0.0)),
                ("hair", mv(-10.0, 0.0, 0.0)),
            ]),
        ),
        (
            "down",
            p(&[
                ("body", mv(-90.0, 28.0, -24.0)),
                ("near_arm", mv(-30.0, 0.0, 0.0)),
            ]),
        ),
        (
            "headless",
            p(&[
                ("head", mv(130.0, 14.0, -6.0)),
                ("hair", mv(0.0, 0.0, 0.0)),
                ("near_arm", mv(15.0, 0.0, 0.0)),
            ]),
        ),
    ]
}

fn write(path: &Path, map: &Map, palette: &[Rgba]) -> Result<(), String> {
    let img = map.paint(palette);
    pixfx_export::write_png_rgba(path, img.w, img.h, &img.rgba).map_err(|e| format!("{e:?}"))
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let (Some(sprite), Some(spec), Some(out)) = (args.get(1), args.get(2), args.get(3)) else {
        return Err("usage: spike_alchemist_parts <sprite.png> <spec.txt> <out dir>".into());
    };
    let bytes = std::fs::read(sprite).map_err(|e| format!("{sprite}: {e}"))?;
    let img = pixfx_export::decode_png_rgba(&bytes)?;
    let palette = colors(&img);
    let map = shrink(&nearest(&img, &palette), img.w, 1.0);
    let spec_text = std::fs::read_to_string(spec).map_err(|e| format!("{spec}: {e}"))?;
    let parts = split(&map, &parse_spec(&spec_text).map_err(|e| e.0)?).map_err(|e| e.0)?;
    let out = Path::new(out);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;

    let all = poses();
    let mut sheet = Map::empty(CANVAS.0 * all.len() as u32, CANVAS.1);
    for (k, (name, pose)) in all.iter().enumerate() {
        let frame = compose(&parts, pose);
        let path = out.join(format!("{name}.png"));
        write(&path, &frame, &palette)?;
        let used: std::collections::BTreeSet<usize> =
            frame.cells.iter().flatten().copied().collect();
        println!("{name}: {} colours of {}", used.len(), palette.len());
        for y in 0..CANVAS.1 {
            for x in 0..CANVAS.0 {
                let src = (y * CANVAS.0 + x) as usize;
                let dst = (y * sheet.w + k as u32 * CANVAS.0 + x) as usize;
                if let (Some(slot), Some(c)) = (sheet.cells.get_mut(dst), frame.cells.get(src)) {
                    *slot = *c;
                }
            }
        }
    }
    write(&out.join("sheet.png"), &sheet, &palette)?;
    Ok(())
}
