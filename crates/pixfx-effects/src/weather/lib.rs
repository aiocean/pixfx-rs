//! Helpers shared by the weather loops. Weather fills its whole canvas edge to edge, so: `sized` gives an
//! instance the canvas its `width` and `height` options ask for; `clip` keeps every pixel and every bit of
//! light inside the canvas; `pass` and `rows` are per-pixel shaders over the canvas; `trips` rains particles
//! that fall a whole number of times a loop, somewhere new each time. All of it is a pure function of t.
//!
//! Port of pixelfx `src/fx/weather/_lib.js`: the helpers snow and fireflies use (`clip`, `pass`, `trips`,
//! `rows`). pixelfx writes `pass` and `rows` straight into the frame buffers for an unrotated painter; these go
//! through the painter's pixel writes, which land on the same pixels and values (pixelfx's own path for a
//! rotated painter).

use pixfx_core::color::Rgba;
use pixfx_core::math::{js_max, js_min, js_round, rand};
use pixfx_core::{Canvas, Meta, NumOpt, Options, Painter};

/// The longest side a weather canvas takes from its options: room for any screen at 2 screen pixels per
/// pixel, and a frame small enough to allocate.
pub const MAX_SIDE: u32 = 2048;

/// The canvas a weather instance draws on: the `width` and `height` options held to 1 to [`MAX_SIDE`]
/// ([`size_options`]) and rounded to whole pixels (one unset or not a number keeps the effect's own), with
/// the origin mid-width and as far above the bottom as on the effect's own canvas, so the ground stays at the
/// foot and a taller canvas adds sky. With neither option it is the effect's own canvas, so the default render
/// is pixelfx's.
pub fn sized(meta: &Meta, opts: &Options) -> Canvas {
    let side = |o: &NumOpt, own: u32| {
        let v = opts.num(o);
        if v.is_nan() { own } else { js_round(v) as u32 }
    };
    let (wo, ho) = size_options(meta.size);
    let (w, h) = (side(&wo, meta.size.0), side(&ho, meta.size.1));
    let below = meta.size.1 as i32 - meta.origin.1;
    Canvas {
        size: (w, h),
        origin: ((w / 2) as i32, h as i32 - below),
    }
}

/// The `width` and `height` options of a weather effect whose own canvas is `size`: [`sized`] reads them, the
/// effect lists them in its meta.
pub const fn size_options(size: (u32, u32)) -> (NumOpt, NumOpt) {
    (
        NumOpt::new("width", size.0 as f64).within(1.0, MAX_SIDE as f64),
        NumOpt::new("height", size.1 as f64).within(1.0, MAX_SIDE as f64),
    )
}

/// `n` things spread over `span` pixels as the effect spreads them over its own `own` pixels: as dense on a
/// wider canvas, and exactly `n` at the effect's own size.
pub fn count(n: usize, span: f64, own: f64) -> usize {
    js_round(n as f64 * span / own) as usize
}

/// The canvas in local coordinates: `x0 <= x < x1`, `y0 <= y < y1`, `w` pixels wide.
struct Bounds {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    w: usize,
}

fn bounds(c: Canvas) -> Bounds {
    let (w, h) = (f64::from(c.size.0), f64::from(c.size.1));
    let (ox, oy) = (f64::from(c.origin.0), f64::from(c.origin.1));
    Bounds {
        x0: -ox,
        y0: -oy,
        x1: w - ox,
        y1: h - oy,
        w: c.size.0 as usize,
    }
}

/// Draw only inside the instance's canvas from here on: colour, blends and light outside it are dropped
/// (pixelfx `clip`, the painter's canvas clip with the canvas around the origin as its box). Draw glows with
/// [`glow`].
pub fn clip(p: &mut Painter, c: Canvas) {
    let b = bounds(c);
    p.clip_box(b.x0, b.y0, b.x1, b.y1);
}

/// The painter's `glow` with its box cut to the canvas first, so big soft lights stay cheap (pixelfx `clip`'s
/// `glow`): it walks local pixel cells and measures from their centres, which differs from `Painter::glow`
/// (screen pixels) at a fractional or rotated placement.
pub fn glow(p: &mut Painter, cv: Canvas, x: f64, y: f64, r: f64, c: Rgba, k: f64) {
    if r < 1.0 || k <= 0.004 {
        return;
    }
    let cv = bounds(cv);
    let (x0, x1) = (
        js_max(cv.x0, libm::floor(x - r)),
        js_min(cv.x1 - 1.0, libm::ceil(x + r)),
    );
    let (y0, y1) = (
        js_max(cv.y0, libm::floor(y - r)),
        js_min(cv.y1 - 1.0, libm::ceil(y + r)),
    );
    let mut py = y0;
    while py <= y1 {
        let dy = (py + 0.5 - y) / r;
        let mut px = x0;
        while px <= x1 {
            let dx = (px + 0.5 - x) / r;
            let d = dx * dx + dy * dy;
            if d < 1.0 {
                let f = 1.0 - libm::sqrt(d);
                p.light(px, py, c, f * f * k);
            }
            px += 1.0;
        }
        py += 1.0;
    }
}

/// What a [`pass`] function sets for its pixel: colour `c` (`Rgba::NONE` for none) with alpha `a` (dithered, or
/// truly blended when `b`), and light colour `l` (`Rgba::NONE`: the pixel's colour) with amount `k`.
#[derive(Clone, Copy, Debug)]
pub struct Out {
    pub c: Rgba,
    pub a: f64,
    pub b: bool,
    pub l: Rgba,
    pub k: f64,
}

impl Out {
    const NONE: Out = Out {
        c: Rgba::NONE,
        a: 1.0,
        b: false,
        l: Rgba::NONE,
        k: 0.0,
    };
}

/// The rows y0..y1 (local, cut to the canvas) as the loops below walk them: first and last row.
fn row_span(cv: &Bounds, y0: f64, y1: f64) -> (f64, f64) {
    (
        js_max(cv.y0, libm::floor(y0)),
        js_min(cv.y1 - 1.0, libm::ceil(y1)),
    )
}

fn put(p: &mut Painter, x: f64, y: f64, o: &Out) {
    if o.c.pack() != 0 {
        if o.b {
            p.blend(x, y, o.c, o.a);
        } else {
            p.dot(x, y, o.c, o.a);
        }
    }
    if o.k > 0.0 {
        p.light(x, y, if o.l.pack() != 0 { o.l } else { o.c }, o.k);
    }
}

/// A per-pixel shader over the canvas rows y0..y1 (local, cut to the canvas): `f(x, y, out)` sets `out` for the
/// pixel centred at (x, y); `out` starts each pixel as colour none, alpha 1, dithered, no light.
pub fn pass(p: &mut Painter, c: Canvas, y0: f64, y1: f64, mut f: impl FnMut(f64, f64, &mut Out)) {
    let cv = bounds(c);
    let (first, last) = row_span(&cv, y0, y1);
    let mut y = first;
    while y <= last {
        let mut x = cv.x0;
        while x < cv.x1 {
            let mut o = Out::NONE;
            f(x + 0.5, y + 0.5, &mut o);
            put(p, x, y, &o);
            x += 1.0;
        }
        y += 1.0;
    }
}

/// One canvas pixel of a [`rows`] row. Alpha and light amount are `f32`, as pixelfx's `Float32Array`s.
#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub c: Rgba,
    pub a: f32,
    pub b: bool,
    pub l: Rgba,
    pub k: f32,
}

impl Cell {
    const NONE: Cell = Cell {
        c: Rgba::NONE,
        a: 1.0,
        b: false,
        l: Rgba::NONE,
        k: 0.0,
    };
}

/// The fastest whole-sky shader: `f(y, row)` fills one canvas row at a time (`y` the row's pixel centre), one
/// [`Cell`] per column (`x - box.x0`). Each row starts as colour none, alpha 1, dithered, no light.
pub fn rows(p: &mut Painter, c: Canvas, y0: f64, y1: f64, mut f: impl FnMut(f64, &mut [Cell])) {
    let cv = bounds(c);
    let (first, last) = row_span(&cv, y0, y1);
    // pixelfx's `R.l` is never reset between rows while this one is; equal today because nothing sets `k > 0`
    let mut row = vec![Cell::NONE; cv.w];
    let mut y = first;
    while y <= last {
        row.fill(Cell::NONE);
        f(y + 0.5, &mut row);
        for (i, cell) in row.iter().enumerate() {
            let o = Out {
                c: cell.c,
                a: f64::from(cell.a),
                b: cell.b,
                l: cell.l,
                k: f64::from(cell.k),
            };
            put(p, cv.x0 + i as f64, y, &o);
        }
        y += 1.0;
    }
}

/// Falling particles for a loop of `d` seconds: `n` slots, each falling `m` times a loop (`m` whole, from `m0`
/// to `m1`), each fall somewhere new. `f(u, k, m, i)`: `u` 0..1 through the current fall, `k` its number
/// (`wrap(k, m)` names it: use `rand(&[seed, i, wrap(k, m), ..])` for its own randoms, the same every loop),
/// `m`, the slot `i`. A fall lasts `d / m` s.
pub fn trips(
    t: f64,
    d: f64,
    n: usize,
    seed: f64,
    m0: f64,
    m1: f64,
    mut f: impl FnMut(f64, f64, f64, f64),
) {
    for i in 0..n {
        let i = i as f64;
        let m = m0 + libm::floor(rand(&[seed, i, 72.0]) * (m1 - m0 + 1.0));
        let s = (t / d) * m + rand(&[seed, i, 71.0]);
        let k = libm::floor(s);
        f(s - k, k, m, i);
    }
}
