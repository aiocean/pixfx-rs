//! `shadow/winter-ordeal`: the winter shadow of heaven. Over the foe the
//! sky darkens and cracks like glass: cracks run out from a keystone in its middle, where an eye opens and looks down,
//! and rings of cracks cut it into slabs. Then the sky falls: ring by ring the slabs tremble, drop, turn and smash on the
//! ground into shards that crumble to dust, and last the keystone with the open eye falls onto the foe (the blow, with
//! its cue). A black drift lies round the feet and melts into shadow.
//!
//! Made here, not ported; its golden hashes pin it. The sky is baked once by `examples/bake_winter_ordeal.rs` into
//! [`super::winter_ordeal_sky`] (the slabs as pictures, the cracks with the time each cell appears), so nothing about
//! the cracks is worked out in a game; this file only times and draws them. The scale follows the comic in
//! `target/ref/shadow_winter-ordeal`: the sky about five fighters wide, its middle well over the head.

use pixfx_core::cue::{self, FEEL, TimedCue};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;
use pixfx_draw::halo::Halo;
use pixfx_draw::math::{TAU, clamp, fbm, rand, smooth};
use pixfx_draw::{RULER, Ruler};

use super::lib::VOID;
use super::winter_ordeal_sky::{CRACKS, HALF, SKY, SLABS};

pub struct WinterOrdeal;

pub static WINTER_ORDEAL: WinterOrdeal = WinterOrdeal;

const W: usize = 224;
const H: usize = 176;
const OX: f64 = 112.0;
const OY: f64 = 168.0;

const META: Meta = Meta {
    id: "shadow/winter-ordeal",
    name: "Winter ordeal",
    desc: "Over the foe the night sky cracks like glass round an opening eye, then falls: slab after slab drops and smashes on the ground, and the keystone with the eye crashes onto the foe; a black drift is left at its feet and melts into shadow.",
    tags: &[
        "shadow", "winter", "night", "eye", "shatter", "sky", "strike", "impact", "one-shot",
    ],
    size: (W as u32, H as u32),
    origin: (OX as i32, OY as i32),
    duration: 2.8,
    looping: false,
    fps: 30,
    seed: 7,
    state: None,
    options: &[FEEL.spec()],
};

// the timeline (the cracks' own times are baked: 0.45 s to about 1.2 s)
/// The sky darkens in.
const GROW: (f64, f64) = (0.0, 0.45);
/// The eye opens in the keystone, and shuts as it strikes.
const OPEN: (f64, f64) = (0.8, 1.15);
/// The slabs of each ring start to fall at these times (inner, middle, outer), a little apart within a ring; each
/// trembles for TREMBLE before.
const FALL_AT: [f64; 3] = [1.22, 1.34, 1.46];
const FALL_SPREAD: f64 = 0.1;
const TREMBLE: f64 = 0.12;
/// Gravity, px/s²: a slab from the sky's bottom reaches the ground in about 0.35 s.
const GRAVITY: f64 = 1100.0;
/// The keystone falls onto the foe and strikes at BLOW (the cue).
const BLOW: f64 = 1.78;
/// A smashed slab's shards fly this long, the last SHATTER_DUST of it crumbling to dust.
const SHATTER: f64 = 0.5;
const SHATTER_DUST: f64 = 0.2;
/// The drift lies, then melts away by the end.
const MELT: (f64, f64) = (2.1, 2.75);

// the scale, from the ruler
/// The eye's half width: 2.4 faces.
fn eye_w() -> f64 {
    f64::from(Ruler::size(RULER.head, 2.4))
}
const DRIFT_W: f64 = RULER.width as f64 + 12.0;
/// The foe's body, feet to head.
const BODY: (f64, f64) = (-(RULER.height as f64) + 4.0, -3.0);
/// How far a halo's heat falls a pixel.
const FALL: f32 = 1.0;
/// A slab of this many pixels or more shakes the screen a little when it lands.
const HEAVY: usize = 140;

/// What a pixel of a slab's picture shows.
#[derive(Clone, Copy, PartialEq)]
enum Px {
    Empty,
    /// Sky, of shard n.
    Sky(u8),
}

/// A slab of sky, read from the baked picture once: its pixels round its middle, when it falls and lands.
struct Slab {
    /// Its picture: top left (from its middle), width, pixels row by row.
    at: (i32, i32),
    w: usize,
    h: usize,
    px: Vec<Px>,
    /// How far its farthest pixel is from its middle.
    reach: f64,
    /// Where its middle is in the sky.
    home: (f64, f64),
    /// It falls at t0 and lands at t1 (on the ground, or on the foe for the keystone).
    t0: f64,
    t1: f64,
    /// How fast it turns as it falls, radians a second.
    spin: f64,
    key: bool,
    tint: usize,
    /// Each shard's middle (from the slab's middle) and pixel count.
    shards: Vec<(f64, f64, usize)>,
}

impl Slab {
    fn at(&self, x: i32, y: i32) -> Px {
        let (dx, dy) = (x - self.at.0, y - self.at.1);
        if dx < 0 || dy < 0 || dx as usize >= self.w || dy as usize >= self.h {
            return Px::Empty;
        }
        self.px
            .get(dy as usize * self.w + dx as usize)
            .copied()
            .unwrap_or(Px::Empty)
    }
    fn size(&self) -> usize {
        self.px.iter().filter(|p| **p != Px::Empty).count()
    }
}

/// The slabs, read from the baked sky, with the times each falls and lands. The times use no seed, so the cues know
/// them too.
fn slabs() -> Vec<Slab> {
    let mut out = Vec::with_capacity(SLABS.len());
    for (i, s) in SLABS.iter().enumerate() {
        let h = s.rows.len();
        let w = s.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let mut cells = Vec::new();
        for (y, row) in s.rows.iter().enumerate() {
            for (x, c) in row.bytes().enumerate() {
                if c != b'.' {
                    cells.push((s.at.0 + x as i32, s.at.1 + y as i32, c.saturating_sub(b'a')));
                }
            }
        }
        let n = cells.len().max(1) as f64;
        let (mx, my) = cells.iter().fold((0.0, 0.0), |a, c| {
            (a.0 + f64::from(c.0) / n, a.1 + f64::from(c.1) / n)
        });
        let (mx, my) = (libm::floor(mx + 0.5) as i32, libm::floor(my + 0.5) as i32);
        let at = (s.at.0 - mx, s.at.1 - my);
        let mut px = vec![Px::Empty; w * h];
        let mut reach: f64 = 0.0;
        let mut bottom: f64 = 0.0;
        let mut shards: Vec<(f64, f64, usize)> = Vec::new();
        for &(x, y, k) in &cells {
            if let Some(slot) = px.get_mut((y - s.at.1) as usize * w + (x - s.at.0) as usize) {
                *slot = Px::Sky(k);
            }
            let (dx, dy) = (f64::from(x - mx), f64::from(y - my));
            reach = reach.max(libm::hypot(dx, dy));
            bottom = bottom.max(dy);
            let k = usize::from(k);
            if shards.len() <= k {
                shards.resize(k + 1, (0.0, 0.0, 0));
            }
            if let Some(sh) = shards.get_mut(k) {
                *sh = (sh.0 + dx, sh.1 + dy, sh.2 + 1);
            }
        }
        for sh in &mut shards {
            let c = sh.2.max(1) as f64;
            *sh = (sh.0 / c, sh.1 / c, sh.2);
        }
        let key = s.ring == 0;
        let home = (f64::from(mx), f64::from(my));
        let jitter = rand(&[i as f64, 41.0]);
        let (t0, t1) = if key {
            // it lands on the foe's head at BLOW
            let drop = (BODY.0 - 2.0) - (home.1 + bottom);
            let t = libm::sqrt(2.0 * drop.max(1.0) / GRAVITY);
            (BLOW - t, BLOW)
        } else {
            let ring = usize::from(s.ring.clamp(1, 3)) - 1;
            let t0 = FALL_AT.get(ring).copied().unwrap_or(1.4) + FALL_SPREAD * jitter;
            let drop = -(home.1 + bottom);
            (t0, t0 + libm::sqrt(2.0 * drop.max(1.0) / GRAVITY))
        };
        out.push(Slab {
            at,
            w,
            h,
            px,
            reach: reach + 1.0,
            home,
            t0,
            t1,
            spin: (rand(&[i as f64, 43.0]) - 0.5) * if key { 0.3 } else { 1.4 },
            key,
            tint: i % 3,
            shards,
        });
    }
    out
}

impl Effect for WinterOrdeal {
    fn meta(&self) -> &Meta {
        &META
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let mut cues = cue::impact(BLOW, 0.6, 0.0, (BODY.0 + BODY.1) / 2.0, opts);
        // a heavy slab hitting the ground shakes it a little
        for s in slabs().iter().filter(|s| !s.key && s.size() >= HEAVY) {
            cues.extend(cue::impact(s.t1, 0.15, s.home.0, 0.0, opts));
        }
        cues
    }

    fn spawn(&self, _opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            slabs: slabs(),
            night: night_mask(),
            seed: seed as f64,
        })
    }
}

struct State {
    slabs: Vec<Slab>,
    /// The night round the sky: each pixel it darkens and how dark.
    night: Vec<(i16, i16, f32)>,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            self.night(p, t);
            return;
        }
        let mut halo = Halo::new(W, H, OX, OY);
        self.sky(p, t);
        self.cracks(p, &mut halo, t);
        self.falling(p, &mut halo, t);
        self.key_eye(p, &mut halo, t);
        self.drift(p, &mut halo, t);
        self.shards(p, t);
        halo.spread(FALL);
        paint_halo(p, &halo);
        self.stare(p, t);
        self.shock(p, t);
    }
}

/// Where a slab is at `t` once falling: its middle, and how far it has turned.
fn fall_at(s: &Slab, t: f64) -> (f64, f64, f64) {
    let k = (t - s.t0).max(0.0);
    let drift = if s.key { 0.0 } else { s.home.0 * 0.12 * k };
    (
        s.home.0 - drift,
        s.home.1 + 0.5 * GRAVITY * k * k,
        s.spin * k,
    )
}

impl State {
    /// One pixel of sky: deep violet, a different shade per slab, darker toward the sky's bottom, with stars.
    fn sky_px(&self, s: &Slab, lx: i32, ly: i32, t: f64) -> pixfx_draw::color::Rgba {
        let (gx, gy) = (s.home.0 + f64::from(lx), s.home.1 + f64::from(ly));
        let star = rand(&[gx, gy, 5.0]);
        if star < 0.022 {
            // the sky's stars ride on its slabs; they twinkle
            let on = rand(&[gx, gy, libm::floor(t * 6.0), self.seed]) < 0.75;
            return if star < 0.006 && on {
                VOID[7]
            } else if on {
                VOID[5]
            } else {
                VOID[3]
            };
        }
        let low = (gy - f64::from(SKY.1)) / HALF.1;
        let grain = fbm(gx * 0.15, gy * 0.15, 3.0, 2);
        let shade = s.tint as f64 * 0.25 + grain + 0.5 * low;
        if shade > 1.3 {
            VOID[4]
        } else if shade > 0.6 {
            VOID[3]
        } else {
            VOID[2]
        }
    }

    /// The slabs still in the sky: the sky grows in from the middle, and a slab about to fall trembles.
    fn sky(&self, p: &mut Painter, t: f64) {
        let grown = smooth(GROW.0, GROW.1, t);
        if grown <= 0.0 {
            return;
        }
        let shake_on = libm::floor(t * 20.0) % 2.0 == 0.0;
        for (i, s) in self.slabs.iter().enumerate() {
            if t >= s.t0 {
                continue;
            }
            let shake = if t > s.t0 - TREMBLE && (shake_on ^ (i % 2 == 0)) {
                1.0
            } else {
                0.0
            };
            for ly in 0..s.h as i32 {
                for lx in 0..s.w as i32 {
                    let (x, y) = (lx + s.at.0, ly + s.at.1);
                    if s.at(x, y) == Px::Empty {
                        continue;
                    }
                    let (gx, gy) = (s.home.0 + f64::from(x), s.home.1 + f64::from(y));
                    let d = libm::hypot(
                        (gx - f64::from(SKY.0)) / HALF.0,
                        (gy - f64::from(SKY.1)) / HALF.1,
                    );
                    // the sky darkens in from its middle, its front dithered
                    if d > grown * 1.2 - 0.2 * bayer4(gx, gy) {
                        continue;
                    }
                    p.px(gx + 0.5 + shake, gy + 0.5, self.sky_px(s, x, y, t));
                }
            }
        }
    }

    /// The cracks: each cell white as the crack reaches it, settling to cold violet, while a slab beside it is still up.
    fn cracks(&self, p: &mut Painter, halo: &mut Halo, t: f64) {
        let grown = smooth(GROW.0, GROW.1, t);
        for &(x, y, ms, side) in CRACKS {
            let up = |k: u8| self.slabs.get(usize::from(k)).is_some_and(|s| t < s.t0);
            if !(up(side[0]) || up(side[1])) {
                continue;
            }
            let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let t0 = f64::from(ms) / 1000.0;
            if t < t0 {
                // not cracked yet: plain sky, grown in as the slabs round it are
                let (gx, gy) = (f64::from(x), f64::from(y));
                let d = libm::hypot(
                    (gx - f64::from(SKY.0)) / HALF.0,
                    (gy - f64::from(SKY.1)) / HALF.1,
                );
                if d <= grown * 1.2 - 0.2 * bayer4(gx, gy) {
                    p.px(fx, fy, VOID[2]);
                }
                continue;
            }
            let fresh = 1.0 - clamp((t - t0) / 0.15, 0.0, 1.0);
            p.px(fx, fy, if fresh > 0.4 { VOID[7] } else { VOID[6] });
            halo.stamp(fx, fy, (1.8 + 2.4 * fresh) as f32, 1);
        }
    }

    /// The slabs falling: each turns a little as it drops and shows its thickness, a lit top edge and a dark underside
    /// two pixels deep. Each screen pixel looks back to the slab's pixel it shows, so a turned slab has no holes.
    fn falling(&self, p: &mut Painter, halo: &mut Halo, t: f64) {
        for s in &self.slabs {
            if t < s.t0 || t >= s.t1 || s.key {
                continue;
            }
            self.draw_slab(p, halo, s, t);
        }
    }

    fn draw_slab(&self, p: &mut Painter, halo: &mut Halo, s: &Slab, t: f64) {
        let (x, y, a) = fall_at(s, t);
        let (c, n) = (libm::cos(a), libm::sin(a));
        let r = s.reach + 3.0;
        let (x0, y0) = (libm::floor(x - r), libm::floor(y - r));
        let (x1, y1) = (libm::ceil(x + r), libm::ceil(y + r));
        let mut sy = y0;
        while sy <= y1 {
            let mut sx = x0;
            while sx <= x1 {
                let (dx, dy) = (sx + 0.5 - x, sy + 0.5 - y);
                let back = |oy: f64| {
                    (
                        libm::floor(dx * c + (dy - oy) * n + 0.5) as i32,
                        libm::floor(-dx * n + (dy - oy) * c + 0.5) as i32,
                    )
                };
                let (lx, ly) = back(0.0);
                if s.at(lx, ly) != Px::Empty {
                    // its broken edge: lit along the top, cold violet round the rest
                    let top = s.at(lx, ly - 1) == Px::Empty;
                    let edge = s.at(lx - 1, ly) == Px::Empty
                        || s.at(lx + 1, ly) == Px::Empty
                        || s.at(lx, ly + 1) == Px::Empty;
                    let col = if top {
                        VOID[6]
                    } else if edge {
                        VOID[4]
                    } else {
                        self.sky_px(s, lx, ly, t)
                    };
                    p.px(sx + 0.5, sy + 0.5, col);
                    if top {
                        halo.stamp(sx + 0.5, sy + 0.5, 1.0, 1);
                    }
                } else {
                    // the underside: the slab seen from below, two pixels under its bottom edge
                    let (ux, uy) = back(1.0);
                    let (vx, vy) = back(2.0);
                    if s.at(ux, uy) != Px::Empty || s.at(vx, vy) != Px::Empty {
                        p.px(sx + 0.5, sy + 0.5, VOID[0]);
                    }
                }
                sx += 1.0;
            }
            sy += 1.0;
        }
    }

    /// The keystone: it holds the eye; once it falls it carries the eye down onto the foe, still open.
    fn key_eye(&self, p: &mut Painter, halo: &mut Halo, t: f64) {
        let Some(s) = self.slabs.iter().find(|s| s.key) else {
            return;
        };
        if t >= s.t1 {
            return;
        }
        let (cx, cy) = if t >= s.t0 {
            self.draw_slab(p, halo, s, t);
            let (x, y, _) = fall_at(s, t);
            (x, y)
        } else {
            s.home
        };
        // the eye sits in the keystone's middle
        let (ex, ey) = (
            cx + f64::from(SKY.0) - s.home.0,
            cy + f64::from(SKY.1) - s.home.1,
        );
        self.eye(p, halo, t, ex, ey);
    }

    /// The eye: it opens in the keystone and stares down; it stays open as it falls and shuts as it strikes.
    fn eye(&self, p: &mut Painter, halo: &mut Halo, t: f64, ex: f64, ey: f64) {
        let o = smooth(OPEN.0, OPEN.1, t) * (1.0 - smooth(BLOW - 0.04, BLOW, t));
        if o <= 0.02 {
            return;
        }
        let hw = eye_w();
        let lid = 7.0 * o;
        let mut x = -hw;
        while x <= hw {
            let u = x / hw;
            let h = lid * (1.0 - u * u);
            let (top, bot) = (libm::floor(ey - h), libm::floor(ey + h * 0.8));
            for y in [top, bot] {
                p.px(ex + x + 0.5, y + 0.5, VOID[7]);
                halo.stamp(ex + x + 0.5, y + 0.5, 3.5, 2);
            }
            let mut y = top + 1.0;
            while y < bot {
                p.px(ex + x + 0.5, y + 0.5, VOID[0]);
                y += 1.0;
            }
            x += 1.0;
        }
        // the pupil, looking down at the foe
        if o > 0.45 {
            let r = 4.5 * o;
            p.disc(ex, ey + 0.8, r + 1.0, VOID[4], 1.0);
            p.disc(ex, ey + 0.8, r, VOID[0], 1.0);
            p.light(ex - 1.0, ey - 0.5, VOID[7], 0.8);
        }
    }

    /// A landed slab's shards: each flies out and up from where the slab hit, falls back, bounces on the ground, and
    /// crumbles to dust (every other pixel, then fewer) at the end. Big pieces first, then small.
    fn shards(&self, p: &mut Painter, t: f64) {
        for (i, s) in self.slabs.iter().enumerate() {
            let k = t - s.t1;
            if !(0.0..SHATTER).contains(&k) {
                continue;
            }
            let (hx, hy, a) = fall_at(s, s.t1);
            let crumble = clamp((k - (SHATTER - SHATTER_DUST)) / SHATTER_DUST, 0.0, 1.0);
            for (j, &(sx, sy, count)) in s.shards.iter().enumerate() {
                if count == 0 {
                    continue;
                }
                let r = |salt: f64| rand(&[self.seed, i as f64, j as f64, salt]);
                let (vx, vy) = (sx * 3.0 + 50.0 * (r(1.0) - 0.5), -40.0 - 70.0 * r(2.0));
                let spin = (r(3.0) - 0.5) * 8.0;
                let (ox, mut oy) = (vx * k, vy * k + 0.5 * GRAVITY * k * k);
                // the shard's middle may not sink under the ground
                let ground = -(hy + sy + 1.0);
                if oy > ground {
                    oy = ground;
                }
                let turn = a + spin * k;
                let (c, n) = (libm::cos(turn), libm::sin(turn));
                for ly in 0..s.h as i32 {
                    for lx in 0..s.w as i32 {
                        let (x, y) = (lx + s.at.0, ly + s.at.1);
                        if s.at(x, y) != Px::Sky(j as u8) {
                            continue;
                        }
                        if crumble > 0.0 && rand(&[f64::from(x), f64::from(y), 9.0]) < crumble {
                            continue;
                        }
                        let (dx, dy) = (f64::from(x) - sx, f64::from(y) - sy);
                        let px = hx + sx + ox + dx * c - dy * n;
                        let py = (hy + sy + oy + dx * n + dy * c).min(-0.5);
                        let col = if crumble > 0.5 {
                            VOID[3]
                        } else {
                            self.sky_px(s, x, y, t)
                        };
                        p.px(px + 0.5, py + 0.5, col);
                    }
                }
            }
            // the dust where it hit
            if k < 0.25 && !s.key {
                p.glow(
                    hx,
                    -2.0,
                    6.0 + s.reach * 0.4,
                    VOID[5],
                    0.5 * (1.0 - k / 0.25),
                );
            }
        }
    }

    /// The night round the sky, darkening in as the sky grows and lifting as it falls: its pixels and their darkness
    /// are laid out at spawn ([`night_mask`]).
    fn night(&self, p: &mut Painter, t: f64) {
        let s = smooth(GROW.0, GROW.1 + 0.25, t) * (1.0 - smooth(FALL_AT[0], BLOW + 0.3, t));
        if s <= 0.0 {
            return;
        }
        for &(x, y, a) in &self.night {
            p.blend(
                f64::from(x) + 0.5,
                f64::from(y) + 0.5,
                VOID[0],
                f64::from(a) * s,
            );
        }
    }

    /// The eye's stare: a dithered cone of violet light from the pupil down onto the foe, and the foe lit cold.
    fn stare(&self, p: &mut Painter, t: f64) {
        let o = smooth(OPEN.0, OPEN.1, t) * (1.0 - smooth(BLOW - 0.4, BLOW, t));
        if o <= 0.02 {
            return;
        }
        let top = f64::from(SKY.1) + 8.0;
        let mut y = libm::floor(top);
        while y < 0.0 {
            let u = (y - top) / -top;
            let half = 3.0 + 13.0 * u;
            let mut x = -libm::ceil(half);
            while x <= half {
                let edge = libm::fabs(x) / half;
                let keep = 0.7 - 0.5 * edge;
                if rand(&[x, y, self.seed, 3.0]) < keep {
                    p.light(x + 0.5, y + 0.5, VOID[4], 0.16 * o * (1.0 - 0.4 * u));
                }
                x += 1.0;
            }
            y += 1.0;
        }
        let head = -(RULER.height as f64) * 0.7;
        p.glow(0.0, head, 16.0, VOID[5], 0.45 * o);
    }

    /// The blow: a white flash, a ring of cold light thrown out from the foe and one along the ground.
    fn shock(&self, p: &mut Painter, t: f64) {
        let k = t - BLOW;
        if !(0.0..0.32).contains(&k) {
            return;
        }
        let mid = (BODY.0 + BODY.1) / 2.0;
        if k < 0.07 {
            p.glow(0.0, mid, 46.0, VOID[7], 0.9 * (1.0 - k / 0.07));
        }
        let u = k / 0.32;
        let r = 8.0 + 50.0 * libm::sqrt(u);
        let c = if u < 0.35 {
            VOID[7]
        } else if u < 0.7 {
            VOID[6]
        } else {
            VOID[5]
        };
        p.ring(0.0, mid, r, 2.0, c, 1.0 - 0.6 * u);
        p.glow_ring(0.0, mid, r, 4.0, VOID[5], 0.5 * (1.0 - u));
        // the same wave running out along the ground, flattened
        for i in 0..120 {
            let a = TAU * f64::from(i) / 120.0;
            p.px(libm::cos(a) * r * 1.3, -0.5 + libm::sin(a) * r * 0.2, c);
        }
        // spikes out from the blow, short and gone fast
        if k < 0.12 {
            for i in 0..10 {
                let a = TAU * f64::from(i) / 10.0 + 0.3;
                let (c0, s0) = (libm::cos(a), libm::sin(a));
                let (r0, r1) = (10.0 + 60.0 * k, 22.0 + 140.0 * k);
                p.line(c0 * r0, mid + s0 * r0, c0 * r1, mid + s0 * r1, VOID[7], 1.0);
            }
        }
    }

    /// The black drift round the feet: it piles up as the snow strikes, then sinks to a pool of shadow and fades.
    fn drift(&self, p: &mut Painter, halo: &mut Halo, t: f64) {
        let rise = smooth(BLOW - 0.05, BLOW + 0.3, t);
        if rise <= 0.0 {
            return;
        }
        let melt = smooth(MELT.0, MELT.1, t);
        if melt >= 1.0 {
            return;
        }
        let w = DRIFT_W * (1.0 - 0.4 * melt);
        let mut x = -libm::ceil(w);
        while x <= w {
            let u = x / w;
            let bump = 0.6 + 0.4 * fbm(x * 0.3, 0.0, self.seed, 2);
            let h = libm::floor(9.0 * (1.0 - u * u) * bump * rise * (1.0 - melt) + 0.5);
            if 1.0 - u * u > 0.0 {
                // the pool of shadow under it, fading as it melts
                p.blend(x + 0.5, -0.5, VOID[0], 0.75 * (1.0 - melt));
            }
            let mut y = 0.0;
            while y < h {
                let top = y + 1.0 >= h;
                let glint = top && rand(&[x, self.seed, 5.0]) < 0.25;
                p.px(
                    x + 0.5,
                    -y - 0.5,
                    if glint {
                        VOID[7]
                    } else if top {
                        VOID[5]
                    } else if y + 2.0 >= h {
                        VOID[3]
                    } else if rand(&[x, y, self.seed]) < 0.3 {
                        VOID[2]
                    } else {
                        VOID[1]
                    },
                );
                if top {
                    halo.stamp(x + 0.5, -y - 0.5, if glint { 1.6 } else { 0.9 }, 0);
                }
                y += 1.0;
            }
            x += 1.0;
        }
    }
}

/// The night round the sky: an oval a little past it, darkest in the middle, falling off in four solid steps with each
/// step's edge dithered into the next. Every pixel it darkens, with how dark (0..1).
fn night_mask() -> Vec<(i16, i16, f32)> {
    let (cx, cy) = (f64::from(SKY.0), f64::from(SKY.1) + HALF.1 * 0.2);
    let (rx, ry) = (HALF.0 * 1.15, HALF.1 * 1.5);
    let mut out = Vec::new();
    let (x0, x1) = (
        libm::floor(cx - rx * 1.2) as i32,
        libm::ceil(cx + rx * 1.2) as i32,
    );
    let (y0, y1) = (
        libm::floor(cy - ry * 1.2) as i32,
        libm::ceil(cy + ry * 1.2) as i32,
    );
    for y in y0.max(-(OY as i32))..=y1.min(-1) {
        for x in x0.max(-(OX as i32))..=x1.min(OX as i32 - 1) {
            let (fx, fy) = (f64::from(x), f64::from(y));
            let d = libm::hypot((fx + 0.5 - cx) / rx, (fy + 0.5 - cy) / ry);
            let q = (1.0 - smooth(0.35, 1.15, d)) * 4.0;
            let step = libm::floor(q);
            let up = if q - step > bayer4(fx, fy) { 1.0 } else { 0.0 };
            let a = 0.75 * (step + up) / 4.0;
            if a > 0.0 {
                out.push((x as i16, y as i16, a as f32));
            }
        }
    }
    out
}

/// A 4 by 4 ordered dither threshold, 0 to 1.
fn bayer4(x: f64, y: f64) -> f64 {
    const M: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    let xi = (libm::floor(x) as i64).rem_euclid(4) as usize;
    let yi = (libm::floor(y) as i64).rem_euclid(4) as usize;
    (f64::from(M[yi][xi]) + 0.5) / 16.0
}

/// The glow round every stroke, painted as steps of cold light (kind 0 and halo pixels): a pale step near the stroke,
/// then violet, then a dithered deep edge. Strokes keep their own colour; only their halo is light.
fn paint_halo(p: &mut Painter, halo: &Halo) {
    for (x, y, h, kind) in halo.each() {
        if kind > 0 && h >= 2.0 {
            continue; // the stroke itself, already drawn
        }
        let (c, k) = if h >= 3.0 {
            (VOID[7], 0.55)
        } else if h >= 2.0 {
            (VOID[6], 0.4)
        } else if h >= 1.0 {
            (VOID[5], 0.28)
        } else if (libm::floor(x) + libm::floor(y)) % 2.0 == 0.0 {
            (VOID[4], 0.18)
        } else {
            continue;
        };
        p.light(x + 0.5, y + 0.5, c, k);
    }
}
