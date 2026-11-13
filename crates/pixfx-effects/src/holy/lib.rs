//! Shared by the holy effects: gold, green and ash ramps, the character's silhouette and recolors, floor ovals,
//! feathers of light (drawn, and simulated as they rock and sink) and plus signs.
//! Files starting with _ are helpers, not effects.
//!
//! Port of pixelfx `src/fx/holy/_lib.js`: the functions feather burst uses (`feather`, `Feathers`).

use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::math::{Rng, TAU, clamp, js_max, js_min, js_round, smooth};
use pixfx_core::{Layer, Painter};

/// A feather of light from its quill at (x, y) pointing along ang, len px long, with a rounded tip; the
/// trailing vane, wid px wide, falls on side (1: clockwise of the shaft, -1: the other way), the leading vane
/// is under half that. R: a ramp, dark to bright (6+ steps). a: alpha. hot 0..1 whitens it (a flash). A white
/// shaft, a lit leading rim, a warm dark trailing edge that parts it from the next feather, a body that cools
/// from cream at the quill to gold, and a tip that thins into light.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn feather(
    p: &mut Painter,
    x: f64,
    y: f64,
    ang: f64,
    len: f64,
    wid: f64,
    ramp: &[Rgba],
    a: f64,
    side: f64,
    hot: f64,
    quill: f64,
) {
    if len < 1.5 || a <= 0.02 {
        return;
    }
    let n = ramp.len();
    let at = |k: usize| {
        n.checked_sub(k)
            .and_then(|i| ramp.get(i))
            .copied()
            .unwrap_or(Rgba::NONE)
    };
    let (ca, sa) = (libm::cos(ang), libm::sin(ang));
    let (x1, y1) = (x + ca * len, y + sa * len);
    let pad = wid + 1.2;
    let h = hot > 0.5;
    let edge = if h { at(3) } else { at(5) };
    let lead = if h { WHITE } else { at(2) };
    let b = if h {
        [WHITE, WHITE, at(2)]
    } else {
        [at(2), at(3), at(4)]
    };
    p.each(
        js_min(x, x1) - pad,
        js_min(y, y1) - pad,
        js_max(x, x1) + pad,
        js_max(y, y1) + pad,
        |p, px, py, sx, sy| {
            let (dx, dy) = (px - x, py - y);
            let u = (dx * ca + dy * sa) / len;
            if !(0.0..=1.05).contains(&u) {
                return;
            }
            let v = (-dx * sa + dy * ca) * side;
            let av = libm::fabs(v);
            let t = js_max(0.0, (u - 0.55) / 0.47);
            let w = wid
                * libm::sqrt(clamp((u - quill) / 0.22, 0.0, 1.0))
                * libm::sqrt(js_max(0.0, 1.0 - libm::pow(t, 2.0)));
            let lim = if v < 0.0 { w * 0.42 } else { w };
            let fade = if u < 0.72 {
                1.0
            } else {
                1.0 - (u - 0.72) * 1.6
            };
            if av < 0.5 && u < 0.9 {
                return p.put(sx, sy, if u < 0.5 { WHITE } else { b[0] }, a);
            }
            if av > lim + 0.05 || w < 0.5 {
                return;
            }
            if lim - av < 1.0 {
                return if v > 0.0 {
                    p.put(sx, sy, if u > 0.85 { b[2] } else { edge }, a * fade)
                } else {
                    p.put(sx, sy, lead, a)
                };
            }
            if u < 0.72 {
                return p.put(sx, sy, if u < 0.38 { b[0] } else { b[1] }, a);
            }
            p.blend_px(sx, sy, b[2], a * fade);
        },
    );
}

/// One loose feather: where it is, how it moves and how it rocks.
struct Loose {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    a: f64,
    len: f64,
    wid: f64,
    life: f64,
    delay: f64,
    back: bool,
    side: f64,
    ph: f64,
    w: f64,
    amp: f64,
    sink: f64,
    va: f64,
    tilt: f64,
    max: f64,
    age: f64,
    landed: bool,
}

/// What a caller says about a new feather. `sink` and `amp` replace the random swing sink and amplitude when
/// given (the random draws are made either way, as pixelfx does).
#[derive(Clone, Copy, Debug, Default)]
pub struct NewFeather {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub a: f64,
    pub len: f64,
    pub wid: f64,
    pub life: f64,
    pub delay: f64,
    pub back: bool,
    pub sink: Option<f64>,
    pub amp: Option<f64>,
}

/// Loose feathers that are thrown, brake hard, then rock like a pendulum as they sink, tilting with each swing
/// and sinking fastest through the bottom of it; on the floor they lie flat and fade.
pub struct Feathers {
    floor: f64,
    list: Vec<Loose>,
}

impl Feathers {
    pub fn new(floor: f64) -> Self {
        Self {
            floor,
            list: Vec::new(),
        }
    }

    pub fn add(&mut self, r: &mut Rng, f: NewFeather) {
        let side = r.sign();
        let ph = r.range(0.0, TAU);
        let w = r.range(3.2, 4.6);
        let amp = r.range(6.0, 11.0);
        let sink = r.range(14.0, 24.0);
        let va = r.range(-9.0, 9.0);
        let tilt = r.range(-0.35, 0.35);
        self.list.push(Loose {
            x: f.x,
            y: f.y,
            vx: f.vx,
            vy: f.vy,
            a: f.a,
            len: f.len,
            wid: f.wid,
            life: f.life,
            delay: f.delay,
            back: f.back,
            side,
            ph,
            w,
            amp: f.amp.unwrap_or(amp),
            sink: f.sink.unwrap_or(sink),
            va,
            tilt,
            max: f.life,
            age: 0.0,
            landed: false,
        });
    }

    pub fn count(&self) -> usize {
        self.list.len()
    }

    /// A random feather's position and delay (pixelfx `R.pick(loose.list)`).
    pub fn pick(&self, r: &mut Rng) -> Option<(f64, f64, f64)> {
        r.pick(&self.list).map(|f| (f.x, f.y, f.delay))
    }

    pub fn update(&mut self, dt: f64) {
        for i in (0..self.list.len()).rev() {
            let floor = self.floor;
            let Some(f) = self.list.get_mut(i) else {
                continue;
            };
            if f.delay > 0.0 {
                f.delay -= dt;
                continue;
            }
            f.life -= dt;
            if f.life <= 0.0 {
                self.list.remove(i);
                continue;
            }
            f.age += dt;
            if f.landed {
                f.a += (js_round(f.a / core::f64::consts::PI) * core::f64::consts::PI - f.a)
                    * js_min(1.0, dt * 10.0);
                continue;
            }
            // the throw: drag kills it fast while it spins
            f.vx -= f.vx * 3.4 * dt;
            f.vy -= f.vy * 3.4 * dt;
            let s = smooth(0.1, 0.6, f.age);
            let ph = f.age * f.w + f.ph;
            let c = libm::cos(ph);
            // the flutter: a swing across, a sink that's fastest through the bottom of each swing
            f.x += (f.vx + libm::sin(ph) * f.amp * s * 0.9) * dt;
            f.y += (f.vy + f.sink * s * (c * c * 1.5 - 0.2)) * dt;
            // it lies along its swing, tipping up at each end
            let rest = js_round(f.a / core::f64::consts::PI) * core::f64::consts::PI
                + f.tilt
                + libm::sin(ph) * 0.6;
            f.a += f.va * dt * (1.0 - s) + (rest - f.a) * js_min(1.0, dt * 5.0 * s);
            if f.y >= floor - 0.5 {
                f.y = floor - 0.5;
                f.landed = true;
                f.life = js_min(f.life, 0.6);
            }
        }
    }

    /// Draw the feathers of one layer (`Layer::Back`: the small ones behind, `Layer::Front`: the rest).
    pub fn draw(&self, p: &mut Painter, ramp: &[Rgba], layer: Layer) {
        let n = ramp.len();
        let glow_col = n
            .checked_sub(3)
            .and_then(|i| ramp.get(i))
            .copied()
            .unwrap_or(Rgba::NONE);
        for f in &self.list {
            let this = if f.back { Layer::Back } else { Layer::Front };
            if f.delay > 0.0 || (layer != Layer::Both && this != layer) {
                continue;
            }
            let k = f.life / f.max;
            let a = js_min(js_min(1.0, f.life / 0.45), f.age / 0.06);
            let (ca, sa) = (libm::cos(f.a), libm::sin(f.a));
            feather(
                p,
                f.x - (ca * f.len) / 2.0,
                f.y - (sa * f.len) / 2.0,
                f.a,
                f.len,
                f.wid,
                ramp,
                a,
                f.side,
                if f.age < 0.06 { 1.0 } else { 0.0 },
                0.2,
            );
            p.glow(
                f.x,
                f.y,
                4.0 + f.len * 0.5,
                glow_col,
                0.22 * a * (0.4 + 0.6 * k),
            );
        }
    }
}
