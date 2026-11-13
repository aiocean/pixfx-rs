//! What hangs on a fighter besides its pose, drawn as layers over its sprite in world space at one pixel per game
//! pixel, feet at the given point.
//!
//! - [`draw_marks`]: a ward's shell round the body, a barrier dome, a mark over the head for each effect on it
//!   (a flame for a burn, a cracked shield for defence down, a seal for silence, a snowflake for a chill) and its
//!   fury pips. Translucent fills blend source-over into the frame's colour layer, as the duel's canvas does.
//! - [`draw_burning`], [`draw_shocked`], [`draw_frozen`]: a state shown on the body itself instead of a recoloured
//!   sprite: flames licking up it, sparks jumping round it, rime growing on it. Each is a function of time, a seed,
//!   the feet and an intensity (0 to 1), so the same call draws the same pixels; draw them after the sprite.
//!   They write opaque pixels to the colour layer and glow to the light layer.

use pixfx_core::blend::fill;
use pixfx_core::color::Rgba;
use pixfx_core::frame::Frame;
use pixfx_core::math::{clamp, js_round, rand};
use pixfx_core::painter::Painter;

/// The body the overlays sit on: 34 pixels tall from the feet to the top of the head (a topknot may rise above).
const BODY_H: i32 = 34;

/// How a barrier dome is patterned.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DomeStyle {
    /// Fixed bands across it, every 8 rows, rim to rim (a bell).
    #[default]
    Banded,
    /// Short facets drifting up it, every 9 rows (a mirror of ice).
    Faceted,
}

/// The marks on a fighter, as [`draw_marks`] draws them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marks {
    /// A ward's shell round the body.
    pub shield: bool,
    /// A barrier dome round the body.
    pub barrier: bool,
    /// A blow has just rung on the barrier: it flares white.
    pub rang: bool,
    pub burning: bool,
    pub def_down: bool,
    pub silence: bool,
    pub chilled: bool,
    /// Fury stacks, a pip each.
    pub fury: u32,
    /// Seconds into the current pose: drives the shell's and dome's flicker and the facets' drift.
    pub pose_t: f64,
    pub dome: DomeStyle,
    /// The dome's colour (the duel uses the fighter's look's glow).
    pub glow: Rgba,
}

impl Default for Marks {
    fn default() -> Self {
        Self {
            shield: false,
            barrier: false,
            rang: false,
            burning: false,
            def_down: false,
            silence: false,
            chilled: false,
            fury: 0,
            pose_t: 0.0,
            dome: DomeStyle::Banded,
            glow: WHITE,
        }
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
fn dome(frame: &mut Frame, m: &Marks, fx: i32, fy: i32) {
    let flare = m.rang;
    let edge = if flare { WHITE } else { m.glow };
    let a = if flare {
        0.95
    } else {
        0.55 + 0.2 * libm::sin(m.pose_t * 10.0)
    };
    let cy = fy - 20;
    let (rx, ry) = (15.0, 24_i32);
    let drift = js_round(m.pose_t * 12.0) as i32;
    for y in -ry..=ry {
        let yf = f64::from(y);
        let ryf = f64::from(ry);
        let half = js_round(rx * libm::sqrt(1.0 - (yf * yf) / (ryf * ryf))) as i32;
        // the shell's rim, and a faint fill inside
        fill(frame, fx - half, cy + y, 1, 1, edge, a);
        fill(frame, fx + half, cy + y, 1, 1, edge, a);
        if half > 1 {
            let inner = if flare { 0.22 } else { 0.07 };
            fill(frame, fx - half + 1, cy + y, half * 2 - 1, 1, m.glow, inner);
        }
        let hf = f64::from(half);
        match m.dome {
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

/// Draw `m` round a fighter whose feet are at (feet_x, feet_y), each rounded to a whole pixel.
pub fn draw_marks(frame: &mut Frame, m: &Marks, feet_x: f64, feet_y: f64) {
    let fx = js_round(feet_x) as i32;
    let fy = js_round(feet_y) as i32;
    if m.shield {
        // a shell of frost qi, flickering at the edges
        let a = 0.45 + 0.25 * libm::sin(m.pose_t * 9.0);
        fill(frame, fx - 11, fy - 41, 23, 1, SHELL, a);
        fill(frame, fx - 11, fy + 1, 23, 1, SHELL, a);
        fill(frame, fx - 12, fy - 40, 1, 41, SHELL, a);
        fill(frame, fx + 12, fy - 40, 1, 41, SHELL, a);
        fill(frame, fx - 11, fy - 40, 23, 41, SHELL, 0.08);
    }
    if m.barrier {
        dome(frame, m, fx, fy);
    }
    let shown = [m.burning, m.def_down, m.silence, m.chilled];
    let count = shown.iter().filter(|&&on| on).count() as f64;
    let mut x = fx - js_round((count * 6.0 - 1.0) / 2.0) as i32;
    let y = fy - 48;
    let px = |frame: &mut Frame, x: i32, y: i32, w: i32, h: i32, c: Rgba| {
        fill(frame, x, y, w, h, c, 1.0)
    };
    if m.burning {
        let lick = i32::from(libm::sin(m.pose_t * 14.0) > 0.0);
        px(frame, x + 1, y + 1, 3, 3, FLAME[2]);
        px(frame, x + 2 - lick, y - 1, 1, 2, FLAME[1]);
        px(frame, x + 2, y + 2, 1, 2, FLAME[0]);
        x += 6;
    }
    if m.def_down {
        // a shield with a crack through it
        px(frame, x, y - 1, 5, 4, BROKEN);
        px(frame, x + 1, y + 3, 3, 1, BROKEN);
        px(frame, x + 2, y + 4, 1, 1, BROKEN);
        px(frame, x + 2, y - 1, 1, 3, RED);
        x += 6;
    }
    if m.silence {
        // a seal over the lips: a ring with a bar through it
        px(frame, x, y, 5, 1, SEAL);
        px(frame, x, y + 4, 5, 1, SEAL);
        px(frame, x, y, 1, 5, SEAL);
        px(frame, x + 4, y, 1, 5, SEAL);
        px(frame, x + 1, y + 2, 3, 1, RED);
        x += 6;
    }
    if m.chilled {
        // a snowflake
        px(frame, x + 2, y - 1, 1, 5, SNOW);
        px(frame, x, y + 1, 5, 1, SNOW);
        px(frame, x + 1, y, 1, 1, SNOW);
        px(frame, x + 3, y + 2, 1, 1, SNOW);
    }
    // fury: a gold pip per stack over the head
    for i in 0..m.fury.min(64) as i32 {
        fill(frame, fx - 7 + i * 3, fy - 43, 2, 1, FURY, 0.9);
    }
}

// ---------- states on the body

/// Half the body's width at `h` pixels above the feet, as [`crate::paint`] draws a standing fighter: the robe
/// and arms up to the neck, then the head.
fn half_width(h: i32) -> i32 {
    if h < 24 { 7 } else { 4 }
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

/// Burning: tongues of flame licking up the body from the feet to the head, each born somewhere on the body,
/// rising and flickering out, with embers drifting off the top and the fire's glow in the light layer.
pub fn draw_burning(
    frame: &mut Frame,
    t: f64,
    seed: u32,
    feet_x: f64,
    feet_y: f64,
    intensity: f64,
) {
    let k = clamp(intensity, 0.0, 1.0);
    if k.is_nan() || k <= 0.0 {
        return;
    }
    let fx = js_round(feet_x) as i32;
    let fy = js_round(feet_y) as i32;
    let mut p = Painter::new(frame);
    // the fire's warm light on and round the body, breathing
    let breath = 0.75 + 0.25 * libm::sin(t * 23.0) * libm::sin(t * 7.3);
    p.glow(
        f64::from(fx) + 0.5,
        f64::from(fy - 14),
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
        let up = (pick(0) * pick(0) * 28.0) as i32;
        let hw = half_width(up);
        let bx = if pick(1) < 0.6 {
            let side = if pick(4) < 0.5 { -1 } else { 1 };
            fx + side * (hw - (pick(5) * 2.0) as i32)
        } else {
            fx - hw + 2 + (pick(5) * f64::from(2 * hw - 3)) as i32
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
        let x0 = f64::from(fx) - 5.0 + r(seed, [i, c, 10]) * 11.0;
        let x = js_round(x0 + libm::sin(t * 5.0 + f64::from(i as i32)) * 2.0 * u) as i32;
        let y = fy - 24 - (u * 20.0) as i32;
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

/// A point `d` steps round the body's outline (one pixel outside it), clockwise from the left foot.
fn round_body(fx: i32, fy: i32, d: i32) -> (i32, i32) {
    let side = BODY_H + 2;
    let top = 2 * half_width(BODY_H) + 3;
    let lap = 2 * side + top;
    let d = d.rem_euclid(lap);
    if d < side {
        let h = d;
        (fx - half_width(h) - 1, fy - h)
    } else if d < side + top {
        (fx - half_width(BODY_H) - 1 + (d - side), fy - BODY_H - 1)
    } else {
        let h = BODY_H - (d - side - top);
        (fx + half_width(h) + 1, fy - h)
    }
}

/// Shocked: arcs of electricity crackling between points round the body, a new set every jolt (1/20 s), now and
/// then none, with sparks popping off them and a cold flash in the light layer.
pub fn draw_shocked(
    frame: &mut Frame,
    t: f64,
    seed: u32,
    feet_x: f64,
    feet_y: f64,
    intensity: f64,
) {
    let k = clamp(intensity, 0.0, 1.0);
    if k.is_nan() || k <= 0.0 {
        return;
    }
    let fx = js_round(feet_x) as i32;
    let fy = js_round(feet_y) as i32;
    let jolt = libm::floor(t * 20.0) as i64;
    let mut p = Painter::new(frame);
    let arcs = js_round(1.0 + 2.0 * k) as i64;
    let lap = 2 * (BODY_H + 2) + 2 * half_width(BODY_H) + 3;
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
        let (ax, ay) = round_body(fx, fy, start);
        let (bx, by) = round_body(fx, fy, start + reach);
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
            f64::from(fx) + 0.5,
            f64::from(fy - 17),
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
pub fn draw_frozen(frame: &mut Frame, t: f64, seed: u32, feet_x: f64, feet_y: f64, intensity: f64) {
    let k = clamp(intensity, 0.0, 1.0);
    if k.is_nan() || k <= 0.0 {
        return;
    }
    let fx = js_round(feet_x) as i32;
    let fy = js_round(feet_y) as i32;
    let mut p = Painter::new(frame);
    p.glow(
        f64::from(fx) + 0.5,
        f64::from(fy - 17),
        20.0,
        ICE_LIGHT,
        0.12 * k,
    );
    // frost speckled over the body
    for h in 1..BODY_H {
        let hw = half_width(h);
        for x in -hw..=hw {
            if r(seed, [i64::from(x), i64::from(h), 1]) < 0.05 * k {
                p.put(f64::from(fx + x), f64::from(fy - h), ICE[2], 1.0);
            }
        }
    }
    // rime: crystals growing out of the sides, leaning up, and off the top of the head
    for side in [-1_i32, 1] {
        for h in 2..BODY_H {
            let roll = r(seed, [i64::from(side), i64::from(h), 2]);
            if roll >= 0.18 + 0.27 * k {
                continue;
            }
            let len = 1 + (r(seed, [i64::from(side), i64::from(h), 3]) * (1.0 + 2.5 * k)) as i32;
            let edge = fx + side * half_width(h);
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
    let hw = half_width(BODY_H);
    for x in -hw..=hw {
        if r(seed, [i64::from(x), 0, 4]) < 0.3 + 0.3 * k {
            let len = 1 + (r(seed, [i64::from(x), 1, 4]) * (1.0 + 2.0 * k)) as i32;
            for d in 0..len {
                let tone = if d + 1 == len { 0 } else { 2 };
                p.put(
                    f64::from(fx + x),
                    f64::from(fy - BODY_H - d),
                    ICE[tone],
                    1.0,
                );
            }
        }
    }
    // a crust of ice round the feet
    let crust = 2 + js_round(2.0 * k) as i32;
    for row in 0..crust {
        let half = 8 - row * 2 + (r(seed, [i64::from(row), 0, 5]) * 2.0) as i32;
        for x in -half..=half {
            let tone = if row + 1 == crust || x.abs() == half {
                1
            } else if (x + row) % 4 == 0 {
                2
            } else {
                3
            };
            p.put(f64::from(fx + x), f64::from(fy - row), ICE[tone], 1.0);
        }
    }
    // a glint: once a cycle a star lights on one crystal, swells and fades
    let cycle_len = 1.3;
    let run = t / cycle_len + r(seed, [0, 0, 6]);
    let c = libm::floor(run) as i64;
    let u = run - libm::floor(run);
    if u < 0.3 {
        let side = if r(seed, [c, 1, 6]) < 0.5 { -1 } else { 1 };
        let h = 4 + (r(seed, [c, 2, 6]) * 28.0) as i32;
        let (gx, gy) = (fx + side * (half_width(h) + 1), fy - h);
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
