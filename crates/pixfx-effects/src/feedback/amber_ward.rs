//! A faceted ward that folds shut and releases four amber flares.

use pixfx_core::{Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{Rng, TAU, lerp, smooth};
use pixfx_draw::{Painter, RULER, Ruler};

pub struct AmberWard;

pub static AMBER_WARD: AmberWard = AmberWard;

const META: Meta = Meta {
    id: "feedback/amber-ward",
    name: "Amber ward",
    desc: "Two amber brackets fold into a faceted shield and release four flares.",
    tags: &["feedback", "shield", "buff", "light", "one-shot"],
    size: (112, 112),
    origin: (56, 56),
    duration: 1.3,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

const PALETTE: [Rgba; 5] = [
    Rgba::hex24(0x462039),
    Rgba::hex24(0x9f4933),
    Rgba::hex24(0xe18a36),
    Rgba::hex24(0xffc968),
    Rgba::hex24(0xfff4cc),
];

const CLOSED: f64 = 0.30;
const BURST: f64 = 0.63;

impl Effect for AmberWard {
    fn meta(&self) -> &Meta {
        &META
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let mut rng = Rng::new((seed ^ (seed >> 32)) as u32);
        Box::new(State {
            ramp: opts.palette_or(&PALETTE),
            // The ward is 1.15 blade lengths across and 1.35 tall.
            half_w: f64::from(Ruler::size(RULER.blade, 1.15)) / 2.0,
            half_h: f64::from(Ruler::size(RULER.blade, 1.35)) / 2.0,
            sparks: std::array::from_fn(|i| {
                let angle = TAU * (i as f64 + rng.next()) / 24.0;
                let reach = lerp(22.0, 47.0, rng.next());
                (libm::cos(angle) * reach, libm::sin(angle) * reach)
            }),
        })
    }
}

struct State {
    ramp: Vec<Rgba>,
    half_w: f64,
    half_h: f64,
    sparks: [(f64, f64); 24],
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front || t <= 0.0 || t >= META.duration {
            return;
        }

        let w = self.half_w;
        let h = self.half_h;
        let opacity = smooth(0.0, 0.10, t) * (1.0 - smooth(1.04, 1.28, t));
        let close = smooth(0.06, CLOSED, t);
        let gap = w * 0.78 * (1.0 - close);
        let side = [(0.0, -h), (w, -0.50 * h), (w, 0.43 * h), (0.0, h)];
        let rim = self.ramp[3];
        let hot = self.ramp[4];

        if t >= CLOSED {
            let face = [
                side[0],
                side[1],
                side[2],
                side[3],
                (-w, side[2].1),
                (-w, side[1].1),
            ];
            let fill = smooth(CLOSED, 0.44, t) * (1.0 - smooth(0.99, 1.18, t));
            let inset = face.map(|(x, y)| (x * 0.79, y * 0.81));
            p.poly(
                &face,
                |x: f64, y: f64| {
                    if x + y * 0.52 < 0.0 {
                        self.ramp[2]
                    } else {
                        self.ramp[1]
                    }
                },
                fill,
            );
            let glint = lerp(-1.6, 1.6, smooth(0.42, 0.60, t));
            p.poly(
                &inset,
                |x: f64, y: f64| {
                    let stripe = (x / w - 0.55 * y / h - glint).abs();
                    let bevel = 0.62 * (x / w).abs() + 0.76 * (y / h).abs();
                    if stripe < 0.11 && (0.42..0.60).contains(&t) {
                        hot
                    } else if stripe < 0.23 && (0.42..0.60).contains(&t) {
                        rim
                    } else if x < 0.0 && bevel > 0.49 {
                        self.ramp[2]
                    } else if x - y * 0.27 < 1.0 {
                        self.ramp[1]
                    } else {
                        self.ramp[0]
                    }
                },
                fill,
            );

            let seal = smooth(0.34, 0.48, t) * (1.0 - smooth(0.90, 1.10, t));
            p.poly(
                &[(0.0, -6.0), (5.0, 0.0), (0.0, 6.0), (-5.0, 0.0)],
                rim,
                seal,
            );
            p.poly(
                &[(0.0, -4.0), (3.0, 0.0), (0.0, 4.0), (-3.0, 0.0)],
                self.ramp[0],
                seal,
            );
            p.dot(0.0, 0.0, hot, seal);
        }

        for sign in [-1.0, 1.0] {
            for [(x0, y0), (x1, y1)] in side.array_windows() {
                let x0 = sign * (x0 + gap);
                let x1 = sign * (x1 + gap);
                p.line(x0, *y0, x1, *y1, rim, opacity);
                p.glow_line(x0, *y0, x1, *y1, 3.0, self.ramp[2], 0.24 * opacity);
            }
            for (delay, height) in [(0.0, 1.0), (0.038, 0.73), (0.075, 0.46)] {
                let tail = smooth(0.06 + delay, 0.14 + delay, t)
                    * (1.0 - smooth(0.19 + delay, 0.30 + delay, t));
                if tail <= 0.0 {
                    continue;
                }
                let lag = 1.0 - smooth(0.06 + delay, CLOSED + delay, t);
                let x = sign * (w + gap + lag * 6.0);
                let y = h * (0.42 - 0.62 * height);
                let elbow = (x + sign * (3.0 + 7.0 * lag), y - 2.0 - 3.0 * height);
                let end = (
                    elbow.0 + sign * (4.0 + 7.0 * lag),
                    elbow.1 - 3.0 - 5.0 * height,
                );
                p.line(x, y, elbow.0, elbow.1, rim, tail * 0.78);
                p.line(elbow.0, elbow.1, end.0, end.1, self.ramp[2], |u: f64| {
                    tail * (1.0 - u) * 0.65
                });
                p.glow_line(x, y, elbow.0, elbow.1, 3.0, self.ramp[2], 0.25 * tail);
            }
        }

        for (x, delay) in [(-21.0, 0.49), (18.0, 0.54), (-10.0, 0.60)] {
            let age = t - delay;
            let glimmer = smooth(0.0, 0.05, age) * (1.0 - smooth(0.08, 0.20, age));
            if glimmer > 0.0 {
                let y = 17.0 - 23.0 * age;
                p.line(x - 2.0, y, x + 2.0, y, rim, glimmer);
                p.line(x, y - 3.0, x, y + 3.0, hot, glimmer);
                p.glow(x, y, 4.0, self.ramp[2], 0.42 * glimmer);
            }
        }

        if (BURST..1.04).contains(&t) {
            let flare = smooth(BURST, 0.70, t) * (1.0 - smooth(0.74, 0.91, t));
            let travel = smooth(BURST, 0.94, t);
            p.glow(0.0, 0.0, 20.0, self.ramp[2], 1.15 * flare);
            p.poly(
                &[(0.0, -11.0), (8.0, 0.0), (0.0, 11.0), (-8.0, 0.0)],
                hot,
                flare,
            );
            let tip = lerp(22.0, 38.0, travel);
            for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                let start = tip - 17.0 * flare;
                let flare_shape = [
                    (dx * start, dy * start),
                    (dx * (start + 2.0) - dy * 3.0, dy * (start + 2.0) + dx * 3.0),
                    (dx * tip, dy * tip),
                    (dx * (start + 2.0) + dy * 3.0, dy * (start + 2.0) - dx * 3.0),
                ];
                p.poly(&flare_shape, hot, flare);
                p.glow(dx * tip, dy * tip, 4.0, rim, 0.45 * flare);
            }

            let ring_progress = smooth(0.70, 1.02, t);
            let ring_alpha = smooth(0.68, 0.75, t) * (1.0 - ring_progress);
            let radius = lerp(16.0, 43.0, ring_progress);
            for (dx, dy) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)] {
                let x = dx * radius * 0.71;
                let y = dy * radius * 0.71;
                let length = lerp(9.0, 3.0, ring_progress);
                p.line(x - dx * length, y, x, y, rim, ring_alpha);
                p.line(x, y, x, y - dy * length, self.ramp[2], ring_alpha);
                p.glow(x, y, 5.0, self.ramp[2], 0.37 * ring_alpha);
            }

            for (i, &(x, y)) in self.sparks.iter().enumerate() {
                let age = t - (0.69 + 0.015 * (i % 5) as f64);
                let progress = smooth(0.0, 0.28, age);
                let strength = smooth(0.0, 0.045, age) * (1.0 - smooth(0.12, 0.28, age));
                if strength <= 0.0 {
                    continue;
                }
                let distance = lerp(0.29, 1.0, progress);
                let tail = distance - 0.17 * (1.0 - progress);
                let color = if i % 3 == 0 { hot } else { rim };
                p.line(
                    x * tail,
                    y * tail,
                    x * distance,
                    y * distance,
                    color,
                    strength,
                );
            }
        }
    }
}
