//! Composite (pixelfx `src/core/fx.js` `composite`) and colour parsing.

use pixfx_core::color::{Rgba, WHITE, hex};
use pixfx_core::frame::{Frame, composite};
use pixfx_core::painter::Painter;

#[test]
fn light_saturates() {
    let mut frame = Frame::new(16, 16, 0, 0);
    let mut p = Painter::new(&mut frame);
    for _ in 0..20 {
        p.glow(8.0, 8.0, 6.0, WHITE, 0.5);
    }
    let mut out = vec![0u8; 16 * 16 * 4];
    composite(&frame, &mut out, None);
    let i = (8 * 16 + 8) * 4;
    assert_eq!(
        &out[i..i + 4],
        &[255, 255, 255, 255],
        "summed light saturates, no wrap"
    );
    // over an opaque black ground the light adds and clamps the same way
    let bg = [0u8, 0, 0, 255].repeat(16 * 16);
    composite(&frame, &mut out, Some(&bg));
    assert_eq!(&out[i..i + 4], &[255, 255, 255, 255]);
}

#[test]
fn composite_light_over_transparent() {
    // pixelfx and beanfx un-premultiply: rgb = light / alpha, alpha = brightest channel.
    // node: composite(light (100,50,0)) -> [255, 128, 0, 100] in a Uint8ClampedArray.
    let mut frame = Frame::new(1, 1, 0, 0);
    frame.light[0] = 0xff00_0000 | (50 << 8) | 100;
    let mut out = [0u8; 4];
    composite(&frame, &mut out, None);
    assert_eq!(out, [255, 128, 0, 100]);
}

#[test]
fn composite_colour_over_ground_and_transparent() {
    let mut frame = Frame::new(2, 1, 0, 0);
    frame.color[0] = Rgba(200, 100, 50, 255).pack();
    frame.color[1] = Rgba(200, 100, 50, 128).pack();
    let mut out = [0u8; 8];
    composite(&frame, &mut out, None);
    assert_eq!(out, [200, 100, 50, 255, 200, 100, 50, 128]);
    let bg = [0u8, 0, 100, 255, 0, 0, 100, 255];
    composite(&frame, &mut out, Some(&bg));
    // 200*128/255 = 100.39 -> 100; 100*128/255 = 50.2 -> 50; 50*f + 100*(1-f) = 74.9 -> 75
    assert_eq!(out, [200, 100, 50, 255, 100, 50, 75, 255]);
}

#[test]
fn hex_parses_short_and_long() -> Result<(), pixfx_core::color::ColorError> {
    assert_eq!(hex("#f80")?, hex("#ff8800ff")?);
    assert_eq!(hex("#f80")?, Rgba(255, 136, 0, 255));
    assert_eq!(hex("102030")?, Rgba(16, 32, 48, 255));
    assert!(hex("#ff88").is_err());
    assert!(hex("#gg0000").is_err());
    Ok(())
}
