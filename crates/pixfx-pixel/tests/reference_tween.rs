// In-betweens made by moving the drawn pixels: a block that moves is halfway at half time with no holes, a shape
// that grows grows steadily, unlike colours swap without mixing, alike ones mix, and in-betweens stay inside a row.
use pixfx_pixel::reference::matrix::{Frame, Matrix, Px};
use pixfx_pixel::reference::tween::{between, motion, name, tweens};

const W: usize = 40;
const H: usize = 30;

/// A frame with a textured block (two colours in a fixed scatter, so it has edges to follow and no repeat to
/// confuse a move) at `(x0, y0)`, `bw` by `bh`.
fn block(x0: usize, y0: usize, bw: usize, bh: usize, a: [u8; 3], b: [u8; 3]) -> Vec<Px> {
    let mut px = vec![Px::Empty; W * H];
    for y in y0..y0 + bh {
        for x in x0..x0 + bw {
            let v = ((x - x0) * 7919 + (y - y0) * 104_729).wrapping_mul(2_654_435_761) >> 13;
            let rgb = if v.is_multiple_of(3) { a } else { b };
            px[y * W + x] = Px::Colour {
                rgb,
                ink: u8::from(rgb == b),
            };
        }
    }
    px
}

fn lit(px: &[Px]) -> Vec<(usize, usize)> {
    px.iter()
        .enumerate()
        .filter(|(_, p)| **p != Px::Empty)
        .map(|(i, _)| (i % W, i / W))
        .collect()
}

/// Empty cells with at least 6 of their 8 neighbours lit.
fn holes(px: &[Px]) -> usize {
    (1..H - 1)
        .flat_map(|y| (1..W - 1).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            px[y * W + x] == Px::Empty
                && (0..9)
                    .filter(|i| *i != 4 && px[(y + i / 3 - 1) * W + x + i % 3 - 1] != Px::Empty)
                    .count()
                    >= 6
        })
        .count()
}

const GOLD: [u8; 3] = [230, 170, 40];
const DARK: [u8; 3] = [120, 70, 20];

#[test]
fn a_moving_block_is_halfway_at_half_time_with_no_holes() -> Result<(), String> {
    let (a, b) = (
        block(8, 8, 12, 12, GOLD, DARK),
        block(14, 10, 12, 12, GOLD, DARK),
    );
    let m = motion(W, H, &a, &b);
    assert!(m.pairs >= 140, "most of the 144 pixels pair: {}", m.pairs);
    let mid = between(&a, &b, &m, 0.5);
    let cells = lit(&mid);
    let (minx, miny) = cells
        .iter()
        .fold((W, H), |(mx, my), &(x, y)| (mx.min(x), my.min(y)));
    assert_eq!(
        (minx, miny),
        (11, 9),
        "the block's corner moved half of (6, 2)"
    );
    assert!(
        cells.len().abs_diff(144) <= 6,
        "the block keeps its size: {}",
        cells.len()
    );
    assert_eq!(holes(&mid), 0);
    assert!(
        mid.iter().all(|p| matches!(
            p,
            Px::Empty
                | Px::Colour {
                    rgb: GOLD | DARK,
                    ..
                }
        )),
        "no colour made up"
    );
    Ok(())
}

#[test]
fn a_growing_shape_grows_steadily() -> Result<(), String> {
    let (a, b) = (
        block(10, 4, 16, 8, GOLD, DARK),
        block(10, 4, 16, 20, GOLD, DARK),
    );
    let m = motion(W, H, &a, &b);
    let sizes: Vec<usize> = (1..8)
        .map(|k| lit(&between(&a, &b, &m, f64::from(k) / 8.0)).len())
        .collect();
    assert!(
        sizes.windows(2).all(|s| s[0] <= s[1]),
        "never shrinks: {sizes:?}"
    );
    assert!(
        sizes[0] < 16 * 12 && sizes[6] > 16 * 16,
        "from near A to near B: {sizes:?}"
    );
    for k in 1..8 {
        assert_eq!(
            holes(&between(&a, &b, &m, f64::from(k) / 8.0)),
            0,
            "t={k}/8"
        );
    }
    Ok(())
}

#[test]
fn unlike_colours_swap_without_mixing_and_alike_ones_mix() -> Result<(), String> {
    const RED: [u8; 3] = [220, 30, 30];
    const BLUE: [u8; 3] = [30, 60, 230];
    let (a, b) = (
        block(8, 8, 20, 14, RED, RED),
        block(8, 8, 20, 14, BLUE, BLUE),
    );
    let m = motion(W, H, &a, &b);
    let blue = |t: f64| -> Result<usize, String> {
        let f = between(&a, &b, &m, t);
        let mut n = 0;
        for p in lit(&f).iter().map(|&(x, y)| f[y * W + x]) {
            match p {
                Px::Colour { rgb: BLUE, .. } => n += 1,
                Px::Colour { rgb: RED, .. } => {}
                other => return Err(format!("t={t}: a mixed or lost pixel {other:?}")),
            }
        }
        Ok(n)
    };
    let counts = (1..10)
        .map(|k| blue(f64::from(k) / 10.0))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        counts[0], 0,
        "nothing swaps before the first swap time: {counts:?}"
    );
    assert_eq!(
        counts[8], 280,
        "everything swapped after the last: {counts:?}"
    );
    assert!(
        counts.windows(2).all(|c| c[0] <= c[1]),
        "a swapped pixel stays swapped: {counts:?}"
    );
    assert!(
        counts.iter().filter(|&&n| n > 0 && n < 280).count() >= 2,
        "swaps spread over time: {counts:?}"
    );

    let (a, b) = (
        block(8, 8, 20, 14, [200, 100, 40], [200, 100, 40]),
        block(8, 8, 20, 14, [240, 120, 40], [240, 120, 40]),
    );
    let m = motion(W, H, &a, &b);
    let mid = between(&a, &b, &m, 0.5);
    assert!(lit(&mid).iter().all(|&(x, y)| matches!(
        mid[y * W + x],
        Px::Colour {
            rgb: [220, 110, 40],
            ..
        }
    )));
    Ok(())
}

#[test]
fn in_betweens_stay_inside_a_row() {
    let f = |name: &str, x: usize| Frame {
        name: name.into(),
        px: block(x, 8, 10, 10, GOLD, DARK),
        pivots: Vec::new(),
    };
    let m = Matrix {
        w: W as u32,
        h: H as u32,
        colours: vec![GOLD, DARK],
        lights: vec![],
        frames: vec![
            f("init-1", 6),
            f("init-2", 9),
            f("idle-1", 12),
            f("idle-2", 14),
        ],
        colour_err: 0.0,
        light_err: 0.0,
    };
    let names: Vec<String> = tweens(&m, 3).into_iter().map(|f| f.name).collect();
    let want: Vec<String> = [("init-1", "init-2"), ("idle-1", "idle-2")]
        .iter()
        .flat_map(|(a, b)| (1..=3).map(|k| name(a, b, k)))
        .collect();
    assert_eq!(names, want);
    assert!(tweens(&m, 0).is_empty());
}

#[test]
fn the_same_frames_give_the_same_in_betweens() {
    let (a, b) = (
        block(8, 8, 12, 12, GOLD, DARK),
        block(13, 11, 12, 12, GOLD, DARK),
    );
    let one = between(&a, &b, &motion(W, H, &a, &b), 0.4);
    assert_eq!(one, between(&a, &b, &motion(W, H, &a, &b), 0.4));
}
