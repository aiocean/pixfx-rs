use std::sync::Arc;

use pixfx_core::{Effect, EffectState, Layer, Meta, Options, Trace};
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{clamp, smooth};
use pixfx_draw::sweep::{Brush, Cell, Look, Pose, Turn, sweep};
use pixfx_draw::{Painter, RULER, Ruler};

const FPS: u32 = 30;
const TRAIL_SECS: f64 = 0.22;
const FOLLOW_THROUGH: f64 = 0.28;

#[derive(Clone, Copy)]
pub(super) struct Swing {
    pub radii: (f64, f64),
    pub start: f64,
    pub turn: f64,
    pub tilt: f64,
    pub width: f64,
}

pub struct Slash {
    meta: Meta,
    swing: Swing,
    follows_trace: bool,
    palette: [Rgba; 4],
}

impl Slash {
    pub(super) const fn new(
        id: &'static str,
        name: &'static str,
        desc: &'static str,
        swing: Swing,
        follows_trace: bool,
        rgb: [u32; 4],
    ) -> Self {
        Self {
            meta: Meta {
                id,
                name,
                desc,
                tags: &["slash", "melee", "one-shot"],
                size: (112, 112),
                origin: (56, 56),
                duration: FOLLOW_THROUGH + TRAIL_SECS,
                looping: false,
                fps: FPS,
                seed: 1,
                state: None,
                options: &[],
            },
            swing,
            follows_trace,
            palette: [
                Rgba::hex24(rgb[0]),
                Rgba::hex24(rgb[1]),
                Rgba::hex24(rgb[2]),
                Rgba::hex24(rgb[3]),
            ],
        }
    }
}

impl Effect for Slash {
    fn meta(&self) -> &Meta {
        &self.meta
    }

    fn traces(&self, _: &Options) -> Option<f64> {
        self.follows_trace.then_some(TRAIL_SECS)
    }

    fn spawn(&self, opts: &Options, _: u64) -> Box<dyn EffectState> {
        let mut ramp = opts.palette_or(&self.palette);
        ramp.reverse();
        let reach = Ruler::size(RULER.hand, self.swing.width) as i32;
        let cells = (-reach..=reach)
            .map(|offset| {
                let band = usize::from(offset != reach);
                Cell {
                    across: f64::from(offset),
                    color: ramp[band],
                    light: if band == 0 { 1.0 } else { 0.3 },
                }
            })
            .collect();
        Box::new(State {
            swing: self.swing,
            brush: Brush {
                cells,
                turn: Turn::Heading,
            },
            ramp,
            trace: None,
        })
    }
}

impl Swing {
    /// Sample an accelerating elliptical swing, keeping the age of each position after the swing ends.
    fn poses(self, t: f64) -> Vec<Pose> {
        const SAMPLES: u32 = 16;
        let newest = t.min(FOLLOW_THROUGH);
        let oldest = (t - TRAIL_SECS).max(0.0);
        if newest <= oldest {
            return Vec::new();
        }
        (0..=SAMPLES)
            .map(|sample| {
                let when = newest - (newest - oldest) * f64::from(sample) / f64::from(SAMPLES);
                let (x, y) = self.point(when);
                Pose {
                    x,
                    y,
                    angle: 0.0,
                    age: t - when,
                }
            })
            .collect()
    }

    fn point(self, when: f64) -> (f64, f64) {
        let (cos, sin) = (libm::cos(self.tilt), libm::sin(self.tilt));
        let (rx, ry) = (
            f64::from(Ruler::size(RULER.blade, self.radii.0)),
            f64::from(Ruler::size(RULER.blade, self.radii.1)),
        );
        let u = clamp(when / FOLLOW_THROUGH, 0.0, 1.0);
        let angle = self.start + self.turn * u * (0.28 + 0.72 * u);
        let x = rx * libm::cos(angle);
        let y = ry * libm::sin(angle);
        (x * cos - y * sin, x * sin + y * cos)
    }
}

struct State {
    swing: Swing,
    brush: Brush,
    ramp: Vec<Rgba>,
    trace: Option<Arc<Trace>>,
}

impl State {
    fn paint_swing(&self, p: &mut Painter, t: f64) {
        let poses = self.swing.poses(t);
        let finish = 1.0 - smooth(FOLLOW_THROUGH + 0.02, FOLLOW_THROUGH + TRAIL_SECS, t);
        let reach = f64::from(Ruler::size(RULER.hand, self.swing.width * 2.4));
        let inward = -self.swing.turn.signum();
        let rim = f64::from(Ruler::size(RULER.outline, 1.0));

        // The leading tip is pinched, the ribbon bulges behind it, and the tail thins back to a point.
        // The fill lies mostly inside the tip's path, keeping a clean cutting edge on the outside.
        if poses.len() >= 2 {
            for (outer, inner, color, alpha) in [
                (-0.12, 1.00, self.ramp[3], 0.86),
                (-0.06, 0.68, self.ramp[2], 1.00),
                (0.03, 0.43, self.ramp[1], 1.00),
                (0.00, 0.14, self.ramp[0], 0.95),
            ] {
                let edge = |i: usize, band: f64| {
                    let a = poses[i.saturating_sub(1)];
                    let b = poses[(i + 1).min(poses.len() - 1)];
                    let q = poses[i];
                    let (dx, dy) = (a.x - b.x, a.y - b.y);
                    let length = libm::hypot(dx, dy).max(1e-6);
                    let u = q.age / TRAIL_SECS;
                    let birth = smooth(0.0, 0.11, t - q.age);
                    let swell = smooth(0.0, 0.27, u) * (1.0 - smooth(0.38, 1.0, u));
                    let width = (rim + reach * swell * birth) * finish;
                    let offset = inward * band * width / length;
                    (q.x - dy * offset, q.y + dx * offset)
                };
                let range = 0..poses.len();
                let bounds = range
                    .clone()
                    .map(|i| edge(i, outer))
                    .chain(range.rev().map(|i| edge(i, inner)))
                    .collect::<Vec<_>>();
                p.poly(&bounds, color, alpha * finish);
            }
            if let Some(head) = poses.first() {
                let flash = smooth(0.06, 0.16, t) * (1.0 - smooth(0.24, 0.36, t));
                p.glow(head.x, head.y, rim * 3.0, self.ramp[1], flash * 0.67);
                p.dot(head.x, head.y, self.ramp[0], flash);
            }
        }

        // A handful of pixel chips peel off the fast part of the swing and cool as they drift.
        for index in 0..9 {
            let when = 0.10 + f64::from(index) * 0.020;
            let age = t - when;
            if !(0.0..0.19).contains(&age) {
                continue;
            }
            let (x, y) = self.swing.point(when);
            let (nx, ny) = self.swing.point(when + 0.008);
            let (dx, dy) = (nx - x, ny - y);
            let fan = 3.0 + f64::from(index % 3) * 2.0;
            let opacity = 1.0 - smooth(0.035, 0.19, age);
            let x = x + age * (dx * 18.0 + dy * inward * fan);
            let y = y + age * (dy * 18.0 - dx * inward * fan) + 42.0 * age * age;
            let color = if age < 0.065 {
                self.ramp[0]
            } else {
                self.ramp[1]
            };
            p.dot(x, y, color, opacity);
            p.light(x, y, color, opacity * 0.45);
        }
    }
}

impl EffectState for State {
    fn set_trace(&mut self, trace: Option<Arc<Trace>>) {
        self.trace = trace;
    }

    fn draw(&self, painter: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front || t >= FOLLOW_THROUGH + TRAIL_SECS {
            return;
        }
        let Some(poses) = self
            .trace
            .as_ref()
            .map(|trace| trace.poses(TRAIL_SECS))
            .filter(|poses| {
                poses
                    .windows(2)
                    .any(|p| p[0].x != p[1].x || p[0].y != p[1].y)
            })
        else {
            self.paint_swing(painter, t);
            return;
        };
        let look = Look {
            secs: TRAIL_SECS,
            fade: 2.9,
            taper: 0.0,
            wave: 0.0,
            phase: 0.0,
            breakup: 0.69,
            shift: t * f64::from(FPS),
            soft: 0.57,
            light: 0.45,
            skip: 0.0,
        };
        sweep(painter, &poses, &self.brush, &self.ramp, &look);
    }
}
