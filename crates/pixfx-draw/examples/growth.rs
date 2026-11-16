//! Draws the growth algorithms side by side, so their settings can be judged by eye:
//! `cargo run --release -p pixfx-draw --example growth -- [OUT.ppm]` (default `target/out/growth.ppm`).
//! Panels: dbm at eta 1, 3 and 6 grown up from the bottom; dbm aimed at a point (lightning); dbm from the middle out
//! (cracks); colonize in a dome (veins); a vein laid three quarters round a ring (a `Path`), branching off it inward. Width follows weight; brightness follows the step a node was born on.

use pixfx_draw::growth::{Colonize, Dbm, Tree, colonize, colonize_from, dbm};
use pixfx_draw::path::Path;

const S: i32 = 96;

fn main() -> std::io::Result<()> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/out/growth.ppm".into());
    let all = |_: i32, _: i32| true;
    let square = (-S / 2, -S / 2, S / 2 - 1, S / 2 - 1);
    // the top row is the target, the walls insulate: it grows up from the bottom, eta setting the look
    let sky: Vec<(i32, i32)> = (-S / 2..S / 2).map(|x| (x, -S / 2)).collect();
    let up = |eta: f64| {
        dbm(
            &[(0, S / 2 - 2)],
            &sky,
            Dbm {
                eta,
                edge: None,
                cells: 260,
                reach: 1.5,
                bounds: square,
                seed: 3.0,
                ..Dbm::default()
            },
            all,
        )
    };
    let panels: Vec<Tree> = vec![
        // coral, cracks, lightning by eta alone, out from the bottom to a grounded edge
        up(1.0),
        up(3.0),
        up(6.0),
        // lightning: insulated walls, so only the target draws it
        dbm(
            &[(-10, -S / 2 + 2)],
            &[(14, S / 2 - 3)],
            Dbm {
                eta: 3.0,
                edge: None,
                cells: 3000,
                reach: 1.5,
                bounds: square,
                seed: 8.0,
                ..Dbm::default()
            },
            all,
        ),
        // cracks: out from the middle, on the square grid
        dbm(
            &[(0, 0)],
            &[],
            Dbm {
                eta: 2.5,
                neighbours: 4,
                cells: 260,
                bounds: square,
                seed: 2.0,
                ..Dbm::default()
            },
            all,
        ),
        // veins: colonizing a dome from its base, leaning up
        colonize(
            &[(-30.0, 46.0), (-12.0, 46.0), (12.0, 46.0), (30.0, 46.0)],
            Colonize {
                attractors: 500,
                step: 2.0,
                influence: 10.0,
                kill: 2.5,
                steps: 160,
                bias: (0.0, -0.3),
                jitter: 0.2,
                branch_chance: 1.0,
                max_turn: core::f64::consts::PI,
                fork_angle: (0.0, core::f64::consts::PI),
                bounds: (-46.0, -46.0, 46.0, 46.0),
                seed: 6.0,
            },
            |x, y| {
                (x / 44.0) * (x / 44.0) + ((y - 46.0) / 88.0) * ((y - 46.0) / 88.0) < 1.0
                    && y < 45.0
            },
        ),
        // a vein laid round a ring from its left, clockwise three quarters of the way (a Path), then branches
        // colonizing the band inside it
        {
            let mut t = Tree::default();
            let way = Path::begin((-38.0, 0.0)).turn((0.0, 0.0), 1.5 * core::f64::consts::PI);
            t.path(&way.points(1.5), 1.5);
            colonize_from(
                t,
                Colonize {
                    attractors: 300,
                    step: 2.0,
                    influence: 9.0,
                    kill: 2.5,
                    steps: 80,
                    bias: (0.0, 0.0),
                    jitter: 0.2,
                    branch_chance: 1.0,
                    max_turn: core::f64::consts::PI,
                    fork_angle: (0.0, core::f64::consts::PI),
                    bounds: (-46.0, -46.0, 46.0, 46.0),
                    seed: 4.0,
                },
                |x, y| (18.0 * 18.0..40.0 * 40.0).contains(&(x * x + y * y)),
            )
        },
    ];
    let (w, h) = ((S + 4) as usize * panels.len(), S as usize);
    let mut px = vec![0u8; w * h * 3];
    for (i, t) in panels.iter().enumerate() {
        let ox = (S + 4) * i as i32 + S / 2;
        let last = f64::from(t.steps.max(1));
        let radii = t.radii(0.5, 2.6, 0.08);
        t.stroke(&radii, |dab| {
            let (cx, cy) = (dab.x + ox, dab.y + S / 2);
            if cx < 0 || cy < 0 || cx as usize >= w || cy as usize >= h {
                return;
            }
            let born = t
                .nodes
                .get(dab.node)
                .map_or(0.0, |n| f64::from(n.born) / last);
            // a tube lit from the upper left
            let facing = -0.6 * dab.normal.0 - 0.8 * dab.normal.1;
            let lit = ((1.0 - 0.55 * born) * (0.75 + 0.35 * facing * dab.depth)).clamp(0.0, 1.0);
            let at = (cy as usize * w + cx as usize) * 3;
            if let Some(p) = px.get_mut(at..at + 3) {
                let c = [
                    (255.0 * lit) as u8,
                    (200.0 * lit * lit) as u8,
                    (110.0 * lit) as u8,
                ];
                if c[0] > p[0] {
                    p.copy_from_slice(&c);
                }
            }
        });
    }
    let mut file = format!("P6\n{w} {h}\n255\n").into_bytes();
    file.extend_from_slice(&px);
    if let Some(dir) = std::path::Path::new(&out).parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&out, file)?;
    println!("{out}");
    Ok(())
}
