//! Painter behaviour against pixelfx `src/core/painter.js`.

use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::frame::Frame;
use pixfx_core::painter::Painter;

fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Every ported Painter method, placed flipped and rotated with fade and light strength, against hashes
/// printed by node from pixelfx 044e12a (the same calls in JS; script in the task-3 report).
#[test]
fn scene_matches_pixelfx() -> Result<(), pixfx_core::ColorError> {
    use pixfx_core::{composite, hex};
    let (a, b, c) = (hex("#c9281f")?, hex("#ffc23d")?, hex("#3a62e0")?);
    let mut frame = Frame::new(40, 32, -4, -3);
    {
        let mut p = Painter::new(&mut frame);
        p.place(18.0, 14.0, true, 0.3);
        p.fade = 0.85;
        p.light_k = 1.2;
        p.ellipse(
            1.0,
            2.0,
            9.0,
            4.5,
            |d: f64, ang: f64| {
                if d > 0.8 {
                    a
                } else if libm::sin(ang) < 0.0 {
                    b
                } else {
                    Rgba::NONE
                }
            },
            |d: f64, _: f64| 1.0 - d * 0.6,
        );
        p.rect(-6.3, -8.6, 7.4, 3.2, c, 0.7);
        p.poly(
            &[(0.0, -10.0), (7.0, -3.0), (2.0, 5.0), (-5.0, 1.0)],
            |x: f64, y: f64| if x > y { a } else { c },
            1.0,
        );
        p.tri(-12.0, 6.0, -4.0, 9.0, -9.0, 14.0, b, 0.4);
        p.line(
            -14.0,
            -12.0,
            12.0,
            9.0,
            |u: f64| if u < 0.5 { a } else { b },
            |u: f64| 1.0 - u * 0.7,
        );
        p.dot(3.0, 3.0, WHITE, 0.45);
        p.px(-2.5, 7.9, WHITE);
        p.blend(4.0, -6.0, c, 0.6);
        p.blend(4.0, -6.0, a, 0.3);
        p.shade(-3.0, -3.0, 3.0, 3.0, |x, y, o| {
            if x * x + y * y > 7.0 {
                return None;
            }
            o.k = 0.3;
            o.l = if x > 0.0 { b } else { Rgba::NONE };
            Some(if x < 0.0 { a } else { Rgba::NONE })
        });
        p.glow(2.0, -2.0, 9.5, b, 0.6);
        p.glow_line(-10.0, 8.0, 10.0, -8.0, 3.5, a, 0.5);
        p.glow_ring(0.0, 0.0, 8.0, 3.0, c, 0.4);
        p.place(5.0, 5.0, false, 0.0);
        p.fade = 1.0;
        p.light_k = 1.0;
        p.rect(10.4, 12.5, 6.6, 4.49, a, 0.55);
        p.ellipse(20.0, 6.0, 3.0, 2.0, b, 1.0);
        p.line(0.0, 0.0, 0.0, 0.0, WHITE, 1.0);
    }
    assert_eq!(frame.color.iter().filter(|&&v| v != 0).count(), 163);
    assert_eq!(frame.light.iter().filter(|&&v| v != 0).count(), 430);
    let words = frame
        .color
        .iter()
        .chain(&frame.light)
        .flat_map(|w| w.to_le_bytes());
    assert_eq!(fnv1a(words), 0x3df4_0804_1601_a99b);
    let mut out = vec![0u8; 40 * 32 * 4];
    composite(&frame, &mut out, None);
    assert_eq!(fnv1a(out.iter().copied()), 0xa186_dc79_3dcd_5ef3);
    let ground = [0x10u8, 0x10, 0x18, 0xff].repeat(40 * 32);
    composite(&frame, &mut out, Some(&ground));
    assert_eq!(fnv1a(out.iter().copied()), 0xa6c4_c561_53a9_250c);
    Ok(())
}
