//! Weather `clip` with its `glow`, and `pass`, match pixelfx (`src/fx/weather/_lib.js`) at whole, fractional,
//! mirrored and rotated placements. Hashes from `pixfx-core/tests/golden/weather_clip.mjs` (pixelfx 044e12a).

use pixfx_core::color::{Rgba, WHITE, hex};
use pixfx_core::frame::frame_hash;
use pixfx_core::{Frame, Meta, Painter};
use pixfx_effects::weather::lib::{clip, glow, pass};

#[test]
fn clip_glow_and_pass_match_pixelfx_at_any_placement() {
    let meta = Meta {
        id: "test/canvas",
        name: "",
        desc: "",
        tags: &[],
        size: (40, 24),
        origin: (20, 18),
        duration: 1.0,
        looping: false,
        fps: 30,
        seed: 1,
        state: None,
    };
    let col = |s: &str| hex(s).unwrap_or(Rgba::NONE);
    let (a, b, c) = (col("#93abd6"), col("#24304f"), col("#ffe26b"));
    let scene = |p: &mut Painter| {
        clip(p, &meta);
        pass(p, &meta, -18.0, 5.0, |x, y, out| {
            let (fx, fy) = (libm::floor(x) as i32, libm::floor(y) as i32);
            out.c = if (fx + fy) & 1 != 0 { a } else { b };
            out.a = 0.8;
            out.b = fx & 3 == 0;
            if fy & 3 == 0 {
                out.l = c;
                out.k = 0.15;
            }
        });
        glow(p, &meta, -20.0, -18.0, 12.0, c, 0.8);
        glow(p, &meta, 19.5, 5.5, 12.0, a, 0.8);
        glow(p, &meta, 0.0, -6.0, 9.0, c, 0.6);
        for x in -22..=21 {
            for y in [-19, -18, 5, 6] {
                p.px(f64::from(x), f64::from(y), WHITE);
            }
        }
        for y in -19..=6 {
            for x in [-21, -20, 19, 20] {
                p.px(f64::from(x), f64::from(y), WHITE);
            }
        }
    };
    let cases = [
        ("whole", (32.0, 24.0), false, 0.0, 0xa494_e427_608b_1486_u64),
        ("half", (32.5, 24.5), false, 0.0, 0x643a_6cbf_f1ba_731b),
        ("third", (32.3, 24.3), false, 0.0, 0xa494_e427_608b_1486),
        (
            "half_mirrored",
            (32.5, 24.5),
            true,
            0.0,
            0x5903_329f_1083_6d8e,
        ),
        ("rotated", (32.0, 24.0), false, 0.5, 0xb2b4_5d44_a9bf_05ec),
    ];
    for (name, (x, y), flip, angle, want) in cases {
        let mut frame = Frame::new(64, 48, 0, 0);
        let mut p = Painter::new(&mut frame);
        p.place(x, y, flip, angle);
        scene(&mut p);
        assert_eq!(frame_hash(&frame), want, "{name}");
    }
}
