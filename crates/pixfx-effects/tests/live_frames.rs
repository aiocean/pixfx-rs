//! A live effect shows the rendered frame its clock is on, whatever steps the host's clock takes: frame k of
//! `render_frames` is what a live instance draws while `floor(t * fps)` is k (wrapped round a loop, held at a
//! one-shot's last frame). Uneven frame times, stalls and zero steps change nothing.

use pixfx_core::effect::MAX_DT;
use pixfx_core::{Frame, Instance, Layer, Options, edge_fade, render_frames};
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
