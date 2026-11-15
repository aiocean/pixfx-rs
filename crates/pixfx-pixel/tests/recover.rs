// Grid recovery: a picture drawn big, at a whole or fractional scale, offset, blurred or noisy, comes back one pixel
// per drawn cell; a picture with no grid comes back unchanged.
use pixfx_pixel::Rgba;
use pixfx_pixel::RgbaImage;
use pixfx_pixel::oklab::{delta_e, lab};
use pixfx_pixel::recover::{fit_height, recover};

const COLOURS: [[u8; 4]; 6] = [
    [20, 16, 32, 255],
    [220, 40, 40, 255],
    [240, 200, 60, 255],
    [40, 120, 220, 255],
    [60, 180, 90, 255],
    [250, 240, 230, 255],
];

/// A 20x24 sprite: a 2-pixel transparent margin around cells coloured from `COLOURS` by a hash of their index.
fn sprite() -> RgbaImage {
    let (w, h) = (20, 24);
    let mut img = RgbaImage::new(w, h);
    for y in 2..h - 2 {
        for x in 2..w - 2 {
            let mut v = (y * w + x).wrapping_mul(2_654_435_761);
            v ^= v >> 15;
            img.set(x, y, COLOURS[(v % 6) as usize]);
        }
    }
    img
}

/// `img` drawn `k` times bigger, nearest neighbour: target pixel `x` shows source pixel `floor(x / k)`.
fn upscale(img: &RgbaImage, k: f64) -> RgbaImage {
    let (w, h) = (
        libm::floor(f64::from(img.w) * k) as u32,
        libm::floor(f64::from(img.h) * k) as u32,
    );
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (sx, sy) = (
                libm::floor(f64::from(x) / k) as u32,
                libm::floor(f64::from(y) / k) as u32,
            );
            if let Some(p) = img.get(sx, sy) {
                out.set(x, y, p);
            }
        }
    }
    out
}

/// The share of cells two pictures of the same size have equal; 0 when their sizes differ.
fn equal_share(a: &RgbaImage, b: &RgbaImage) -> f64 {
    if (a.w, a.h) != (b.w, b.h) {
        return 0.0;
    }
    let same = a
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.rgba.as_chunks::<4>().0.iter())
        .filter(|(p, q)| p == q)
        .count();
    same as f64 / f64::from(a.w * a.h)
}

#[test]
fn nearest_upscales_x2_to_x12_recover_exactly() {
    let s = sprite();
    for k in 2..=12 {
        let r = recover(&upscale(&s, f64::from(k)));
        assert_eq!(
            (r.picture.w, r.picture.h),
            (20, 24),
            "x{k}: pitch {}",
            r.pitch
        );
        assert!(
            r.picture == s,
            "x{k}: {} equal",
            equal_share(&r.picture, &s)
        );
        assert!(
            (r.pitch - f64::from(k)).abs() < 0.05,
            "x{k}: pitch {}",
            r.pitch
        );
    }
}

#[test]
fn fractional_scale_7_5_recovers_size() {
    let s = sprite();
    let r = recover(&upscale(&s, 7.5));
    assert_eq!((r.picture.w, r.picture.h), (20, 24), "pitch {}", r.pitch);
    let share = equal_share(&r.picture, &s);
    assert!(share >= 0.95, "{share}");
}

#[test]
fn offset_first_cell() {
    let s = sprite();
    let big = upscale(&s, 6.0);
    let img = big.crop(3, 2, big.w - 3, big.h - 2);
    let r = recover(&img);
    assert_eq!((r.picture.w, r.picture.h), (20, 24), "pitch {}", r.pitch);
    assert_eq!((r.grid.cols[1], r.grid.rows[1]), (3, 4));
    let share = equal_share(&r.picture, &s);
    assert!(share >= 0.95, "{share}");
}

/// `img` blurred by a 3x3 box (clipped at the borders), every channel.
fn blur(img: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(img.w, img.h);
    for y in 0..img.h {
        for x in 0..img.w {
            let mut sum = [0u32; 4];
            let mut n = 0;
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (sx, sy) = (i64::from(x) + dx, i64::from(y) + dy);
                    if sx < 0 || sy < 0 {
                        continue;
                    }
                    if let Some(p) = img.get(sx as u32, sy as u32) {
                        for c in 0..4 {
                            sum[c] += u32::from(p[c]);
                        }
                        n += 1;
                    }
                }
            }
            let avg = sum.map(|v| ((v + n / 2) / n) as u8);
            out.set(x, y, avg);
        }
    }
    out
}

/// `img` with each colour channel moved by up to 6 either way, from a fixed sequence.
fn noise(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    let mut v: u32 = 0x9e37_79b9;
    for (i, c) in out.rgba.iter_mut().enumerate() {
        if i % 4 == 3 {
            continue;
        }
        v ^= v << 13;
        v ^= v >> 17;
        v ^= v << 5;
        let d = (v % 13) as i32 - 6;
        *c = (i32::from(*c) + d).clamp(0, 255) as u8;
    }
    out
}

#[test]
fn bilinear_and_noise() {
    let s = sprite();
    let r = recover(&noise(&blur(&upscale(&s, 8.0))));
    assert_eq!((r.picture.w, r.picture.h), (20, 24), "pitch {}", r.pitch);
    // Noise moves colours a little: a cell counts as equal when it looks the same.
    let near = r
        .picture
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(s.rgba.as_chunks::<4>().0.iter())
        .filter(|(p, q)| {
            (p[3] >= 128) == (q[3] >= 128)
                && (q[3] < 128
                    || delta_e(
                        lab(Rgba(p[0], p[1], p[2], 255)),
                        lab(Rgba(q[0], q[1], q[2], 255)),
                    ) < 0.04)
        })
        .count();
    let share = near as f64 / f64::from(s.w * s.h);
    assert!(share >= 0.90, "{share}");
}

#[test]
fn no_grid_is_pitch_1() {
    let s = sprite();
    let r = recover(&s);
    assert_eq!(r.pitch, 1.0);
    assert!(r.picture == s);
}

const FIXTURES: [&str; 5] = [
    "draft-style",
    "draft-grid",
    "hero-base",
    "hero-style",
    "hero-grid",
];

fn fixture(name: &str) -> Result<(RgbaImage, (u32, u32)), String> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/");
    let png = std::fs::read(format!("{dir}{name}.png")).map_err(|e| e.to_string())?;
    let img = pixfx_export::decode_png_rgba(&png)?;
    let txt = std::fs::read_to_string(format!("{dir}{name}.txt")).map_err(|e| e.to_string())?;
    let figure = txt
        .lines()
        .find_map(|l| l.strip_prefix("figure "))
        .and_then(|f| f.split_once('x'))
        .and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)))
        .ok_or(format!("{name}: no figure line"))?;
    Ok((img, figure))
}

/// The box of the cells whose colour differs from the top-left cell's (the background), in cells.
fn figure_box(img: &RgbaImage) -> (u32, u32) {
    let Some(b) = img.get(0, 0) else {
        return (0, 0);
    };
    let back = lab(Rgba(b[0], b[1], b[2], 255));
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..img.h {
        for x in 0..img.w {
            let Some(p) = img.get(x, y) else { continue };
            if delta_e(lab(Rgba(p[0], p[1], p[2], 255)), back) > 0.05 {
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
    }
    if x0 > x1 {
        return (0, 0);
    }
    (x1 - x0 + 1, y1 - y0 + 1)
}

#[test]
fn fixtures_recover_their_counted_cells() -> Result<(), String> {
    for name in FIXTURES {
        let (img, (fw, fh)) = fixture(name)?;
        let r = recover(&img);
        let (w, h) = figure_box(&r.picture);
        eprintln!(
            "{name}: pitch {:.2}, picture {}x{}, figure {w}x{h} (counted {fw}x{fh}), confidence {:.3}",
            r.pitch, r.picture.w, r.picture.h, r.total
        );
        assert!(
            w.abs_diff(fw) <= 1 && h.abs_diff(fh) <= 1,
            "{name}: {w}x{h}, counted {fw}x{fh}"
        );
    }
    Ok(())
}

#[test]
fn deterministic() -> Result<(), String> {
    for name in FIXTURES {
        let (img, _) = fixture(name)?;
        assert!(recover(&img) == recover(&img), "{name}");
    }
    Ok(())
}

#[test]
fn fit_height_halves() {
    let s = sprite();
    assert!(fit_height(&upscale(&s, 2.0), 24) == s);
}

#[test]
fn a_recovered_picture_is_solid_or_empty() {
    // A 4x4-pixel grid of 3x3 cells drawn at alpha 253, as image models often return.
    let mut img = RgbaImage::new(12, 12);
    for y in 0..12 {
        for x in 0..12 {
            if (x / 3 + y / 3) % 2 == 0 {
                img.set(x, y, [200, 40, 40, 253]);
            }
        }
    }
    let found = recover(&img);
    assert!(
        found
            .picture
            .rgba
            .chunks(4)
            .all(|p| p.get(3) == Some(&255) || p.get(3) == Some(&0))
    );
    assert!(found.picture.rgba.chunks(4).any(|p| p.get(3) == Some(&255)));
}
