//! Shared by the water effects: the water ramp, droplets that fly over a water surface seen at a low angle and
//! land with rings, ripple trains, lacy foam, foam billows, wet stone. Files starting with _ are helpers.
//! The droplet sprites (a pixel, a shaded 2x2, a round bead with a highlight, each trailing a tail along its
//! motion) are adapted from Sprite Fusion's open-source blood (https://www.spritefusion.com/pixel-art-vfx/blood),
//! MIT License, Copyright (c) 2026 Hugo Duprez.
//!
//! Port of pixelfx `src/fx/water/_lib.js`: the functions water-jet uses (`WATER`, `bead`).

use pixfx_core::Painter;
use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::math::{js_min, or_one};

/// Water, deep to white: deep color only in shadow, tinted middles, white crests.
pub const WATER: [Rgba; 8] = [
    Rgba::hex24(0x0a1630),
    Rgba::hex24(0x123063),
    Rgba::hex24(0x1b5596),
    Rgba::hex24(0x2a7fc4),
    Rgba::hex24(0x45a9e3),
    Rgba::hex24(0x86d3f5),
    Rgba::hex24(0xcdf1ff),
    Rgba::hex24(0xffffff),
];

/// A droplet at (x, y) moving at (vx, vy) px/s: s 1 is a pixel, 2 a shaded 2x2, 3 a round bead with a
/// highlight; a tail trails back along its motion, longer the faster it goes. `c`: an 8-step water ramp.
/// JS default: `a = 1`.
#[expect(clippy::too_many_arguments, reason = "pixelfx signature")]
pub fn bead(p: &mut Painter, x: f64, y: f64, vx: f64, vy: f64, s: f64, c: &[Rgba; 8], a: f64) {
    let h = libm::hypot(vx, vy);
    let sp = or_one(h);
    let (ux, uy) = (vx / sp, vy / sp);
    let tail = js_min(5.0, sp * 0.011);
    if tail >= 1.0 {
        p.line(
            x - ux * (tail + 1.0),
            y - uy * (tail + 1.0),
            x - ux * 1.2,
            y - uy * 1.2,
            |u: f64| {
                if u < 0.45 {
                    c[3]
                } else if s > 1.0 {
                    c[5]
                } else {
                    c[4]
                }
            },
            a,
        );
    }
    if s >= 3.0 {
        p.circle(x, y, 1.75, c[4], a);
        p.circle(x - 0.25, y - 0.25, 1.15, c[6], a);
        p.dot(x - 0.7, y - 0.7, WHITE, a);
    } else if s >= 2.0 {
        let (bx, by) = (libm::floor(x), libm::floor(y));
        p.dot(bx, by, WHITE, a);
        p.dot(bx + 1.0, by, c[6], a);
        p.dot(bx, by + 1.0, c[5], a);
        p.dot(bx + 1.0, by + 1.0, c[4], a);
    } else {
        p.dot(x, y, if sp > 160.0 { c[6] } else { c[5] }, a);
    }
}
