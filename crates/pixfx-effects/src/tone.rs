//! Tone painting: an effect paints each pixel as a tone, 0 (its darkest edge) to 1 (its white core), and a ramp
//! of [`STEPS`] colours turns the tone into colour. [`RAMPS`] holds nine colourways, picked with the `variant`
//! option ([`VARIANT`]); a palette the caller gives replaces them.
//!
//! The ramps are read from the "Free" pack (Part 1): every sheet there is one grey animation drawn
//! through nine gradient maps, a row each, so row k of every sheet is ramp k. Each step is the sheet's own colour at
//! one of 16 evenly spaced grey levels (51 to 255); the mean channel error against the full gradient is 2 of 255.

use pixfx_core::{NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, step_at};
use pixfx_draw::math::{TAU, clamp, js_round, rand};

/// Colours in a ramp.
pub const STEPS: usize = 16;

/// The brightest step, as a tone.
pub const LAST: f64 = (STEPS - 1) as f64;

/// Which colourway (0 to 8) of [`RAMPS`].
pub const VARIANT: NumOpt = NumOpt::new("variant", 0.0).within(0.0, 8.0);

/// The nine colourways, dark to bright.
pub const RAMPS: [[Rgba; STEPS]; 9] = [
    // 0 ember: red to yellow
    [
        Rgba::hex24(0xae2d48),
        Rgba::hex24(0xb7354c),
        Rgba::hex24(0xc64253),
        Rgba::hex24(0xd54e5a),
        Rgba::hex24(0xe55c61),
        Rgba::hex24(0xeb6b65),
        Rgba::hex24(0xef7a69),
        Rgba::hex24(0xf2886d),
        Rgba::hex24(0xf69871),
        Rgba::hex24(0xfaa674),
        Rgba::hex24(0xfeb478),
        Rgba::hex24(0xfec27c),
        Rgba::hex24(0xfecd81),
        Rgba::hex24(0xfdd884),
        Rgba::hex24(0xfde388),
        Rgba::hex24(0xfcf08d),
    ],
    // 1 violet: indigo to pink
    [
        Rgba::hex24(0x230096),
        Rgba::hex24(0x3100ab),
        Rgba::hex24(0x3e00bf),
        Rgba::hex24(0x530bc9),
        Rgba::hex24(0x6c19d1),
        Rgba::hex24(0x8225d9),
        Rgba::hex24(0x9531e0),
        Rgba::hex24(0xa83ce6),
        Rgba::hex24(0xbd48ee),
        Rgba::hex24(0xd052f4),
        Rgba::hex24(0xe35dfb),
        Rgba::hex24(0xf16dfc),
        Rgba::hex24(0xf67ff6),
        Rgba::hex24(0xfa8ef0),
        Rgba::hex24(0xfea0ea),
        Rgba::hex24(0xffa4e9),
    ],
    // 2 frost: blue to ice white
    [
        Rgba::hex24(0x133eba),
        Rgba::hex24(0x1340bb),
        Rgba::hex24(0x0d58c4),
        Rgba::hex24(0x086fcd),
        Rgba::hex24(0x0389d7),
        Rgba::hex24(0x0c9dde),
        Rgba::hex24(0x2aaee4),
        Rgba::hex24(0x46bde9),
        Rgba::hex24(0x66cfef),
        Rgba::hex24(0x82dff4),
        Rgba::hex24(0x9eeffa),
        Rgba::hex24(0xbcfeff),
        Rgba::hex24(0xc7feff),
        Rgba::hex24(0xd0feff),
        Rgba::hex24(0xdbfffe),
        Rgba::hex24(0xe7fffe),
    ],
    // 3 lime: deep green to lime
    [
        Rgba::hex24(0x115339),
        Rgba::hex24(0x115339),
        Rgba::hex24(0x1b673e),
        Rgba::hex24(0x257b44),
        Rgba::hex24(0x309249),
        Rgba::hex24(0x399f4c),
        Rgba::hex24(0x41a84c),
        Rgba::hex24(0x49b14d),
        Rgba::hex24(0x51bb4e),
        Rgba::hex24(0x59c44f),
        Rgba::hex24(0x73cf59),
        Rgba::hex24(0x94dc65),
        Rgba::hex24(0xb1e770),
        Rgba::hex24(0xcaf17a),
        Rgba::hex24(0xcbf17a),
        Rgba::hex24(0xcbf17a),
    ],
    // 4 copper: brown to sand
    [
        Rgba::hex24(0x5f2e2a),
        Rgba::hex24(0x653530),
        Rgba::hex24(0x6b3b36),
        Rgba::hex24(0x71413b),
        Rgba::hex24(0x854f3e),
        Rgba::hex24(0x955a41),
        Rgba::hex24(0xa46543),
        Rgba::hex24(0xb36f46),
        Rgba::hex24(0xbf7b4a),
        Rgba::hex24(0xc68550),
        Rgba::hex24(0xcc8f56),
        Rgba::hex24(0xd49a5d),
        Rgba::hex24(0xdba463),
        Rgba::hex24(0xe2b173),
        Rgba::hex24(0xebc187),
        Rgba::hex24(0xf4d29c),
    ],
    // 5 silver: grey to white
    [
        Rgba::hex24(0x333333),
        Rgba::hex24(0x414141),
        Rgba::hex24(0x4e4e4e),
        Rgba::hex24(0x5b5b5b),
        Rgba::hex24(0x696969),
        Rgba::hex24(0x777777),
        Rgba::hex24(0x858585),
        Rgba::hex24(0x929292),
        Rgba::hex24(0xa0a0a0),
        Rgba::hex24(0xadadad),
        Rgba::hex24(0xbababa),
        Rgba::hex24(0xc9c9c9),
        Rgba::hex24(0xd7d7d7),
        Rgba::hex24(0xe3e3e3),
        Rgba::hex24(0xf0f0f0),
        Rgba::hex24(0xffffff),
    ],
    // 6 dusk: plum to tan
    [
        Rgba::hex24(0x3a294c),
        Rgba::hex24(0x432e55),
        Rgba::hex24(0x503661),
        Rgba::hex24(0x5d3d6d),
        Rgba::hex24(0x6b4672),
        Rgba::hex24(0x784d71),
        Rgba::hex24(0x845470),
        Rgba::hex24(0x8f5b6f),
        Rgba::hex24(0x9b626e),
        Rgba::hex24(0xa6696d),
        Rgba::hex24(0xae706e),
        Rgba::hex24(0xb27b70),
        Rgba::hex24(0xb68472),
        Rgba::hex24(0xb98d74),
        Rgba::hex24(0xbd9676),
        Rgba::hex24(0xc1a078),
    ],
    // 7 crimson: night red to gold
    [
        Rgba::hex24(0x201528),
        Rgba::hex24(0x301431),
        Rgba::hex24(0x3e133a),
        Rgba::hex24(0x4c1342),
        Rgba::hex24(0x5c124b),
        Rgba::hex24(0x761449),
        Rgba::hex24(0x8f1747),
        Rgba::hex24(0xa71945),
        Rgba::hex24(0xc21b42),
        Rgba::hex24(0xd91e40),
        Rgba::hex24(0xe23546),
        Rgba::hex24(0xe8554f),
        Rgba::hex24(0xee7256),
        Rgba::hex24(0xf28b5d),
        Rgba::hex24(0xf7a765),
        Rgba::hex24(0xfdc66d),
    ],
    // 8 indigo: purple to steel blue
    [
        Rgba::hex24(0x281042),
        Rgba::hex24(0x35124f),
        Rgba::hex24(0x42155c),
        Rgba::hex24(0x4e1768),
        Rgba::hex24(0x5c1a76),
        Rgba::hex24(0x5f1f7f),
        Rgba::hex24(0x5d2685),
        Rgba::hex24(0x5a2d8b),
        Rgba::hex24(0x583492),
        Rgba::hex24(0x553b98),
        Rgba::hex24(0x53439c),
        Rgba::hex24(0x514f9f),
        Rgba::hex24(0x4f59a2),
        Rgba::hex24(0x4e62a4),
        Rgba::hex24(0x4c6ca7),
        Rgba::hex24(0x4a77aa),
    ],
];

/// The ramp `opts` ask for: the caller's palette, else the colourway `variant` names.
pub fn ramp(opts: &Options) -> Vec<Rgba> {
    let v = opts.num(&VARIANT);
    let i = if v.is_nan() { 0 } else { js_round(v) as usize };
    let Some(way) = RAMPS.get(i).or(RAMPS.first()) else {
        return Vec::new();
    };
    opts.palette_or(way)
}

/// The colour of tone `tone` (0..1, clamped) on `ramp`; nothing for NaN.
pub fn color(ramp: &[Rgba], tone: f64) -> Rgba {
    if tone.is_nan() {
        return Rgba::NONE;
    }
    let last = ramp.len().saturating_sub(1) as f64;
    step_at(ramp, js_round(clamp(tone, 0.0, 1.0) * last))
}

/// Strokes in tone: tapered segments (round caps), each with a tone at either end in ramp steps (0 the darkest
/// step, 15 the brightest), gathered for a frame and then
/// painted pixel by pixel ([`Ink::paint`]). Where strokes overlap the brighter tone wins, so a frame need not be
/// drawn back to front. `fold` repeats every stroke round the origin (4: four copies a quarter turn apart).
pub struct Ink {
    segs: Vec<Seg>,
    fold: u32,
    rim: f64,
    grain: (f64, f64, f64),
}

/// A point of a stroke: `(x, y, half-width, tone)`.
pub type Knot = (f64, f64, f64, f64);

struct Seg {
    a: (f64, f64),
    b: (f64, f64),
    r: (f64, f64),
    tone: (f64, f64),
    lo: (f64, f64),
    hi: (f64, f64),
}

impl Ink {
    /// No strokes yet, repeated `fold` times round the origin (1: once).
    pub fn new(fold: u32) -> Self {
        Ink {
            segs: Vec::new(),
            fold: fold.max(1),
            rim: 0.0,
            grain: (0.0, 1.0, 0.0),
        }
    }

    /// Darken each stroke toward its edge: its tone times `1 - rim` at the edge, full on its centre line.
    pub fn rim(mut self, rim: f64) -> Self {
        self.rim = rim;
        self
    }

    /// Speckle bright strokes as a painter's brush does: a share `share` of the pixels at or past step 11, picked
    /// by `seed`, take `keep` of their tone.
    pub fn grain(mut self, share: f64, keep: f64, seed: f64) -> Self {
        self.grain = (share, keep, seed);
        self
    }

    /// A segment from `a` to `b`, half-width `r.0` to `r.1`, tone `tone.0` to `tone.1`.
    pub fn seg(&mut self, a: (f64, f64), b: (f64, f64), r: (f64, f64), tone: (f64, f64)) {
        if !(r.0 > 0.0 || r.1 > 0.0) {
            return;
        }
        let m = r.0.max(r.1);
        self.segs.push(Seg {
            a,
            b,
            r,
            tone,
            lo: (a.0.min(b.0) - m, a.1.min(b.1) - m),
            hi: (a.0.max(b.0) + m, a.1.max(b.1) + m),
        });
    }

    /// A round dot.
    pub fn dot(&mut self, x: f64, y: f64, r: f64, tone: f64) {
        self.seg((x, y), (x, y), (r, r), (tone, tone));
    }

    /// An arc of radius `radius` round `centre` from angle `a0` to `a1` (radians, clockwise on screen from the
    /// right), half-width `r.0` to `r.1` and tone `tone.0` to `tone.1` along it, in segments about 3 pixels long.
    pub fn arc(
        &mut self,
        centre: (f64, f64),
        radius: f64,
        a0: f64,
        a1: f64,
        r: (f64, f64),
        tone: (f64, f64),
    ) {
        let n = (libm::fabs(a1 - a0) * radius / 3.0).clamp(1.0, 64.0) as usize;
        let at = |k: usize| {
            let f = k as f64 / n as f64;
            let a = a0 + (a1 - a0) * f;
            (
                centre.0 + radius * libm::cos(a),
                centre.1 + radius * libm::sin(a),
                r.0 + (r.1 - r.0) * f,
                tone.0 + (tone.1 - tone.0) * f,
            )
        };
        for k in 0..n {
            let (p, q) = (at(k), at(k + 1));
            self.seg((p.0, p.1), (q.0, q.1), (p.2, q.2), (p.3, q.3));
        }
    }

    /// A path through points `(x, y, half-width, tone)`, a segment between each pair.
    pub fn path(&mut self, pts: &[Knot]) {
        for w in pts.windows(2) {
            if let [a, b] = w {
                self.seg((a.0, a.1), (b.0, b.1), (a.2, b.2), (a.3, b.3));
            }
        }
    }

    /// The brightest tone any stroke (or its folded copies) gives the point, if one covers it, grained.
    ///
    /// The grain is pinned to the stroke, not the screen: a speckle is picked by the stroke and the pixel's place
    /// along and across it, so it rides with the stroke as it moves and turns instead of shimmering under it.
    pub fn tone(&self, x: f64, y: f64) -> Option<f64> {
        let mut best: Option<(f64, f64, f64, usize)> = None;
        for k in 0..self.fold {
            let (px, py) = turn(x, y, self.fold, k);
            for (i, s) in self.segs.iter().enumerate() {
                if px < s.lo.0 || px > s.hi.0 || py < s.lo.1 || py > s.hi.1 {
                    continue;
                }
                if let Some((v, along, across)) = s.tone_at(px, py, self.rim)
                    && best.is_none_or(|b| v > b.0)
                {
                    best = Some((v, along, across, i));
                }
            }
        }
        let (v, along, across, i) = best?;
        let (share, keep, seed) = self.grain;
        let speck = v >= 11.0
            && share > 0.0
            && rand(&[i as f64, libm::floor(along), libm::floor(across), seed]) < share;
        Some(if speck { v * keep } else { v })
    }

    /// Paint every covered pixel of the box `[-half, half)` square round the origin in the ramp colour of its
    /// tone, with its own light: `light` times how far its tone is past step 10, in steps.
    pub fn paint(&self, p: &mut Painter, ramp: &[Rgba], half: f64, light: f64) {
        if self.segs.is_empty() {
            return;
        }
        p.shade(-half, -half, half - 1.0, half - 1.0, |x, y, o| {
            let v = self.tone(x, y)?;
            o.k = light * (v - 10.0).max(0.0);
            Some(color(ramp, v / LAST))
        });
    }
}

/// A point turned back by `k` of `fold` turns about the origin, so it can be tested against the unturned strokes;
/// quarter turns are exact.
fn turn(x: f64, y: f64, fold: u32, k: u32) -> (f64, f64) {
    if (4 * k).is_multiple_of(fold) {
        return match (4 * k / fold) % 4 {
            1 => (y, -x),
            2 => (-x, -y),
            3 => (-y, x),
            _ => (x, y),
        };
    }
    let a = -f64::from(k) * TAU / f64::from(fold);
    let (s, c) = (libm::sin(a), libm::cos(a));
    (x * c - y * s, x * s + y * c)
}

impl Seg {
    /// The tone the segment gives the point, if it covers it, and the point's place in pixels along the segment
    /// from `a` and across it (a dot: right and down from its centre).
    fn tone_at(&self, x: f64, y: f64, rim: f64) -> Option<(f64, f64, f64)> {
        let (dx, dy) = (self.b.0 - self.a.0, self.b.1 - self.a.1);
        let len2 = dx * dx + dy * dy;
        let h = if len2 > 0.0 {
            clamp(((x - self.a.0) * dx + (y - self.a.1) * dy) / len2, 0.0, 1.0)
        } else {
            0.0
        };
        let (cx, cy) = (self.a.0 + dx * h, self.a.1 + dy * h);
        let d = libm::sqrt((x - cx) * (x - cx) + (y - cy) * (y - cy));
        let r = self.r.0 + (self.r.1 - self.r.0) * h;
        if d > r {
            return None;
        }
        let v = self.tone.0 + (self.tone.1 - self.tone.0) * h;
        let (ux, uy) = if len2 > 0.0 {
            let len = libm::sqrt(len2);
            (dx / len, dy / len)
        } else {
            (1.0, 0.0)
        };
        let (ox, oy) = (x - self.a.0, y - self.a.1);
        let v = if r > 0.0 { v * (1.0 - rim * d / r) } else { v };
        Some((v, ox * ux + oy * uy, oy * ux - ox * uy))
    }
}

/// The pose at moment `s` of a stroke keyed at whole moments (`keys[k]` at `s = k`), held after the last key. Each of a knot's four numbers runs on a smooth curve through its keys ([`ease`]), so a
/// stroke never changes speed with a jolt at a key.
pub fn pose<const N: usize>(keys: &[[Knot; N]], s: f64) -> [Knot; N] {
    let last = keys.len().saturating_sub(1);
    let s = clamp(s, 0.0, last as f64);
    let i = (libm::floor(s) as usize).min(last);
    let f = s - i as f64;
    let at = |k: usize| keys.get(k.min(last));
    let (Some(p), Some(a), Some(b), Some(q)) =
        (at(i.saturating_sub(1)), at(i), at(i + 1), at(i + 2))
    else {
        return [(0.0, 0.0, 0.0, 0.0); N];
    };
    // the slope at the last key is 0, so the hold after it joins smoothly; the first key is where the effect
    // starts, so it leaves at the speed of its first span instead of from rest
    let (first, end) = (i == 0, i + 1 >= last);
    let one = |p: f64, a: f64, b: f64, q: f64| {
        let m0 = if first { b - a } else { slope(a - p, b - a) };
        let m1 = if end { 0.0 } else { slope(b - a, q - b) };
        ease(a, b, m0, m1, f)
    };
    core::array::from_fn(|n| {
        let (p, a, b, q) = (p[n], a[n], b[n], q[n]);
        (
            one(p.0, a.0, b.0, q.0),
            one(p.1, a.1, b.1, q.1),
            one(p.2, a.2, b.2, q.2),
            one(p.3, a.3, b.3, q.3),
        )
    })
}

/// The slope a smooth curve takes at a key, from the slopes of the spans either side: their harmonic mean, or 0
/// where the value turns back or holds, so the curve never overshoots its keys.
fn slope(before: f64, after: f64) -> f64 {
    if before * after <= 0.0 {
        0.0
    } else {
        2.0 * before * after / (before + after)
    }
}

/// The cubic from `a` to `b` over a span of 1 with slopes `m0` and `m1` at its ends, at `f` (0..1).
fn ease(a: f64, b: f64, m0: f64, m1: f64, f: f64) -> f64 {
    let (f2, f3) = (f * f, f * f * f);
    (2.0 * f3 - 3.0 * f2 + 1.0) * a
        + (f3 - 2.0 * f2 + f) * m0
        + (3.0 * f2 - 2.0 * f3) * b
        + (f3 - f2) * m1
}

/// A value keyed at moments `(s, value)` (in order), on a smooth curve through the keys (as [`pose`]) and held
/// past either end.
pub fn curve(keys: &[(f64, f64)], s: f64) -> f64 {
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else {
        return 0.0;
    };
    if s <= first.0 {
        return first.1;
    }
    // the slope of the span from key k to the next, 0 past either end
    let span = |k: usize| match (keys.get(k), keys.get(k + 1)) {
        (Some(a), Some(b)) if b.0 > a.0 => (b.1 - a.1) / (b.0 - a.0),
        _ => 0.0,
    };
    for (k, w) in keys.windows(2).enumerate() {
        if let [a, b] = w
            && s <= b.0
        {
            let width = b.0 - a.0;
            if width <= 0.0 {
                return b.1;
            }
            let m0 = if k == 0 {
                0.0
            } else {
                slope(span(k - 1), span(k))
            };
            let m1 = if k + 2 >= keys.len() {
                0.0
            } else {
                slope(span(k), span(k + 1))
            };
            return ease(a.1, b.1, m0 * width, m1 * width, (s - a.0) / width);
        }
    }
    last.1
}
