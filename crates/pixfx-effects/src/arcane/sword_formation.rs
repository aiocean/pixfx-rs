//! Sword formation: eight flying spirit swords gather one by one behind the caster, fan out together into a ring
//! behind the caster's back and turn there, held, until the game orders an attack (`Scene::order`); then they
//! hang where they are, each turning to its way (`arcane/held-sword`), and leave one after another, 0.1 s apart,
//! nearest the point first, as `arcane/flying-sword` volley pieces trailing `projectiles/trail`, the scene flying
//! them to any distance. Converge: each sword first flies out the way it points, then curves onto the point and
//! flies on, bursting (`arcane/sword-strike`) on the first body it meets. Pierce: each flies straight at the point
//! and on, through every body it meets. A sword that meets nothing flies on 200 pixels past the point and fades.
//!
//! Made here from the sheets of `pixfx-ref`'s `examples/specs/arcane/sword-formation.json`: the look is the
//! model's, the count, the order and the paths are set here. Its golden hashes
//! pin it. It opens once (gathering, fanning out) and loops from `FAN_END`: the ring turns an eighth of a turn
//! each loop, so with eight like swords every loop starts where the last ended. The ring and the gathering are
//! behind the character.

use core::f64::consts::{PI, TAU};

use pixfx_core::volley::{Launch, Order, Shot, Tail, Volley};
use pixfx_core::{Effect, EffectState, Layer, Meta, OptValue, Options};
use pixfx_draw::Painter;
use pixfx_draw::math::{clamp, smooth};

use super::flying_sword::{FLYING_SWORD, PIVOT};
use super::held_sword::HELD_SWORD;
use super::sword::{AQUA, GLOW, ICE, Pose, SKY, STEEL, TIP, WHITE, sword};
use super::sword_strike::SWORD_STRIKE;
use crate::projectiles::TRAIL;

pub struct SwordFormation;

pub static SWORD_FORMATION: SwordFormation = SwordFormation;

const META: Meta = Meta {
    id: "arcane/sword-formation",
    name: "Sword formation",
    desc: "Eight flying swords gather one by one behind the caster, fan out into a ring and turn there until the game orders a strike: then they dive one after another, 0.1 s apart, at the point it names, curving in together (converge), once round it on a ring first (orbit) or straight through it (pierce), to any distance.",
    tags: &[
        "arcane",
        "swords",
        "flying-swords",
        "cultivation",
        "volley",
        "loop",
    ],
    size: (72, 72),
    origin: (36, 64),
    duration: (FAN_END_FRAME + PERIOD_FRAMES) / FPS,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

/// Always eight swords.
const SWORDS: usize = 8;

/// The ring: its centre behind the shoulders, the pivots' radius (tips reach 25 out: 56 across with the glow).
const CX: f64 = -8.0;
const CY: f64 = -30.0;
const RING: f64 = 14.0;
/// The gathered swords, upright side by side behind the caster: their pivots' height and spacing.
const STACK_Y: f64 = -24.0;
const STACK_GAP: f64 = 5.0;

/// Seconds. Sword `k` of the stack appears at `GATHER + ORDER position * GATHER_GAP`, forming over `FORM`;
/// they fan out together from `FAN` to `FAN_END`, then the ring turns an eighth of a turn every `PERIOD`.
/// The opening and the loop end on whole frames, so the loop closes exactly.
const FPS: f64 = 30.0;
const GATHER: f64 = 0.05;
const GATHER_GAP: f64 = 0.07;
const FORM: f64 = 0.15;
const FAN: f64 = 0.7;
const FAN_END_FRAME: f64 = 28.0;
const FAN_END: f64 = FAN_END_FRAME / FPS;
const PERIOD_FRAMES: f64 = 36.0;
const PERIOD: f64 = PERIOD_FRAMES / FPS;
/// Seconds between swords leaving the ring.
const GAP: f64 = 0.1;
/// How fast a sword flies, pixels a second, and how long a flight takes at least and at most.
const SPEED: f64 = 300.0;
const MIN_FLIGHT: f64 = 0.2;
const MAX_FLIGHT: f64 = 2.4;
/// From a flying sword's origin to its tip.
const NOSE: f64 = PIVOT + TIP;
/// How far a sword flies on past the point it was aimed at, striking what it meets, before it fades.
const BEYOND: f64 = 200.0;

/// The order the stack fills in (middle out), as stack positions left to right.
const ORDER: [usize; SWORDS] = [3, 4, 2, 5, 1, 6, 0, 7];
/// The ring angle each stack position fans out to (left to right across the stack: tips pointing left, down-left,
/// up-left, up, down, up-right, down-right, right), so no two cross.
const RING_ANGLE: [f64; SWORDS] = [
    PI,
    0.75 * PI,
    -0.75 * PI,
    -0.5 * PI,
    0.5 * PI,
    -0.25 * PI,
    0.25 * PI,
    0.0,
];

impl Effect for SwordFormation {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(Formation)
    }
    fn loop_from(&self) -> Option<f64> {
        Some(FAN_END)
    }
    fn order(
        &self,
        opts: &Options,
        t: f64,
        at: (f64, f64),
        flip: bool,
        order: &Order,
    ) -> Vec<Launch> {
        let seed = opts.seed.unwrap_or(META.seed) as f64;
        volley(t, at, flip, order, seed)
    }
}

/// The ring's turn at `t`: none until it has fanned out, then an eighth of a turn every `PERIOD`.
fn turn(t: f64) -> f64 {
    (t - FAN_END).max(0.0) / PERIOD * TAU / SWORDS as f64
}

/// The shortest turn from angle `a` to angle `b`.
fn turn_to(a: f64, b: f64) -> f64 {
    let d = libm::fmod(b - a + PI, TAU);
    let d = if d < 0.0 { d + TAU } else { d };
    d - PI
}

/// Sword `k` (a stack position) in the ring at `t`: its pivot and the way it points, in local pixels.
fn ring(k: usize, t: f64) -> (f64, f64, f64) {
    let a = RING_ANGLE[k] + turn(t);
    (CX + RING * libm::cos(a), CY + RING * libm::sin(a), a)
}

/// How the swords fly at an order: a sword nearest the target first, the rest `GAP` apart, at `SPEED`; striking
/// with the tip sunk two pixels in, or piercing `BEYOND` pixels past.
static VOLLEY: Volley = Volley {
    piece: &FLYING_SWORD,
    held: Some(&HELD_SWORD),
    tail: Some(Tail {
        effect: &TRAIL,
        opts: tail,
    }),
    hit: Some(&SWORD_STRIKE),
    poise: 0.12,
    gap: GAP,
    speed: SPEED,
    min_secs: MIN_FLIGHT,
    max_secs: MAX_FLIGHT,
    nose: NOSE,
    sink: 2.0,
    bend: 0.2,
    bend_min: 12.0,
    bend_max: 28.0,
    spread: (6.0, 12.0),
    ease: 1.0,
    beyond: BEYOND,
    lands: false,
    // once round on a ring 1.2 fighters out (the fighter is 40 rows, RULER), clear of a body standing at the point
    orbit_turns: 1.0,
    orbit_radius: 48.0,
};

/// Each flying sword's trail: in the sword's colours, white at the guard to steel, five pixels wide, as long as
/// the sword flew in 0.28 s (84 pixels at full speed), starting under its grip (6 pixels back from its origin, the
/// sword drawn over it).
fn tail() -> Options {
    let mut o = Options {
        palette: Some(vec![WHITE, ICE, AQUA, SKY, STEEL]),
        ..Options::default()
    };
    for (k, v) in [("secs", 0.28), ("width", 5.0), ("wave", 1.2), ("skip", 6.0)] {
        o.extra.insert(k.to_string(), OptValue::Num(v));
    }
    o
}

/// The volley: each sword from where it is in the ring at `t` (on the stage: `at` its origin, mirrored with
/// `flip`), as a piece whose origin is `-PIVOT` ahead of its guard.
fn volley(t: f64, at: (f64, f64), flip: bool, order: &Order, seed: f64) -> Vec<Launch> {
    let sx = if flip { -1.0 } else { 1.0 };
    let swords: Vec<Shot> = (0..SWORDS)
        .map(|k| {
            let (x, y, a) = ring(k, t);
            // mirrored, a sword pointing right points left
            let heading = if flip { PI - a } else { a };
            Shot {
                at: (
                    at.0 + sx * x - PIVOT * libm::cos(heading),
                    at.1 + y - PIVOT * libm::sin(heading),
                ),
                heading,
            }
        })
        .collect();
    VOLLEY.aim(&swords, order, seed)
}

struct Formation;

impl EffectState for Formation {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            return;
        }
        for k in 0..SWORDS {
            if let Some(pose) = pose(k, t) {
                speck(p, k, t);
                sword(p, &pose);
            }
        }
        // a faint bloom inside the ring
        let k = smooth(FAN, FAN_END, t);
        if k > 0.0 {
            p.glow(CX, CY, 30.0, GLOW, 0.25 * k);
        }
    }
}

/// Sword `k` (a stack position) at `t`, or none before it appears.
fn pose(k: usize, t: f64) -> Option<Pose> {
    let slot = ORDER.iter().position(|&o| o == k).unwrap_or(0);
    let born = GATHER + slot as f64 * GATHER_GAP;
    if t < born {
        return None;
    }
    let up = -0.5 * PI;
    let stack = (CX + (k as f64 - 3.5) * STACK_GAP, STACK_Y);
    let mut pose = Pose {
        x: stack.0,
        y: stack.1,
        angle: up,
        grown: clamp((t - born) / FORM, 0.0, 1.0),
        hot: clamp(1.0 - (t - born - FORM * 0.5) / 0.2, 0.0, 1.0),
        alpha: 1.0,
    };
    if t < FAN {
        // gathered, bobbing a pixel
        pose.y += libm::floor(libm::sin(t * 9.0 + k as f64) * 0.8 + 0.5);
        return Some(pose);
    }
    let (rx, ry, ra) = ring(k, t);
    let f = smooth(FAN, FAN_END, t);
    pose.x = stack.0 + (rx - stack.0) * f;
    pose.y = stack.1 + (ry - stack.1) * f;
    pose.angle = up + turn_to(up, ra) * f;
    if t >= FAN_END {
        // a glint as each sword passes the top of the ring
        let d = libm::fabs(turn_to(ra, up));
        pose.hot = clamp(1.0 - d / 0.35, 0.0, 1.0) * 0.8;
    }
    Some(pose)
}

/// The speck of light sword `k` forms from.
fn speck(p: &mut Painter, k: usize, t: f64) {
    let slot = ORDER.iter().position(|&o| o == k).unwrap_or(0);
    let since = t - GATHER - slot as f64 * GATHER_GAP;
    if !(0.0..0.2).contains(&since) {
        return;
    }
    let x = CX + (k as f64 - 3.5) * STACK_GAP;
    let y = STACK_Y - 6.0;
    let s = 1.0 - since / 0.2;
    p.dot(x, y, WHITE, 1.0);
    if s > 0.4 {
        for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            p.dot(x + dx, y + dy, ICE, 1.0);
        }
    }
    p.glow(x, y, 6.0, GLOW, 0.8 * s);
}
