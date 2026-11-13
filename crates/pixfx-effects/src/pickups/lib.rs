//! Helpers for the pickups: a scratch layer to build outlined sprites in, floor shadows and glints.
//! Not an effect (pixelfx keeps these in `_lib.js`).
//!
//! Port of pixelfx `src/fx/pickups/_lib.js`: the functions coin-spin uses (`Layer`, `coin3d`, `glint`,
//! `blobShadow`). The JS `Layer` is [`CoinLayer`] here so it does not shadow [`pixfx_core::Layer`].

use pixfx_core::Painter;
use pixfx_core::color::{Rgba, WHITE, scale};
use pixfx_core::math::{js_max, js_min, js_round, js_sign};

/// A scratch layer: draw a sprite into it, give it a dark outline, then stamp it onto the frame. Lets a
/// procedural shape get a hand-made 1 px outline. Only what coin-spin needs of pixelfx's `Layer`: `begin`,
/// `fill`, a single-colour 4-neighbour `outline` and `draw`.
pub struct CoinLayer {
    w: usize,
    h: usize,
    ox: i32,
    oy: i32,
    color: Vec<u32>,
    tmp: Vec<u32>,
}

impl CoinLayer {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            ox: 0,
            oy: 0,
            color: vec![0; w * h],
            tmp: vec![0; w * h],
        }
    }

    /// Clear it and put its top-left at local (x0, y0).
    pub fn begin(&mut self, x0: f64, y0: f64) {
        self.ox = libm::floor(x0) as i32;
        self.oy = libm::floor(y0) as i32;
        self.color.fill(0);
    }

    fn set(&mut self, x: i32, y: i32, c: Rgba) {
        let (i, j) = (x - self.ox, y - self.oy);
        if i >= 0
            && j >= 0
            && (i as usize) < self.w
            && (j as usize) < self.h
            && let Some(k) = self.color.get_mut(j as usize * self.w + i as usize)
        {
            *k = c.pack();
        }
    }

    /// Fill each pixel from `f(x, y)` at the pixel's centre (x + 0.5, y + 0.5): a colour or `Rgba::NONE`.
    pub fn fill(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        mut f: impl FnMut(f64, f64) -> Rgba,
    ) {
        let mut y = libm::floor(y0) as i32;
        while f64::from(y) <= y1 {
            let mut x = libm::floor(x0) as i32;
            while f64::from(x) <= x1 {
                let c = f(f64::from(x) + 0.5, f64::from(y) + 0.5);
                if c.pack() != 0 {
                    self.set(x, y, c);
                }
                x += 1;
            }
            y += 1;
        }
    }

    /// A 1 px outline of colour `c` around everything drawn (up, down, left and right neighbours).
    pub fn outline(&mut self, c: Rgba) {
        let (w, h) = (self.w, self.h);
        self.tmp.fill(0);
        let at = |col: &[u32], i: usize, j: usize| col.get(j * w + i).is_some_and(|&v| v != 0);
        for j in 0..h {
            for i in 0..w {
                if at(&self.color, i, j) {
                    continue;
                }
                let n = (i > 0 && at(&self.color, i - 1, j))
                    || (i < w - 1 && at(&self.color, i + 1, j))
                    || (j > 0 && at(&self.color, i, j - 1))
                    || (j < h - 1 && at(&self.color, i, j + 1));
                if n && let Some(t) = self.tmp.get_mut(j * w + i) {
                    *t = c.pack();
                }
            }
        }
        for (k, &t) in self.color.iter_mut().zip(&self.tmp) {
            if t != 0 {
                *k = t;
            }
        }
    }

    /// Stamp the layer onto `p` at alpha `a` (dithered).
    pub fn draw(&self, p: &mut Painter, a: f64) {
        for (k, &c) in self.color.iter().enumerate() {
            if c == 0 {
                continue;
            }
            let x = (k % self.w) as i32 + self.ox;
            let y = (k / self.w) as i32 + self.oy;
            p.dot(f64::from(x), f64::from(y), Rgba::unpack(c), a);
        }
    }
}

const INK: Rgba = Rgba::hex24(0x0b0814);

/// A soft floor shadow at x, half-width rx: dithered darkness on the floor line, fuller in the middle, with a
/// shorter second row under it. k (0..1) is its strength: lift the thing above and the shadow shrinks and thins.
pub fn blob_shadow(p: &mut Painter, x: f64, rx: f64, k: f64, y: f64) {
    if rx < 0.8 || k <= 0.0 {
        return;
    }
    let mut i = libm::floor(x - rx);
    while i <= libm::ceil(x + rx) {
        let u = libm::fabs(i + 0.5 - x) / rx;
        if u <= 1.0 {
            let a = k * (1.0 - u * u);
            p.dot(i, y, INK, a * 1.1);
            if u < 0.6 {
                p.dot(i, y + 1.0, INK, a * 0.6);
            }
        }
        i += 1.0;
    }
}

/// A four-point star glint: arms of arm px in c, dimmer tips, a white heart, its own light.
pub fn glint(p: &mut Painter, x: f64, y: f64, arm: f64, c: Rgba, k: f64) {
    let tip = scale(c, 0.8);
    let n = js_round(arm);
    let (cx, cy) = (libm::floor(x), libm::floor(y));
    if n < 0.0 {
        return;
    }
    let mut d = 1.0;
    while d <= n {
        let cc = if d == n && n > 1.0 { tip } else { c };
        p.px(cx + d, cy, cc);
        p.px(cx - d, cy, cc);
        p.px(cx, cy + d, cc);
        p.px(cx, cy - d, cc);
        d += 1.0;
    }
    p.px(cx, cy, if n > 0.0 { WHITE } else { c });
    p.glow(cx + 0.5, cy + 0.5, 3.0 + n * 2.5, c, k);
}

/// A thick coin turning on its upright axis: radius r, thickness th, turned by theta (0 faces you). Each row
/// shows the face (squashed by cos theta, a raised rim, a recessed field, an embossed star) and, beside it,
/// the reeded edge band (th |sin theta| wide). Lit from the top left; the face flashes white as it squares
/// to the light. Built in `l` with a dark outline. `c`: 8-step ramp. Returns the flash strength 0..1.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn coin3d(
    l: &mut CoinLayer,
    p: &mut Painter,
    x: f64,
    y: f64,
    r: f64,
    th: f64,
    theta: f64,
    c: &[Rgba; 8],
) -> f64 {
    let ramp = |i: f64| c.get(i as usize).copied().unwrap_or(Rgba::NONE);
    let (ct, st) = (libm::cos(theta), libm::sin(theta));
    let sg = if ct < 0.0 { -1.0 } else { 1.0 };
    let (act, ast) = (libm::fabs(ct), libm::fabs(st));
    let xc = (th / 2.0) * st * sg;
    let nfx = st * sg;
    let (lx, ly, lz) = (-0.55, 0.45, 0.7);
    let ln = libm::sqrt(lx * lx + ly * ly + lz * lz);
    // the face's normal stays level, so its flash comes when it squares to the light's level direction
    let lam = (nfx * lx + act * lz) / ln;
    let (hx, hz) = (lx / ln, lz / ln + 1.0);
    let hn = libm::hypot(hx, hz);
    let base = js_max(2.0, js_min(5.0, js_round(2.0 + lam * 3.2)));
    let (cx, cy) = (js_round(x), js_round(y));
    let b = libm::ceil(r + th) + 2.0;
    let mut flash = 0.0;
    l.begin(cx - b, cy - b);
    l.fill(cx - b, cy - b, cx + b, cy + b, |px, py| {
        let (sx, sy) = (px - cx, -(py - cy));
        if libm::fabs(sy) > r {
            return Rgba::NONE;
        }
        let q = sy / r;
        let s = libm::sqrt(1.0 - q * q);
        let hw = r * s * act;
        let xr = -(if st < 0.0 { -1.0 } else { 1.0 }) * s * r * ct;
        let rw = (th / 2.0) * ast;
        if libm::fabs(sx - xc) <= hw && act > 0.08 {
            // on the face: un-squash to the face's own coordinates
            let (fx, fy) = ((sx - xc) / act, sy);
            let d = libm::hypot(fx, fy) / r;
            // a faint curve across the face makes the flash sweep over it as it turns
            let tx = nfx + (fx / r) * 0.3 * act;
            let sp = libm::pow(
                js_max(0.0, (tx * hx + act * hz) / hn / libm::hypot(tx, act)),
                130.0,
            );
            flash = js_max(flash, sp);
            // relief in whole ramp steps so it shows at every angle: a raised rim lit on its upper left, a
            // recessed field, a raised diamond with lit and shaded faces
            let lit = (-fx * 0.6 + fy * 0.8) / js_max(0.5, libm::hypot(fx, fy));
            let k = if d > 0.76 {
                base + if lit > 0.75 {
                    2.0
                } else if lit > 0.3 {
                    1.0
                } else if lit < -0.35 {
                    -1.0
                } else {
                    0.0
                }
            } else {
                let q = (libm::fabs(fx) + libm::fabs(fy)) / (r * 0.52);
                if q >= 1.0 {
                    base - 1.0
                } else if q < 0.45 || (fx <= 0.0 && fy >= 0.0) {
                    base + 1.0
                } else if fx > 0.0 && fy < 0.0 {
                    base - 2.0
                } else {
                    base
                }
            };
            if sp > 0.5 {
                return if sp > 0.8 {
                    WHITE
                } else {
                    ramp(js_min(7.0, k + 2.0))
                };
            }
            return ramp(js_max(2.0, js_min(6.0, k)));
        }
        if libm::fabs(sx - xr) <= rw + 0.05 {
            // the reeded edge: grooves across the band, lit by the edge's own normal
            let cphi = if xr < 0.0 { -s } else { s };
            let n = [cphi * ct, sy / r, -cphi * st];
            let lt = (n[0] * lx + n[1] * ly + n[2] * lz) / ln;
            let groove = libm::floor((libm::asin(sy / r) + core::f64::consts::PI) * 9.0) % 2.0;
            let edge = libm::fabs(sx - xr) > rw - 0.6 && js_sign(sx - xr) == js_sign(xc - xr);
            return ramp(js_max(
                1.0,
                js_min(
                    6.0,
                    js_round(2.6 + lt * 2.6) - groove + if edge { 1.0 } else { 0.0 },
                ),
            ));
        }
        Rgba::NONE
    });
    l.outline(ramp(1.0));
    l.draw(p, 1.0);
    flash
}
