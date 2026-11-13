//! A player's frame cropped to its lit box keeps every pixel it shows: the box holds every pixel with colour
//! alpha or light, and the cropped bytes put back at the box's offset rebuild the whole frame's bytes.

use pixfx_core::{Layer, Options, PixelBox, Player};
use pixfx_effects::EFFECTS;

/// Put `part` (`b.w * b.h * 4` bytes) back into a zeroed `w` × `h` RGBA buffer at the box's offset.
fn uncrop(part: &[u8], b: PixelBox, w: u32, h: u32) -> Vec<u8> {
    let mut out = vec![0u8; w as usize * h as usize * 4];
    let row = b.w as usize * 4;
    for (r, src) in part.chunks(row.max(1)).enumerate().take(b.h as usize) {
        let at = ((b.y as usize + r) * w as usize + b.x as usize) * 4;
        if let Some(dst) = out.get_mut(at..at + row) {
            dst.copy_from_slice(src);
        }
    }
    out
}

#[test]
fn a_frame_cropped_to_its_lit_box_loses_nothing() -> Result<(), String> {
    let mut cropped_any = false;
    for &fx in EFFECTS {
        let meta = fx.meta();
        let opts = Options {
            seed: Some(3),
            ..Options::default()
        };
        let mut player = Player::new(fx, opts, meta.glow_margin());
        player.update(meta.duration * 0.3);
        for layer in [Layer::Back, Layer::Front] {
            player.draw(layer, player.placement());
            let (w, h) = (player.frame().w, player.frame().h);
            let full_color = player.color_rgba().to_vec();
            let full_light = player.light_rgba().to_vec();
            let Some(b) = player.lit_box() else {
                if full_color.iter().chain(&full_light).any(|&v| v != 0) {
                    return Err(format!("{}: no lit box, yet pixels show", meta.id));
                }
                continue;
            };
            cropped_any |= b.w < w || b.h < h;
            let color = player.color_rgba_in(b).to_vec();
            let light = player.light_rgba_in(b).to_vec();
            if color.len() != (b.w * b.h * 4) as usize {
                return Err(format!(
                    "{}: {} colour bytes for {b:?}",
                    meta.id,
                    color.len()
                ));
            }
            if uncrop(&color, b, w, h) != full_color || uncrop(&light, b, w, h) != full_light {
                return Err(format!(
                    "{}: {layer:?} cropped to {b:?} differs from the whole frame",
                    meta.id
                ));
            }
        }
    }
    if !cropped_any {
        return Err("no effect's lit box was smaller than its frame".into());
    }
    Ok(())
}

#[test]
fn a_box_past_the_frame_is_clipped_to_it() {
    let Some(&fx) = EFFECTS.first() else { return };
    let mut player = Player::new(fx, Options::default(), 0);
    player.draw(Layer::Both, player.placement());
    let (w, h) = (player.frame().w, player.frame().h);
    let past = PixelBox {
        x: w - 1,
        y: h - 1,
        w: 10,
        h: 10,
    };
    assert_eq!(player.color_rgba_in(past).len(), 4);
    let outside = PixelBox {
        x: w + 5,
        y: 0,
        w: 3,
        h: 3,
    };
    assert!(player.light_rgba_in(outside).is_empty());
}
