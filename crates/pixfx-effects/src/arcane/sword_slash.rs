//! Sword slash: a spirit sword (`super::sword`, drawn `size` times its size so it reads at a character's size)
//! appears beside the foe, winds back and cuts down through it in one fast arc. The blade sweeps a crescent of its
//! own pixels: each blade row's edge colour, swept with the blade's angle (`pixfx_draw::sweep`, `Brush` turned by
//! `Turn::Angle`), white at the blade and cooling along the sword's palette behind it. It fires the impact cue
//! where the cut crosses `hitX`, `hitY`, overshoots, slows, and crumbles from the tip as the sword ring's swords do.
//! Made here; its golden hashes pin it.

use core::f64::consts::PI;

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{clamp, rand, smooth};
use pixfx_draw::sweep::{Brush, Cell, Look, Pose as SweepPose, Turn, sweep};

use super::sword::{
    AQUA, CYAN, DUSK, GLOW, ICE, PIVOT_ROW, Pose, SKY, SPRITE, STEEL, TIP, WHITE, colour_of,
    sword_at,
};

pub struct SwordSlash;

pub static SWORD_SLASH: SwordSlash = SwordSlash;

const SIZE: NumOpt = NumOpt::new("size", 2.0).within(1.0, 3.0);
/// Where the cut crosses, in local pixels from the feet (chest height in front of the origin by default).
const HIT_X: NumOpt = NumOpt::new("hitX", 0.0).within(-40.0, 40.0);
const HIT_Y: NumOpt = NumOpt::new("hitY", -22.0).within(-60.0, 0.0);

const META: Meta = Meta {
    id: "arcane/sword-slash",
    name: "Sword slash",
    desc: "A spirit sword appears beside the foe, winds back and cuts down through it in one fast arc, sweeping a crescent of its own blade that cools from white to steel; it flashes where the cut lands (hitX, hitY), overshoots and crumbles from the tip.",
    tags: &[
        "arcane",
        "impact",
        "swords",
        "cultivation",
        "melee",
        "one-shot",
    ],
    size: (128, 112),
    origin: (64, 84),
    duration: SCATTER_END,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[FEEL.spec(), SIZE.spec(), HIT_X.spec(), HIT_Y.spec()],
};

/// The timeline, seconds: it grows out of its guard, winds back, cuts, follows through, crumbles.
const APPEAR_END: f64 = 0.15;
const WINDUP_END: f64 = 0.30;
const SWING_END: f64 = 0.42;
const FOLLOW_END: f64 = 0.60;
const SCATTER_END: f64 = 0.85;

/// The blade's angles (radians, 0 along +x, y down, so growing turns it clockwise): at rest up behind, wound back
/// further, through the foe pointing straight ahead, ending low in front and overshooting a little.
const REST: f64 = -2.0;
const BACK: f64 = -2.35;
const HIT: f64 = 0.0;
const END: f64 = 0.75;
const OVER: f64 = 0.95;

/// How far from the guard the cut crosses the hit point, as a share of the blade.
const BITE: f64 = 0.7;
/// The share of the blade, from the guard, that leaves no crescent: only its outer part sweeps one.
const CRESCENT: f64 = 0.4;
/// How long the crescent lasts behind the blade, seconds.
const SMEAR: f64 = 0.14;
/// The crescent's colours, from the blade's edge out to where it cools.
const RAMP: [Rgba; 7] = [WHITE, ICE, AQUA, CYAN, SKY, STEEL, DUSK];

/// The blade's angle at `t`.
fn angle(t: f64) -> f64 {
    if t < APPEAR_END {
        REST
    } else if t < WINDUP_END {
        REST + (BACK - REST) * smooth(APPEAR_END, WINDUP_END, t)
    } else if t < SWING_END {
        // gathering speed all the way through the cut
        let u = (t - WINDUP_END) / (SWING_END - WINDUP_END);
        BACK + (END - BACK) * u * u
    } else if t < FOLLOW_END {
        let u = (t - SWING_END) / (FOLLOW_END - SWING_END);
        END + (OVER - END) * (1.0 - (1.0 - u) * (1.0 - u))
    } else {
        OVER
    }
}

/// When the blade points through the hit point.
fn hit_time() -> f64 {
    let u = libm::sqrt((HIT - BACK) / (END - BACK));
    WINDUP_END + u * (SWING_END - WINDUP_END)
}

impl Effect for SwordSlash {
    fn meta(&self) -> &Meta {
        &META
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(hit_time(), 0.5, opts.num(&HIT_X), opts.num(&HIT_Y), opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let scale = opts.num(&SIZE);
        let hit = (opts.num(&HIT_X), opts.num(&HIT_Y));
        let bite = BITE * TIP * scale;
        Box::new(Slash {
            scale,
            hit,
            pivot: (hit.0 - bite * libm::cos(HIT), hit.1 - bite * libm::sin(HIT)),
            brush: blade(scale),
            seed: seed as f64,
        })
    }
}

/// The blade as a brush: its outer part (past `CRESCENT`) to the tip, each engine pixel along it the colour of its row's
/// edge (column 3: ice at the tip darkening to steel at the guard), brighter toward the tip, turned with the blade.
fn blade(scale: f64) -> Brush {
    let reach = TIP * scale;
    let mut cells = Vec::new();
    let mut d = CRESCENT * reach;
    while d <= reach {
        let row = libm::floor(PIVOT_ROW - d / scale + 0.5);
        let pick = |col: usize| {
            SPRITE
                .get(row as usize)
                .and_then(|r| r.get(col))
                .and_then(|&ch| colour_of(ch))
        };
        if let Some(color) = pick(3).or_else(|| pick(2)) {
            cells.push(Cell {
                across: d,
                color,
                light: 0.3 + 0.7 * d / reach,
            });
        }
        d += 1.0;
    }
    Brush {
        cells,
        turn: Turn::Angle(-PI / 2.0),
    }
}

struct Slash {
    scale: f64,
    hit: (f64, f64),
    /// Where the blade turns: its guard, behind the hit point along the way it points as it cuts.
    pivot: (f64, f64),
    brush: Brush,
    seed: f64,
}

impl Slash {
    /// The crescent: the blade's poses from the start of the cut up to `t`, newest first.
    fn crescent(&self, p: &mut Painter, t: f64) {
        if t < WINDUP_END {
            return;
        }
        let step = 1.0 / 240.0;
        let mut poses = Vec::new();
        let mut age = 0.0;
        while age <= SMEAR && t - age >= WINDUP_END {
            poses.push(SweepPose {
                x: self.pivot.0,
                y: self.pivot.1,
                angle: angle(t - age),
                age,
            });
            age += step;
        }
        let look = Look {
            secs: SMEAR,
            fade: 4.0,
            taper: 0.3,
            wave: 0.0,
            phase: 0.0,
            breakup: 0.45,
            shift: libm::floor(t * 30.0),
            light: 0.6,
            skip: 0.0,
            soft: 1.0,
        };
        sweep(p, &poses, &self.brush, &RAMP, &look);
    }

    /// Where the cut lands: a white flash and sparks thrown on along the way the blade goes.
    fn flash(&self, p: &mut Painter, t: f64) {
        let since = t - hit_time();
        if !(0.0..0.2).contains(&since) {
            return;
        }
        let k = 1.0 - since / 0.2;
        let (hx, hy) = self.hit;
        p.glow(hx, hy, 6.0 * self.scale, WHITE, 0.9 * k);
        p.glow(hx, hy, 12.0 * self.scale, GLOW, 0.5 * k);
        // the blade's way at the hit, square to it, clockwise
        let way = HIT + PI / 2.0;
        for i in 0..5 {
            let fi = f64::from(i);
            let a = way + (rand(&[fi, self.seed, 53.0]) - 0.5) * 1.2;
            let speed = 60.0 + 60.0 * rand(&[fi, self.seed, 59.0]);
            let x = hx + libm::cos(a) * speed * since;
            let y = hy + libm::sin(a) * speed * since + 80.0 * since * since;
            p.dot(
                libm::floor(x),
                libm::floor(y),
                if i % 2 == 0 { WHITE } else { ICE },
                k,
            );
            p.light(x, y, GLOW, 0.5 * k);
        }
    }
}

impl EffectState for Slash {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        self.crescent(p, t);
        // a glint as it forms, white-hot through the cut
        let glint = clamp(1.0 - libm::fabs(t - 0.12) / 0.06, 0.0, 1.0);
        let cut = if (WINDUP_END..SWING_END).contains(&t) {
            0.35
        } else {
            0.0
        };
        sword_at(
            p,
            &Pose {
                x: self.pivot.0,
                y: self.pivot.1,
                angle: angle(t),
                grown: smooth(0.0, APPEAR_END * 0.8, t),
                hot: glint.max(cut),
                alpha: 1.0 - smooth(FOLLOW_END, SCATTER_END, t),
            },
            self.scale,
        );
        self.flash(p, t);
    }
}
