// Colours: reduce keeps a few of the picture's own colours, the same ones whatever the pixel order; snap puts a
// picture on a given palette, by nearest colour or by stretched lightness.
use pixfx_pixel::oklab::{delta_e, lab};
use pixfx_pixel::reduce::{SnapMode, reduce, snap};
use pixfx_pixel::{Rgba, RgbaImage, colors};

/// A `w` x `h` picture whose pixels, row by row, are `px`.
fn picture(w: u32, h: u32, px: &[[u8; 4]]) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for (i, &p) in px.iter().enumerate() {
        img.set(i as u32 % w, i as u32 / w, p);
    }
    img
}

/// 4000 distinct colours across the RGB cube, one a pixel.
fn thousands() -> Vec<[u8; 4]> {
    (0..4000u32)
        .map(|i| {
            let s = |v: u32| (v & 15) as u8 * 17;
            [s(i), s(i >> 4), s(i >> 8), 255]
        })
        .collect()
}

fn sorted(mut c: Vec<Rgba>) -> Vec<(u8, u8, u8)> {
    c.sort_by_key(|c| (c.0, c.1, c.2));
    c.into_iter().map(|c| (c.0, c.1, c.2)).collect()
}

#[test]
fn thousands_of_colours_come_down_to_at_most_32_of_their_own() {
    let img = picture(80, 50, &thousands());
    assert_eq!(colors(&img).len(), 4000);
    let out = reduce(&img, 32, 0.03);
    let kept = colors(&out);
    assert!(
        !kept.is_empty() && kept.len() <= 32,
        "{} colours",
        kept.len()
    );
    let had = colors(&img);
    assert!(
        kept.iter().all(|c| had.contains(c)),
        "a colour the picture never had"
    );
}

#[test]
fn the_pixel_order_does_not_change_the_colours() {
    let px = thousands();
    let n = px.len();
    // A fixed permutation: 7919 is prime and does not divide 4000.
    let shuffled: Vec<[u8; 4]> = (0..n)
        .filter_map(|i| px.get(i * 7919 % n).copied())
        .collect();
    let a = reduce(&picture(80, 50, &px), 32, 0.03);
    let b = reduce(&picture(80, 50, &shuffled), 32, 0.03);
    assert_eq!(sorted(colors(&a)), sorted(colors(&b)));
}

#[test]
fn a_grey_ramp_reduced_keeps_dark_to_light_order() {
    let ramp: Vec<[u8; 4]> = (0..10u8).map(|i| [i * 28, i * 28, i * 28, 255]).collect();
    let out = reduce(&picture(10, 1, &ramp), 4, 0.03);
    assert_eq!(colors(&out).len(), 4);
    let ls: Vec<f64> = (0..10)
        .filter_map(|x| out.get(x, 0))
        .map(|[r, g, b, _]| lab(Rgba(r, g, b, 255)).l)
        .collect();
    assert_eq!(ls.len(), 10);
    assert!(ls.windows(2).all(|w| w[0] <= w[1]), "{ls:?}");
}

#[test]
fn snap_nearest_takes_the_closest_palette_colour() {
    let palette = [
        Rgba(0, 0, 0, 255),
        Rgba(255, 255, 255, 255),
        Rgba(200, 40, 40, 255),
        Rgba(40, 60, 200, 255),
    ];
    let px = thousands();
    let img = picture(80, 50, &px);
    let out = snap(&img, &palette, SnapMode::Nearest);
    for (i, &[r, g, b, _]) in px.iter().enumerate() {
        let c = lab(Rgba(r, g, b, 255));
        let mut best = (f64::INFINITY, 0);
        for (j, &p) in palette.iter().enumerate() {
            let d = delta_e(c, lab(p));
            if d < best.0 {
                best = (d, j);
            }
        }
        let want = palette.get(best.1).map(|p| [p.0, p.1, p.2, 255]);
        assert_eq!(out.get(i as u32 % 80, i as u32 / 80), want, "pixel {i}");
    }
}

#[test]
fn snap_lightness_spreads_a_dark_picture_over_the_whole_ramp() {
    let ramp: Vec<Rgba> = (0..5u8)
        .map(|i| Rgba(i * 63, i * 63, i * 63, 255))
        .collect();
    let dark: Vec<[u8; 4]> = (0..10u8)
        .map(|i| [10 + i * 5, 10 + i * 5, 10 + i * 5, 255])
        .collect();
    let img = picture(10, 1, &dark);
    let out = colors(&snap(&img, &ramp, SnapMode::Lightness));
    assert!(out.contains(&ramp[0]) && out.contains(&ramp[4]), "{out:?}");
    // Nearest would put every dark grey on black or the first step.
    assert!(!colors(&snap(&img, &ramp, SnapMode::Nearest)).contains(&ramp[4]));
}

#[test]
fn empty_pixels_stay_empty() {
    let px = [
        [0, 0, 0, 0],
        [10, 20, 30, 40],
        [200, 30, 30, 255],
        [30, 200, 30, 255],
    ];
    let img = picture(4, 1, &px);
    let palette = [Rgba(255, 0, 0, 255), Rgba(0, 255, 0, 255)];
    for out in [
        reduce(&img, 1, 0.03),
        snap(&img, &palette, SnapMode::Nearest),
        snap(&img, &palette, SnapMode::Lightness),
    ] {
        assert_eq!(out.get(0, 0), Some([0, 0, 0, 0]));
        assert_eq!(out.get(1, 0), Some([10, 20, 30, 40]));
        assert_eq!(out.get(2, 0).map(|p| p[3]), Some(255));
    }
}

#[test]
fn a_big_picture_with_thousands_of_colours_reduces() {
    let px = thousands();
    let all: Vec<[u8; 4]> = (0..1024 * 1024)
        .filter_map(|i| px.get(i * 31 % px.len()).copied())
        .collect();
    let out = reduce(&picture(1024, 1024, &all), 32, 0.03);
    let n = colors(&out).len();
    assert!(n > 0 && n <= 32, "{n} colours");
}
