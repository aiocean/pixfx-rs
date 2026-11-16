//! The teardrops `impacts/star-converge` (four), `impacts/twin-converge` (two) and `projectiles/drop-dart` (one)
//! share: drawn by hand from "Free pack, Part 1/24.png", "23.png" and "05.png", whose drops are the
//! same pixels (05 moved 8 right and 4 up).

use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;

use crate::tone::{Ink, Knot, curve, pose};

/// Seconds a moment of the sheet lasts.
pub const MOMENT: f64 = 0.06;
/// Moments from the first glint to the last fleck.
pub const MOMENTS: f64 = 14.0;

/// The top teardrop at each moment, outer end, body, inner end: sliding in, swelling, swinging round toward the
/// upper left, meeting the others in the star, then a thin blade.
const DROP: [[Knot; 3]; 14] = [
    [
        (-6.0, -35.0, 0.5, 0.0),
        (-6.0, -33.0, 0.6, 0.0),
        (-5.5, -31.0, 0.5, 0.0),
    ],
    [
        (-6.5, -31.5, 0.6, 3.0),
        (-6.0, -29.5, 1.6, 7.0),
        (-5.5, -27.5, 0.6, 3.0),
    ],
    [
        (-6.0, -31.0, 1.5, 9.0),
        (-5.5, -28.5, 2.8, 12.0),
        (-5.0, -23.5, 0.5, 3.0),
    ],
    [
        (-6.5, -31.0, 1.0, 6.0),
        (-6.0, -25.5, 3.0, 13.0),
        (-4.0, -19.5, 0.6, 4.0),
    ],
    [
        (-7.0, -31.5, 0.5, 1.0),
        (-5.0, -21.5, 3.0, 13.0),
        (-5.5, -12.5, 0.5, 1.0),
    ],
    [
        (-7.5, -29.5, 0.5, 0.0),
        (-4.5, -15.0, 3.5, 15.0),
        (-8.0, -9.5, 0.8, 4.0),
    ],
    [
        (-6.5, -27.0, 0.5, 0.0),
        (-5.0, -13.0, 3.2, 14.0),
        (-6.0, -8.0, 0.8, 4.0),
    ],
    [
        (-9.0, -22.0, 0.5, 1.0),
        (-5.5, -12.0, 3.5, 15.0),
        (-3.5, -6.0, 0.8, 4.0),
    ],
    [
        (-13.0, -14.0, 0.5, 3.0),
        (-7.0, -7.0, 2.0, 13.0),
        (-1.0, -1.0, 1.5, 12.0),
    ],
    [
        (-10.0, -12.0, 0.5, 4.0),
        (-4.0, -4.0, 3.0, 15.0),
        (0.0, 0.0, 3.0, 15.0),
    ],
    [
        (5.5, -13.5, 0.5, 1.0),
        (2.5, -8.5, 1.2, 4.0),
        (0.5, -4.5, 0.5, 2.0),
    ],
    [
        (6.0, -18.0, 0.5, 1.0),
        (4.5, -13.0, 1.0, 3.0),
        (2.0, -7.0, 0.5, 1.0),
    ],
    [
        (5.0, -19.5, 0.5, 1.0),
        (6.0, -16.0, 1.0, 2.0),
        (4.5, -12.0, 0.5, 1.0),
    ],
    [
        (5.0, -18.0, 0.5, 0.0),
        (5.5, -16.5, 0.5, 1.0),
        (5.0, -15.0, 0.5, 0.0),
    ],
];

/// Paint the drops at moment `s`: `fold` of them a whole turn apart round the origin (4: top, right, bottom, left),
/// the first turned `quarters` quarter turns clockwise from the top, all moved by `shift` pixels.
pub fn paint(p: &mut Painter, ramp: &[Rgba], s: f64, fold: u32, quarters: u32, shift: (f64, f64)) {
    let mut drop = pose(&DROP, s);
    for k in &mut drop {
        let (x, y) = match quarters % 4 {
            1 => (-k.1, k.0),
            2 => (-k.0, -k.1),
            3 => (k.1, -k.0),
            _ => (k.0, k.1),
        };
        *k = (x + shift.0, y + shift.1, k.2, k.3);
    }
    let mut ink = Ink::new(fold).grain(0.15, 0.7, 24.0);
    ink.path(&drop);
    // the star flares where they meet (light on its own pixels only, nothing outside them)
    let flare = curve(&[(7.0, 0.0), (9.0, 1.0), (10.5, 0.0)], s);
    ink.paint(p, ramp, 32.0, 0.08 * (1.0 + 2.0 * flare));
}
