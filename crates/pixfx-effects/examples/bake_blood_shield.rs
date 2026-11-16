//! Bakes the veins of `shadow/blood-shield` once: grown with `pixfx_draw::growth` and written as data into
//! `src/shadow/blood_shield_veins.rs`, which the effect reads as it is; nothing here runs in a game.
//!
//!   cargo run --release -p pixfx-effects --no-default-features --features shadow-blood-shield \
//!     --example bake_blood_shield -- [--seed N]
//!
//! Six main veins are laid along set paths (`MAIN_VEINS`, made to wander by `jag`), side branches grow off them by
//! space colonization (`colonize_from`) walled inside the rim, forks are turned to Murray's angles and stubs pruned,
//! and the tree is stroked as tubes tapering from trunk to twig, lit from the upper left. Each pixel keeps its tone (an
//! index into the effect's palette) and when it grew (0 the trunks' feet, 1 the last twig). The dome's size, outline,
//! light and tones below must be the effect's; its test checks the map is the size of its dome.

use std::fmt::Write as _;

use pixfx_draw::growth::{Colonize, Tree, colonize_from, jag};

/// The effect's dome: half width, height and the power of its outline (`blood_shield.rs` RX, H, N).
const RX: f64 = 43.0;
const H: f64 = 64.0;
const N: f64 = 2.5;
/// Toward the light, upper left (`blood_shield.rs` LIGHT).
const LIGHT: (f64, f64) = (-0.6, -0.8);
/// The palette's tones the veins use (`blood_shield.rs` BLOOD): deep, a twig's lit edge, bright.
const DEEP: usize = 4;
const BRANCH_LIT: usize = 5;
const BRIGHT: usize = 8;

/// The vein map's size: the whole dome in engine pixels, x from `-RX`, y from `-H` (the crown) to 0 (the base).
const MAP_W: usize = 2 * RX as usize + 1;
const MAP_ROWS: usize = H as usize + 1;
/// No vein at this pixel.
const NO_VEIN: u8 = u8::MAX;

/// The main veins as MAIN draws them, as fractions of the dome (u across from -1 to 1, v up from 0 to 1): two up the
/// walls, one up the middle and one right of it that forks; a path that starts on another is a fork off it.
const MAIN_VEINS: [&[(f64, f64)]; 6] = [
    &[
        (-0.8, 0.0),
        (-0.8, 0.3),
        (-0.72, 0.56),
        (-0.56, 0.76),
        (-0.34, 0.9),
        (-0.12, 0.95),
    ],
    &[
        (-0.46, 0.0),
        (-0.4, 0.24),
        (-0.5, 0.44),
        (-0.56, 0.62),
        (-0.46, 0.8),
    ],
    &[
        (-0.06, 0.0),
        (0.0, 0.2),
        (-0.12, 0.42),
        (-0.06, 0.62),
        (0.06, 0.8),
        (0.0, 0.95),
    ],
    &[
        (0.42, 0.0),
        (0.46, 0.28),
        (0.4, 0.5),
        (0.24, 0.72),
        (0.12, 0.9),
    ],
    &[(0.4, 0.5), (0.52, 0.66), (0.5, 0.84), (0.36, 0.93)],
    &[
        (0.8, 0.0),
        (0.8, 0.3),
        (0.72, 0.56),
        (0.58, 0.76),
        (0.66, 0.84),
    ],
];

/// How far out a point is, 1 on the outline: the dome is the superellipse |x/RX|^N + |y/H|^N = 1 above the base.
fn reach(x: f64, y: f64) -> f64 {
    let sx = libm::fabs(x) / RX;
    let sy = (-y).max(0.0) / H;
    libm::pow(libm::pow(sx, N) + libm::pow(sy, N), 1.0 / N)
}

/// The veins as MAIN draws them, grown rather than copied: the main veins laid along [`MAIN_VEINS`] (made to wander
/// by `jag`), then side branches grown off them by space colonization (`colonize_from`), walled inside the rim,
/// then painted as strokes tapering from trunk to twig. Each pixel holds its tone (an index into the effect's palette: a dark core, a lighter edge) and when it grew
/// (0 the trunks' feet, 1 the last twig), so INIT grows them and the heartbeat runs out along them.
fn grow_veins(seed: f64) -> Vec<(u8, f32, f32)> {
    let mut tree = Tree::default();
    for (i, path) in MAIN_VEINS.iter().enumerate() {
        let points: Vec<(f64, f64)> = path.iter().map(|&(u, v)| (u * RX, -v * H)).collect();
        tree.path(&jag(&points, 3, 0.12, seed * 31.0 + i as f64), 1.5);
    }
    let laid = tree.nodes.len();
    let mut tree = colonize_from(
        tree,
        Colonize {
            attractors: 200,
            step: 2.0,
            influence: 16.0,
            kill: 2.5,
            steps: 44,
            bias: (0.0, -1.0),
            jitter: 0.3,
            branch_chance: 0.08,
            max_turn: 0.3,
            fork_angle: (0.35, 0.75),
            bounds: (-RX, -H, RX, 0.0),
            seed,
        },
        |x, y| y < -1.0 && reach(x, y) < 0.9,
    );
    tree.murray(1.0, 0.6, laid, |x, y| y < -1.0 && reach(x, y) < 0.9);
    tree.prune(5);
    let radii = tree.radii(0.6, 3.2, 0.06);
    let last = f64::from(tree.steps.max(1));
    let mut map = vec![(NO_VEIN, 1.0f32, 1.0f32); MAP_W * MAP_ROWS];
    tree.stroke(&radii, |dab| {
        let (col, row) = (dab.x + RX as i32, dab.y + H as i32);
        if col < 0 || row < 0 || col as usize >= MAP_W || row as usize >= MAP_ROWS {
            return;
        }
        // a tube lit from the upper left: its lit side bright, a dark core, its far side in shadow; a fat trunk gets a
        // glint on its lit side, a twig only a dark line with a lit edge
        let facing = LIGHT.0 * dab.normal.0 + LIGHT.1 * dab.normal.1;
        let s = facing * dab.depth;
        let tone = if dab.radius >= 1.2 {
            if s > 0.5 && dab.radius >= 2.2 {
                BRIGHT
            } else {
                libm::round(2.0 + 4.0 * s).clamp(0.0, 7.0) as usize
            }
        } else if s > 0.2 {
            BRANCH_LIT
        } else {
            DEEP
        } as u8;
        let born = tree
            .nodes
            .get(dab.node)
            .map_or(1.0, |n| f64::from(n.born) / last) as f32;
        let depth = dab.depth as f32;
        if let Some(px) = map.get_mut(row as usize * MAP_W + col as usize) {
            // the stroke whose centre is nearer wins the tone; the earlier growth wins the time
            let tone = if depth < px.2 { tone } else { px.0 };
            *px = (tone, px.1.min(born), px.2.min(depth));
        }
    });
    map
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let seed: f64 = args
        .iter()
        .position(|a| a == "--seed")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.0);
    let out = "crates/pixfx-effects/src/shadow/blood_shield_veins.rs";
    let map = grow_veins(seed);
    let mut veins = String::new();
    let mut count = 0;
    for (i, &(tone, born, _)) in map.iter().enumerate() {
        if tone == NO_VEIN {
            continue;
        }
        let (col, row) = (i % MAP_W, i / MAP_W);
        // f32's Debug form reads back as the same bits
        let _ = writeln!(veins, "    ({col}, {row}, {tone}, {born:?}),");
        count += 1;
    }
    let text = format!(
        "//! Baked by `examples/bake_blood_shield.rs` (seed {seed}); do not edit, run the script again.\n\
         //! The veins of `shadow/blood-shield`, grown for its dome ({RX} by {H}).\n\
         \n\
         /// The vein map: the whole dome in engine pixels, x from `-RX`, y from `-H` (the crown) to 0 (the base).\n\
         pub const MAP_W: usize = {MAP_W};\n\
         pub const MAP_ROWS: usize = {MAP_ROWS};\n\
         \n\
         /// Every vein pixel: its column and row on the map, its tone (an index into the palette) and when it grew (0 the\n\
         /// trunks' feet, 1 the last twig).\n\
         pub static VEINS: [(u8, u8, u8, f32); {count}] = [\n{veins}];\n"
    );
    std::fs::write(out, text)?;
    println!("{out}: {count} vein pixels (seed {seed})");
    Ok(())
}
