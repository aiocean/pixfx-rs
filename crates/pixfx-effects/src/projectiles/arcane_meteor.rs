//! A four-point flare leads a cyan meteor, then the head burns out into a violet wake.
//! Motion reference: https://img.itch.zone/aW1hZ2UvMjg1MzYxMy8yMzk2MzI0OC5naWY=/original/HWEfZH.gif

use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::color::{Rgba, mix};
use pixfx_draw::math::lerp;
use pixfx_draw::sweep::{Brush, Look, Pose, sweep};
use pixfx_draw::{Painter, RULER, Ruler};

pub struct ArcaneMeteor;
pub static ARCANE_METEOR: ArcaneMeteor = ArcaneMeteor;

const FPS: u32 = 33;
// Start offset from the landing point passed to play().
const START_X: NumOpt = NumOpt::new("startX", 34.0).within(-48.0, 48.0);
const START_Y: NumOpt = NumOpt::new("startY", -36.0).within(-56.0, 0.0);
const META: Meta = Meta {
    id: "projectiles/arcane-meteor",
    name: "Arcane Meteor",
    desc: "A star flares as a cyan meteor sweeps down-left, then its violet wake breaks apart.",
    tags: &["projectiles", "meteor", "arcane", "loop"],
    size: (144, 120),
    origin: (20, 100),
    duration: 24.0 / FPS as f64,
    looping: true,
    fps: FPS,
    seed: 1,
    state: None,
    options: &[START_X.spec(), START_Y.spec()],
};

const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x2103b6),
    Rgba::hex24(0x4407ed),
    Rgba::hex24(0x8d27ff),
    Rgba::hex24(0xf02dff),
    Rgba::hex24(0x31eaff),
    Rgba::hex24(0xf5faff),
];

impl Effect for ArcaneMeteor {
    fn meta(&self) -> &Meta {
        &META
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let mut colors = PALETTE;
        for (slot, color) in colors.iter_mut().zip(opts.palette_or(&PALETTE)) {
            *slot = color;
        }
        let start = (opts.num(&START_X), opts.num(&START_Y));
        let angle = if start == (34.0, -36.0) || start == (0.0, 0.0) {
            0.0
        } else {
            libm::atan2(start.1, start.0) - libm::atan2(-36.0, 34.0)
        };
        Box::new(Meteor {
            colors,
            start,
            wake_axis: (
                0.77 * libm::cos(angle) + 0.64 * libm::sin(angle),
                -0.64 * libm::cos(angle) + 0.77 * libm::sin(angle),
            ),
        })
    }
}

struct Meteor {
    colors: [Rgba; 6],
    start: (f64, f64),
    wake_axis: (f64, f64),
}

impl EffectState for Meteor {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Front {
            return;
        }
        let frame = (t * f64::from(FPS) + 1e-9) as u32;
        // Start where the meteor first appears (reference frame 20).
        // Each pose holds through its authored beat, followed by seven clear ticks.
        let (x, y, length, width, heat, star) = match frame {
            0..=2 => (34.0, -36.0, 0.59, 0.74, 0.40, 0.23),
            3..=5 => (23.0, -25.0, 1.0, 1.0, 0.78, 1.0),
            6..=8 => (14.0, -16.0, 0.98, 0.91, 1.0, 0.0),
            9..=10 => (3.0, -5.0, 1.04, 1.08, 1.0, 0.0),
            11 => (0.0, 0.0, 0.91, 0.91, 0.66, 0.0),
            12..=15 => (0.0, 0.0, 0.79, 0.88, 0.21, 0.0),
            16 => (1.0, -4.0, 0.40, 0.54, 0.0, 0.0),
            _ => return,
        };

        let (x, y) = (x / 34.0 * self.start.0, y / -36.0 * self.start.1);
        let [indigo, violet, lilac, magenta, cyan, white] = self.colors;
        // The wake reaches two and a half blade lengths along the diagonal. The head spans one face.
        let tail = f64::from(Ruler::size(RULER.blade, 2.5)) * length;
        let radius = f64::from(Ruler::size(RULER.head, 0.5)) * width;
        let fan = if frame < 11 { 1.0 } else { 0.79 };

        wake(
            p,
            (x, y),
            self.wake_axis,
            tail * 0.92,
            0.0,
            0.0,
            radius * 2.7,
            &[violet, violet, violet, indigo],
            0.80,
        );
        for (lane, reach, bend, thickness) in [
            (-1.4_f64, 0.77, -0.3, 0.36),
            (-0.95, 1.03, -0.65, 0.47),
            (-0.55, 0.85, -0.20, 0.65),
            (0.49, 0.87, 0.27, 0.62),
            (0.91, 0.96, 0.30, 0.47),
            (1.46, 0.77, 0.58, 0.32),
            (0.0, 0.94, 0.0, 1.85),
        ] {
            let ramp = if heat >= 0.4 && lane.abs() < 0.7 {
                [magenta, lilac, violet, indigo]
            } else {
                [violet, violet, violet, indigo]
            };
            wake(
                p,
                (x, y),
                self.wake_axis,
                tail * reach,
                radius * lane * fan,
                bend,
                radius * thickness,
                &ramp,
                if frame >= 11 { 0.56 } else { 0.83 },
            );
        }

        p.disc(x, y, radius * 0.83, violet, 1.0);
        if heat >= 0.4 {
            let hot = mix(magenta, cyan, heat);
            let core = mix(white, hot, 0.35);
            let ramp = [core, hot, magenta, lilac];
            wake(
                p,
                (x, y),
                self.wake_axis,
                tail * 0.68,
                0.0,
                -0.12,
                radius * 1.50,
                &ramp,
                0.79,
            );
            p.disc(x - 0.5, y + 0.5, radius * 0.67, hot, 1.0);
            p.disc(x - 1.0, y + 1.0, radius * 0.33, core, 1.0);
            p.glow(x, y, radius * 2.8, lilac, heat * 0.34);
            p.glow(x + radius, y - radius, radius * 2.1, hot, heat * 0.22);
        } else if heat > 0.0 {
            p.disc(x - 0.5, y + 0.5, radius * 0.43, lilac, 1.0);
            p.glow(x, y, radius * 1.9, lilac, heat);
        }

        if (9..=16).contains(&frame) {
            let spread = lerp(0.80, 1.55, f64::from(frame.min(12) - 9) / 3.0);
            for (dx, dy, len) in [
                (-1.45, -0.25, 0.21),
                (-1.12, -0.87, 0.14),
                (-0.75, -1.56, 0.14),
                (0.79, 0.89, 0.18),
                (1.42, 0.38, 0.13),
                (1.84, -0.11, 0.10),
            ] {
                let ramp = if frame < 12 {
                    [cyan, lilac, violet, indigo]
                } else {
                    [violet, violet, indigo, indigo]
                };
                wake(
                    p,
                    (x + dx * radius * spread, y + dy * radius * spread),
                    self.wake_axis,
                    tail * len,
                    0.0,
                    0.2 * dx,
                    1.0,
                    &ramp,
                    0.54,
                );
            }
        }

        if star > 0.0 {
            flare(
                p,
                x + radius * 0.74,
                y - radius * 0.64,
                radius * 3.9 * star,
                [violet, cyan, white],
            );
        }
    }
}

/// A tapered brush along a gently bent diagonal. Lanes diverge at the tail,
/// leaving dark gaps between the long, sharp tongues.
#[allow(clippy::too_many_arguments)]
fn wake(
    p: &mut Painter,
    (x, y): (f64, f64),
    (ax, ay): (f64, f64),
    length: f64,
    lane: f64,
    bend: f64,
    width: f64,
    ramp: &[Rgba; 4],
    breakup: f64,
) {
    let path = [0.0, 0.33, 0.68, 1.0].map(|age| {
        let spine = length * age;
        let offset = lane * (0.52 + 1.24 * age) + bend * libm::sin(age * 7.0) * age * 3.0;
        Pose {
            x: x + ax * spine - ay * offset,
            y: y + ay * spine + ax * offset,
            angle: 0.0,
            age,
        }
    });
    let brush = Brush::column(&ramp[..2], width);
    sweep(
        p,
        &path,
        &brush,
        ramp,
        &Look {
            secs: 1.0,
            fade: 3.0,
            taper: 0.0,
            wave: 0.0,
            phase: 0.0,
            breakup,
            shift: 0.0,
            soft: 1.0,
            light: 0.13,
            skip: 0.0,
        },
    );
}

fn flare(p: &mut Painter, x: f64, y: f64, radius: f64, [violet, cyan, white]: [Rgba; 3]) {
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        p.poly(
            &[
                (x + dy * radius * 0.09, y + dx * radius * 0.09),
                (x + dx * radius, y + dy * radius),
                (x - dy * radius * 0.09, y - dx * radius * 0.09),
            ],
            violet,
            1.0,
        );
        for i in 0..radius as u32 {
            let d = f64::from(i);
            let u = d / radius;
            let color = mix(white, cyan, (u * 1.75).min(1.0));
            p.blend(x + dx * d, y + dy * d, color, 1.0 - u * u * u);
        }
    }
    p.disc(x, y, radius * 0.12, white, 1.0);
    p.glow(x, y, radius * 0.85, cyan, 0.56);
}
