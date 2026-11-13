//! What hangs on a fighter besides its pose, drawn as layers over its sprite in world space at one pixel per game
//! pixel, feet at the given point.
//!
//! - [`draw_marks`]: a list of [`Mark`]s drawn in its order: a ward's shell round the body, a barrier dome, a sign
//!   over the head for each effect on it (a flame for a burn, a cracked shield for defence down, a seal for silence,
//!   a snowflake for a chill), lined up and centred over the head, and fury pips. Translucent fills blend
//!   source-over into the frame's colour layer, as the duel's canvas does.
//! - [`draw_burning`], [`draw_shocked`], [`draw_frozen`]: a state shown on the body itself instead of a recoloured
//!   sprite: flames licking up it, sparks jumping round it, rime growing on it. Each follows the body's
//!   [`Silhouette`] (taken from its pose) and is a function of time, a seed, the [`Figure`] and an intensity (0 to
//!   1), so the same call draws the same pixels; draw them after the sprite. They write opaque pixels to the colour
//!   layer and glow to the light layer.

use crate::pose::Pose;
use pixfx_core::blend::fill;
use pixfx_core::color::Rgba;
use pixfx_core::frame::Frame;
use pixfx_core::math::{clamp, js_round, rand};
use pixfx_core::painter::Painter;

/// How a barrier dome is patterned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DomeStyle {
    /// Fixed bands across it, every 8 rows, rim to rim (a bell).
    #[default]
    Banded,
    /// Short facets drifting up it, every 9 rows (a mirror of ice).
    Faceted,
}

/// One thing hanging on a fighter, as [`draw_marks`] draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    /// A ward's shell round the body.
    Shield,
    /// A barrier dome round the body in colour `glow` (the duel uses the fighter's look's glow), flaring white
    /// when a blow has just `rang` on it.
    Barrier {
        style: DomeStyle,
        glow: Rgba,
        rang: bool,
    },
    /// Signs over the head, one each, lined up in the list's order.
    Burning,
    DefDown,
    Silence,
    Chilled,
    /// Fury stacks, a pip each.
    Fury(u32),
}

impl Mark {
    /// A sign over the head (lined up with the others), rather than something round the body.
    fn sign(&self) -> bool {
        matches!(
            self,
            Mark::Burning | Mark::DefDown | Mark::Silence | Mark::Chilled
        )
    }
}

const WHITE: Rgba = Rgba::hex24(0xffffff);
const SHELL: Rgba = Rgba::hex24(0xa6ecff);
const FLAME: [Rgba; 4] = [
    Rgba::hex24(0xfff0b0),
    Rgba::hex24(0xffd84a),
    Rgba::hex24(0xff9a3a),
    Rgba::hex24(0xd2401c),
];
const BROKEN: Rgba = Rgba::hex24(0x8f8aa8);
const RED: Rgba = Rgba::hex24(0xff4a3a);
const SNOW: Rgba = Rgba::hex24(0xbff2ff);
const SEAL: Rgba = Rgba::hex24(0xc9c3e6);
const FURY: Rgba = Rgba::hex24(0xffc23d);

/// A barrier: a dome round the body, faint while it waits and flaring white where a blow has just rung on it.
fn dome(
    frame: &mut Frame,
    style: DomeStyle,
    glow: Rgba,
    flare: bool,
    pose_t: f64,
    fx: i32,
    fy: i32,
) {
    let edge = if flare { WHITE } else { glow };
    let a = if flare {
        0.95
    } else {
        0.55 + 0.2 * libm::sin(pose_t * 10.0)
    };
    let cy = fy - 20;
    let (rx, ry) = (15.0, 24_i32);
    let drift = js_round(pose_t * 12.0) as i32;
    for y in -ry..=ry {
        let yf = f64::from(y);
        let ryf = f64::from(ry);
        let half = js_round(rx * libm::sqrt(1.0 - (yf * yf) / (ryf * ryf))) as i32;
        // the shell's rim, and a faint fill inside
        fill(frame, fx - half, cy + y, 1, 1, edge, a);
        fill(frame, fx + half, cy + y, 1, 1, edge, a);
        if half > 1 {
            let inner = if flare { 0.22 } else { 0.07 };
            fill(frame, fx - half + 1, cy + y, half * 2 - 1, 1, glow, inner);
        }
        let hf = f64::from(half);
        match style {
            DomeStyle::Banded if y % 8 == 0 => {
                fill(frame, fx - half, cy + y, half * 2 + 1, 1, edge, a * 0.45);
            }
            DomeStyle::Faceted if (y + drift) % 9 == 0 => {
                let w = (js_round(hf * 0.8) as i32).max(1);
                fill(
                    frame,
                    fx - js_round(hf * 0.4) as i32,
                    cy + y,
                    w,
                    1,
                    edge,
                    a * 0.45,
                );
            }
            _ => {}
        }
    }
    fill(frame, fx - 3, cy - ry, 7, 1, edge, a);
    fill(frame, fx - 3, cy + ry, 7, 1, edge, a);
}

/// Draw `marks` in their order round a fighter whose feet are at (feet_x, feet_y), each rounded to a whole pixel;
/// `pose_t`, seconds into its pose, drives the shell's and dome's flicker, the facets' drift and the flame's lick.
pub fn draw_marks(frame: &mut Frame, marks: &[Mark], pose_t: f64, feet_x: f64, feet_y: f64) {
    let fx = js_round(feet_x) as i32;
    let fy = js_round(feet_y) as i32;
    let count = marks.iter().filter(|m| m.sign()).count() as f64;
    let mut x = fx - js_round((count * 6.0 - 1.0) / 2.0) as i32;
    let y = fy - 48;
    let px = |frame: &mut Frame, x: i32, y: i32, w: i32, h: i32, c: Rgba| {
        fill(frame, x, y, w, h, c, 1.0)
    };
    for &m in marks {
        match m {
            Mark::Shield => {
                // a shell of frost qi, flickering at the edges
                let a = 0.45 + 0.25 * libm::sin(pose_t * 9.0);
                fill(frame, fx - 11, fy - 41, 23, 1, SHELL, a);
                fill(frame, fx - 11, fy + 1, 23, 1, SHELL, a);
                fill(frame, fx - 12, fy - 40, 1, 41, SHELL, a);
                fill(frame, fx + 12, fy - 40, 1, 41, SHELL, a);
                fill(frame, fx - 11, fy - 40, 23, 41, SHELL, 0.08);
            }
            Mark::Barrier { style, glow, rang } => dome(frame, style, glow, rang, pose_t, fx, fy),
            Mark::Burning => {
                let lick = i32::from(libm::sin(pose_t * 14.0) > 0.0);
                px(frame, x + 1, y + 1, 3, 3, FLAME[2]);
                px(frame, x + 2 - lick, y - 1, 1, 2, FLAME[1]);
                px(frame, x + 2, y + 2, 1, 2, FLAME[0]);
            }
            Mark::DefDown => {
                // a shield with a crack through it
                px(frame, x, y - 1, 5, 4, BROKEN);
                px(frame, x + 1, y + 3, 3, 1, BROKEN);
                px(frame, x + 2, y + 4, 1, 1, BROKEN);
                px(frame, x + 2, y - 1, 1, 3, RED);
            }
            Mark::Silence => {
                // a seal over the lips: a ring with a bar through it
                px(frame, x, y, 5, 1, SEAL);
                px(frame, x, y + 4, 5, 1, SEAL);
                px(frame, x, y, 1, 5, SEAL);
                px(frame, x + 4, y, 1, 5, SEAL);
                px(frame, x + 1, y + 2, 3, 1, RED);
            }
            Mark::Chilled => {
                // a snowflake
                px(frame, x + 2, y - 1, 1, 5, SNOW);
                px(frame, x, y + 1, 5, 1, SNOW);
                px(frame, x + 1, y, 1, 1, SNOW);
                px(frame, x + 3, y + 2, 1, 1, SNOW);
            }
            Mark::Fury(n) => {
                // a gold pip per stack over the head
                for i in 0..n.min(64) as i32 {
                    fill(frame, fx - 7 + i * 3, fy - 43, 2, 1, FURY, 0.9);
                }
            }
        }
        if m.sign() {
            x += 6;
        }
    }
}

// ---------- states on the body

/// The rough shape of a fighter's body, taken from its pose, that fire, sparks and ice follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Silhouette {
    /// Upright on its feet or in the air, 34 pixels tall: the robe and arms up to the neck, then the head.
    Standing,
    /// Low: ducking, sitting up, sitting cross-legged or tucked in a roll, about 20 pixels tall, reaching forward.
    Crouched,
    /// Flat on its back on the floor, head behind it, feet in front.
    Lying,
    /// Thrown and flat in the air: lying, a few pixels above where its feet would be.
    Airborne,
    /// Spinning head over heels about its middle.
    Tumbling,
}

impl Silhouette {
    /// The shape of a fighter in `pose`, as [`crate::paint`] draws it.
    pub fn of(pose: Pose) -> Silhouette {
        match pose {
            Pose::Duck | Pose::Sit | Pose::Meditate | Pose::Roll => Silhouette::Crouched,
            Pose::Down => Silhouette::Lying,
            Pose::Launched | Pose::Fall => Silhouette::Airborne,
            Pose::Tumble => Silhouette::Tumbling,
            Pose::Idle
            | Pose::Walk
            | Pose::Jump
            | Pose::Fly
            | Pose::Land
            | Pose::Crouch
            | Pose::Kneel
            | Pose::Draw
            | Pose::Slash
            | Pose::Channel
            | Pose::Cast
            | Pose::Guard
            | Pose::Hurt
            | Pose::Frozen
            | Pose::Shocked
            | Pose::Win => Silhouette::Standing,
        }
    }

    /// Its bands from the bottom up: from `h` pixels above the feet, the body spans `back` to `front` pixels
    /// along the way it faces (back negative). The last band reaches the top.
    fn bands(self) -> &'static [Band] {
        match self {
            Silhouette::Standing => &[Band(0, -7, 7), Band(24, -4, 4)],
            Silhouette::Crouched => &[Band(0, -7, 10), Band(8, -6, 6)],
            Silhouette::Lying => &[Band(0, -22, 14)],
            Silhouette::Airborne => &[Band(6, -22, 14)],
            Silhouette::Tumbling => &[Band(2, -6, 6), Band(8, -10, 10), Band(28, -6, 6)],
        }
    }

    /// Height of the top of the body above the feet.
    fn top(self) -> i32 {
        match self {
            Silhouette::Standing => 34,
            Silhouette::Crouched => 20,
            Silhouette::Lying => 6,
            Silhouette::Airborne => 12,
            Silhouette::Tumbling => 34,
        }
    }
}

/// A band of a [`Silhouette`]: from height, back, front.
#[derive(Clone, Copy, Debug)]
struct Band(i32, i32, i32);

/// A fighter as the states on its body see it: its feet in world pixels, which way it faces (1 right, -1 left)
/// and its silhouette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Figure {
    pub feet_x: f64,
    pub feet_y: f64,
    pub face: f64,
    pub silhouette: Silhouette,
}

/// A figure placed in whole pixels: the feet (`fx`, `fy`) and its shape's numbers, everything relative to the
/// feet, x to the right on screen.
struct Shape {
    fx: i32,
    fy: i32,
    sil: Silhouette,
    flip: bool,
    /// Lowest and highest row of the body above the feet.
    bottom: i32,
    top: i32,
}

impl Shape {
    fn of(f: &Figure) -> Shape {
        let sil = f.silhouette;
        Shape {
            fx: js_round(f.feet_x) as i32,
            fy: js_round(f.feet_y) as i32,
            sil,
            flip: f.face < 0.0,
            bottom: sil.bands().first().map_or(0, |b| b.0),
            top: sil.top(),
        }
    }

    /// The body's left and right edge at `h` above the feet, relative to the feet (below the lowest band, the
    /// lowest band's).
    fn span(&self, h: i32) -> (i32, i32) {
        let bands = self.sil.bands();
        let band = bands
            .iter()
            .rev()
            .find(|b| b.0 <= h)
            .or(bands.first())
            .map_or((0, 0), |b| (b.1, b.2));
        if self.flip { (-band.1, -band.0) } else { band }
    }

    /// The middle of the body, relative to the feet: x across, h up.
    fn middle(&self) -> (i32, i32) {
        let (l, r) = self.span(self.bottom);
        ((l + r) / 2, (self.bottom + self.top) / 2)
    }
}

/// A value in [0, 1) from a seed and integers.
fn r(seed: u32, ns: [i64; 3]) -> f64 {
    rand(&[f64::from(seed), ns[0] as f64, ns[1] as f64, ns[2] as f64])
}

/// One opaque pixel and its share of light.
fn spot(p: &mut Painter<'_>, x: i32, y: i32, c: Rgba, light: Rgba, k: f64) {
    let (x, y) = (f64::from(x), f64::from(y));
    p.put(x, y, c, 1.0);
    p.add(x, y, light, k);
}

const FIRE_LIGHT: Rgba = Rgba(255, 110, 30, 255);

/// Burning: tongues of flame licking up the body from its bottom to its top, each born somewhere on the body,
/// rising and flickering out, with embers drifting off the top and the fire's glow in the light layer.
pub fn draw_burning(frame: &mut Frame, t: f64, seed: u32, at: &Figure, intensity: f64) {
    let k = clamp(intensity, 0.0, 1.0);
    if k.is_nan() || k <= 0.0 {
        return;
    }
    let shape = Shape::of(at);
    let (fx, fy) = (shape.fx, shape.fy);
    let (mx, mh) = shape.middle();
    let mut p = Painter::new(frame);
    // the fire's warm light on and round the body, breathing; a little below its middle, as fire climbs
    let breath = 0.75 + 0.25 * libm::sin(t * 23.0) * libm::sin(t * 7.3);
    p.glow(
        f64::from(fx + mx) + 0.5,
        f64::from(fy - (mh - 3)),
        22.0,
        FIRE_LIGHT,
        0.45 * k * breath,
    );
    let tongues = js_round(6.0 + 6.0 * k) as i64;
    for i in 0..tongues {
        let life = 0.35 + 0.25 * r(seed, [i, 0, 0]);
        let run = t / life + r(seed, [i, 1, 0]);
        let cycle = libm::floor(run);
        let u = run - cycle;
        let c = cycle as i64;
        let pick = |j: i64| r(seed, [i, c, j + 2]);
        // where it starts: mostly low on the body (fire climbs), mostly at its edges so it breaks the outline
        let up =
            shape.bottom + (pick(0) * pick(0) * f64::from(shape.top - shape.bottom - 6)) as i32;
        let (left, right) = shape.span(up);
        let bx = if pick(1) < 0.6 {
            let inset = (pick(5) * 2.0) as i32;
            if pick(4) < 0.5 {
                fx + left + inset
            } else {
                fx + right - inset
            }
        } else {
            fx + left + 2 + (pick(5) * f64::from(right - left - 3)) as i32
        };
        let base = fy - 1 - up - (u * 6.0) as i32;
        let tall = (5.0 + pick(2) * 7.0) * (0.6 + 0.4 * k);
        let h = js_round(tall * libm::sqrt(libm::sin(core::f64::consts::PI * u))) as i32;
        if h < 2 {
            continue;
        }
        let phase = pick(3) * 6.0;
        let lean = if pick(6) < 0.5 { -1 } else { 1 };
        for row in 0..h {
            let f = f64::from(row) / f64::from(h);
            // the tongue sways more toward its tip
            let sway =
                js_round(libm::sin(t * 13.0 + phase + f64::from(row) * 0.8) * f * 1.6) as i32;
            let (x, y) = (bx + sway, base - row);
            if f < 0.25 {
                // the broad root: white-hot at the very bottom, yellow above, orange sides
                let core = if row == 0 { FLAME[0] } else { FLAME[1] };
                spot(&mut p, x - 1, y, FLAME[2], FIRE_LIGHT, 0.1 * k);
                spot(&mut p, x + 1, y, FLAME[2], FIRE_LIGHT, 0.1 * k);
                spot(&mut p, x, y, core, FIRE_LIGHT, 0.25 * k);
            } else if f < 0.55 {
                spot(&mut p, x + lean, y, FLAME[2], FIRE_LIGHT, 0.1 * k);
                spot(&mut p, x, y, FLAME[1], FIRE_LIGHT, 0.18 * k);
            } else if f < 0.85 {
                spot(&mut p, x, y, FLAME[2], FIRE_LIGHT, 0.12 * k);
            } else {
                spot(&mut p, x, y, FLAME[3], FIRE_LIGHT, 0.08 * k);
            }
        }
    }
    // embers thrown off the top, rising and cooling
    let embers = js_round(1.0 + 3.0 * k) as i64;
    for i in 0..embers {
        let life = 0.7 + 0.4 * r(seed, [i, 0, 9]);
        let run = t / life + r(seed, [i, 1, 9]);
        let cycle = libm::floor(run);
        let u = run - cycle;
        let c = cycle as i64;
        let x0 = f64::from(fx + mx) - 5.0 + r(seed, [i, c, 10]) * 11.0;
        let x = js_round(x0 + libm::sin(t * 5.0 + f64::from(i as i32)) * 2.0 * u) as i32;
        let y = fy - (shape.top - 10).max(mh) - (u * 20.0) as i32;
        let col = if u < 0.5 { FLAME[1] } else { FLAME[3] };
        spot(&mut p, x, y, col, FIRE_LIGHT, 0.5 * k * (1.0 - u));
    }
}

const SPARK: [Rgba; 3] = [
    Rgba::hex24(0xffffff),
    Rgba::hex24(0xfff6a0),
    Rgba::hex24(0x9fdcff),
];
const SPARK_LIGHT: Rgba = Rgba(150, 200, 255, 255);

impl Shape {
    /// The steps up one side of the outline, and across its top.
    fn sides(&self) -> (i32, i32) {
        let (l, r) = self.span(self.top);
        (self.top - self.bottom + 2, r - l + 3)
    }

    /// The steps once round the outline.
    fn lap(&self) -> i32 {
        let (side, top) = self.sides();
        2 * side + top
    }

    /// A point `d` steps round the body's outline (one pixel outside it), clockwise from the bottom left.
    fn round(&self, d: i32) -> (i32, i32) {
        let (fx, fy) = (self.fx, self.fy);
        let (side, top) = self.sides();
        let d = d.rem_euclid(self.lap());
        if d < side {
            let h = self.bottom + d;
            (fx + self.span(h).0 - 1, fy - h)
        } else if d < side + top {
            (
                fx + self.span(self.top).0 - 1 + (d - side),
                fy - self.top - 1,
            )
        } else {
            let h = self.top - (d - side - top);
            (fx + self.span(h).1 + 1, fy - h)
        }
    }
}

/// Shocked: arcs of electricity crackling between points round the body, a new set every jolt (1/20 s), now and
/// then none, with sparks popping off them and a cold flash in the light layer.
pub fn draw_shocked(frame: &mut Frame, t: f64, seed: u32, at: &Figure, intensity: f64) {
    let k = clamp(intensity, 0.0, 1.0);
    if k.is_nan() || k <= 0.0 {
        return;
    }
    let shape = Shape::of(at);
    let (fx, fy) = (shape.fx, shape.fy);
    let (mx, mh) = shape.middle();
    let jolt = libm::floor(t * 20.0) as i64;
    let mut p = Painter::new(frame);
    let arcs = js_round(1.0 + 2.0 * k) as i64;
    let lap = shape.lap();
    let mut lit = false;
    for j in 0..arcs {
        let pick = |n: i64| r(seed, [jolt, j, n]);
        // a gap between jolts now and then: the crackle stutters
        if pick(0) < 0.25 {
            continue;
        }
        lit = true;
        let start = (pick(1) * f64::from(lap)) as i32;
        let reach = 7 + (pick(2) * 14.0) as i32;
        let (ax, ay) = shape.round(start);
        let (bx, by) = shape.round(start + reach);
        // a crooked path: a few kinks pushed off the straight line
        let kinks = 3;
        let mut prev = (ax, ay);
        for s in 1..=kinks + 1 {
            let f = f64::from(s) / f64::from(kinks + 1);
            let mut x = js_round(f64::from(ax) + f64::from(bx - ax) * f) as i32;
            let mut y = js_round(f64::from(ay) + f64::from(by - ay) * f) as i32;
            if s <= kinks {
                x += js_round((pick(3 + i64::from(s)) - 0.5) * 6.0) as i32;
                // never below the floor the feet stand on
                y = (y + js_round((pick(9 + i64::from(s)) - 0.5) * 6.0) as i32).min(fy);
            }
            bolt(&mut p, prev, (x, y), k);
            prev = (x, y);
        }
        // sparks popping off the ends
        for (n, (sx, sy)) in [(ax, ay), (bx, by)].into_iter().enumerate() {
            let n = n as i64;
            let dx = js_round((pick(20 + n) - 0.5) * 6.0) as i32;
            let dy = js_round((pick(22 + n) - 0.5) * 6.0) as i32;
            spot(
                &mut p,
                sx + dx,
                (sy + dy).min(fy),
                SPARK[2],
                SPARK_LIGHT,
                0.6 * k,
            );
        }
    }
    if lit {
        let flash = 0.12 + 0.12 * r(seed, [jolt, 99, 0]);
        p.glow(
            f64::from(fx + mx) + 0.5,
            f64::from(fy - mh),
            20.0,
            SPARK_LIGHT,
            flash * k,
        );
    }
}

/// One straight piece of an arc: a white-hot core with a pale yellow fringe and a blue halo of light.
fn bolt(p: &mut Painter<'_>, a: (i32, i32), b: (i32, i32), k: f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let n = dx.abs().max(dy.abs()).max(1);
    for s in 0..=n {
        let f = f64::from(s) / f64::from(n);
        let x = js_round(f64::from(a.0) + f64::from(dx) * f) as i32;
        let y = js_round(f64::from(a.1) + f64::from(dy) * f) as i32;
        spot(p, x, y, SPARK[0], SPARK_LIGHT, 0.9 * k);
        // the fringe on the side away from the line's direction
        let (ox, oy) = if dx.abs() >= dy.abs() { (0, 1) } else { (1, 0) };
        p.add(f64::from(x + ox), f64::from(y + oy), SPARK_LIGHT, 0.45 * k);
        p.add(f64::from(x - ox), f64::from(y - oy), SPARK_LIGHT, 0.45 * k);
    }
    let (cx0, cy0, cx1, cy1) = (
        f64::from(a.0) + 0.5,
        f64::from(a.1) + 0.5,
        f64::from(b.0) + 0.5,
        f64::from(b.1) + 0.5,
    );
    p.glow_line(cx0, cy0, cx1, cy1, 3.0, SPARK_LIGHT, 0.3 * k);
    // the fringe: the middle pixel of each piece is drawn yellow next to the core
    let (mx, my) = ((a.0 + b.0) / 2, (a.1 + b.1) / 2);
    p.put(f64::from(mx + 1), f64::from(my), SPARK[1], 1.0);
}

const ICE: [Rgba; 4] = [
    Rgba::hex24(0xffffff),
    Rgba::hex24(0xe8fbff),
    Rgba::hex24(0xa6ecff),
    Rgba::hex24(0x5fa8d8),
];
const ICE_LIGHT: Rgba = Rgba(120, 200, 255, 255);

/// Frozen: rime crystals bristling off the body's sides and head, frost speckled over it, a crust of ice round
/// the feet, a faint cold light, and a glint that wakes on a crystal now and then. The ice stays where it grew
/// (it depends on the seed, not the time); only the glint moves.
pub fn draw_frozen(frame: &mut Frame, t: f64, seed: u32, at: &Figure, intensity: f64) {
    let k = clamp(intensity, 0.0, 1.0);
    if k.is_nan() || k <= 0.0 {
        return;
    }
    let shape = Shape::of(at);
    let (fx, fy) = (shape.fx, shape.fy);
    let (bottom, top) = (shape.bottom, shape.top);
    let (mx, mh) = shape.middle();
    let mut p = Painter::new(frame);
    p.glow(
        f64::from(fx + mx) + 0.5,
        f64::from(fy - mh),
        20.0,
        ICE_LIGHT,
        0.12 * k,
    );
    // frost speckled over the body
    for h in bottom + 1..top {
        let (left, right) = shape.span(h);
        for x in left..=right {
            if r(seed, [i64::from(x), i64::from(h), 1]) < 0.05 * k {
                p.put(f64::from(fx + x), f64::from(fy - h), ICE[2], 1.0);
            }
        }
    }
    // rime: crystals growing out of the sides, leaning up, and off the top of the head
    for side in [-1_i32, 1] {
        for h in bottom + 2..top {
            let roll = r(seed, [i64::from(side), i64::from(h), 2]);
            if roll >= 0.18 + 0.27 * k {
                continue;
            }
            let len = 1 + (r(seed, [i64::from(side), i64::from(h), 3]) * (1.0 + 2.5 * k)) as i32;
            let (left, right) = shape.span(h);
            let edge = fx + if side < 0 { left } else { right };
            for d in 0..=len {
                let tone = if d == len {
                    0
                } else if d == 0 {
                    3
                } else {
                    2
                };
                // every second pixel steps up a row: crystals point up and out
                p.put(
                    f64::from(edge + side * d),
                    f64::from(fy - h - d / 2),
                    ICE[tone],
                    1.0,
                );
            }
        }
    }
    let (left, right) = shape.span(top);
    for x in left..=right {
        if r(seed, [i64::from(x), 0, 4]) < 0.3 + 0.3 * k {
            let len = 1 + (r(seed, [i64::from(x), 1, 4]) * (1.0 + 2.0 * k)) as i32;
            for d in 0..len {
                let tone = if d + 1 == len { 0 } else { 2 };
                p.put(f64::from(fx + x), f64::from(fy - top - d), ICE[tone], 1.0);
            }
        }
    }
    // a crust of ice round the bottom of the body
    let (left, right) = shape.span(bottom);
    let (cx, reach) = ((left + right) / 2, (right - left) / 2 + 1);
    let crust = 2 + js_round(2.0 * k) as i32;
    for row in 0..crust {
        let half = reach - row * 2 + (r(seed, [i64::from(row), 0, 5]) * 2.0) as i32;
        for x in -half..=half {
            let tone = if row + 1 == crust || x.abs() == half {
                1
            } else if (x + row) % 4 == 0 {
                2
            } else {
                3
            };
            p.put(
                f64::from(fx + cx + x),
                f64::from(fy - bottom - row),
                ICE[tone],
                1.0,
            );
        }
    }
    // a glint: once a cycle a star lights on one crystal, swells and fades
    let cycle_len = 1.3;
    let run = t / cycle_len + r(seed, [0, 0, 6]);
    let c = libm::floor(run) as i64;
    let u = run - libm::floor(run);
    if u < 0.3 {
        let side = if r(seed, [c, 1, 6]) < 0.5 { -1 } else { 1 };
        let h = bottom + 4 + (r(seed, [c, 2, 6]) * f64::from(top - bottom - 6)) as i32;
        let (left, right) = shape.span(h);
        let gx = fx + if side < 0 { left - 1 } else { right + 1 };
        let gy = fy - h;
        let s = libm::sin(core::f64::consts::PI * u / 0.3);
        let arm = js_round(s * 2.0) as i32;
        spot(&mut p, gx, gy, ICE[0], ICE_LIGHT, 0.9 * k * s);
        for d in 1..=arm {
            for (dx, dy) in [(d, 0), (-d, 0), (0, d), (0, -d)] {
                spot(
                    &mut p,
                    gx + dx,
                    gy + dy,
                    ICE[if d == arm { 2 } else { 0 }],
                    ICE_LIGHT,
                    0.5 * k * s,
                );
            }
        }
    }
}
