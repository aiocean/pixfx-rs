//! Staggered violet comets spiral up around an actor and burn out into fragments.

use std::f64::consts::PI;
use std::sync::Arc;

use pixfx_core::{
    Actor, Area, Canvas, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette,
};
use pixfx_draw::color::{Rgba, mix};
use pixfx_draw::math::{js_round, lerp, rand};
use pixfx_draw::sweep::{Brush, Cell, Look, Pose, Turn, sweep, sweep_filtered};
use pixfx_draw::{Painter, RULER, Ruler};

pub struct CometWeave;

pub static COMET_WEAVE: CometWeave = CometWeave;

const SPREAD: NumOpt = NumOpt::new("spread", 1.0).within(0.5, 2.0);
const LENGTH: NumOpt = NumOpt::new("length", 1.0).within(0.5, 5.0);
const SPARKS: NumOpt = NumOpt::new("sparks", 5.0).within(0.0, 12.0);
const W: NumOpt = NumOpt::new("w", 160.0).within(32.0, 512.0);
const H: NumOpt = NumOpt::new("h", 168.0).within(32.0, 512.0);
const META: Meta = Meta {
    id: "feedback/comet-weave",
    name: "Comet Weave",
    desc: "Staggered violet comets spiral up around the sprite and break into dark fragments.",
    tags: &["feedback", "arcane", "character", "streak"],
    size: (160, 168),
    origin: (80, 112),
    duration: 29.0 / 33.0,
    looping: false,
    fps: 33,
    seed: 0xc041_e759,
    state: None,
    options: &[
        SPREAD.spec(),
        LENGTH.spec(),
        SPARKS.spec(),
        W.spec(),
        H.spec(),
    ],
};

const PALETTE: [Rgba; 7] = [
    Rgba::hex24(0x160a2b),
    Rgba::hex24(0x311052),
    Rgba::hex24(0x4f18a0),
    Rgba::hex24(0x7d29eb),
    Rgba::hex24(0xb843ff),
    Rgba::hex24(0xfa78ff),
    Rgba::hex24(0xffe5ff),
];

impl Effect for CometWeave {
    fn meta(&self) -> &Meta {
        &META
    }

    fn wraps_actor(&self) -> bool {
        true
    }

    fn canvas(&self, opts: &Options) -> Canvas {
        let (w, h) = (opts.num(&W) as u32, opts.num(&H) as u32);
        Canvas {
            size: (w, h),
            origin: ((w / 2) as i32, (h * 2 / 3) as i32),
        }
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let mut ramp = PALETTE;
        for (swatch, color) in ramp.iter_mut().zip(opts.palette_or(&PALETTE)) {
            *swatch = color;
        }
        let mut state = State {
            canvas: self.canvas(opts),
            comets: Vec::new(),
            spread: opts.num(&SPREAD),
            length: opts.num(&LENGTH),
            sparks: opts.num(&SPARKS) as u32,
            seed: f64::from(seed as u32),
            ramp,
        };
        state.set_actor(None);
        Box::new(state)
    }
}

struct State {
    canvas: Canvas,
    comets: Vec<Comet>,
    spread: f64,
    length: f64,
    sparks: u32,
    seed: f64,
    ramp: [Rgba; 7],
}

struct Flight {
    launch: f64,
    flight: f64,
    wake: f64,
    weight: f64,
    /// Starting angle in half turns. Even half turns pass in front of the sprite.
    phase: f64,
}

const FLIGHTS: [Flight; 5] = [
    Flight {
        launch: -0.055,
        flight: 0.60,
        wake: 0.29,
        weight: 1.0,
        phase: 0.65,
    },
    Flight {
        launch: 0.06,
        flight: 0.45,
        wake: 0.18,
        weight: 0.64,
        phase: 1.48,
    },
    Flight {
        launch: 0.12,
        flight: 0.50,
        wake: 0.25,
        weight: 0.77,
        phase: 0.24,
    },
    Flight {
        launch: 0.20,
        flight: 0.45,
        wake: 0.22,
        weight: 0.62,
        phase: 1.10,
    },
    Flight {
        launch: 0.28,
        flight: 0.39,
        wake: 0.17,
        weight: 0.48,
        phase: 1.79,
    },
];

const HALF_TURNS: f64 = 3.0;

struct Comet {
    flight: &'static Flight,
    center_x: f64,
    feet: f64,
    height: f64,
    length: f64,
    radius: f64,
    seed: f64,
}

fn orbit_layer(half_turn: f64) -> Layer {
    if (libm::floor(half_turn) as i32) % 2 == 0 {
        Layer::Front
    } else {
        Layer::Back
    }
}

impl Comet {
    fn at(&self, time: f64) -> (f64, f64) {
        let u = time / self.flight.flight;
        let angle = (self.flight.phase + HALF_TURNS * u) * PI;
        let radius = self.radius * lerp(1.0, 0.67, u);
        (
            self.center_x + radius * libm::cos(angle),
            self.feet
                + self.height * self.length * lerp(0.10, -1.32, u)
                + radius * 0.26 * self.length * libm::sin(angle),
        )
    }

    fn layer_at(&self, time: f64) -> Layer {
        orbit_layer(self.flight.phase + HALF_TURNS * time / self.flight.flight)
    }

    fn poses(&self, t: f64, newest: f64, oldest: f64) -> [Pose; 13] {
        std::array::from_fn(|i| {
            let time = lerp(newest, oldest, i as f64 / 12.0);
            let (x, y) = self.at(time);
            Pose {
                x,
                y,
                angle: 0.0,
                age: t - time,
            }
        })
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer, ramp: &[Rgba; 7], sparks: u32) {
        let f = self.flight;
        let elapsed = t - f.launch;
        if elapsed <= 0.0 || elapsed >= f.flight + f.wake {
            return;
        }

        let shaded;
        let ramp = if layer == Layer::Back {
            shaded = ramp.map(|color| mix(color, ramp[0], 0.22));
            &shaded
        } else {
            ramp
        };

        let after = (elapsed - f.flight).max(0.0);
        let cooling = (after / f.wake).clamp(0.0, 1.0);
        let flare = (1.0 - (elapsed / f.flight - 0.62).max(0.0) / 0.38).clamp(0.0, 1.0);
        let flare = flare * (1.0 - ((t - 0.36) / 0.24).clamp(0.0, 1.0));
        let newest = elapsed.min(f.flight);
        let oldest = (elapsed - f.wake).max(0.0);
        // The widest core is an arm (3 px), edged by violet and ink on either side.
        let half = f64::from(Ruler::size(RULER.limb, f.weight)) - 1.0;
        let mut fade_ramp = *ramp;
        fade_ramp[5] = mix(ramp[3], ramp[5], 0.35 + 0.65 * flare);
        fade_ramp[4] = mix(ramp[3], ramp[4], 0.55 + 0.45 * flare);
        fade_ramp = fade_ramp.map(|c| mix(c, ramp[0], cooling * 0.88));
        let [ink, shadow, violet, blue_violet, magenta, core, _] = fade_ramp;
        fade_ramp.reverse();
        let brush = Brush {
            cells: (-3i32..=3)
                .map(|i| {
                    let color = match i {
                        -2 => blue_violet,
                        -1 => magenta,
                        0 => core,
                        1 => violet,
                        2 => shadow,
                        _ => ink,
                    };
                    Cell {
                        across: f64::from(i) * (half + 1.0) / 3.0,
                        color,
                        light: if i.abs() < 2 { 1.0 } else { 0.0 },
                    }
                })
                .collect(),
            turn: Turn::Heading,
        };
        let look = Look {
            secs: f.wake,
            fade: 5.5,
            taper: 0.0,
            wave: 0.86,
            phase: self.seed,
            breakup: 0.72 - cooling * 0.58,
            shift: 0.0,
            soft: 0.91,
            light: 0.27 * flare,
            skip: 0.0,
        };
        let ridge = Brush {
            cells: [(0.0, ink), (1.0, violet), (2.0, ink)]
                .into_iter()
                .map(|(across, color)| Cell {
                    across,
                    color,
                    light: 0.0,
                })
                .collect(),
            turn: Turn::Heading,
        };
        let ridge_look = Look {
            wave: 1.7,
            phase: self.seed + 1.4,
            breakup: 0.40 - cooling * 0.30,
            ..look
        };

        let first_turn = libm::floor(f.phase + HALF_TURNS * oldest / f.flight);
        let last_turn = libm::floor(f.phase + HALF_TURNS * newest / f.flight);
        let crosses = first_turn != last_turn;
        if crosses || orbit_layer(first_turn) == layer {
            let poses = self.poses(elapsed, newest, oldest);
            let visible = |age| !crosses || self.layer_at(elapsed - age) == layer;
            if elapsed - oldest >= f.wake * 0.22 {
                sweep_filtered(p, &poses, &ridge, &fade_ramp, &ridge_look, |age| {
                    age >= f.wake * 0.22 && visible(age)
                });
            }
            if crosses {
                sweep_filtered(p, &poses, &brush, &fade_ramp, &look, visible);
            } else {
                sweep(p, &poses, &brush, &fade_ramp, &look);
            }
        }

        let (x, y) = self.at(newest);
        if flare > 0.08 && self.layer_at(newest) == layer {
            let (x, y) = (js_round(x), js_round(y));
            // A round cap about one hand in radius on the lead comet.
            let cap = f64::from(Ruler::size(RULER.hand, 1.0)) * f.weight;
            let cap = cap * lerp(0.8, 1.22, flare);
            p.disc(x, y, cap, ramp[1], 1.0);
            p.disc(x, y, cap * 0.72, mix(ramp[3], ramp[5], flare), 1.0);
            p.disc(x, y, cap * 0.43, mix(ramp[4], ramp[6], flare), 1.0);
            // Bloom reaches about three quarters of a face width on the lead comet.
            let radius = f64::from(Ruler::size(RULER.head, 0.75 * f.weight));
            p.glow(x, y, radius, ramp[4], 0.48 * flare);
        }

        for i in 0..sparks {
            let key = f64::from(i);
            let birth = f.flight * lerp(0.22, 0.99, rand(&[self.seed, key, 0.0]));
            let age = elapsed - birth;
            let life = lerp(0.07, 0.17, rand(&[self.seed, key, 1.0]));
            if !(0.0..life).contains(&age) || self.layer_at(birth) != layer {
                continue;
            }
            let (x, y) = self.at(birth);
            let u = age / life;
            let drift = lerp(-5.0, 5.0, rand(&[self.seed, key, 2.0]));
            p.px(x + drift * u, y + (1.0 + u) * u, mix(ramp[3], ramp[0], u));
        }
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        let (w, h) = self.canvas.size;
        let (ox, oy) = self.canvas.origin;
        let bounds = match actor {
            Some(actor) => Silhouette::of(
                &actor,
                Area {
                    x: -ox,
                    y: -oy,
                    w,
                    h,
                },
            )
            .bounds(),
            None => Some(Area {
                x: -(RULER.width as i32) / 2,
                y: -(RULER.height as i32),
                w: RULER.width,
                h: RULER.height,
            }),
        };
        self.comets.clear();
        let Some(b) = bounds else { return };
        let cx = f64::from(b.x) + f64::from(b.w) * 0.5;
        let feet = f64::from(b.y) + f64::from(b.h);
        let height = f64::from(b.h.max(RULER.head));
        // Start one face width beyond the sprite’s sides, or wider for a tall sprite.
        let radius =
            (f64::from(b.w) * 0.5 + f64::from(RULER.head)).max(height * 0.61) * self.spread;
        for (i, flight) in FLIGHTS.iter().enumerate() {
            self.comets.push(Comet {
                flight,
                center_x: cx,
                feet,
                height,
                length: self.length,
                radius,
                seed: (self.seed + i as f64 * 619.0) % 1024.0,
            });
        }
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        for comet in &self.comets {
            comet.draw(p, t, layer, &self.ramp, self.sparks);
        }
    }
}
