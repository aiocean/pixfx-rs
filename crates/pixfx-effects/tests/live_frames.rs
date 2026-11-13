//! A live effect shows the rendered frame its clock is on, whatever steps the host's clock takes: frame k of
//! `render_frames` is what a live instance draws while `floor(t * fps)` is k (wrapped round a loop, held at a
//! one-shot's last frame). Uneven frame times, stalls and zero steps change nothing.

use pixfx_core::effect::MAX_DT;
use pixfx_core::{Instance, Layer, Options, edge_fade, render_frames};
use pixfx_draw::Frame;
use pixfx_effects::EFFECTS;

/// A fixed sequence of host steps: mostly 60 and 144 Hz with jitter, some zero steps (a hit-stop) and stalls
/// longer than [`MAX_DT`].
fn host_steps(len: usize) -> Vec<f64> {
    let mut s: u32 = 0x9e37_79b9;
    (0..len)
        .map(|_| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            match s % 16 {
                0 => 0.0,
                1 => MAX_DT * 1.6,
                2..=8 => 1.0 / 60.0 + f64::from(s % 1000) * 4e-6,
                _ => 1.0 / 144.0 + f64::from(s % 1000) * 2e-6,
            }
        })
        .collect()
}

#[test]
fn live_play_draws_the_rendered_frame_for_its_clock() -> Result<(), String> {
    for &fx in EFFECTS {
        let meta = fx.meta();
        let opts = Options {
            seed: Some(5),
            ..Options::default()
        };
        let rendered = render_frames(fx, &opts, 0);
        let canvas = fx.canvas(&opts);
        let at_origin = Options {
            x: f64::from(canvas.origin.0),
            y: f64::from(canvas.origin.1),
            ..opts
        };
        let mut inst = Instance::new(fx, at_origin);
        // past one loop (or the one-shot's end) so loops come round
        let mut checked = 0;
        for (i, dt) in host_steps(400).into_iter().enumerate() {
            inst.update(dt);
            if inst.t() > meta.duration * 1.4 + 0.1 {
                break;
            }
            // drawing every step of the heavy screen effects is slow in a debug build; every 5th step still
            // lands on frames across the whole clip
            if i % 5 != 0 {
                continue;
            }
            let k = inst.frame();
            let mut live = Frame::new(rendered.w, rendered.h, 0, 0);
            inst.draw(&mut live, Layer::Both, inst.placement());
            if !meta.is_screen() {
                edge_fade(&mut live);
            }
            let want = rendered
                .frames
                .get(k as usize)
                .ok_or_else(|| format!("{}: frame {k} is past the render", meta.id))?;
            if live != *want {
                return Err(format!(
                    "{}: live frame {k} at t={:.4} differs from the render",
                    meta.id,
                    inst.t()
                ));
            }
            checked += 1;
        }
        if checked < 5 {
            return Err(format!("{}: only {checked} frames checked", meta.id));
        }
    }
    Ok(())
}

/// Two players of one effect with equal options show the same frame id whenever their clocks are on the same
/// frame, and then draw byte-equal frames; a stopped player shows no shared frame.
#[test]
fn players_on_the_same_frame_id_draw_the_same_frame() -> Result<(), String> {
    use pixfx_core::Player;
    for &fx in EFFECTS {
        let opts = Options::default();
        let margin = fx.meta().glow_margin();
        let (mut a, mut b) = (
            Player::new(fx, opts.clone(), margin),
            Player::new(fx, opts.clone(), margin),
        );
        let place = a.placement();
        // b reaches the same clock by another road: a loop one whole loop ahead (it comes round to the same
        // frame), and every step split in three
        if fx.meta().looping {
            let mut left = fx.meta().duration;
            while left > 0.0 {
                let dt = left.min(0.2);
                b.update(dt);
                left -= dt;
            }
        }
        let mut agreed = 0;
        for dt in host_steps(120) {
            a.update(dt);
            for _ in 0..3 {
                b.update(dt / 3.0);
            }
            let (ia, ib) = (a.shown(place), b.shown(place));
            if ia.is_none() || ia != ib {
                continue;
            }
            a.draw(Layer::Both, place);
            b.draw(Layer::Both, place);
            if a.frame() != b.frame() {
                return Err(format!("{}: same frame id, different pixels", fx.meta().id));
            }
            agreed += 1;
        }
        if agreed == 0 && fx.meta().looping {
            return Err(format!(
                "{}: the two players never met on a frame",
                fx.meta().id
            ));
        }
        a.stop();
        if a.shown(place).is_some() {
            return Err(format!(
                "{}: a stopped player still shows a shared frame",
                fx.meta().id
            ));
        }
    }
    Ok(())
}

/// What a player draws, fading or not, changes only when its clock frame does: drawing twice within one clock
/// frame gives the same pixels, so a host need not redraw on every display refresh.
#[test]
fn drawing_changes_only_with_the_clock_frame() -> Result<(), String> {
    use pixfx_core::Player;
    for &fx in EFFECTS {
        let mut p = Player::new(fx, Options::default(), fx.meta().glow_margin());
        let place = p.placement();
        p.update(0.2);
        p.stop();
        for _ in 0..12 {
            // a step that ends early in a frame, then one that stays inside it
            p.update(1.0 / 90.0);
            if p.done() {
                break;
            }
            let before = p.clock_frame();
            p.draw(Layer::Both, place);
            let first = p.frame().clone();
            p.update(1e-4);
            if p.clock_frame() != before {
                continue;
            }
            p.draw(Layer::Both, place);
            if *p.frame() != first {
                return Err(format!(
                    "{}: redrawn within one clock frame, pixels changed",
                    fx.meta().id
                ));
            }
        }
    }
    Ok(())
}
