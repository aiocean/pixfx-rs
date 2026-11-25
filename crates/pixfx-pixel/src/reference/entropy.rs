//! How much a frame changes into the next, measured as entropy: the agent's gauge of what a step of its own drawing
//! does next to the same step of the reference.
//!
//! Each pixel becomes a symbol: empty, or one of [`LEVELS`] brightness levels (over black). The change from frame
//! `a` to frame `b` is the conditional entropy H(b | a) of the symbols at the same place, divided by its largest
//! value: 0 when `b` follows from `a` pixel for pixel (nothing changed, or every pixel of one level became the same
//! other level), 1 when knowing `a` tells nothing about `b`. A shape moving, growing or flashing raises it in
//! proportion to the pixels it touches.
//!
//! Read it as steps along a row: a reference row whose steps are about even (0.20 0.24 0.29 0.25) and an effect
//! whose steps bunch up (0.20 0.33 0.16 0.06) are timed differently, even when each frame looks right: the effect
//! finishes early. A step far below the reference's means too little happens there; far above, too much at once.

use crate::RgbaImage;

/// Brightness levels a drawn pixel falls into; with "empty" that makes `LEVELS + 1` symbols.
pub const LEVELS: u8 = 4;
/// Luma (0..255, over black) below which a pixel is empty.
pub const EMPTY: f32 = 16.0;

/// Each pixel of `img` as a symbol: 0 empty, else 1 to [`LEVELS`] from dark to bright, its colour taken over black.
pub fn symbols(img: &RgbaImage) -> Vec<u8> {
    img.rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            let a = f32::from(p[3]) / 255.0;
            let l =
                (0.299 * f32::from(p[0]) + 0.587 * f32::from(p[1]) + 0.114 * f32::from(p[2])) * a;
            if l < EMPTY {
                0
            } else {
                1 + ((l / 256.0 * f32::from(LEVELS)) as u8).min(LEVELS - 1)
            }
        })
        .collect()
}

/// The change from symbols `a` to symbols `b` (0..1), see the module. Fails when they differ in length.
pub fn change(a: &[u8], b: &[u8]) -> Result<f64, String> {
    if a.len() != b.len() {
        return Err(format!(
            "frames of {} and {} pixels cannot be compared",
            a.len(),
            b.len()
        ));
    }
    const N: usize = LEVELS as usize + 1;
    let mut joint = [[0usize; N]; N];
    let mut from = [0usize; N];
    for (&x, &y) in a.iter().zip(b) {
        let (x, y) = (usize::from(x).min(N - 1), usize::from(y).min(N - 1));
        joint[x][y] += 1;
        from[x] += 1;
    }
    let total = a.len().max(1) as f64;
    let mut h = 0.0;
    for (x, row) in joint.iter().enumerate() {
        for &c in row.iter().filter(|&&c| c > 0) {
            let p = c as f64 / total;
            h -= p * libm::log2(c as f64 / from[x] as f64);
        }
    }
    Ok(h / libm::log2(N as f64))
}

/// The change of every step along `frames`: one less than there are frames.
pub fn steps(frames: &[RgbaImage]) -> Result<Vec<f64>, String> {
    let s: Vec<Vec<u8>> = frames.iter().map(symbols).collect();
    s.windows(2).map(|w| change(&w[0], &w[1])).collect()
}
