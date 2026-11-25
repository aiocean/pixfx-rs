//! A descending electric bolt and a ground flash for a UI impact.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{Rng, lerp, to_uint32};
use pixfx_draw::shapes::{bolt, wedge};
use pixfx_draw::{Painter, RULER, Ruler};

use super::lib::{Spike, star};

pub struct Thunderstrike;

pub static THUNDERSTRIKE: Thunderstrike = Thunderstrike;

const FPS: u32 = 15;
const IMPACT_FRAME: usize = 4;
const META: Meta = Meta {
    id: "impacts/thunderstrike",
    name: "Thunderstrike",
    desc: "A gold bolt wrapped in teal lightning drops onto the click point and bursts into a fan of sparks.",
    tags: &["impact", "lightning", "electric", "interface", "one-shot"],
    size: (120, 116),
    origin: (60, 91),
    duration: 12.0 / FPS as f64,
    looping: false,
    fps: FPS,
    seed: 13,
    state: None,
    options: &[FEEL.spec()],
};

const PALETTE: [Rgba; 7] = [
    Rgba::hex24(0x18557e),
    Rgba::hex24(0x21afb9),
    Rgba::hex24(0x9ef4d4),
    Rgba::hex24(0xdb792e),
    Rgba::hex24(0xffbc4c),
    Rgba::hex24(0xffe291),
    Rgba::hex24(0xfff9e2),
];

impl Effect for Thunderstrike {
    fn meta(&self) -> &Meta {
        &META
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(IMPACT_FRAME as f64 / f64::from(FPS), 0.6, 0.0, 0.0, opts)
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let mut rng = Rng::new(to_uint32(seed as f64));
        // The drop starts 1.7 standing heights up. The arcs reach 1.55 blade lengths.
        let height = f64::from(Ruler::size(RULER.height, 1.7));
        let reach = f64::from(Ruler::size(RULER.blade, 1.55));
        let arcs = std::array::from_fn(|i| {
            let angle = -0.18 - i as f64 * 0.38 + rng.range(-0.12, 0.12);
            let distance = reach * rng.range(0.65, 1.0);
            let x = libm::cos(angle) * distance;
            let y = libm::sin(angle) * distance;
            let path = bolt(&mut rng, x * 0.14, -1.0, x, y, 0.28, 3);
            let breaks = std::array::from_fn(|_| rng.range(0.0, 1.0));
            Arc { path, breaks }
        });
        // Flash rays range from one blade to 1.8 blade lengths.
        let spikes = std::array::from_fn(|i| Spike {
            a: -0.12 - i as f64 * 0.48 + rng.range(-0.1, 0.1),
            len: f64::from(Ruler::size(RULER.blade, rng.range(1.0, 1.8))),
            w: f64::from(Ruler::size(RULER.hand, 1.0)),
        });
        let ramp = opts.palette_or(&PALETTE);
        let colors = std::array::from_fn(|i| ramp.get(i).copied().unwrap_or(PALETTE[i]));
        Box::new(State {
            height,
            arcs,
            spikes,
            colors,
        })
    }
}

struct Arc {
    path: Vec<(f64, f64)>,
    breaks: [f64; 8],
}

struct State {
    height: f64,
    arcs: [Arc; 8],
    spikes: [Spike; 7],
    colors: [Rgba; 7],
}

impl State {
    fn descent(&self, p: &mut Painter, frame: usize) {
        // Each pose shifts the tip down and changes the direction of the kink.
        let (tail, bend, tip, width) = [
            ((0.47, -1.0), (0.28, -0.77), (0.16, -0.61), 2.2),
            ((0.33, -0.80), (0.08, -0.55), (0.18, -0.38), 4.0),
            ((0.25, -0.66), (-0.04, -0.35), (0.07, -0.16), 6.5),
            ((0.10, -0.51), (0.16, -0.24), (0.0, 0.0), 7.5),
        ][frame];
        let (x0, y0) = (tail.0 * self.height, tail.1 * self.height);
        let (x1, y1) = (bend.0 * self.height, bend.1 * self.height);
        let (x2, y2) = (tip.0 * self.height, tip.1 * self.height);
        let [blue, teal, mint, ember, gold, hot, core] = self.colors;
        wedge(p, x0, y0, x1, y1, 0.5, width, ember, 1.0);
        wedge(p, x1, y1, x2, y2, width, 1.0, gold, 1.0);
        p.line(x0, y0, x1, y1, hot, 1.0);
        p.line(x1, y1, x2, y2, core, 1.0);

        // The side arc crosses the bolt as its tip travels toward the origin.
        let mut previous = (x0, y0);
        for step in 1..=16 {
            let progress = f64::from(step) / 16.0;
            let (x, y) = if progress < 0.5 {
                (lerp(x0, x1, progress * 2.0), lerp(y0, y1, progress * 2.0))
            } else {
                let along = progress * 2.0 - 1.0;
                (lerp(x1, x2, along), lerp(y1, y2, along))
            };
            let phase = progress * 11.0 - frame as f64 * 1.2;
            let swing = libm::sin(phase);
            let envelope = 4.0 * progress * (1.0 - progress);
            let next = (x + swing * width * 1.5 * envelope, y);
            if libm::fabs(swing) > 0.25 {
                let color = if swing > 0.0 { mint } else { teal };
                p.line(previous.0, previous.1, next.0, next.1, color, 1.0);
            }
            if step == 6 || step == 12 {
                let fork = if swing < 0.0 { -1.0 } else { 1.0 };
                let reach = width * envelope * 0.65;
                let knee = (next.0 + fork * reach, y - 2.0);
                p.line(next.0, next.1, knee.0, knee.1, teal, 1.0);
                p.line(knee.0, knee.1, knee.0 + fork * 2.0, y - 5.0, blue, 1.0);
                if frame >= 2 {
                    p.px(knee.0 + fork * 3.0, y - 7.0, mint);
                }
            }
            previous = next;
        }
    }

    fn flash(&self, p: &mut Painter, frame: usize) {
        let [_, _, _, rim, gold, hot, white] = self.colors;
        let (spread, thickness, bloom) = [
            (0.40, 0.85, 0.55),
            (0.88, 1.05, 0.70),
            (1.18, 0.78, 0.48),
            (0.55, 0.35, 0.25),
        ][frame - IMPACT_FRAME];
        star(
            p,
            0.0,
            0.0,
            &self.spikes,
            spread,
            thickness,
            [rim, gold, hot, white],
            1.0,
            4.0,
        );
        let needle = f64::from(Ruler::size(RULER.blade, 2.2)) * spread;
        let foot = -2.0 * spread;
        p.line(1.0, foot, -1.0, -needle, gold, 1.0);
        p.line(1.0, foot, 0.0, -needle * 0.7, white, 1.0);
        p.glow(0.0, 0.0, 15.0 * spread, gold, bloom);
    }

    fn sparks(&self, p: &mut Painter, frame: usize) {
        let age = (frame - 5) as f64 / 5.0;
        let [_, _, _, ember, gold, hot, core] = self.colors;
        for (i, spike) in self.spikes.iter().enumerate() {
            if age > 0.55 + 0.08 * (i % 4) as f64 {
                continue;
            }
            let angle = spike.a + if i % 2 == 0 { 0.26 } else { -0.26 };
            let distance = spike.len * (0.48 + 0.9 * age);
            let x = libm::cos(angle) * distance;
            let y = libm::sin(angle) * distance + 11.0 * age * age;
            let color = if age < 0.25 { hot } else { ember };
            p.px(x, y, color);
            if age < 0.45 {
                let dx = libm::cos(angle);
                let dy = libm::sin(angle);
                p.line(x, y, x - dx * 3.0, y - dy * 3.0, gold, 1.0);
                p.px(x, y, if age < 0.25 { core } else { hot });
                if i % 2 == 0 {
                    p.px(x + dy * 3.0, y - dx * 3.0, ember);
                }
            }
        }
    }

    fn aftershock(&self, p: &mut Painter, frame: usize) {
        let age = (frame - 5) as f64 / 5.5;
        let spread = 0.65 + 0.65 * age;
        let [blue, teal, mint, ..] = self.colors;
        for arc in &self.arcs {
            for (i, (segment, &break_at)) in arc.path.windows(2).zip(&arc.breaks).enumerate() {
                if break_at < age || (frame == 5 && i > 3) {
                    continue;
                }
                let (x0, y0) = segment[0];
                let (x1, y1) = segment[1];
                let color = if i < 2 || age > 0.65 {
                    blue
                } else if age < 0.4 && i >= 2 {
                    mint
                } else {
                    teal
                };
                p.line(
                    x0 * spread,
                    y0 * spread,
                    x1 * spread,
                    y1 * spread,
                    color,
                    1.0,
                );
            }
        }
    }
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        // The timeline samples authored poses at the advertised frame rate.
        let frame = pixfx_draw::math::js_round(t * f64::from(FPS)) as usize;
        if frame < IMPACT_FRAME {
            self.descent(p, frame);
        }
        if (5..11).contains(&frame) {
            self.aftershock(p, frame);
        }
        if (IMPACT_FRAME..8).contains(&frame) {
            self.flash(p, frame);
        }
        if (5..10).contains(&frame) {
            self.sparks(p, frame);
        }
    }
}
