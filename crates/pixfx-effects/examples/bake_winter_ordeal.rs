//! Bakes the sky of `shadow/winter-ordeal` once: the cracks and the slabs they cut, written as data into
//! `src/shadow/winter_ordeal_sky.rs`, which the effect reads as it is; nothing here runs in a game.
//!
//!   cargo run --release -p pixfx-effects --no-default-features --features shadow-winter-ordeal \
//!     --example bake_winter_ordeal -- [--seed N] [--preview OUT.ppm]
//!
//! The sky is a ragged oval over the foe with a keystone in its middle (the eye). Main cracks run from the keystone's
//! rim out past the sky's edge, and three rings of cracks join each main crack to the next (a few left out, so some slabs
//! are bigger): each crack is a lightning line (`pixfx_draw::growth::jag`, midpoint displacement) laid cell by cell. The sky's cells the cracks wall in, grouped across sides, are the slabs; each slab is cut again into a few
//! shards (nearest of a few points) for when it smashes. Crack cells keep the time they appear (a crack grows along its
//! bolt) and the slabs either side, so a crack shows while one of them is still up.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use pixfx_draw::growth::jag;
use pixfx_draw::math::{fbm, rand};

/// The sky's middle (local pixels, the feet at 0) and half size.
const SKY: (i32, i32) = (0, -112);
const HALF: (f64, f64) = (100.0, 40.0);
/// The keystone round the eye: an oval this half size.
const KEY: (f64, f64) = (17.0, 9.0);
/// Main cracks, and the rings' distances out (a share of the way from the keystone to the edge).
const SPOKES: usize = 11;
const RINGS: [f64; 3] = [0.3, 0.58, 0.84];
/// When the cracks grow: the spokes in turn from SPOKE_T, the rings from RING_T, each taking GROW_T.
const SPOKE_T: f64 = 0.45;
const SPOKE_GAP: f64 = 0.04;
const RING_T: f64 = 0.85;
const RING_GAP: f64 = 0.012;
const GROW_T: f64 = 0.08;
/// A slab smashes into this many shards at most.
const SHARDS: usize = 6;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1).cloned())
    };
    let seed: f64 = arg("--seed").and_then(|s| s.parse().ok()).unwrap_or(3.0);
    let out = "crates/pixfx-effects/src/shadow/winter_ordeal_sky.rs";

    let bounds = (
        SKY.0 - HALF.0 as i32 - 4,
        SKY.1 - HALF.1 as i32 - 4,
        SKY.0 + HALF.0 as i32 + 4,
        SKY.1 + HALF.1 as i32 + 4,
    );
    // the sky: an oval with a ragged rim
    let sky = |x: i32, y: i32| {
        let (u, v) = (f64::from(x - SKY.0) / HALF.0, f64::from(y - SKY.1) / HALF.1);
        let rim = 1.0 + 0.22 * (fbm(f64::from(x) * 0.09, f64::from(y) * 0.09, seed, 3) - 0.5);
        u * u + v * v < rim * rim
    };
    let key = |x: i32, y: i32| {
        let (u, v) = (f64::from(x - SKY.0) / KEY.0, f64::from(y - SKY.1) / KEY.1);
        u * u + v * v < 1.0
    };
    // the last sky cell along a ray from the middle, at a share of the way from the keystone's rim to the sky's edge
    let along = |a: f64, k: f64| -> (i32, i32) {
        let (c, s) = (libm::cos(a), libm::sin(a));
        let mut edge = (SKY.0, SKY.1);
        let mut r = 0.0;
        while r < 200.0 {
            let (x, y) = (
                SKY.0 + libm::floor(c * r * HALF.0 / HALF.1 * 0.5 + 0.5) as i32,
                0,
            );
            let _ = (x, y);
            let p = (
                SKY.0 + libm::floor(c * r + 0.5) as i32,
                SKY.1 + libm::floor(s * r * HALF.1 / HALF.0 + 0.5) as i32,
            );
            if !sky(p.0, p.1) {
                break;
            }
            edge = p;
            r += 0.5;
        }
        let from = (
            f64::from(SKY.0) + c * KEY.0 * 1.15,
            f64::from(SKY.1) + s * KEY.1 * 1.15,
        );
        (
            libm::floor(from.0 + (f64::from(edge.0) - from.0) * k + 0.5) as i32,
            libm::floor(from.1 + (f64::from(edge.1) - from.1) * k + 0.5) as i32,
        )
    };
    // crack cells, with when each appears
    let mut crack: BTreeMap<(i32, i32), f64> = BTreeMap::new();
    // the keystone's rim is a crack from the start of the rings: the eye's slab is cut free last
    for y in bounds.1..=bounds.3 {
        for x in bounds.0..=bounds.2 {
            if sky(x, y)
                && !key(x, y)
                && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|&(dx, dy)| key(x + dx, y + dy))
            {
                crack.insert((x, y), RING_T + 0.25);
            }
        }
    }
    // a crack along a polyline made jagged (the lightning line, `growth::jag`), laid cell by cell with 8-connected steps
    // (a diagonal step still seals the sky either side), each cell appearing as the crack runs along it
    let lay = |crack: &mut BTreeMap<(i32, i32), f64>, line: &[(f64, f64)], t0: f64| {
        let total: f64 = line
            .windows(2)
            .map(|w| match w {
                [a, b] => libm::hypot(b.0 - a.0, b.1 - a.1),
                _ => 0.0,
            })
            .sum::<f64>()
            .max(1.0);
        let mut run = 0.0;
        for w in line.windows(2) {
            let [a, b] = w else { continue };
            let len = libm::hypot(b.0 - a.0, b.1 - a.1);
            let steps = libm::ceil(len.max(1.0) * 2.0) as i32;
            for s in 0..=steps {
                let k = f64::from(s) / f64::from(steps);
                let c = (
                    libm::floor(a.0 + (b.0 - a.0) * k + 0.5) as i32,
                    libm::floor(a.1 + (b.1 - a.1) * k + 0.5) as i32,
                );
                if sky(c.0, c.1) && !key(c.0, c.1) {
                    crack
                        .entry(c)
                        .or_insert(t0 + GROW_T * (run + len * k) / total);
                }
            }
            run += len;
        }
    };
    let point = |a: f64, k: f64| {
        let p = along(a, k);
        (f64::from(p.0), f64::from(p.1))
    };
    // the spokes: from the keystone's rim out past the sky's edge, jagged
    let angles: Vec<f64> = (0..SPOKES)
        .map(|i| {
            core::f64::consts::TAU * (i as f64 + 0.3 * (rand(&[seed, i as f64, 1.0]) - 0.5))
                / SPOKES as f64
        })
        .collect();
    let mut spokes: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, &a) in angles.iter().enumerate() {
        let (from, to) = (point(a, 0.0), point(a, 1.25));
        let line = jag(&[from, to], 4, 0.16, seed * 13.0 + i as f64);
        lay(&mut crack, &line, SPOKE_T + SPOKE_GAP * i as f64);
        spokes.push(line);
    }
    // the point a share `k` of the way along a spoke's jagged line (to its sky edge, at 1/1.25 of it)
    let on = |line: &[(f64, f64)], k: f64| -> (f64, f64) {
        let total: f64 = line
            .windows(2)
            .map(|w| match w {
                [a, b] => libm::hypot(b.0 - a.0, b.1 - a.1),
                _ => 0.0,
            })
            .sum();
        let mut want = total * k / 1.25;
        for w in line.windows(2) {
            let [a, b] = w else { continue };
            let len = libm::hypot(b.0 - a.0, b.1 - a.1);
            if want <= len && len > 0.0 {
                let u = want / len;
                return (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
            }
            want -= len;
        }
        line.last().copied().unwrap_or((0.0, 0.0))
    };
    // the rings: in each sector, an arc from one spoke's line to the next, bowed out through the sector's middle at the
    // ring's distance, jagged; a few sectors left out so some slabs are bigger
    let mut k = 0;
    for (r, &ring) in RINGS.iter().enumerate() {
        for i in 0..SPOKES {
            if rand(&[seed, r as f64, i as f64, 2.0]) < 0.22 {
                continue;
            }
            let a0 = angles.get(i).copied().unwrap_or(0.0);
            let mut a1 = angles.get((i + 1) % SPOKES).copied().unwrap_or(0.0);
            if a1 < a0 {
                a1 += core::f64::consts::TAU;
            }
            let (Some(s0), Some(s1)) = (spokes.get(i), spokes.get((i + 1) % SPOKES)) else {
                continue;
            };
            let arc = [
                on(s0, ring),
                point((a0 + a1) / 2.0, ring * 1.06),
                on(s1, ring),
            ];
            let line = jag(&arc, 3, 0.14, seed * 17.0 + k as f64);
            lay(&mut crack, &line, RING_T + RING_GAP * k as f64);
            k += 1;
        }
    }
    // the slabs: the sky's whole cells grouped across sides; the keystone is one slab
    let mut slab: BTreeMap<(i32, i32), usize> = BTreeMap::new();
    let mut slabs: Vec<Vec<(i32, i32)>> = Vec::new();
    for y in bounds.1..=bounds.3 {
        for x in bounds.0..=bounds.2 {
            if !sky(x, y) || crack.contains_key(&(x, y)) || slab.contains_key(&(x, y)) {
                continue;
            }
            let id = slabs.len();
            let mut cells = vec![(x, y)];
            slab.insert((x, y), id);
            let mut i = 0;
            while let Some(&(cx, cy)) = cells.get(i) {
                i += 1;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = (cx + dx, cy + dy);
                    if sky(n.0, n.1) && !crack.contains_key(&n) && !slab.contains_key(&n) {
                        slab.insert(n, id);
                        cells.push(n);
                    }
                }
            }
            slabs.push(cells);
        }
    }
    // tiny bits stay as crack: a slab of fewer than 5 cells is merged into the cracks round it
    let small: Vec<usize> = slabs
        .iter()
        .enumerate()
        .filter(|(_, s)| s.len() < 5)
        .map(|(i, _)| i)
        .collect();
    for &i in &small {
        for &c in slabs.get(i).map(Vec::as_slice).unwrap_or(&[]) {
            slab.remove(&c);
            crack.insert(c, RING_T + 0.3);
        }
    }
    let keep: Vec<usize> = (0..slabs.len()).filter(|i| !small.contains(i)).collect();
    let renum: BTreeMap<usize, usize> = keep.iter().enumerate().map(|(n, &o)| (o, n)).collect();
    let slabs: Vec<Vec<(i32, i32)>> = keep.iter().filter_map(|&o| slabs.get(o).cloned()).collect();
    let slab: BTreeMap<(i32, i32), usize> = slab
        .iter()
        .filter_map(|(&c, o)| renum.get(o).map(|&n| (c, n)))
        .collect();

    let mut src = String::new();
    let _ = writeln!(
        src,
        "//! Baked by `examples/bake_winter_ordeal.rs` (seed {seed}); do not edit, run the script again."
    );
    let _ = writeln!(
        src,
        "//! The sky of `shadow/winter-ordeal`: its slabs and the cracks between them, in local pixels."
    );
    let _ = writeln!(src);
    let _ = writeln!(src, "/// The sky's middle (the eye) and half size.");
    let _ = writeln!(src, "pub const SKY: (i32, i32) = ({}, {});", SKY.0, SKY.1);
    let _ = writeln!(
        src,
        "pub const HALF: (f64, f64) = ({:?}, {:?});",
        HALF.0, HALF.1
    );
    let _ = writeln!(src);
    let _ = writeln!(
        src,
        "/// A slab of sky: its top left, its ring (0 the keystone with the eye, 1 inner, 2 middle, 3 outer), and its"
    );
    let _ = writeln!(
        src,
        "/// picture, a row per line: `.` not the slab, `a` to `{}` the shard each pixel smashes into.",
        (b'a' + SHARDS as u8 - 1) as char
    );
    let _ = writeln!(
        src,
        "pub struct Slab {{
    pub at: (i32, i32),
    pub ring: u8,
    pub rows: &'static [&'static str],
}}"
    );
    let _ = writeln!(src);
    let _ = writeln!(src, "pub const SLABS: &[Slab] = &[");
    for (i, cells) in slabs.iter().enumerate() {
        let lo = cells
            .iter()
            .fold((i32::MAX, i32::MAX), |a, c| (a.0.min(c.0), a.1.min(c.1)));
        let hi = cells
            .iter()
            .fold((i32::MIN, i32::MIN), |a, c| (a.0.max(c.0), a.1.max(c.1)));
        let n = cells.len() as f64;
        let centre = cells.iter().fold((0.0, 0.0), |a, c| {
            (a.0 + f64::from(c.0) / n, a.1 + f64::from(c.1) / n)
        });
        let is_key = key(libm::floor(centre.0) as i32, libm::floor(centre.1) as i32)
            && key(SKY.0, SKY.1)
            && cells.contains(&SKY);
        let d = libm::hypot(
            (centre.0 - f64::from(SKY.0)) / HALF.0,
            (centre.1 - f64::from(SKY.1)) / HALF.1,
        );
        let ring = if is_key {
            0
        } else if d < 0.45 {
            1
        } else if d < 0.75 {
            2
        } else {
            3
        };
        // its shards: nearest of a few points in it (more for a bigger slab)
        let count = (cells.len() / 60).clamp(2, SHARDS);
        let points: Vec<(i32, i32)> = (0..count)
            .filter_map(|j| {
                cells
                    .get((rand(&[seed, i as f64, j as f64, 5.0]) * n) as usize)
                    .copied()
            })
            .collect();
        let mut rows = Vec::new();
        for y in lo.1..=hi.1 {
            let mut row = String::new();
            for x in lo.0..=hi.0 {
                if slab.get(&(x, y)) == Some(&i) {
                    let near = points
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, p)| {
                            (p.0 - x) * (p.0 - x)
                                + (p.1 - y) * (p.1 - y)
                                + (rand(&[f64::from(x), f64::from(y), seed]) * 30.0) as i32
                        })
                        .map_or(0, |(j, _)| j);
                    row.push((b'a' + near as u8) as char);
                } else {
                    row.push('.');
                }
            }
            rows.push(row);
        }
        let _ = writeln!(
            src,
            "    Slab {{
        at: ({}, {}),
        ring: {ring},
        rows: &[",
            lo.0, lo.1
        );
        for r in rows {
            let _ = writeln!(src, "            \"{r}\",");
        }
        let _ = writeln!(
            src,
            "        ],
    }},"
        );
    }
    let _ = writeln!(src, "];");
    let _ = writeln!(src);
    let _ = writeln!(
        src,
        "/// A crack cell: where, the millisecond it appears, and the slabs on either side (255: none); it shows while one"
    );
    let _ = writeln!(src, "/// of them is still up.");
    let _ = writeln!(src, "pub const CRACKS: &[(i16, i16, u16, [u8; 2])] = &[");
    for (&(x, y), &t) in &crack {
        if !sky(x, y) {
            continue;
        }
        let mut side: Vec<u8> = Vec::new();
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
        ] {
            if let Some(&s) = slab.get(&(x + dx, y + dy))
                && !side.contains(&(s as u8))
                && side.len() < 2
            {
                side.push(s as u8);
            }
        }
        let a = side.first().copied().unwrap_or(255);
        let b = side.get(1).copied().unwrap_or(255);
        let _ = writeln!(src, "    ({x}, {y}, {}, [{a}, {b}]),", (t * 1000.0) as u16);
    }
    let _ = writeln!(src, "];");
    std::fs::write(out, src)?;
    eprintln!(
        "{} slabs, {} crack cells -> {out}",
        slabs.len(),
        crack.len()
    );

    // a preview: each slab its own shade, cracks white
    if let Some(path) = arg("--preview") {
        let (w, h) = (
            (bounds.2 - bounds.0 + 1) as usize,
            (bounds.3 - bounds.1 + 1) as usize,
        );
        let s = 4;
        let mut bytes = format!("P6 {} {} 255\n", w * s, h * s).into_bytes();
        for y in 0..h * s {
            for x in 0..w * s {
                let c = (bounds.0 + (x / s) as i32, bounds.1 + (y / s) as i32);
                let px = if crack.contains_key(&c) && sky(c.0, c.1) {
                    [235, 225, 255]
                } else if let Some(&i) = slab.get(&c) {
                    let k = (rand(&[i as f64, 7.0]) * 90.0) as u8;
                    [40 + k / 2, 30 + k / 3, 70 + k]
                } else {
                    [10, 8, 16]
                };
                bytes.extend_from_slice(&px);
            }
        }
        std::fs::write(path, bytes)?;
    }
    Ok(())
}
