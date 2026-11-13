//! Painter behaviour against pixelfx `src/core/painter.js`.

use pixfx_core::color::{Rgba, WHITE};
use pixfx_core::frame::Frame;
use pixfx_core::painter::Painter;

/// pixelfx `add` stores `Math.min(255, d + c * k) | 0`: light channels truncate, they do not round.
fn js_glow_value(cx: f64, cy: f64, r: f64, k: f64, sx: f64, sy: f64) -> u32 {
    let dx = (sx + 0.5 - cx) / r;
    let dy = (sy + 0.5 - cy) / r;
    let d = dx * dx + dy * dy;
    if d >= 1.0 {
        return 0;
    }
    let f = 1.0 - libm::sqrt(d);
    let kk = f * f * k;
    if kk <= 0.004 {
        return 0;
    }
    (255.0 * kk).min(255.0) as u32
}

#[test]
fn glow_peak_and_falloff() {
    let mut frame = Frame::new(16, 16, 0, 0);
    Painter::new(&mut frame).glow(8.0, 8.0, 4.0, WHITE, 0.5);
    for y in 0..16u32 {
        for x in 0..16u32 {
            let l = frame.light[(y * 16 + x) as usize];
            let want = js_glow_value(8.0, 8.0, 4.0, 0.5, f64::from(x), f64::from(y));
            assert_eq!(l & 0xff, want, "r at ({x},{y})");
            assert_eq!((l >> 8) & 0xff, want, "g at ({x},{y})");
            assert_eq!((l >> 16) & 0xff, want, "b at ({x},{y})");
            let (dx, dy) = (f64::from(x) + 0.5 - 8.0, f64::from(y) + 0.5 - 8.0);
            if dx * dx + dy * dy >= 16.0 {
                assert_eq!(l, 0, "pixel ({x},{y}) at distance >= 4 is dark");
            }
        }
    }
    // the four pixels round the centre are the peak; pixelfx renders 86 there (node probe)
    assert_eq!(frame.light[7 * 16 + 7] & 0xff, 86);
    assert_eq!(frame.light[8 * 16 + 8] & 0xff, 86);
    // a pixel where Math.round would give 47 but pixelfx truncates to 46 (node probe)
    assert_eq!(frame.light[7 * 16 + 6] & 0xff, 46);
}

#[test]
fn paint_clips_offscreen() {
    let mut frame = Frame::new(8, 8, 0, 0);
    let c = Rgba(10, 20, 30, 255);
    Painter::new(&mut frame).rect(-10.0, -10.0, 30.0, 30.0, c, 1.0);
    assert_eq!(frame.color.len(), 64);
    assert!(
        frame.color.iter().all(|&p| p == c.pack()),
        "every pixel painted"
    );
    assert!(frame.light.iter().all(|&l| l == 0));
}

#[test]
fn place_flip_mirrors_cells() {
    // a flipped painter puts local pixel (2, 0) at screen x = tx - 3 (the cell [2, 3) mirrored to [-3, -2))
    let mut frame = Frame::new(16, 4, 0, 0);
    let c = Rgba(1, 2, 3, 255);
    let mut p = Painter::new(&mut frame);
    p.place(8.0, 1.0, true, 0.0);
    p.px(2.0, 0.0, c);
    assert_eq!(frame.color[16 + 5], c.pack());
    assert_eq!(frame.color.iter().filter(|&&v| v != 0).count(), 1);
}

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

#[test]
fn reach_is_the_distance_to_the_farthest_frame_corner() {
    // frame x in [-2, 8], y in [10, 16]; origin at (1, 12): farthest corner (8, 16) is 7 across, 4 down
    let mut f = Frame::new(10, 6, -2, 10);
    let mut p = Painter::new(&mut f);
    p.place(1.0, 12.0, true, 0.9);
    assert_eq!(
        p.reach(),
        libm::hypot(7.0, 4.0),
        "flip and angle keep distances"
    );
    // a local point just past reach, in any direction, lands off the frame
    let d = p.reach() + 1.0;
    for a in [0.0, 1.0, 2.5, 4.0, 5.5] {
        p.px(d * libm::cos(a), d * libm::sin(a), WHITE);
    }
    assert!(f.color.iter().all(|&c| c == 0));
}

/// Frame hashes from `tests/golden/painter_shapes.mjs` (pixelfx 044e12a).
#[test]
fn shapes_match_pixelfx() {
    use pixfx_core::color::hex;
    use pixfx_core::frame::frame_hash;

    let orange = hex("#ff8a2a").unwrap_or(Rgba::NONE);
    let blue = hex("#3f8cff").unwrap_or(Rgba::NONE);
    let purple = hex("#9a48ec").unwrap_or(Rgba::NONE);
    let yellow = hex("#ffe26b").unwrap_or(Rgba::NONE);
    type Draw = fn(&mut Painter, Rgba, Rgba, Rgba, Rgba);
    type Case = (&'static str, bool, f64, u64, Draw);
    let cases: [Case; 7] = [
        (
            "disc_soft",
            false,
            0.0,
            0xa6d9_425d_cbd2_cd25,
            |p, o, _, _, _| p.disc(0.0, -16.0, 9.0, o, 0.6),
        ),
        (
            "disc_tiny",
            false,
            0.0,
            0xabfd_01da_ef3f_6e27,
            |p, o, _, _, _| p.disc(0.0, -16.0, 0.4, o, 1.0),
        ),
        (
            "circle_fn",
            false,
            0.0,
            0xdd29_8569_e4f9_1185,
            |p, _, b, _, _| {
                p.circle(
                    0.0,
                    -16.0,
                    9.0,
                    |d: f64, _: f64| if d < 0.5 { WHITE } else { b },
                    1.0,
                )
            },
        ),
        (
            "circle_tiny",
            false,
            0.0,
            0x601d_c656_9710_d3ec,
            |p, _, b, _, _| p.circle(0.0, -16.0, 0.5, b, 1.0),
        ),
        (
            "ring_fn",
            false,
            0.0,
            0x512b_fc52_e1a2_4802,
            |p, _, _, v, _| p.ring(0.0, -16.0, 12.0, 3.0, v, |v: f64, _: f64| v),
        ),
        (
            "light",
            false,
            0.0,
            0xab08_116b_ef82_0aaa,
            |p, _, _, _, y| p.light(3.0, -5.0, y, 0.7),
        ),
        (
            "disc_mirrored_rotated",
            true,
            0.6,
            0xc39e_3120_e7ba_e241,
            |p, o, _, _, _| p.disc(0.0, -16.0, 9.0, o, 0.6),
        ),
    ];
    for (name, flip, angle, want, draw) in cases {
        let mut frame = Frame::new(64, 48, 0, 0);
        let mut p = Painter::new(&mut frame);
        p.place(32.0, 40.0, flip, angle);
        draw(&mut p, orange, blue, purple, yellow);
        assert_eq!(frame_hash(&frame), want, "{name}");
    }
}

/// Frame hashes from `tests/golden/painter_shapes.mjs` (pixelfx 044e12a), drawn through pixelfx transitions
/// `clipTo`: the painter keeps only writes whose local cell is inside the 320 x 180 screen.
#[test]
fn local_clip_matches_pixelfx() {
    use pixfx_core::color::hex;
    use pixfx_core::frame::frame_hash;

    let orange = hex("#ff8a2a").unwrap_or(Rgba::NONE);
    let yellow = hex("#ffe26b").unwrap_or(Rgba::NONE);
    let straddle = |p: &mut Painter| {
        p.glow(2.0, 3.0, 14.0, yellow, 0.8);
        p.disc(1.0, 4.0, 9.0, orange, 0.6);
        p.circle(-2.0, 6.0, 4.0, WHITE, 1.0);
    };
    let far = |p: &mut Painter| {
        p.glow(318.0, 177.0, 14.0, yellow, 0.8);
        p.disc(319.0, 176.0, 9.0, orange, 0.6);
    };
    type Case<'a> = (
        &'static str,
        (f64, f64, bool, f64),
        u64,
        &'a dyn Fn(&mut Painter),
    );
    let cases: [Case; 2] = [
        (
            "clip_origin_mirrored_rotated",
            (32.0, 24.0, true, 0.6),
            0x79a8_e5af_d0bd_b16f,
            &straddle,
        ),
        (
            "clip_far_corner",
            (-288.0, -150.0, false, 0.0),
            0x7e11_b54b_d5bf_93d1,
            &far,
        ),
    ];
    for (name, (x, y, flip, angle), want, draw) in cases {
        let render = |clip: bool| {
            let mut frame = Frame::new(64, 48, 0, 0);
            let mut p = Painter::new(&mut frame);
            p.place(x, y, flip, angle);
            if clip {
                p.clip_local(320.0, 180.0);
            }
            draw(&mut p);
            frame_hash(&frame)
        };
        assert_eq!(render(true), want, "{name}");
        // the shapes straddle the edge: without the clip they draw more
        assert_ne!(render(false), want, "{name} unclipped");
    }
}

#[test]
fn reset_removes_the_clip() {
    let mut f = Frame::new(8, 8, 0, 0);
    let mut p = Painter::new(&mut f);
    p.clip_local(2.0, 2.0);
    p.put(5.0, 5.0, WHITE, 1.0);
    p.reset();
    p.put(6.0, 6.0, WHITE, 1.0);
    assert_eq!(f.color.iter().filter(|&&c| c != 0).count(), 1);
}

/// Frame hashes from `tests/golden/unlight.mjs` (pixelfx 044e12a): shadow `unlight` is `each` over a box with
/// `dim_light` on the pixels its `dark` test accepts.
#[test]
fn dim_light_matches_pixelfx_unlight() {
    use pixfx_core::color::hex;
    use pixfx_core::frame::frame_hash;

    let violet = hex("#9d5cec").unwrap_or(Rgba::NONE);
    let yellow = hex("#ffe26b").unwrap_or(Rgba::NONE);
    let cases = [
        ("unlight_all", 0.0, false, 0.0, 0x990d_c616_6efd_904e_u64),
        (
            "unlight_keep_third",
            0.35,
            false,
            0.0,
            0x3f1d_9c98_60ee_8778,
        ),
        (
            "unlight_mirrored_rotated",
            0.0,
            true,
            0.6,
            0x61e5_c93e_9ea0_e405,
        ),
    ];
    for (name, keep, flip, angle, want) in cases {
        let mut frame = Frame::new(64, 48, 0, 0);
        let mut p = Painter::new(&mut frame);
        p.place(32.0, 40.0, flip, angle);
        p.glow(0.0, -16.0, 18.0, violet, 0.9);
        p.glow(6.0, -12.0, 9.0, yellow, 0.6);
        p.each(-10.0, -26.0, 10.0, -6.0, |p, x, y, sx, sy| {
            if x * x + (y + 16.0) * (y + 16.0) < 49.0 {
                p.dim_light(sx, sy, keep);
            }
        });
        assert_eq!(frame_hash(&frame), want, "{name}");
    }
}
