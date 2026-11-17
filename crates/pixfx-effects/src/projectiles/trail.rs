//! Trail: the streak a moving thing leaves, drawn along the way it came (`pixfx_core::Trace`, handed over by
//! `pixfx_scene::Scene::trail`): straight behind a mover that flies straight, curving where it curved, as long as
//! it went in `secs` (a fast mover leaves a long trail, a slow one a short one), shrinking away once it stops. A
//! ribbon `width` pixels wide at its head thinning to one, waving `wave` pixels across, coloured along the
//! palette from its head to its end, the edges a step darker, breaking up by dither toward its end, lit along its
//! middle. `skip` leaves its first pixels to the mover (a sword's blade). `turn` 1 turns the ribbon with the mover's
//! own angle instead of its way, so a mover spinning in place sweeps a disc. It is a column brush swept along the
//! trace (`pixfx_draw::sweep`), for a thin mover (a sword's point, a drop, a bolt). Played on a character (it wraps
//! the sprite it plays on) it smears that body instead: the sprite's back edge, away from the way it goes, as tall as
//! it is and in its own colours (`Brush::edge`, one in from its outline), standing upright as the body does,
//! dissolving behind it, narrowing to `taper` of its height and turning translucent from `soft` along it (a pointed,
//! soft tail: `taper` 0, `soft` 0.3); `width`, `wave`, `turn` and the palette shape only the ribbon. A broad
//! effect of its own (a mirror) smears its own edge in its own draw (`arcane/bagua-mirror-flying`). Lay it under any
//! projectile
//! (`pixfx_scene::Scene::trail`), or play it on its own and move it with a character: it draws the way it was
//! moved. It draws on the back layer, behind a character.
//!
//! With no trace handed over (its gif, sheet or page) it draws the trail of a mover flying straight along +x at
//! 200 pixels a second. Made here; its golden hashes pin it.

use std::sync::Arc;

use core::f64::consts::TAU;

use pixfx_core::{Actor, Effect, EffectState, Layer, Meta, NumOpt, Options, Trace, TracePoint};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::sweep::{Brush, Look, Turn, sweep};

pub struct TrailFx;

pub static TRAIL: TrailFx = TrailFx;

const SECS: NumOpt = NumOpt::new("secs", 0.12).within(0.02, 1.0);
const WIDTH: NumOpt = NumOpt::new("width", 3.0).within(1.0, 7.0);
const WAVE: NumOpt = NumOpt::new("wave", 1.2).within(0.0, 4.0);
const SKIP: NumOpt = NumOpt::new("skip", 0.0).within(0.0, 40.0);
const TURN: NumOpt = NumOpt::new("turn", 0.0).within(0.0, 1.0);
/// On a body: how wide its smear is at the end, a share of the body's height (1 as tall as it, 0 a point); from
/// where along it the smear turns translucent, clear at its end (1 never: the dither alone eats it); and from where
/// the dither starts eating it.
const TAPER: NumOpt = NumOpt::new("taper", 1.0).within(0.0, 1.0);
const SOFT: NumOpt = NumOpt::new("soft", 1.0).within(0.0, 1.0);
const BREAKUP: NumOpt = NumOpt::new("breakup", 0.2).within(0.0, 1.0);

const META: Meta = Meta {
    id: "projectiles/trail",
    name: "Trail",
    desc: "The streak a moving thing leaves along the way it came: straight or curving with it, longer the faster it goes, waving softly and breaking up toward its end. Lay it under any projectile.",
    tags: &["projectiles", "trail", "streak", "loop"],
    size: (160, 160),
    origin: (80, 80),
    duration: LOOP,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[
        SECS.spec(),
        WIDTH.spec(),
        WAVE.spec(),
        SKIP.spec(),
        TURN.spec(),
        TAPER.spec(),
        SOFT.spec(),
        BREAKUP.spec(),
    ],
};

/// Eight frames a wave, so the dither (four wide) closes too.
const LOOP: f64 = 8.0 / 30.0;

/// White at the head to a deep blue at the end.
const PALETTE: [Rgba; 5] = [
    Rgba::hex24(0xf2fdfd),
    Rgba::hex24(0xa4f7fd),
    Rgba::hex24(0x63eefd),
    Rgba::hex24(0x56b6e2),
    Rgba::hex24(0x3d85b0),
];

impl Effect for TrailFx {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            ramp: opts.palette_or(&PALETTE),
            secs: opts.num(&SECS),
            width: opts.num(&WIDTH),
            wave: opts.num(&WAVE),
            skip: opts.num(&SKIP),
            turn: opts.num(&TURN),
            taper: opts.num(&TAPER),
            soft: opts.num(&SOFT),
            breakup: opts.num(&BREAKUP),
            trace: None,
            body: None,
        })
    }
    fn wraps_actor(&self) -> bool {
        true
    }
    fn traces(&self, opts: &Options) -> Option<f64> {
        Some(opts.num(&SECS))
    }
}

struct State {
    ramp: Vec<Rgba>,
    secs: f64,
    width: f64,
    wave: f64,
    skip: f64,
    turn: f64,
    taper: f64,
    soft: f64,
    breakup: f64,
    trace: Option<Arc<Trace>>,
    /// The sprite it plays on, as its opaque pixels in its own space, when it wraps one.
    body: Option<Vec<(f64, f64, Rgba)>>,
}

/// Seconds of the way a body went that say which way it is going (its back edge is the side away from it).
const BODY_WINDOW: f64 = 0.05;

/// Pixels a second a mover flies in a preview (no trace handed over: its gif, sheet or page).
const PREVIEW_SPEED: f64 = 200.0;

/// The trace of a mover that flew straight along +x at `PREVIEW_SPEED`, now at the origin.
fn straight(secs: f64) -> Trace {
    let back = secs * 1.5;
    Trace {
        points: vec![
            TracePoint {
                x: 0.0,
                y: 0.0,
                angle: 0.0,
                age: 0.0,
            },
            TracePoint {
                x: -PREVIEW_SPEED * back,
                y: 0.0,
                angle: 0.0,
                age: back,
            },
        ],
    }
}

impl EffectState for State {
    fn set_trace(&mut self, trace: Option<Arc<Trace>>) {
        self.trace = trace;
    }

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.body = actor.map(|a| {
            a.opaque()
                .map(|(x, y, c)| (f64::from(x), f64::from(y), c))
                .collect()
        });
    }

    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        // behind: under the mover, and under a character it rides on
        if layer != Layer::Back {
            return;
        }
        let preview;
        let trace = match &self.trace {
            Some(t) => t.as_ref(),
            None => {
                preview = straight(self.secs);
                &preview
            }
        };
        let frame = libm::floor(t * 30.0 + 1e-6);
        // on a body: its own back edge, as tall as it is, in its own colours, dissolving behind it
        if let Some(body) = &self.body {
            let Some(way) = trace.heading(0.0, BODY_WINDOW) else {
                return;
            };
            // the back is the side away from the way it goes; the body stays upright as it goes either way, so the
            // brush stands upright too (a brush turned with the way would swing round the body when it turns back)
            let (c, s) = (libm::cos(way), libm::sin(way));
            let mut brush = Brush::edge(
                body.iter()
                    .map(|&(x, y, color)| (x * c + y * s, y, color, 0.0)),
                // one in from its outline: the body's own colour
                1,
            );
            brush.turn = Turn::Angle(0.0);
            let look = Look {
                secs: self.secs.max(1e-6),
                fade: 0.0,
                taper: self.taper,
                wave: 0.0,
                phase: 0.0,
                breakup: self.breakup,
                shift: frame,
                light: 0.0,
                skip: 0.0,
                soft: self.soft,
            };
            let mut poses = trace.poses(look.secs);
            for q in &mut poses {
                q.angle = 0.0;
            }
            sweep(p, &poses, &brush, &self.ramp, &look);
            return;
        }
        let mut brush = Brush::column(&self.ramp, self.width);
        if self.turn >= 0.5 {
            brush.turn = Turn::Angle(0.0);
        }
        let look = Look {
            secs: self.secs.max(1e-6),
            // head to end, the whole ramp
            fade: self.ramp.len() as f64,
            // thinning to one pixel
            taper: 0.0,
            wave: self.wave,
            phase: frame / (LOOP * 30.0) * TAU,
            breakup: 0.55,
            shift: frame,
            light: 0.35,
            skip: self.skip,
            soft: 1.0,
        };
        sweep(p, &trace.poses(look.secs), &brush, &self.ramp, &look);
    }
}
