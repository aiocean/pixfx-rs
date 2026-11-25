//! An effect's frames beside the sheet's cells: the loop that turns a reference into painter code. Draw, render,
//! put each frame next to the cell of the same moment at the same scale, measure, fix, again.
//!
//! A template cell holds the guide figure at the duel's ruler (`pixfx_draw::RULER`) in a 96-pixel cell,
//! engine pixels for engine pixels; so a cell covers [`ENGINE_CELL`] engine pixels whatever its own resolution, the
//! figure's feet [`FEET`] pixels down, centred across. A cell of `n` squares draws one square as `96 / n` engine
//! pixels ([`art_ruler`] gives the ruler in squares). [`effect_cell`] cuts that window round an effect frame's origin and
//! averages it down to the cell's size, so the two are compared pixel for pixel; [`board`] lays the pairs out.

use crate::RgbaImage;

/// Engine pixels a template cell covers.
pub const ENGINE_CELL: u32 = 96;
/// Engine pixels from the cell's top to where the feet stand (the origin).
pub const FEET: u32 = 88;

/// The duel's ruler as a template of `cell` squares a side draws it: every length times `cell / ENGINE_CELL`,
/// never under one square. At 96 a square is an engine pixel; at 64 the 40-pixel fighter is 27 squares tall.
pub fn art_ruler(cell: u32) -> pixfx_draw::Ruler {
    pixfx_draw::RULER.scaled(f64::from(cell) / f64::from(ENGINE_CELL))
}

/// The window of an effect frame a cell covers, `[x0, y0]` of an [`ENGINE_CELL`] square, from the frame's origin.
pub fn window(origin: (i32, i32)) -> [i32; 2] {
    [origin.0 - (ENGINE_CELL / 2) as i32, origin.1 - FEET as i32]
}

/// The cell window round `origin` of `frame` (an opaque picture, over black), averaged down to `w` x `h`. Outside
/// the frame is black.
pub fn effect_cell(frame: &RgbaImage, origin: (i32, i32), w: u32, h: u32) -> RgbaImage {
    let [x0, y0] = window(origin);
    let mut out = RgbaImage::new(w, h);
    let (fw, fh) = (
        f64::from(ENGINE_CELL) / f64::from(w.max(1)),
        f64::from(ENGINE_CELL) / f64::from(h.max(1)),
    );
    for y in 0..h {
        for x in 0..w {
            let (ax, bx) = (
                libm::floor(f64::from(x) * fw) as i32,
                libm::floor(f64::from(x + 1) * fw) as i32,
            );
            let (ay, by) = (
                libm::floor(f64::from(y) * fh) as i32,
                libm::floor(f64::from(y + 1) * fh) as i32,
            );
            let mut sum = [0u32; 3];
            let mut n = 0u32;
            for sy in ay..by.max(ay + 1) {
                for sx in ax..bx.max(ax + 1) {
                    let (px, py) = (x0 + sx, y0 + sy);
                    let p = if px < 0 || py < 0 {
                        None
                    } else {
                        frame.get(px as u32, py as u32)
                    };
                    let p = p.unwrap_or([0, 0, 0, 255]);
                    for c in 0..3 {
                        sum[c] += u32::from(p[c]) * u32::from(p[3]) / 255;
                    }
                    n += 1;
                }
            }
            let n = n.max(1);
            out.set(
                x,
                y,
                [
                    (sum[0] / n) as u8,
                    (sum[1] / n) as u8,
                    (sum[2] / n) as u8,
                    255,
                ],
            );
        }
    }
    out
}

/// One row of the board: a moment's cells from the sheet and the effect's frames at the same moments.
pub struct Pair {
    pub sheet: Vec<RgbaImage>,
    pub effect: Vec<RgbaImage>,
}

/// The board: for each pair, a line of sheet cells over a line of effect frames, every picture drawn `side` pixels
/// square (nearest neighbour), `gap` dark pixels between them.
pub fn board(pairs: &[Pair], side: u32, gap: u32) -> RgbaImage {
    let cols = pairs
        .iter()
        .map(|p| p.sheet.len().max(p.effect.len()))
        .max()
        .unwrap_or(0) as u32;
    let lines = pairs.len() as u32 * 2;
    let w = cols * (side + gap) + gap;
    let h = lines * (side + gap) + gap;
    let mut out = RgbaImage::new(w, h);
    for p in out.rgba.as_chunks_mut::<4>().0 {
        *p = [20, 20, 26, 255];
    }
    for (k, pair) in pairs.iter().enumerate() {
        for (line, pics) in [&pair.sheet, &pair.effect].into_iter().enumerate() {
            let y0 = gap + (k as u32 * 2 + line as u32) * (side + gap);
            for (c, pic) in pics.iter().enumerate() {
                let x0 = gap + c as u32 * (side + gap);
                for y in 0..side {
                    for x in 0..side {
                        let sx = x * pic.w.max(1) / side;
                        let sy = y * pic.h.max(1) / side;
                        out.set(x0 + x, y0 + y, pic.get(sx, sy).unwrap_or([0, 0, 0, 255]));
                    }
                }
            }
        }
    }
    out
}
