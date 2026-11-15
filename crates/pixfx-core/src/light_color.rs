//! The colour an effect lights its surroundings in: what a host tints the ground, walls or fighters round it with.

use pixfx_draw::Rgba;
use pixfx_draw::math::js_min;

use crate::effect::{Effect, Layer, MAX_DT, Options};
use crate::player::Player;

/// Times across the effect's duration it is painted at to learn its light.
const SAMPLES: u32 = 8;

/// The colour `effect` with `opts` (palette, light and its own options included) lights its surroundings in,
/// brightened so its brightest channel is 255: the hue of its light, whatever its strength. It is the mean of
/// every lit pixel's light, each weighted by how bright it is, over [`SAMPLES`] frames spread evenly across one
/// loop or the whole one-shot. None when the effect casts no light in any of them (`light: 0`, or an effect
/// that paints colour only).
pub fn light_color(effect: &'static dyn Effect, opts: Options) -> Option<Rgba> {
    let meta = effect.meta();
    let mut player = Player::new(effect, opts, meta.glow_margin());
    let step = meta.duration / f64::from(SAMPLES);
    let mut sum = [0u64; 3];
    for k in 0..SAMPLES {
        // the middle of each slice: a one-shot is done at its end and empty at its start
        let dt = if k == 0 { step / 2.0 } else { step };
        advance(&mut player, dt);
        player.draw(Layer::Both, player.placement());
        for &word in &player.frame().light {
            // xBGR: red in the low byte
            sum[0] += u64::from(word & 0xff);
            sum[1] += u64::from((word >> 8) & 0xff);
            sum[2] += u64::from((word >> 16) & 0xff);
        }
    }
    brightest(sum)
}

/// Step `player` on by `dt` seconds in steps it takes whole (each at most [`MAX_DT`]).
fn advance(player: &mut Player, dt: f64) {
    let mut left = dt;
    while left > 0.0 {
        let d = js_min(left, MAX_DT);
        player.update(d);
        left -= d;
    }
}

/// `sum` scaled so its largest channel is 255, rounded to whole channels; none when it is all 0.
fn brightest([r, g, b]: [u64; 3]) -> Option<Rgba> {
    let top = r.max(g).max(b);
    if top == 0 {
        return None;
    }
    // integer maths: exact and the same on every target
    let ch = |c: u64| ((c * 255 + top / 2) / top) as u8;
    Some(Rgba(ch(r), ch(g), ch(b), 255))
}

#[cfg(test)]
mod tests {
    use super::brightest;
    use pixfx_draw::Rgba;

    #[test]
    fn brightest_keeps_the_hue_at_full_strength() {
        assert_eq!(brightest([10, 5, 0]), Some(Rgba(255, 128, 0, 255)));
        assert_eq!(brightest([0, 0, 0]), None);
    }
}
