//! Twin lightning arcs circle the opaque bounds of the current actor pose.

use std::f64::consts::PI;
use std::sync::Arc;

use pixfx_core::{
    Actor, Area, Canvas, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette,
};
use pixfx_draw::color::{Rgba, mix};
use pixfx_draw::growth::{Tree, jag};
use pixfx_draw::halo::Halo;
use pixfx_draw::math::{js_round, lerp, rand};
use pixfx_draw::path::Path;
use pixfx_draw::{Painter, RULER, Ruler};

pub struct VoltOrbit;

pub static VOLT_ORBIT: VoltOrbit = VoltOrbit;

const GAP: NumOpt = NumOpt::new("gap", 1.0).within(0.5, 2.0);
const JAG: NumOpt = NumOpt::new("jag", 1.0).within(0.0, 1.5);
const SPARKS: NumOpt = NumOpt::new("sparks", 4.0).within(0.0, 8.0);
const W: NumOpt = NumOpt::new("w", 144.0).within(32.0, 512.0);
const H: NumOpt = NumOpt::new("h", 168.0).within(32.0, 512.0);

const META: Meta = Meta {
    id: "feedback/volt-orbit",
    name: "Volt Orbit",
    desc: "Two forked arcs orbit the sprite, flaring from cobalt to white-gold and shedding sparks from their tails.",
    tags: &["feedback", "lightning", "electric", "character", "loop"],
    size: (144, 168),
    origin: (72, 112),
    duration: 0.4,
    looping: true,
    fps: 30,
    seed: 0x71c8_ef2b,
    state: None,
    options: &[GAP.spec(), JAG.spec(), SPARKS.spec(), W.spec(), H.spec()],
};

const PALETTE: [Rgba; 6] = [
    Rgba::hex24(0x102269),
    Rgba::hex24(0x2059e3),
    Rgba::hex24(0x62d8ff),
    Rgba::hex24(0x595015),
    Rgba::hex24(0xffdc43),
    Rgba::hex24(0xffffcf),
];

impl Effect for VoltOrbit {
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
            orbit: None,
            gap: opts.num(&GAP),
            jag: opts.num(&JAG),
            sparks: opts.num(&SPARKS) as u32,
            seed: seed as u32 as f64,
            ramp,
        };
        state.set_actor(None);
        Box::new(state)
    }
}

struct Orbit {
    center: (f64, f64),
    radius: (f64, f64),
}

struct State {
    canvas: Canvas,
    orbit: Option<Orbit>,
    gap: f64,
    jag: f64,
    sparks: u32,
    seed: f64,
    ramp: [Rgba; 6],
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        let (ox, oy) = self.canvas.origin;
        let (w, h) = self.canvas.size;
        let bounds = match actor {
            Some(a) => Silhouette::of(
                &a,
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
        self.orbit = bounds.map(|b| {
            let (rx, ry) = (f64::from(b.w) * 0.5, f64::from(b.h) * 0.5);
            // The orbit clears the silhouette by about one face width.
            let gap = self.gap * f64::from(Ruler::size(RULER.head, 1.0));
            Orbit {
                center: (f64::from(b.x) + rx, f64::from(b.y) + ry),
                radius: ((rx + gap).max(ry * 0.84), ry + gap),
            }
        });
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            return;
        }
        let Some(orbit) = &self.orbit else {
            return;
        };

        let phase = t / META.duration;
        let frame = js_round(t * f64::from(META.fps));
        // A half turn swaps the two heads. The charge builds late and falls away at the seam.
        let charge = (1.0 - ((phase - 0.63) / 0.46).abs()).clamp(0.0, 1.0);
        let (w, h) = self.canvas.size;
        let (ox, oy) = self.canvas.origin;
        let mut halo = Halo::new(w as usize, h as usize, f64::from(ox), f64::from(oy));

        for side in [0.0, 1.0] {
            let angle = 2.2 + (side - phase) * PI;
            let seed = self.seed + side * 137.0 + libm::floor(frame / 2.0) * 997.0;
            self.arc(p, &mut halo, orbit, angle, charge, seed);
        }

        halo.spread(0.28);
        let [blue_shadow, blue, cyan, gold_shadow, gold, white] = self.ramp;
        let shadow = mix(blue_shadow, gold_shadow, charge);
        let corona = mix(blue_shadow, blue, 0.45);
        for (x, y, heat, kind) in halo.each() {
            let color = match kind {
                4 => mix(cyan, white, charge),
                3 => mix(blue, gold, charge),
                2 => mix(blue, gold, charge * 0.42),
                1 => blue,
                _ if heat >= 0.95 => mix(corona, gold_shadow, charge),
                _ if heat >= 0.5 => shadow,
                _ => Rgba(shadow.0, shadow.1, shadow.2, (heat * 320.0) as u8),
            };
            p.px(x, y, color);
        }
    }
}

impl State {
    fn arc(
        &self,
        p: &mut Painter,
        halo: &mut Halo,
        orbit: &Orbit,
        angle: f64,
        charge: f64,
        seed: f64,
    ) {
        let sweep = lerp(1.56, 2.08, charge);
        let unit_arc = Path::begin((libm::cos(angle), libm::sin(angle))).turn((0.0, 0.0), sweep);
        let (cx, cy) = orbit.center;
        let (rx, ry) = orbit.radius;
        let project = |(x, y)| (cx + rx * x, cy + ry * y);
        let anchors: Vec<_> = unit_arc.points(0.36).into_iter().map(project).collect();
        let main = jag(&anchors, 2, 0.46 * self.jag, seed);

        let mut tree = Tree::default();
        tree.path(&main, 1.0);
        let main_nodes = tree.nodes.len();
        let tail_step = tree.steps.max(1);
        for (i, fraction) in [0.21, 0.39, 0.57, 0.73].into_iter().enumerate() {
            let Some(&(x, y)) = main.get((fraction * main.len() as f64) as usize) else {
                continue;
            };
            let a = angle + fraction * sweep;
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            // Each fork slips outward by up to half a face width and trails along the orbit.
            let fork =
                f64::from(Ruler::size(RULER.head, 0.5)) * lerp(0.55, 1.15, rand(&[seed, i as f64]));
            let end = (
                x + fork * (side * libm::cos(a) - 0.8 * libm::sin(a)),
                y + fork * (side * libm::sin(a) + 0.8 * libm::cos(a)),
            );
            tree.path(
                &jag(&[(x, y), end], 2, 0.38 * self.jag, seed + i as f64),
                1.0,
            );
        }

        tree.weigh();
        let radii = tree.radii(0.65, lerp(0.9, 1.4, charge), 0.08);
        tree.stroke(&radii, |dab| {
            let node = &tree.nodes[dab.node];
            let age = (f64::from(node.born) / f64::from(tail_step)).min(1.0);
            let core = 1.0 - age;
            let kind = if dab.node >= main_nodes {
                2
            } else if age < 0.34 && dab.depth < 0.7 {
                4
            } else if age < 0.76 {
                3
            } else {
                1
            };
            halo.stamp(
                f64::from(dab.x),
                f64::from(dab.y),
                (0.48 + 0.72 * core + 0.43 * charge) as f32,
                kind,
            );
        });

        let (hx, hy) = project(unit_arc.start());
        let head_light = mix(self.ramp[2], self.ramp[5], charge);
        p.glow(
            hx,
            hy,
            f64::from(Ruler::size(RULER.head, 0.8)),
            head_light,
            0.28 + charge * 0.52,
        );
        halo.stamp(js_round(hx), js_round(hy), 1.65, 4);

        for i in 0..self.sparks {
            let index = f64::from(i);
            let a = angle + sweep * lerp(0.55, 1.26, rand(&[seed, index, 1.0]));
            let offset = lerp(2.0, 7.0, rand(&[seed, index, 2.0]));
            let x = cx + (rx + offset) * libm::cos(a);
            let y = cy + (ry + offset) * libm::sin(a);
            halo.stamp(js_round(x), js_round(y), 0.32, 1);
        }
    }
}
