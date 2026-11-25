//! A local gust with three rolling ribbons. Movement history adds billow to
//! the gust while its origin and axes stay with the effect object.

use std::{f64::consts::TAU, sync::Arc};

use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options, Trace};
use pixfx_draw::color::{Rgba, mix};
use pixfx_draw::math::smooth;
use pixfx_draw::{Painter, RULER, Ruler};

pub struct VioletWake;
pub static VIOLET_WAKE: VioletWake = VioletWake;

const SECS: NumOpt = NumOpt::new("secs", 0.37).within(0.12, 0.65);
const WIDTH: NumOpt = NumOpt::new("width", 1.0).within(0.5, 2.0);
const META: Meta = Meta {
    id: "projectiles/violet-wake",
    name: "Violet Wake",
    desc: "Three violet ribbons roll in place and peel apart into wisps. Move the effect with a dash or a projectile.",
    tags: &["projectiles", "dash", "trail", "violet"],
    size: (112, 64),
    origin: (56, 32),
    duration: 0.79,
    looping: false,
    fps: 33,
    seed: 1,
    state: None,
    options: &[SECS.spec(), WIDTH.spec()],
};

const PALETTE: [Rgba; 5] = [
    Rgba::hex24(0x130733),
    Rgba::hex24(0x3012a9),
    Rgba::hex24(0x6330ff),
    Rgba::hex24(0xc83dff),
    Rgba::hex24(0xff86ed),
];

impl Effect for VioletWake {
    fn meta(&self) -> &Meta {
        &META
    }

    fn traces(&self, opts: &Options) -> Option<f64> {
        Some(opts.num(&SECS))
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let mut palette = PALETTE;
        for (slot, color) in palette.iter_mut().zip(opts.palette_or(&PALETTE)) {
            *slot = color;
        }
        Box::new(Wake {
            palette,
            secs: opts.num(&SECS),
            width: opts.num(&WIDTH),
            trace: None,
        })
    }
}

struct Wake {
    palette: [Rgba; 5],
    secs: f64,
    width: f64,
    trace: Option<Arc<Trace>>,
}

impl EffectState for Wake {
    fn set_trace(&mut self, trace: Option<Arc<Trace>>) {
        self.trace = trace;
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            return;
        }
        // The trace supplies speed only. All geometry stays in effect-local coordinates.
        let speed = self
            .trace
            .as_ref()
            .map_or(0.0, |trace| trace.speed(0.0, 0.08));
        let billow = 1.0 + 0.22 * smooth(25.0, 200.0, speed);
        let span = Ruler::size(RULER.blade, 2.75);
        let spread = f64::from(Ruler::size(RULER.head, 0.55));
        let curl = f64::from(Ruler::size(RULER.head, 0.72));
        let crest_width = f64::from(Ruler::size(RULER.hand, self.width));
        let [ink, indigo, violet, magenta, pink] = self.palette;

        for (lane, delay, weight, turns, phase) in [
            (-1.0, 0.015, 0.76, 0.92, 0.7),
            (0.0, 0.04, 1.0, 0.78, 0.1),
            (1.0, 0.085, 0.64, 0.70, -0.65),
        ] {
            let age = t - delay;
            let rise = smooth(0.0, 0.19, age);
            let fade = 1.0 - smooth(0.16 + self.secs * 0.35, 0.57 + self.secs * 0.16, age);
            let energy = rise * fade;
            if energy <= 0.0 {
                continue;
            }
            let travel = age * 1.55;
            let tear = smooth(0.29, 0.61, age);
            for col in 0..=span {
                let u = f64::from(col) / f64::from(span);
                let edge = 2.0 * (u - 0.5).abs();
                let tip = 1.0 - smooth(0.61, 1.0, edge);
                let length = 0.24 + 0.76 * rise;
                if edge > length {
                    continue;
                }
                let gap = (u * (4.0 + lane * 0.35) - travel * 0.9 + phase).rem_euclid(1.0);
                if gap > 1.0 - tear * (0.28 + edge * 0.48) {
                    continue;
                }

                // A crest travels inside each ribbon while both ends stay local.
                let angle = TAU * (u * turns - travel) + phase;
                let wave = libm::sin(angle);
                let roll = libm::sin(angle * 2.0 + 0.5) * 0.18;
                let curl_envelope = tip * (0.4 + 0.6 * rise);
                let x = f64::from(col) - f64::from(span) * 0.5 + lane * 2.0;
                let y = lane * spread * (0.6 + 0.4 * rise + 0.45 * tear)
                    + curl * billow * (wave + roll) * curl_envelope;
                let crest = smooth(0.18, 0.85, -wave);
                let heat = energy * weight * tip * crest * (1.0 - tear);
                let cold = (1.0 - rise).max(tear);
                let color = mix(
                    mix(indigo, violet, (1.0 - cold) * weight),
                    mix(magenta, pink, smooth(0.6, 1.0, heat)),
                    smooth(0.15, 0.75, heat),
                );
                let alpha = energy
                    * (0.85 + 0.15 * weight)
                    * (0.32 + 0.68 * smooth(0.0, 0.25, length - edge));
                let half = (crest_width * tip * energy * (0.25 + 0.26 * crest)) as i32;

                if half > 0 {
                    for across in [-half - 1, half + 1] {
                        let y = y + f64::from(across);
                        p.blend_px(p.cell_x(x, y), p.cell_y(x, y), ink, alpha * 0.88);
                    }
                }
                for across in -half..=half {
                    let y = y + f64::from(across);
                    p.blend_px(p.cell_x(x, y), p.cell_y(x, y), color, alpha);
                }
                if heat > 0.13 {
                    for (d, strength) in [(1.0, 0.23), (2.0, 0.13), (3.0, 0.07), (4.0, 0.03)] {
                        p.light(x, y + d, violet, heat * strength);
                        p.light(x, y - d, violet, heat * strength);
                    }
                }
            }
        }
    }
}
