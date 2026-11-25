//! `melee/sword-slash`: the smear a blade leaves through an overhead cut on the character it plays on, from raised
//! behind the head, over it and forward to shoulder height, dissolving behind the blade, with a light blow as the
//! blade arrives. A rigid bar (the arm out to the hilt, then the blade, both sized by the ruler) turns about the
//! shoulder and is swept with [`pixfx_draw::sweep`] turned by its own angle ([`Turn::Angle`]): the blade that turns
//! sweeps its own pixels. Made here, not ported; its golden hashes pin it.
//!
//! It wraps its actor ([`Effect::wraps_actor`]) and reads the sprite for where the swing is: the shoulder, one head
//! above the silhouette's centroid, taken from the first sprite handed over and held for the one-shot, so the arc
//! does not jump when the pose changes mid-smear. With no sprite it swings from the ruler fighter's shoulder.
//!
//! Every frame is a function of `t` and the options: no state advances, any frame stands alone, a respawn is the
//! same frames. The arc is the effect's own path, not a trace: a trace holds the character's placement angle, which
//! a swing never turns.

use std::sync::Arc;

use core::f64::consts::{FRAC_PI_2, PI};

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{
    Actor, Area, Canvas, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette,
};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{clamp, js_min};
use pixfx_draw::ruler::{RULER, Ruler};
use pixfx_draw::sweep::{Brush, Cell, Look, Pose, Turn, sweep};

pub struct SwordSlash;

pub static SWORD_SLASH: SwordSlash = SwordSlash;

const META: Meta = Meta {
    id: "melee/sword-slash",
    name: "Sword slash",
    desc: "The smear a blade leaves through an overhead cut on the character it plays on: raised behind the head, over it and forward to shoulder height, dissolving behind the blade, with a light blow as it arrives. Play it at the character's feet as it strikes.",
    tags: &[
        "melee",
        "slash",
        "sword",
        "streak",
        "character",
        "impact",
        "one-shot",
        "directional",
    ],
    // `canvas` sizes it from `w` and `h`; this is their default
    size: SIZE,
    origin: ((SIZE.0 / 2) as i32, RULER.feet.1),
    duration: 0.35,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[
        ARC.spec(),
        SECS.spec(),
        BLADE.spec(),
        W.spec(),
        H.spec(),
        FEEL.spec(),
    ],
};

/// The sweep, in degrees: the blade starts `arc` degrees back from forward (135: raised behind the head, 45 past
/// vertical) and ends forward.
const ARC: NumOpt = NumOpt::new("arc", 135.0).within(30.0, 180.0);
/// How long the smear stays before the dither has eaten it; its max plus `SWING` is the duration.
const SECS: NumOpt = NumOpt::new("secs", 0.2).within(0.05, 0.25);
/// The blade's length in pixels: the ruler's blade (`RULER.blade`, ratio 1.0).
const BLADE: NumOpt = NumOpt::new("blade", RULER.blade as f64).within(4.0, 32.0);
/// The canvas, the feet `FLOOR_ROOM` px above its bottom middle.
const FLOOR_ROOM: u32 = 16;
const SIZE: (u32, u32) = (128, RULER.feet.1 as u32 + FLOOR_ROOM);
const W: NumOpt = NumOpt::new("w", SIZE.0 as f64).within(32.0, 512.0);
const H: NumOpt = NumOpt::new("h", SIZE.1 as f64).within(32.0, 512.0);

/// The blade's turn from raised to forward, in seconds: a cut is fast, the smear is what reads.
const SWING: f64 = 0.1;
/// Shoulder to the hilt, the arm out: one and a half heads in the fighter's Slash pose.
fn hilt() -> u32 {
    Ruler::size(RULER.head, 1.5)
}
/// The shoulder sits one head above the body's centroid.
fn shoulder_above_middle() -> u32 {
    Ruler::size(RULER.head, 1.0)
}
/// The ruler fighter's shoulder with no sprite, at 57.5% of its height above the soles.
const STAND_IN_PIVOT: (f64, f64) = (0.0, -(((RULER.height * 23 + 20) / 40) as f64));
/// Poses across the swing: at most 22.5 degrees each at the widest `arc`; `sweep` subsamples to half a pixel.
const SAMPLES: u32 = 8;
/// `sweep` lays a brush along +y at angle 0; a blade points along +x at heading 0.
const BRUSH_BASE: f64 = -FRAC_PI_2;
/// A light-medium blow: shake and a 4-frame hit-stop, no ripple, no flash.
const WEIGHT: f64 = 0.4;
/// The fighter's blade colours (edge, blade, dull) ending in the trail's deep blue, so the two streaks fade alike.
const PALETTE: [Rgba; 4] = [
    Rgba::hex24(0xffffff),
    Rgba::hex24(0xdcfbff),
    Rgba::hex24(0x8fc8e8),
    Rgba::hex24(0x3d85b0),
];

impl Effect for SwordSlash {
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
            origin: ((w / 2) as i32, h.saturating_sub(FLOOR_ROOM) as i32),
        }
    }

    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        // the blow when the blade arrives forward, at the stand-in's tip (cues have no sprite to read)
        impact(
            SWING,
            WEIGHT,
            f64::from(hilt()) + BLADE.default,
            STAND_IN_PIVOT.1,
            opts,
        )
    }

    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        let ramp = opts.palette_or(&PALETTE);
        let blade = opts.num(&BLADE) as u32;
        Box::new(State {
            arc: opts.num(&ARC) * PI / 180.0,
            secs: opts.num(&SECS),
            brush: brush(&ramp, blade),
            ramp,
            // the stand-in shoulder: a scene that never hands a sprite never calls `set_actor`
            pivot: STAND_IN_PIVOT,
            read: false,
        })
    }
}

/// The bar from the hilt to the tip, measured from the shoulder: the inner half the blade's colour, the outer half
/// its white edge, lit. A ramp of one colour uses it for both; an empty ramp is an empty brush, nothing drawn.
fn brush(ramp: &[Rgba], blade: u32) -> Brush {
    let hilt = hilt();
    let cells = match (ramp.first(), ramp.get(1).or(ramp.first())) {
        (Some(&edge), Some(&body)) => (0..blade)
            .map(|k| {
                let outer = k >= blade / 2;
                Cell {
                    across: f64::from(hilt + k),
                    color: if outer { edge } else { body },
                    light: if outer { 1.0 } else { 0.4 },
                }
            })
            .collect(),
        _ => Vec::new(),
    };
    Brush {
        cells,
        turn: Turn::Angle(BRUSH_BASE),
    }
}

struct State {
    /// The sweep in radians.
    arc: f64,
    secs: f64,
    ramp: Vec<Rgba>,
    brush: Brush,
    /// The shoulder the bar turns about: the stand-in until a sprite says otherwise.
    pivot: (f64, f64),
    /// A sprite's shoulder was taken: later sprites are ignored, so the arc holds through a pose change.
    read: bool,
}

impl State {
    /// The blade's heading at `tau` seconds into the swing (radians, y down, 0 forward): from `-arc` (raised behind)
    /// through straight up to forward, at one speed.
    fn heading(&self, tau: f64) -> f64 {
        -self.arc * (1.0 - clamp(tau / SWING, 0.0, 1.0))
    }
}

impl EffectState for State {
    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        // the swing is read from the sprite the character struck with
        if self.read {
            return;
        }
        let Some(a) = actor else {
            return;
        };
        let area = Area {
            x: a.x(),
            y: a.y(),
            w: a.w(),
            h: a.h(),
        };
        let Some((cx, cy)) = Silhouette::of(&a, area).centroid() else {
            return;
        };
        self.pivot = (cx, cy - f64::from(shoulder_above_middle()));
        self.read = true;
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (x, y) = self.pivot;
        let pose = |tau: f64| Pose {
            x,
            y,
            angle: self.heading(tau),
            age: t - tau,
        };
        // newest first: the blade where it is now, then where it was through the swing
        let head = js_min(t, SWING);
        let mut poses = vec![pose(head)];
        for i in (0..SAMPLES).rev() {
            let tau = SWING * f64::from(i) / f64::from(SAMPLES);
            if tau < head {
                poses.push(pose(tau));
            }
        }
        let look = Look {
            secs: self.secs,
            // head to end, the whole ramp
            fade: self.ramp.len() as f64,
            taper: 1.0,
            wave: 0.0,
            phase: 0.0,
            // the older half dithers away, crawling a pixel a frame
            breakup: 0.5,
            shift: libm::floor(t * f64::from(META.fps) + 1e-6),
            light: 0.5,
            skip: 0.0,
            soft: 1.0,
        };
        sweep(p, &poses, &self.brush, &self.ramp, &look);
    }
}
