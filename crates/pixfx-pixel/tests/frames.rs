// Grid recovery for a sheet's frames: frames drawn from one low-resolution picture at a non-integer scale (model
// pixels 2 and 3 sheet pixels wide, mixed) come back at the picture's size, every frame the same size, a sparse
// frame too; a frame of noise shows no grid.
use pixfx_pixel::RgbaImage;
use pixfx_pixel::frames::{grid_view, recover_frames};
use pixfx_pixel::recover::collapse;

const COLOURS: [[u8; 4]; 5] = [
    [220, 40, 40, 255],
    [240, 200, 60, 255],
    [40, 120, 220, 255],
    [60, 180, 90, 255],
    [250, 240, 230, 255],
];
const W: u32 = 40;
const H: u32 = 44;

fn hash(i: u32, seed: u32) -> u32 {
    let mut v = i.wrapping_add(seed).wrapping_mul(2_654_435_761);
    v ^= v >> 15;
    v.wrapping_mul(2_246_822_519) ^ (v >> 13)
}

/// A `W`x`H` picture on transparent: cells painted from `COLOURS` where `keep` of 100 say so.
fn picture(seed: u32, keep: u32) -> RgbaImage {
    let mut img = RgbaImage::new(W, H);
    for y in 2..H - 2 {
        for x in 2..W - 2 {
            let v = hash(y * W + x, seed);
            if v % 100 < keep {
                img.set(x, y, COLOURS[((v >> 8) % 5) as usize]);
            }
        }
    }
    img
}

/// `img` drawn `kx` by `ky` times bigger, nearest neighbour.
fn upscale(img: &RgbaImage, kx: f64, ky: f64) -> RgbaImage {
    let (w, h) = (
        libm::floor(f64::from(img.w) * kx) as u32,
        libm::floor(f64::from(img.h) * ky) as u32,
    );
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (
                libm::floor(f64::from(x) / kx) as u32,
                libm::floor(f64::from(y) / ky) as u32,
            );
            out.set(x, y, img.get(sx, sy).unwrap_or([0; 4]));
        }
    }
    out
}

/// Pixels where two pictures differ, painted against empty.
fn differ(a: &RgbaImage, b: &RgbaImage) -> usize {
    (0..a.h)
        .flat_map(|y| (0..a.w).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let (p, q) = (a.get(x, y).unwrap_or([0; 4]), b.get(x, y).unwrap_or([0; 4]));
            (p[3] >= 128) != (q[3] >= 128) || (p[3] >= 128 && p != q)
        })
        .count()
}

#[test]
fn frames_share_one_grid_at_the_picture_size() -> Result<(), String> {
    let (kx, ky) = (2.7, 2.45);
    let sources = [picture(1, 90), picture(2, 90), picture(3, 6)];
    let frames: Vec<RgbaImage> = sources.iter().map(|p| upscale(p, kx, ky)).collect();
    let g = recover_frames(&frames)?;
    assert_eq!(g.size, (W, H), "pitch {:?}", g.pitch);
    assert!(
        (g.pitch.0 - kx).abs() < 0.05 && (g.pitch.1 - ky).abs() < 0.05,
        "pitch {:?}",
        g.pitch
    );
    for (i, (f, src)) in g.frames.iter().zip(&sources).enumerate() {
        assert_eq!(f.grid.cols.len() as u32, W + 1, "frame {i}");
        assert_eq!(f.grid.rows.len() as u32, H + 1, "frame {i}");
        let (back, _) = collapse(&frames[i], &f.grid);
        let wrong = differ(&back, src);
        assert!(
            wrong * 100 <= (W * H) as usize,
            "frame {i}: {wrong} pixels wrong"
        );
        assert!(
            f.strength.0 > 0.5 && f.strength.1 > 0.5,
            "frame {i}: strength {:?}",
            f.strength
        );
    }
    Ok(())
}

#[test]
fn noise_shows_no_grid() -> Result<(), String> {
    let dense = upscale(&picture(1, 90), 2.7, 2.45);
    let mut noise = RgbaImage::new(dense.w, dense.h);
    for y in 0..noise.h {
        for x in 0..noise.w {
            noise.set(x, y, COLOURS[(hash(y * noise.w + x, 9) % 5) as usize]);
        }
    }
    let g = recover_frames(&[dense, noise])?;
    let (good, bad) = (&g.frames[0], &g.frames[1]);
    assert!(
        bad.strength.0 < 0.2 && bad.strength.1 < 0.2,
        "noise strength {:?}",
        bad.strength
    );
    assert!(
        good.strength.0 > bad.strength.0 + 0.3,
        "{:?} vs {:?}",
        good.strength,
        bad.strength
    );
    Ok(())
}

#[test]
fn frames_of_different_sizes_are_refused() {
    let a = RgbaImage::new(30, 30);
    let b = RgbaImage::new(31, 30);
    assert!(recover_frames(&[a, b]).is_err());
}

#[test]
fn grid_view_draws_the_cuts() -> Result<(), String> {
    let img = upscale(&picture(1, 90), 3.0, 3.0);
    let g = recover_frames(std::slice::from_ref(&img))?;
    let line = [0, 255, 255, 255];
    let v = grid_view(&img, &g.frames[0].grid, [0, 0, 30, 30], 4, line);
    assert_eq!((v.w, v.h), (120, 120));
    // a cut at a multiple of 3 shows as a line down the view
    let col = g.frames[0]
        .grid
        .cols
        .iter()
        .copied()
        .find(|&c| c > 0 && c < 30);
    let c = col.ok_or("no cut inside the view")?;
    assert!((0..120).all(|y| v.get(c * 4, y) == Some(line)));
    Ok(())
}

/// `img` without its first `dx` columns and `dy` rows.
fn shifted(img: &RgbaImage, dx: u32, dy: u32) -> RgbaImage {
    let mut out = RgbaImage::new(img.w - dx, img.h - dy);
    for y in 0..out.h {
        for x in 0..out.w {
            out.set(x, y, img.get(x + dx, y + dy).unwrap_or([0; 4]));
        }
    }
    out
}

#[test]
fn a_cell_cut_mid_pixel_keeps_the_count() -> Result<(), String> {
    // the first model pixel is cut short; whether a sliver of it counts is a matter of a pixel in the empty border,
    // so the count is right within one
    for (dx, dy) in [(1, 1), (2, 0), (0, 2)] {
        let frames: Vec<RgbaImage> = [picture(1, 90), picture(2, 90)]
            .iter()
            .map(|p| shifted(&upscale(p, 2.7, 2.45), dx, dy))
            .collect();
        let g = recover_frames(&frames)?;
        assert!(
            g.size.0.abs_diff(W) <= 1 && g.size.1.abs_diff(H) <= 1,
            "shift {dx},{dy}: {:?} for {W}x{H}, pitch {:?}",
            g.size,
            g.pitch
        );
    }
    Ok(())
}

#[test]
fn busiest_finds_the_drawn_part() {
    // a 30x30 checker in the bottom right of an empty 120x120 picture
    let mut img = RgbaImage::new(120, 120);
    for y in 85..115 {
        for x in 85..115 {
            img.set(x, y, COLOURS[((x + y) % 2) as usize * 4]);
        }
    }
    let [x0, y0, x1, y1] = pixfx_pixel::frames::busiest(&img, 32);
    assert_eq!((x1 - x0, y1 - y0), (32, 32));
    assert!(
        x0 >= 72 && y0 >= 72,
        "the square lands on the checker: {x0},{y0}"
    );
}
