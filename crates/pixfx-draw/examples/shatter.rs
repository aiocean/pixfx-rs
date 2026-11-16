//! Shatters two shapes and plays it, so the settings can be judged by eye:
//! `cargo run --release -p pixfx-draw --example shatter -- [OUT_DIR]` (default `target/out/shatter`) writes
//! `frame-NNN.ppm`, one per tick. Left: a pane held by its rim (only pieces walled in by cracks alone fall).
//! Right: a wall held at its base (every piece cut off from the base falls). Each strike grows over a few ticks in
//! white; the pieces it cuts off flash, then fall and turn.

use std::collections::{BTreeMap, BTreeSet};

use pixfx_draw::shatter::{Cracks, Hold, Shatter, shatter};

const PANEL: (i32, i32) = (112, 150);
const SCALE: usize = 3;
/// Ticks a strike takes to grow, and ticks between strikes.
const GROW: usize = 4;
const GAP: usize = 3;

struct Shape {
    broken: Shatter,
    inside: fn(i32, i32) -> bool,
    bounds: (i32, i32, i32, i32),
    tint: (f64, f64, f64),
}

fn pane(x: i32, y: i32) -> bool {
    let (u, v) = (f64::from(x) / 46.0, f64::from(y) / 20.0);
    u * u + v * v < 1.0
}

fn wall(x: i32, y: i32) -> bool {
    x.abs() < 28 && y > -38
}

fn main() -> std::io::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/out/shatter".into());
    std::fs::create_dir_all(&dir)?;
    let shape = |inside: fn(i32, i32) -> bool, cracks: Cracks, tint| Shape {
        broken: shatter(cracks, inside),
        inside,
        bounds: cracks.bounds,
        tint,
    };
    let shapes = [
        shape(
            pane,
            Cracks {
                strikes: 14,
                bounds: (-48, -24, 47, 23),
                hold: Hold::Rim,
                seed: 4.0,
                ..Cracks::default()
            },
            (0.4, 0.45, 0.6),
        ),
        shape(
            wall,
            Cracks {
                strikes: 12,
                bounds: (-30, -40, 29, 39),
                hold: Hold::At(0, 39),
                inner: 0.5,
                reach: (8.0, 26.0),
                seed: 9.0,
                ..Cracks::default()
            },
            (0.55, 0.45, 0.4),
        ),
    ];
    let strikes = shapes
        .iter()
        .map(|s| s.broken.strikes.len())
        .max()
        .unwrap_or(0);
    let ticks = strikes * (GROW + GAP) + 30;
    let (w, h) = (PANEL.0 as usize * 2, PANEL.1 as usize);
    for tick in 0..ticks {
        let mut px = vec![[18u8, 16, 26]; w * h];
        for (i, s) in shapes.iter().enumerate() {
            let o = (PANEL.0 * i as i32 + PANEL.0 / 2, 6 - s.bounds.1);
            draw(&mut px, (w, h), s, tick, o);
        }
        let mut bytes = format!("P6 {} {} 255\n", w * SCALE, h * SCALE).into_bytes();
        for y in 0..h * SCALE {
            for x in 0..w * SCALE {
                let c = px
                    .get((y / SCALE) * w + x / SCALE)
                    .copied()
                    .unwrap_or([0, 0, 0]);
                bytes.extend_from_slice(&c);
            }
        }
        std::fs::write(format!("{dir}/frame-{tick:03}.ppm"), bytes)?;
    }
    Ok(())
}

/// The tick strike `k` starts growing on.
fn start(k: usize) -> usize {
    k * (GROW + GAP)
}

/// One shape at a tick: what still stands, the cracks grown so far, the pieces falling.
fn draw(px: &mut [[u8; 3]], (w, h): (usize, usize), s: &Shape, tick: usize, o: (i32, i32)) {
    let mut put = |x: i32, y: i32, c: [u8; 3]| {
        let (sx, sy) = (x + o.0, y + o.1);
        if sx >= 0
            && sy >= 0
            && (sx as usize) < w
            && (sy as usize) < h
            && let Some(p) = px.get_mut(sy as usize * w + sx as usize)
        {
            *p = c;
        }
    };
    let shade = |k: f64| {
        [
            (s.tint.0 * k * 255.0) as u8,
            (s.tint.1 * k * 255.0) as u8,
            (s.tint.2 * k * 255.0) as u8,
        ]
    };
    // the tick each cell cracks, and the piece that takes it
    let cracked: BTreeMap<(i32, i32), usize> = s
        .broken
        .strikes
        .iter()
        .enumerate()
        .flat_map(|(k, st)| st.cells.iter().map(move |&c| (c, k)))
        .collect();
    let taken: BTreeMap<(i32, i32), usize> = s
        .broken
        .pieces
        .iter()
        .enumerate()
        .flat_map(|(i, p)| p.cells.iter().map(move |&c| (c, i)))
        .collect();
    let (x0, y0, x1, y1) = s.bounds;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !(s.inside)(x, y) {
                continue;
            }
            let piece = taken.get(&(x, y)).and_then(|&i| s.broken.pieces.get(i));
            if piece.is_some_and(|p| tick >= start(p.strike) + GROW + 2) {
                continue; // falling: drawn below
            }
            let flash = piece.is_some_and(|p| tick >= start(p.strike) + GROW);
            let crack = cracked
                .get(&(x, y))
                .is_some_and(|&k| tick >= start(k) + GROW);
            let c = if flash {
                [235, 230, 255]
            } else if crack {
                [200, 190, 255]
            } else {
                shade(0.8)
            };
            put(x, y, c);
        }
    }
    for (i, piece) in s.broken.pieces.iter().enumerate() {
        let t0 = start(piece.strike) + GROW + 2;
        if tick < t0 {
            continue;
        }
        // it falls with gravity, drifts out from the middle and turns about its own middle; each screen pixel looks
        // back to the cell it shows, so a turned piece has no holes
        let k = (tick - t0) as f64;
        let turn = 0.05 * k * if i % 2 == 0 { 1.0 } else { -1.0 };
        let (c, sn) = (libm::cos(turn), libm::sin(turn));
        let (mx, my) = (
            piece.centre.0 + k * piece.centre.0 / 60.0,
            piece.centre.1 + 0.35 * k * k,
        );
        let tone = shade(0.5 + 0.12 * (i % 3) as f64);
        let cells: BTreeSet<(i32, i32)> = piece.cells.iter().copied().collect();
        let r = piece
            .cells
            .iter()
            .map(|&(x, y)| {
                libm::hypot(f64::from(x) - piece.centre.0, f64::from(y) - piece.centre.1)
            })
            .fold(0.0, f64::max)
            + 1.0;
        let (lx, ly) = (libm::floor(mx - r) as i32, libm::floor(my - r) as i32);
        for sy in ly..=ly + 2 * r as i32 + 1 {
            for sx in lx..=lx + 2 * r as i32 + 1 {
                let (dx, dy) = (f64::from(sx) + 0.5 - mx, f64::from(sy) + 0.5 - my);
                let cell = (
                    libm::floor(piece.centre.0 + dx * c + dy * sn + 0.5) as i32,
                    libm::floor(piece.centre.1 - dx * sn + dy * c + 0.5) as i32,
                );
                if cells.contains(&cell) {
                    let edge = cracked.contains_key(&cell);
                    put(sx, sy, if edge { [150, 140, 200] } else { tone });
                }
            }
        }
    }
    // the bolt growing: its nodes born so far, white
    for (k, st) in s.broken.strikes.iter().enumerate() {
        if tick < start(k) || tick >= start(k) + GROW {
            continue;
        }
        let shown =
            f64::from(st.tree.steps.max(1)) * ((tick - start(k)) as f64 + 1.0) / GROW as f64;
        for n in st.tree.nodes.iter().filter(|n| f64::from(n.born) <= shown) {
            put(
                libm::floor(n.x + 0.5) as i32,
                libm::floor(n.y + 0.5) as i32,
                [255, 255, 255],
            );
        }
    }
}
