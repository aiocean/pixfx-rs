//! Spike: the alchemist painted from shapes (polygons, bars, lines, a few hand-placed dots), no picture data.
//! Two poses from a handful of pose numbers.
//!
//!   cargo run -p pixfx-pixel --example spike_alchemist_code

use pixfx_pixel::RgbaImage;
use std::path::Path;

type Col = [u8; 4];

const fn hex(v: u32) -> Col {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255]
}

// 16 colours, all from the reference drawing's legend
const INK: Col = hex(0x1d1023); // outline, hair
const HAIR: Col = hex(0x322837); // hair sheen
const WHITE: Col = hex(0xfcfcfd);
const SILK: Col = hex(0xf2effa);
const PALE: Col = hex(0xe6dffc);
const SHADE: Col = hex(0xc6bae5);
const SKIN: Col = hex(0xfbe2cb);
const BLUSH: Col = hex(0xf3cbb3);
const BLUE: Col = hex(0x9dbaed);
const RIBBON: Col = hex(0xacc9fa);
const SASH: Col = hex(0x5062a7);
const TEAL: Col = hex(0x5d91a6);
const STEEL: Col = hex(0xd1d2d7);
const GREY: Col = hex(0xcfcdd2);
const SHEEN: Col = hex(0xe1e0e3);

/// The few numbers a pose is made of.
struct Pose {
    /// right hand (sword hand) position
    hand: (f64, f64),
    /// sword direction in radians, 0 = forward, positive = down
    sword: f64,
    /// upper body shift forward in pixels
    lean: f64,
    /// how far the hair tip and ribbons swing back
    sway: f64,
}

const IDLE: Pose = Pose {
    hand: (31.0, 31.0),
    sword: 0.58,
    lean: 0.0,
    sway: 0.0,
};

const GUARD: Pose = Pose {
    hand: (29.0, 22.0),
    sword: -1.05,
    lean: 1.0,
    sway: 2.0,
};

/// Paints on a 64x64 sprite in the drawing's own 47x56 frame, feet on the bottom row.
struct Pen {
    img: RgbaImage,
}

impl Pen {
    const OX: f64 = 8.0;
    const OY: f64 = 8.0;

    fn dot(&mut self, x: f64, y: f64, c: Col) {
        let (x, y) = (libm::round(x + Self::OX), libm::round(y + Self::OY));
        if x >= 0.0 && y >= 0.0 {
            self.img.set(x as u32, y as u32, c);
        }
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: Col) {
        for j in 0..h as i32 {
            for i in 0..w as i32 {
                self.dot(x + f64::from(i), y + f64::from(j), c);
            }
        }
    }

    /// A `w` x `w` block stamped along (x0, y0) to (x1, y1).
    fn bar(&mut self, (x0, y0): (f64, f64), (x1, y1): (f64, f64), w: f64, c: Col) {
        let n = libm::fmax(libm::fmax((x1 - x0).abs(), (y1 - y0).abs()), 1.0);
        for i in 0..=n as i32 {
            let k = f64::from(i) / n;
            self.rect(
                libm::round(x0 + (x1 - x0) * k),
                libm::round(y0 + (y1 - y0) * k),
                w,
                w,
                c,
            );
        }
    }

    fn line(&mut self, a: (f64, f64), b: (f64, f64), c: Col) {
        self.bar(a, b, 1.0, c);
    }

    /// Any simple polygon, filled by pixel centres (even-odd).
    fn poly(&mut self, p: &[(f64, f64)], c: Col) {
        let lo = p.iter().fold(f64::INFINITY, |m, q| libm::fmin(m, q.1));
        let hi = p.iter().fold(f64::NEG_INFINITY, |m, q| libm::fmax(m, q.1));
        let mut y = libm::floor(lo);
        while y <= hi {
            let mid = y + 0.5;
            let mut xs: Vec<f64> = Vec::new();
            for i in 0..p.len() {
                let (a, b) = (p[i], p[(i + 1) % p.len()]);
                if (a.1 <= mid) != (b.1 <= mid) {
                    xs.push(a.0 + (b.0 - a.0) * (mid - a.1) / (b.1 - a.1));
                }
            }
            xs.sort_by(f64::total_cmp);
            for s in xs.chunks(2) {
                if let [l, r] = s {
                    let mut x = libm::ceil(l - 0.5);
                    while x + 0.5 < *r {
                        self.dot(x, y, c);
                        x += 1.0;
                    }
                }
            }
            y += 1.0;
        }
    }

    /// Ink around everything painted: every empty pixel touching paint becomes outline.
    fn outline(&mut self) {
        let (w, h) = (self.img.w, self.img.h);
        let solid = |img: &RgbaImage, x: i64, y: i64| {
            x >= 0 && y >= 0 && img.get(x as u32, y as u32).is_some_and(|p| p[3] > 0)
        };
        let src = self.img.clone();
        for y in 0..h as i64 {
            for x in 0..w as i64 {
                if !solid(&src, x, y)
                    && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                        .iter()
                        .any(|(dx, dy)| solid(&src, x + dx, y + dy))
                {
                    self.img.set(x as u32, y as u32, INK);
                }
            }
        }
    }
}

fn alchemist(pose: &Pose) -> RgbaImage {
    let mut p = Pen {
        img: RgbaImage::new(64, 64),
    };
    let l = pose.lean;
    let s = pose.sway;

    // hair down the back, a sheen strand through it
    p.poly(
        &[
            (15.0 + l, 3.0),
            (25.0 + l, 3.0),
            (27.0 + l, 9.0),
            (18.0 + l, 14.0),
            (16.0 + l, 19.0),
            (14.0, 30.0),
            (11.0 - s, 30.0),
            (9.0 - s, 28.0),
            (9.0 - s, 21.0),
            (13.0 + l, 15.0),
            (15.0 + l, 9.0),
            (16.0 + l, 4.0),
        ],
        INK,
    );
    p.bar((15.0 + l, 8.0), (14.0 + l, 12.0), 3.0, HAIR);
    p.bar((13.0 + l, 13.0), (10.0, 19.0), 2.0, HAIR);
    p.line((9.0 - s * 0.5, 20.0), (9.0 - s, 25.0), HAIR);

    // skirt: white, lilac on the far side, the under-robe stripe down the front
    let skirt = [
        (14.0, 29.0),
        (27.0, 29.0),
        (28.0, 40.0),
        (30.0, 48.0),
        (29.0, 53.0),
        (8.0, 53.0),
        (6.0, 49.0),
        (7.0, 47.0),
        (12.0, 42.0),
        (12.0, 35.0),
    ];
    p.poly(&skirt, WHITE);
    p.poly(
        &[
            (14.0, 33.0),
            (19.0, 33.0),
            (18.0, 44.0),
            (17.0, 49.0),
            (16.0, 53.0),
            (8.0, 53.0),
            (6.0, 49.0),
            (7.0, 47.0),
            (12.0, 42.0),
        ],
        SHADE,
    );
    p.poly(
        &[(14.0, 36.0), (19.0, 33.0), (19.0, 42.0), (13.0, 43.0)],
        PALE,
    );
    p.poly(
        &[(22.0, 46.0), (26.0, 46.0), (26.0, 52.0), (21.0, 52.0)],
        SHADE,
    );
    p.line((23.0, 29.0), (23.0, 43.0), BLUE);
    p.bar((23.0, 44.0), (23.0, 46.0), 2.0, BLUE);
    // inner robe edge running down-left into the ribbons
    p.bar((21.0, 24.0), (13.0, 36.0), 2.0, BLUE);
    // hem and slippers
    p.rect(7.0, 49.0, 5.0, 1.0, SILK);
    p.rect(8.0, 50.0, 3.0, 1.0, SILK);
    p.rect(11.0, 53.0, 3.0, 1.0, WHITE);
    p.rect(26.0, 53.0, 5.0, 1.0, GREY);
    p.rect(26.0, 52.0, 5.0, 1.0, INK);
    p.rect(7.0, 51.0, 6.0, 1.0, INK);

    // torso
    p.poly(
        &[
            (18.0 + l, 14.0),
            (25.0 + l, 14.0),
            (27.0 + l, 18.0),
            (27.0, 29.0),
            (17.0, 29.0),
            (15.0 + l, 22.0),
            (16.0 + l, 17.0),
        ],
        WHITE,
    );
    p.poly(
        &[
            (16.0 + l, 18.0),
            (19.0 + l, 18.0),
            (20.0, 29.0),
            (16.0, 29.0),
        ],
        PALE,
    );
    // collar: lapel ink, blue facing
    p.line((20.0 + l, 14.0), (19.0 + l, 19.0), INK);
    p.line((21.0 + l, 15.0), (22.0 + l, 19.0), BLUE);
    p.dot(23.0 + l, 15.0, SHEEN);
    // sash
    p.rect(19.0 + l, 20.0, 2.0, 3.0, INK);
    p.rect(21.0 + l, 20.0, 4.0, 1.0, SASH);
    p.rect(21.0 + l, 21.0, 4.0, 1.0, WHITE);
    p.dot(21.0 + l, 21.0, RIBBON);
    p.rect(21.0 + l, 22.0, 4.0, 1.0, BLUE);
    p.line((25.0 + l, 20.0), (25.0 + l, 22.0), INK);
    // inner robe below the sash, pale, its blue edge running down
    p.poly(
        &[(19.0, 23.0), (23.0, 23.0), (22.0, 29.0), (18.0, 29.0)],
        PALE,
    );
    p.line((23.0, 23.0), (23.0, 29.0), BLUE);
    p.bar((21.0, 24.0), (19.0, 28.0), 2.0, BLUE);

    // far sleeve, wide, hanging, hand peeking out below
    p.poly(
        &[
            (14.0 + l, 20.0),
            (18.0 + l, 20.0),
            (18.0, 29.0),
            (15.0, 30.0),
            (10.0, 29.0),
            (10.0, 25.0),
        ],
        WHITE,
    );
    p.poly(
        &[(10.0, 25.0), (14.0, 24.0), (14.0, 26.0), (10.0, 27.0)],
        SILK,
    );
    p.poly(
        &[(15.0, 25.0), (18.0, 24.0), (18.0, 29.0), (15.0, 30.0)],
        SHADE,
    );
    p.line((18.0 + l, 20.0), (18.0, 29.0), INK);
    p.rect(11.0, 31.0, 2.0, 3.0, SKIN);
    p.dot(12.0, 31.0, BLUSH);
    p.line((10.0, 30.0), (13.0, 30.0), INK);

    // sword arm: a sleeve from the shoulder to the cuff, the hand bare
    let sh = (26.0 + l, 19.0);
    let (hx, hy) = pose.hand;
    let cuff = (hx - 4.0, hy + 1.0);
    p.poly(
        &[
            sh,
            (sh.0 + 2.0, sh.1),
            (sh.0 + 3.0, sh.1 + 7.0),
            (cuff.0 + 4.0, cuff.1 + 2.0),
            (cuff.0, cuff.1 + 2.0),
        ],
        WHITE,
    );
    p.bar((sh.0 + 1.0, sh.1 + 2.0), (cuff.0 + 1.0, cuff.1), 1.0, SHADE);
    // seam between body and sword-side sleeve
    p.line((25.0 + l, 20.0), (26.0, 30.0), INK);
    p.rect(hx, hy, 2.0, 2.0, SKIN);
    p.dot(hx + 1.0, hy + 1.0, BLUSH);

    // head: bun with ornament and pin, fringe, face
    p.rect(17.0 + l, 1.0, 4.0, 2.0, INK);
    p.dot(18.0 + l, 1.0, SILK);
    p.dot(19.0 + l, 1.0, WHITE);
    p.dot(18.0 + l, 2.0, GREY);
    p.line((14.0 + l, 3.0), (17.0 + l, 3.0), TEAL);
    p.poly(
        &[
            (21.0 + l, 7.0),
            (25.0 + l, 7.0),
            (25.0 + l, 11.0),
            (23.0 + l, 13.0),
            (19.0 + l, 13.0),
            (19.0 + l, 9.0),
        ],
        SKIN,
    );
    p.rect(17.0 + l, 4.0, 8.0, 3.0, HAIR);
    p.dot(17.0 + l, 4.0, SHEEN);
    p.dot(17.0 + l, 6.0, SHEEN);
    p.line((20.0 + l, 7.0), (20.0 + l, 11.0), INK);
    p.dot(22.0 + l, 9.0, INK);
    p.dot(21.0 + l, 8.0, BLUSH);
    p.rect(21.0 + l, 11.0, 2.0, 2.0, BLUSH);
    p.rect(21.0 + l, 13.0, 2.0, 1.0, SKIN);
    p.rect(22.0 + l, 14.0, 2.0, 1.0, SKIN);

    // sword: steel blade, light edge, hilt in the hand
    let (dx, dy) = (libm::cos(pose.sword), libm::sin(pose.sword));
    let tip = (hx + 1.0 + dx * 15.0, hy + 2.0 + dy * 15.0);
    p.bar((hx + 2.0, hy + 2.0), tip, 2.0, STEEL);
    p.line((hx + 2.0, hy + 2.0), tip, WHITE);
    p.dot(hx + 2.0, hy + 3.0, hex(0xa0bfd1));

    p.outline();

    // ribbons trailing back from the sash, light enough to go without ink
    p.line((9.0, 35.0), (2.0 - s, 37.0), RIBBON);
    p.line((5.0, 37.0), (1.0 - s, 38.0), RIBBON);
    p.bar((0.0 - s, 39.0), (0.0 - s, 40.0), 2.0, RIBBON);
    p.line((10.0, 36.0), (9.0, 37.0), RIBBON);
    p.bar((8.0, 38.0), (4.0 - s, 42.0), 2.0, RIBBON);
    p.line((8.0, 40.0), (6.0 - s, 42.0), RIBBON);
    p.dot(25.0, 55.0, INK);
    p.img
}

fn main() -> Result<(), String> {
    let out = Path::new("target/art/lialchemist/spike-code");
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let (idle, guard) = (alchemist(&IDLE), alchemist(&GUARD));
    let mut sheet = RgbaImage::new(128, 64);
    for (i, img) in [&idle, &guard].into_iter().enumerate() {
        for y in 0..64 {
            for x in 0..64 {
                if let Some(px) = img.get(x, y) {
                    sheet.set(x + 64 * i as u32, y, px);
                }
            }
        }
    }
    for (name, img) in [("idle", &idle), ("guard", &guard), ("sheet", &sheet)] {
        pixfx_export::write_png_rgba(&out.join(format!("{name}.png")), img.w, img.h, &img.rgba)
            .map_err(|e| format!("{name}: {e:?}"))?;
    }
    let src = include_str!("spike_alchemist_code.rs");
    let body = src
        .split("fn alchemist(")
        .nth(1)
        .and_then(|r| r.split("\nfn main(").next())
        .map_or(0, |b| b.lines().count());
    println!(
        "painter fn alchemist: {body} lines; whole file {} lines",
        src.lines().count()
    );
    Ok(())
}
