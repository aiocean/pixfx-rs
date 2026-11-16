//! What a stage shows and where, decided in plain Rust for every shell: which effects play, where, how much room
//! round them, at what scale, in which order their layers are drawn, when finished ones go, which frames are drawn
//! and shared, and the game's answer to the cues the effects fire (a [`Feel`]: shake, hit-stop, flash, ripples).
//! A shell (Canvas 2D, Phaser, Pixi, gpui) only makes, refills, shows and frees surfaces as [`Shell`] says.
#![forbid(unsafe_code)]

use std::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;

mod frames;

pub use frames::{ActorFrame, EffectKey, Frames, InlinePainter, Job, OwnAt, Painter, Target};

use core::f64::consts::{PI, TAU};
use frames::frame_bytes;
use pixfx_core::cue::Cue;

use pixfx_core::volley::{Launch, Order};
use pixfx_core::{
    Actor, Clip, Clock, Feel, FrameId, Layer, Placement, TimedCue, Trace, TracePoint, valid_dt,
};
use pixfx_draw::math::js_round;
use pixfx_draw::math::{clamp, smooth};
use pixfx_draw::{Frame, composite_px};

/// A change to where and how an effect draws ([`Scene::set`] after [`PlacementChange::over`]): each field left out
/// keeps its value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(deny_unknown_fields)
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub struct PlacementChange {
    /// The effect's origin, in game pixels (rounded to a whole pixel).
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "ts", tsify(optional))]
    pub x: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "ts", tsify(optional))]
    pub y: Option<f64>,
    /// Mirror left-right.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "ts", tsify(optional))]
    pub flip: Option<bool>,
    /// Rotation, radians.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "ts", tsify(optional))]
    pub angle: Option<f64>,
    /// Glow strength.
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "ts", tsify(optional))]
    pub light: Option<f64>,
}

impl PlacementChange {
    /// `base` with this change put over it.
    pub fn over(self, base: Placement) -> Placement {
        Placement {
            x: self.x.unwrap_or(base.x),
            y: self.y.unwrap_or(base.y),
            flip: self.flip.unwrap_or(base.flip),
            angle: self.angle.unwrap_or(base.angle),
            light: self.light.unwrap_or(base.light),
        }
    }
}

/// A target rectangle in canvas pixels (game pixels times the scale).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: u64,
    pub h: u64,
}

/// What the host can see, in game pixels (world units for a host with a camera). A scene with a view draws only the
/// effects whose frame, canvas plus glow margin, overlaps it; the rest keep playing and firing cues unseen.
///
/// `scroll_x`/`scroll_y` say how far the camera has scrolled (0 for a host without one). An effect with a scroll
/// factor `f` ([`Scene::set_scroll_factor`]) moves `f` times as far as the camera does, so the camera sees the view
/// moved back by `(1 - f) * scroll` of it: an effect fixed to the camera (`f = 0`) sees the camera's own rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub scroll_x: f64,
    pub scroll_y: f64,
}

impl View {
    /// Whether `rect` (canvas pixels, `scale` per game pixel) of an effect with scroll factor `f` overlaps this
    /// view; rectangles that only touch do not.
    fn overlaps(self, rect: Rect, f: f64, scale: NonZeroU32) -> bool {
        let s = f64::from(scale.get());
        let back = 1.0 - f;
        let (x0, y0) = (
            (self.x - back * self.scroll_x) * s,
            (self.y - back * self.scroll_y) * s,
        );
        let (x1, y1) = (x0 + self.w * s, y0 + self.h * s);
        // canvas pixels past 2^53 lose precision as f64; no canvas reaches them
        let (rx, ry) = (rect.x as f64, rect.y as f64);
        rx < x1 && rx + rect.w as f64 > x0 && ry < y1 && ry + rect.h as f64 > y0
    }
}

/// One blit: `layer` of the instance `handle`, drawn into `rect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pass {
    pub handle: u32,
    pub layer: Layer,
    pub rect: Rect,
}

/// Something that happened on the stage, queued in order until the host drains it ([`Scene::drain_events`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "type", rename_all = "lowercase")
)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum StageEvent {
    /// The instance is gone: it finished, faded out after a stop, or was removed.
    End { handle: u32 },
    /// The instance fired a cue: `t` seconds into its playback, in game pixels on the stage (a ripple's or a
    /// sound's `x`/`y`), `pan` -1 to 1 across the stage once its width is known. The stage's [`Feel`] has already
    /// taken it; a sound is the host's to play.
    Cue {
        handle: u32,
        t: f64,
        #[cfg_attr(feature = "ts", tsify(type = "number | null"))]
        pan: Option<f64>,
        cue: Cue,
    },
    /// Piece `index` of the volley instance `handle` was ordered to launch ([`Scene::order`]) struck at game
    /// pixel (`x`, `y`): the body `body` it met ([`Scene::set_body`]), or none (the point it lands on, or a
    /// [`Scene::burst`]). What the strike does to the game (damage, a reel) is the game's to decide.
    Strike {
        handle: u32,
        index: u32,
        x: f64,
        y: f64,
        #[cfg_attr(feature = "ts", tsify(type = "number | null"))]
        body: Option<u32>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageError {
    /// A position was NaN or infinite.
    NotFinite,
    /// Every `u32` handle has been given out once.
    OutOfHandles,
    /// A time step was NaN, infinite or negative.
    BadStep,
    /// A frame painted elsewhere came back at another size than the one asked for.
    BadFrame,
    /// An actor's sprite sits too far from its effect to place in the effect's space.
    ActorTooFar,
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotFinite => "x and y must be finite numbers",
            Self::OutOfHandles => "every handle has been used",
            Self::BadStep => "dt must be a finite number of seconds, 0 or more",
            Self::BadFrame => "a painted frame came back at another size than the one asked for",
            Self::ActorTooFar => "the actor's sprite is too far from its effect",
        })
    }
}

impl std::error::Error for StageError {}

/// What a host does with drawn frames: keep one drawn layer of one effect in a surface (a scratch canvas, a
/// texture, an image), draw a surface onto its screen, and free a surface the scene no longer needs. The [`Scene`] decides what to draw and when: an effect is drawn only when what
/// it shows changes, and a frame other effects already show is drawn once and shared.
pub trait Shell {
    type Surface;
    type Error;
    /// A new surface holding `frame`, one layer of one effect.
    fn make(&mut self, frame: &Frame) -> Result<Self::Surface, Self::Error>;
    /// Put `frame` (the size `surface` was made for) in `surface`, replacing what it held.
    fn refill(&mut self, surface: &mut Self::Surface, frame: &Frame) -> Result<(), Self::Error>;
    /// Draw `surface` into `pass.rect`: colour normally, then light additively.
    fn show(&mut self, pass: Pass, surface: &Self::Surface) -> Result<(), Self::Error>;
    /// Let go of `surface`: the scene dropped it (past the frame budget, or its effect ended). A host whose
    /// surfaces hold host objects releases them here, or queues them to release once the frame is painted.
    fn free(&mut self, surface: Self::Surface) -> Result<(), Self::Error>;
}

/// How a shell takes a frame painted elsewhere ([`Frames::deliver`], [`Scene::deliver_own`]): from `D`, what the
/// painter sent back. Every shell takes a [`Frame`]; a shell whose painter sends pixels straight to it (a worker
/// posting RGBA to a JS shell) takes their shape instead, so the pixels never pass through the scene.
pub trait Receive<D>: Shell {
    /// A new surface holding `painted`.
    fn make_from(&mut self, painted: &D) -> Result<Self::Surface, Self::Error>;
    /// Put `painted` in `surface`, replacing what it held.
    fn refill_from(&mut self, surface: &mut Self::Surface, painted: &D) -> Result<(), Self::Error>;
    /// Bytes a surface holding `painted` takes, counted against the frame budget.
    fn bytes_of(&self, painted: &D) -> u64;
}

impl<H: Shell> Receive<Frame> for H {
    fn make_from(&mut self, painted: &Frame) -> Result<H::Surface, H::Error> {
        self.make(painted)
    }

    fn refill_from(&mut self, surface: &mut H::Surface, painted: &Frame) -> Result<(), H::Error> {
        self.refill(surface, painted)
    }

    fn bytes_of(&self, painted: &Frame) -> u64 {
        frame_bytes(painted)
    }
}

/// A layer's own frame: the surface an effect fading after `stop()`, or wrapping its actor, draws into, and what
/// it last drew there.
struct Own<S> {
    surface: S,
    /// What it was drawn at: it is drawn again only when that changes.
    drawn: OwnAt,
}

struct Entry<S> {
    handle: u32,
    /// Where the effect's origin sits, in whole game pixels.
    at: (i64, i64),
    /// How the player draws the effect on its own frame: `x`/`y` its origin there, the rest as the host sets.
    place: Placement,
    /// What it plays, and its time.
    clip: Clip,
    clock: Clock,
    /// Its own frames while it fades out or wraps its actor (back, front, both); shared frames live in [`Frames`].
    own: [Option<Own<S>>; 3],
    /// The own frame each layer asked a painter for and has not been handed yet ([`Scene::deliver_own`]).
    asked: [Option<OwnAt>; 3],
    /// The character it plays on, in its own space, as the host last set it ([`Scene::set_actor`]).
    actor: Option<ActorFrame>,
    /// The sprite as the host gave it, in game pixels relative to the effect's origin, unmirrored: mirrored again
    /// into `actor` when the effect's flip changes.
    actor_given: Option<Arc<Actor>>,
    /// Versions given out to this entry's actors; the next set takes the next one.
    actor_sets: u64,
    /// The shared frame each layer showed last: what it shows again while it waits for a draw.
    last: [Option<FrameId>; 3],
    /// The shared frame each layer waits on from a painter elsewhere. One at a time: until it comes back the layer
    /// asks for no newer frame, so a painter that falls behind gets no backlog, and when it comes back it shows
    /// even if the clock has moved past it.
    awaiting: [Option<FrameId>; 3],
    /// The host frame each layer first waited for a draw in, past the draw budget; `None` while it is up to date.
    waiting: [Option<u64>; 3],
    /// Plays real time through a hit-stop instead of freezing with the rest ([`Scene::set_plays_through_hitstop`]).
    through_hitstop: bool,
    /// It answered an order ([`Scene::order`]): its volley flies in its place, so it is no longer drawn, and it
    /// ends when the last piece is gone.
    ordered: bool,
    /// The instance whose ordered volley launched it (a piece in flight, or what a piece played where it struck).
    launched_by: Option<u32>,
    /// The mover it draws the way of, for a trail ([`Scene::trail`]).
    trail: Option<Trail>,
    /// How far it moves when the camera scrolls, as a share of the scroll (1: with the world, 0: fixed to the
    /// camera); the view culls it where the camera shows it ([`View`]).
    scroll_factor: f64,
}

impl<S> Entry<S> {
    /// Put the sprite the host gave in the effect's own space, mirrored when the effect draws flipped, under a new
    /// version.
    fn place_actor(&mut self) {
        self.actor_sets = self.actor_sets.wrapping_add(1);
        let version = self.actor_sets;
        let flip = self.place.flip;
        self.actor = self.actor_given.as_ref().map(|a| ActorFrame {
            version,
            actor: if flip {
                Arc::new(a.mirrored())
            } else {
                Arc::clone(a)
            },
        });
    }

    /// The actor its effect wraps, if it wraps one and the host set one.
    fn wrapped(&self) -> Option<&ActorFrame> {
        self.actor.as_ref().filter(|_| self.clip.wraps_actor())
    }

    /// What its own frame would be drawn at now.
    fn own_at(&self) -> OwnAt {
        OwnAt {
            clock: self.clock.clock_frame(),
            place: self.place,
            actor: self.wrapped().map_or(0, |a| a.version),
            trace: self.trail.as_ref().map_or(0, |t| t.version),
        }
    }

    /// The trace it draws, for a trail.
    fn traced(&self) -> Option<&Arc<Trace>> {
        self.trail.as_ref().map(|t| &t.trace)
    }
}

/// Why a frame was not put in a surface: the shell failed, the draw budget leaves it for a later host frame, or it
/// is being painted elsewhere.
enum Fill<E> {
    Shell(E),
    Wait,
}

/// The playing instances, in the order they were played, the frames they share and the game-to-canvas scale.
pub struct Scene<S> {
    /// Which of the scenes sharing a [`Frames`] this is.
    id: u32,
    scale: NonZeroU32,
    /// The next handle to give out; handles start at 1 and are never reused.
    next: Option<u32>,
    entries: Vec<Entry<S>>,
    /// Counts this scene's layer draws, to tell how long an effect has waited for one.
    draws: u64,
    /// Events not yet drained, oldest first.
    events: Vec<StageEvent>,
    /// The answer to the cues the effects fire; `None` when the host answers them with its own.
    feel: Option<Feel>,
    /// The stage's width in game pixels, for a cue's pan; 0 leaves cues without pan.
    screen_w: f64,
    /// Surfaces dropped since the last draw (past the frame budget, or their effect ended), handed to the shell's
    /// [`Shell::free`] at the start of the next draw.
    released: Vec<S>,
    /// Effects gone since the last draw, whose state the painter lets go of then.
    ended: Vec<u32>,
    /// Layers drawn at most per host frame ([`Scene::set_draw_budget`]); `None` draws every one that changed.
    draw_budget: Option<NonZeroU32>,
    /// Layers drawn in the current host frame.
    drawn: u32,
    /// What the host can see; `None` draws every effect.
    view: Option<View>,
    /// Seconds the scene has played (hit-stops add nothing): the clock volleys launch and fly by.
    time: f64,
    /// The pieces of ordered volleys, waiting to launch or in flight.
    flights: Vec<Flight>,
    /// The bodies volleys strike, by id.
    bodies: Vec<(u32, Body)>,
}

/// What a trail follows: the mover's game pixel at each scene time it was seen, oldest first, kept for `secs`;
/// the trace made of them in the trail's own space, under a version that changes with it.
struct Trail {
    /// The instance it follows, or none for a trail played on its own, which follows where the game moves it.
    mover: Option<u32>,
    secs: f64,
    /// When, where and turned how far.
    seen: Vec<(f64, (i64, i64), f64)>,
    trace: Arc<Trace>,
    version: u64,
}

/// One piece of a volley: held as instance `held` from `ordered` until `due`, then flying as instance `flying`
/// since `started`, mirrored as the instance that launched it was (`flip`).
struct Flight {
    /// The instance that was ordered, and which piece of its volley this is.
    by: u32,
    index: u32,
    launch: Launch,
    flip: bool,
    ordered: f64,
    due: f64,
    held: Option<u32>,
    flying: Option<u32>,
    started: f64,
    /// The body that ordered it, never struck; the bodies it struck; whether it struck its mark.
    caster: Option<u32>,
    hits: Vec<u32>,
    marked: bool,
    /// How far along its way it was, its tip and heading there, at the last step.
    dist: f64,
    tip: (f64, f64),
    heading: f64,
}

/// A body a volley's pieces strike when they meet it ([`Scene::set_body`]): a box in game pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Body {
    /// How far along the segment from `a` to `b` (0 to 1) it first enters the box, if it does (0 when `a` is
    /// inside).
    fn entered(&self, a: (f64, f64), b: (f64, f64)) -> Option<f64> {
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        for (p, d, min, max) in [
            (a.0, b.0 - a.0, self.x, self.x + self.w),
            (a.1, b.1 - a.1, self.y, self.y + self.h),
        ] {
            if d.abs() < 1e-12 {
                if p < min || p > max {
                    return None;
                }
                continue;
            }
            let (t0, t1) = ((min - p) / d, (max - p) / d);
            let (t0, t1) = if t0 < t1 { (t0, t1) } else { (t1, t0) };
            lo = lo.max(t0);
            hi = hi.min(t1);
            if lo > hi {
                return None;
            }
        }
        Some(lo)
    }
}

/// Headings a flying piece is drawn at: a full turn in this many steps, so pieces heading the same way share frames
/// and turn the way pixel art turns, a step at a time.
const HEADINGS: f64 = 32.0;

/// `a` snapped to the nearest of [`HEADINGS`].
fn snap_heading(a: f64) -> f64 {
    let step = TAU / HEADINGS;
    js_round(a / step) * step
}

/// How a piece heading `a` is drawn: its flip and angle. A piece keeps the mirroring of the instance that launched
/// it, so it looks as it did there (its lit side, its guard) whichever way it flies: mirrored, a piece drawn
/// pointing along +x points along `a` turned half a turn less.
fn orient(a: f64, flip: bool) -> (bool, f64) {
    if flip {
        (true, snap_heading(a - PI))
    } else {
        (false, snap_heading(a))
    }
}

/// The shortest turn from angle `a` to angle `b`, radians.
fn turn_to(a: f64, b: f64) -> f64 {
    (b - a + PI).rem_euclid(TAU) - PI
}

impl<S> Scene<S> {
    /// A scene of a [`Frames`] ([`Frames::scene`]), `id` telling it from the others.
    pub(crate) fn new(id: u32, scale: NonZeroU32) -> Self {
        Self {
            id,
            scale,
            next: Some(1),
            entries: Vec::new(),
            draws: 0,
            events: Vec::new(),
            feel: Some(Feel::new()),
            screen_w: 0.0,
            released: Vec::new(),
            ended: Vec::new(),
            draw_budget: None,
            drawn: 0,
            view: None,
            time: 0.0,
            flights: Vec::new(),
            bodies: Vec::new(),
        }
    }

    /// The answer to the cues the effects fire: where the screen sits ([`Feel::shake`]), its flash and ripples.
    /// `None` when the host answers them itself ([`Scene::set_feel`]).
    pub fn feel(&self) -> Option<&Feel> {
        self.feel.as_ref()
    }

    /// Change the answer: the game's own impacts ([`Feel::add_trauma`], [`Feel::freeze`], [`Feel::slow`]).
    pub fn feel_mut(&mut self) -> Option<&mut Feel> {
        self.feel.as_mut()
    }

    /// Answer cues with `feel` (the stage's own: freezes its effects, shakes, flashes), or with nothing: the host
    /// has its own answer, fed from the `cue` events, and the stage plays every second `update` gives it.
    pub fn set_feel(&mut self, feel: Option<Feel>) {
        self.feel = feel;
    }

    /// The stage's width in game pixels, so a cue's pan follows where it fires; 0 leaves cues without pan.
    pub fn set_screen_w(&mut self, w: f64) {
        self.screen_w = if w.is_finite() && w > 0.0 { w } else { 0.0 };
    }

    /// Draw only the effects whose frame overlaps `view` from the next draw on; `None` (the default) draws them all.
    /// Effects outside it still play, fire their cues and end on time. A view with a NaN or infinite field is refused
    /// and the view stays as it was.
    pub fn set_view(&mut self, view: Option<View>) -> Result<(), StageError> {
        if let Some(v) = view
            && ![v.x, v.y, v.w, v.h, v.scroll_x, v.scroll_y]
                .iter()
                .all(|n| n.is_finite())
        {
            return Err(StageError::NotFinite);
        }
        self.view = view;
        Ok(())
    }

    /// Which of the scenes sharing a [`Frames`] this is.
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn scale(&self) -> NonZeroU32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: NonZeroU32) {
        self.scale = scale;
    }

    /// Play `clip` from its start with its origin at game pixel (`x`, `y`), rounded as JS `Math.round` does;
    /// returns its handle. An effect that draws the way a mover came ([`pixfx_core::Effect::traces`]) played on its
    /// own trails itself: moved with [`Scene::move_to`] (with a fighter, a cursor), it draws the way it went, and
    /// once stopped it shrinks to where it was last and ends.
    pub fn play(&mut self, clip: Clip, x: f64, y: f64) -> Result<u32, StageError> {
        let at = game_pixel(x, y)?;
        let handle = self.next.ok_or(StageError::OutOfHandles)?;
        self.next = handle.checked_add(1);
        let angle = clip.placement().angle;
        let trail = clip.effect().traces(clip.opts()).map(|secs| Trail {
            mover: None,
            secs: secs.max(0.0),
            seen: vec![(self.time, at, angle)],
            trace: Arc::new(Trace::default()),
            version: 1,
        });
        self.entries.push(Entry {
            handle,
            at,
            place: clip.placement(),
            clock: clip.clock(),
            clip,
            own: [None, None, None],
            asked: [None, None, None],
            last: [None, None, None],
            awaiting: [None, None, None],
            waiting: [None, None, None],
            actor: None,
            actor_given: None,
            actor_sets: 0,
            through_hitstop: false,
            scroll_factor: 1.0,
            ordered: false,
            launched_by: None,
            trail,
        });
        Ok(handle)
    }

    /// Play `clip` as the trail of instance `mover`: an effect that draws the way a mover came
    /// ([`pixfx_core::Effect::traces`]), handed the mover's last positions (as many seconds of them as it asks
    /// for) in its own space after every update, so it bends where the mover turned and is as long as it was fast.
    /// It draws just under the mover, plays where the mover is, and once the mover is gone it shrinks to where it
    /// was last and ends. Its frames are its own. `Ok(None)` if no instance has this handle or the effect draws
    /// no trace.
    pub fn trail(&mut self, mover: u32, clip: Clip) -> Result<Option<u32>, StageError> {
        let Some(secs) = clip.effect().traces(clip.opts()) else {
            return Ok(None);
        };
        let Some(m) = self.entries.iter().position(|e| e.handle == mover) else {
            return Ok(None);
        };
        let (at, by, angle) = (
            self.entries[m].at,
            self.entries[m].launched_by,
            self.entries[m].place.angle,
        );
        let h = self.play(clip, at.0 as f64, at.1 as f64)?;
        if let Some(mut e) = self.entries.pop() {
            e.launched_by = by;
            e.trail = Some(Trail {
                mover: Some(mover),
                secs: secs.max(0.0),
                seen: vec![(self.time, at, angle)],
                trace: Arc::new(Trace::default()),
                version: 1,
            });
            // drawn under its mover
            self.entries.insert(m, e);
        }
        Ok(Some(h))
    }

    /// The mover instance `handle` trails, if it is a trail ([`Scene::trail`]).
    pub fn follows(&self, handle: u32) -> Option<u32> {
        self.entries
            .iter()
            .find(|e| e.handle == handle)
            .and_then(|e| e.trail.as_ref().and_then(|t| t.mover))
    }

    /// Bring every trail to its mover: where the mover is now joins what it has seen, what is older than the
    /// trail asks for goes (but the last point before that, so the trace reaches its full age), and a trail whose
    /// mover is gone shrinks until nothing of its trace is left, then ends.
    fn follow(&mut self) {
        let time = self.time;
        let movers: Vec<(u32, (i64, i64), f64)> = self
            .entries
            .iter()
            .map(|e| (e.handle, e.at, e.place.angle))
            .collect();
        let mut spent = Vec::new();
        for e in &mut self.entries {
            let stopped = e.clock.stopped();
            // the trace in the trail's own space: the painter mirrors x, then turns by the trail's angle, so the
            // trace is turned back, then mirrored back
            let sx = if e.place.flip { -1.0 } else { 1.0 };
            let own = e.place.angle;
            let (cs, sn) = (libm::cos(own), libm::sin(own));
            let Some(t) = &mut e.trail else { continue };
            let now = match t.mover {
                Some(mover) => movers.iter().find(|m| m.0 == mover).map(|m| (m.1, m.2)),
                None => (!stopped).then_some((e.at, e.place.angle)),
            };
            if let Some((at, angle)) = now {
                t.seen.push((time, at, angle));
                e.at = at;
            }
            let keep = t
                .seen
                .iter()
                .rposition(|s| time - s.0 > t.secs)
                .unwrap_or(0);
            t.seen.drain(..keep);
            let alive = t.seen.iter().any(|s| time - s.0 <= t.secs);
            if now.is_none() && !alive {
                spent.push(e.handle);
                continue;
            }
            let trace = Trace {
                points: t
                    .seen
                    .iter()
                    .rev()
                    .map(|&(when, (x, y), angle)| {
                        let (dx, dy) = ((x - e.at.0) as f64, (y - e.at.1) as f64);
                        let a = angle - own;
                        TracePoint {
                            x: sx * (dx * cs + dy * sn),
                            y: -dx * sn + dy * cs,
                            // mirrored like x: a heading a turns to PI - a
                            angle: if sx < 0.0 { PI - a } else { a },
                            age: time - when,
                        }
                    })
                    .collect(),
            };
            if *t.trace != trace {
                t.trace = Arc::new(trace);
                t.version = t.version.wrapping_add(1);
            }
        }
        for h in spent {
            self.remove(h);
        }
    }

    /// Put the instance's origin at game pixel (`x`, `y`), rounded as [`Scene::play`] does, keeping its time:
    /// an effect that follows something moves without restarting. `Ok(false)` if no instance has this handle.
    pub fn move_to(&mut self, handle: u32, x: f64, y: f64) -> Result<bool, StageError> {
        let at = game_pixel(x, y)?;
        Ok(self
            .entries
            .iter_mut()
            .find(|e| e.handle == handle)
            .map(|e| e.at = at)
            .is_some())
    }

    /// Where and how the instance is drawn: `x`/`y` its origin in whole game pixels, the rest as its effect
    /// reads them.
    pub fn placement(&self, handle: u32) -> Option<Placement> {
        self.entries.iter().find(|e| e.handle == handle).map(|e| {
            let p = e.place;
            // a game pixel past 2^53 loses precision as f64; no canvas reaches it
            Placement {
                x: e.at.0 as f64,
                y: e.at.1 as f64,
                ..p
            }
        })
    }

    /// Draw the instance as `p` says from now on, keeping its time: `x`/`y` move its origin as
    /// [`Scene::move_to`] does, `flip`/`angle`/`light` change how it draws. `Ok(false)` if no instance has this
    /// handle.
    pub fn set(&mut self, handle: u32, p: Placement) -> Result<bool, StageError> {
        let at = game_pixel(p.x, p.y)?;
        let Some(e) = self.entries.iter_mut().find(|e| e.handle == handle) else {
            return Ok(false);
        };
        e.at = at;
        let flipped = e.place.flip != p.flip;
        // the player's own x/y keep the origin on its frame; the stage places the frame
        e.place = Placement {
            x: e.place.x,
            y: e.place.y,
            ..p
        };
        if flipped {
            e.place_actor();
        }
        Ok(true)
    }

    /// The character the instance plays on, as its sprite shows now: `actor` with its top-left at game pixel
    /// (`actor.x()`, `actor.y()`), or `None` to play on none. An effect that wraps its actor
    /// ([`pixfx_core::Effect::wraps_actor`]) grips this sprite from its next step, and its frames become its own;
    /// any other effect ignores it. Set it again each time the sprite's frame changes (or it turns); the same sprite
    /// at the same place relative to the effect's origin changes nothing, so a host may set it every frame. The
    /// sprite keeps its place relative to the effect's origin when the effect moves, as a sprite the effect follows
    /// does.
    /// `Ok(false)` if no instance has this handle.
    pub fn set_actor(&mut self, handle: u32, actor: Option<Actor>) -> Result<bool, StageError> {
        let Some(e) = self.entries.iter_mut().find(|e| e.handle == handle) else {
            return Ok(false);
        };
        let given = match actor {
            Some(a) => {
                let (dx, dy) = (i64::from(a.x()) - e.at.0, i64::from(a.y()) - e.at.1);
                let x = i32::try_from(dx).map_err(|_| StageError::ActorTooFar)?;
                let y = i32::try_from(dy).map_err(|_| StageError::ActorTooFar)?;
                Some(a.moved_to(x, y))
            }
            None => None,
        };
        if given.as_ref() == e.actor_given.as_deref() {
            return Ok(true);
        }
        e.actor_given = given.map(Arc::new);
        e.place_actor();
        Ok(true)
    }

    /// How far the instance moves when the camera scrolls, as a share of the scroll: 1 (the default) with the
    /// world, 0 fixed to the camera, between for parallax. The host places it so; the view culls it where the camera
    /// shows it ([`View`]). A factor that is not finite is refused. `Ok(false)` if no instance has this handle.
    pub fn set_scroll_factor(&mut self, handle: u32, f: f64) -> Result<bool, StageError> {
        if !f.is_finite() {
            return Err(StageError::NotFinite);
        }
        Ok(self
            .entries
            .iter_mut()
            .find(|e| e.handle == handle)
            .map(|e| e.scroll_factor = f)
            .is_some())
    }

    /// Let the instance keep playing real time through a hit-stop while the rest of the scene freezes (`on`), as a
    /// hit spark does, or freeze with the rest again (`!on`, the default). Slow motion, which starts after
    /// the stop, slows it with the rest. Its own hit-stops do not renew the stop it plays through. False if no
    /// instance has this handle.
    pub fn set_plays_through_hitstop(&mut self, handle: u32, on: bool) -> bool {
        self.entries
            .iter_mut()
            .find(|e| e.handle == handle)
            .map(|e| e.through_hitstop = on)
            .is_some()
    }

    /// Let the instance end (loops fade out); false if no instance has this handle.
    pub fn stop(&mut self, handle: u32) -> bool {
        self.entries
            .iter_mut()
            .find(|e| e.handle == handle)
            .map(|e| e.clock.stop())
            .is_some()
    }

    /// Drop the instance now, queueing its end; false if no instance has this handle.
    pub fn remove(&mut self, handle: u32) -> bool {
        let Some(i) = self.entries.iter().position(|e| e.handle == handle) else {
            return false;
        };
        let e = self.entries.remove(i);
        self.released
            .extend(e.own.into_iter().flatten().map(|o| o.surface));
        self.ended.push(handle);
        self.events.push(StageEvent::End { handle });
        true
    }

    /// Order the instance to strike: its effect answers with a volley ([`pixfx_core::Effect::order`]) from where
    /// it stands and the frame it shows, the scene stops drawing it and launches the volley's pieces in its place,
    /// flies them, sends a [`StageEvent::Strike`] for each, and ends the instance once the last piece is gone.
    /// `Ok(false)` if no instance has this handle, it was ordered already or stopped, or its effect takes no orders.
    pub fn order(&mut self, handle: u32, order: Order) -> Result<bool, StageError> {
        if !order.x.is_finite() || !order.y.is_finite() {
            return Err(StageError::NotFinite);
        }
        let Some(e) = self
            .entries
            .iter_mut()
            .find(|e| e.handle == handle && !e.ordered && !e.clock.stopped())
        else {
            return Ok(false);
        };
        let fps = f64::from(e.clip.meta().fps);
        // the time of the frame it shows, so the volley starts where the pieces are seen
        let t = e.clock.frame() as f64 / fps;
        let at = (e.at.0 as f64, e.at.1 as f64);
        let volley = e
            .clip
            .effect()
            .order(e.clip.opts(), t, at, e.place.flip, &order);
        if volley.is_empty() {
            return Ok(false);
        }
        e.ordered = true;
        let flip = e.place.flip;
        for (i, launch) in volley.into_iter().enumerate() {
            // held where the piece was in the instance, pointing as it did, from this frame on
            let (flip_now, angle) = orient(launch.rest, flip);
            let opts = pixfx_core::Options {
                flip: flip_now,
                angle,
                ..launch.opts.clone()
            };
            let (x, y) = launch.path.start();
            let launch_heading = launch.path.heading(0.0);
            let tip = launch.tip((x, y), launch_heading);
            let held = if launch.after > 0.0 {
                let still = launch.held.unwrap_or(launch.effect);
                Some(self.play_launched(Clip::live(still, opts), x, y, handle)?)
            } else {
                None
            };
            self.flights.push(Flight {
                by: handle,
                index: u32::try_from(i).unwrap_or(u32::MAX),
                flip,
                ordered: self.time,
                due: self.time + launch.after.max(0.0),
                launch,
                held,
                flying: None,
                started: 0.0,
                caster: order.caster,
                hits: Vec::new(),
                marked: false,
                dist: 0.0,
                tip,
                heading: launch_heading,
            });
        }
        Ok(true)
    }

    /// Launch the volley pieces that are due, fly the ones in the air to the scene's time, strike, and end what
    /// is done: a piece past its path, and an ordered instance whose pieces are all gone.
    /// Play `clip` for the volley of instance `by` ([`Scene::launched`]).
    fn play_launched(&mut self, clip: Clip, x: f64, y: f64, by: u32) -> Result<u32, StageError> {
        let h = self.play(clip, x, y)?;
        if let Some(e) = self.entries.last_mut() {
            e.launched_by = Some(by);
        }
        Ok(h)
    }

    /// The instances the volley of `by` launched that still play, oldest first: its pieces in flight and what they
    /// played where they struck. A host that paints each effect on its own (a gpui view per effect) paints these
    /// where it paints `by`; they keep playing after `by` itself is gone.
    pub fn launched(&self, by: u32) -> impl Iterator<Item = u32> + '_ {
        self.entries
            .iter()
            .filter(move |e| e.launched_by == Some(by))
            .map(|e| e.handle)
    }

    /// Piece `f` strikes at game pixel `at` heading `heading`: the strike event, and its hit effect played there
    /// turned to its heading.
    fn strike(
        &mut self,
        f: &Flight,
        at: (f64, f64),
        heading: f64,
        body: Option<u32>,
    ) -> Result<(), StageError> {
        let (flip, angle) = orient(heading, f.flip);
        self.events.push(StageEvent::Strike {
            handle: f.by,
            index: f.index,
            x: at.0,
            y: at.1,
            body,
        });
        if let Some(hit) = f.launch.hit {
            let opts = pixfx_core::Options {
                flip,
                angle,
                ..f.launch.hit_opts.clone()
            };
            self.play_launched(Clip::live(hit, opts), at.0, at.1, f.by)?;
        }
        Ok(())
    }

    /// Body `id` is the box `body` in game pixels from now on (the game sets it each frame as the thing moves), or
    /// none to drop it. A volley's piece that meets it strikes it ([`StageEvent::Strike`] names it), unless the
    /// body ordered that volley ([`Order::caster`]). Throws nothing; a box that is not finite is dropped.
    pub fn set_body(&mut self, id: u32, body: Option<Body>) {
        self.bodies.retain(|(b, _)| *b != id);
        if let Some(b) = body.filter(|b| [b.x, b.y, b.w, b.h].iter().all(|v| v.is_finite())) {
            let at = self.bodies.partition_point(|(b, _)| *b < id);
            self.bodies.insert(at, (id, b));
        }
    }

    /// The game says the volley piece `piece` (one of [`Scene::launched`]) hit something: it strikes where its tip
    /// is (a strike event naming no body, its hit effect played there) and goes. For a game with collisions of its
    /// own. False if `piece` is not a piece in flight.
    pub fn burst(&mut self, piece: u32) -> Result<bool, StageError> {
        let Some(i) = self.flights.iter().position(|f| f.flying == Some(piece)) else {
            return Ok(false);
        };
        let f = self.flights.remove(i);
        self.strike(&f, f.tip, f.heading, None)?;
        self.remove(piece);
        Ok(true)
    }

    fn fly(&mut self) -> Result<(), StageError> {
        let mut flights = std::mem::take(&mut self.flights);
        let mut gone = Vec::new();
        for (i, f) in flights.iter_mut().enumerate() {
            let l = &f.launch;
            if f.flying.is_none() {
                let (x, y) = l.path.start();
                let aim = l.path.heading(0.0);
                if self.time < f.due {
                    // held, turning from where it pointed to where it will leave
                    let Some(h) = f.held else { continue };
                    let Some(place) = self.placement(h) else {
                        gone.push(i);
                        continue;
                    };
                    let k = smooth(f.ordered, f.due, self.time);
                    let (flip, angle) = orient(l.rest + turn_to(l.rest, aim) * k, f.flip);
                    self.set(
                        h,
                        Placement {
                            x,
                            y,
                            flip,
                            angle,
                            ..place
                        },
                    )?;
                    continue;
                }
                let (flip, angle) = orient(aim, f.flip);
                // what stood in its place goes; the piece starts its own clock as it leaves
                if let Some(h) = f.held.take() {
                    self.remove(h);
                }
                let opts = pixfx_core::Options {
                    flip,
                    angle,
                    ..l.opts.clone()
                };
                let h = self.play_launched(Clip::live(l.effect, opts), x, y, f.by)?;
                if let Some((tail, opts)) = &l.tail {
                    self.trail(h, Clip::live(*tail, opts.clone()))?;
                }
                f.flying = Some(h);
                f.started = f.due;
            }
            let Some(h) = f.flying else { continue };
            let s = if l.secs > 0.0 {
                clamp((self.time - f.started) / l.secs, 0.0, 1.0)
            } else {
                1.0
            };
            let dist = l.along(s);
            let ((x, y), heading) = l.point(dist);
            let (flip, angle) = orient(heading, f.flip);
            let Some(place) = self.placement(h) else {
                // the host took it away
                gone.push(i);
                continue;
            };
            self.set(
                h,
                Placement {
                    x,
                    y,
                    flip,
                    angle,
                    ..place
                },
            )?;
            // what its tip met since the last step, in the order it met them: bodies, and its mark
            let tip = l.tip((x, y), heading);
            let (through, mark) = (l.through, l.mark);
            let mut met: Vec<(f64, Option<u32>)> = self
                .bodies
                .iter()
                .filter(|(id, _)| Some(*id) != f.caster && !f.hits.contains(id))
                .filter_map(|(id, b)| b.entered(f.tip, tip).map(|t| (t, Some(*id))))
                .collect();
            if let Some(m) = mark
                && !f.marked
                && dist >= m
            {
                let t = if dist > f.dist {
                    (m - f.dist) / (dist - f.dist)
                } else {
                    1.0
                };
                met.push((clamp(t, 0.0, 1.0), None));
            }
            met.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut spent = false;
            for (t, body) in met {
                let at = (
                    f.tip.0 + (tip.0 - f.tip.0) * t,
                    f.tip.1 + (tip.1 - f.tip.1) * t,
                );
                match body {
                    Some(id) => f.hits.push(id),
                    None => f.marked = true,
                }
                self.strike(f, at, heading, body)?;
                if !through {
                    spent = true;
                    break;
                }
            }
            f.dist = dist;
            f.tip = tip;
            f.heading = heading;
            if spent {
                self.remove(h);
                gone.push(i);
            } else if s >= 1.0 {
                // struck nothing more by the end of its way: it fades out there
                self.stop(h);
                gone.push(i);
            }
        }
        for i in gone.into_iter().rev() {
            flights.remove(i);
        }
        let spent: Vec<u32> = self
            .entries
            .iter()
            .filter(|e| e.ordered && !flights.iter().any(|f| f.by == e.handle))
            .map(|e| e.handle)
            .collect();
        for h in spent {
            self.remove(h);
        }
        self.flights = flights;
        Ok(())
    }

    /// The events since the last drain, oldest first; the queue is empty afterwards.
    pub fn drain_events(&mut self) -> Vec<StageEvent> {
        std::mem::take(&mut self.events)
    }

    /// The playing handles, oldest first.
    pub fn handles(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.iter().map(|e| e.handle)
    }

    /// Where the instance's canvas lands on the target canvas.
    pub fn rect(&self, handle: u32) -> Option<Rect> {
        self.entries
            .iter()
            .find(|e| e.handle == handle)
            .map(|e| self.rect_of(e))
    }

    /// Whether the effect's frame is seen: it overlaps the view where the camera shows the effect, or there is no
    /// view.
    fn in_view(&self, e: &Entry<S>) -> bool {
        // an ordered instance is drawn no more: its volley flies in its place
        !e.ordered
            && self
                .view
                .is_none_or(|v| v.overlaps(self.rect_of(e), e.scroll_factor, self.scale))
    }

    /// The instance's frame (its canvas plus margin) placed so the effect's origin lands on `e.at`.
    fn rect_of(&self, e: &Entry<S>) -> Rect {
        let s = u64::from(self.scale.get());
        let origin = e.clip.canvas().origin;
        let pad = i64::from(e.clip.margin());
        let (w, h) = e.clip.size();
        let corner = |at: i64, origin: i32| {
            at.saturating_sub(i64::from(origin))
                .saturating_sub(pad)
                .saturating_mul(s as i64)
        };
        Rect {
            x: corner(e.at.0, origin.0),
            y: corner(e.at.1, origin.1),
            w: u64::from(w) * s,
            h: u64::from(h) * s,
        }
    }

    /// Seconds since the instance started.
    pub fn t(&self, handle: u32) -> Option<f64> {
        self.entries
            .iter()
            .find(|e| e.handle == handle)
            .map(|e| e.clock.t())
    }

    /// Let `dt` real seconds pass: the [`Feel`] says how much of it the effects play (none in a hit-stop, a
    /// fraction in slow motion), every instance advances by that (one marked with
    /// [`Scene::set_plays_through_hitstop`] plays all of `dt` in a hit-stop, and its own hit-stops then do not
    /// renew the stop), the cues they fire go to the feel and queue as events, and the instances that are done go, their ends queued. Returns the seconds played, for a host that
    /// freezes its own world with the effects. A `dt` that fails [`valid_dt`] (NaN, infinite or negative) is an
    /// error and changes nothing.
    pub fn update(&mut self, dt: f64) -> Result<f64, StageError> {
        if !valid_dt(dt) {
            return Err(StageError::BadStep);
        }
        // a step that starts in a hit-stop plays nothing for the scene (`advance` gives 0 for all of it)
        let frozen = self.feel.as_ref().is_some_and(Feel::frozen);
        let run = self.feel.as_mut().map_or(dt, |f| f.advance(dt));
        let mut fired = Vec::new();
        let mut felt = Vec::new();
        for e in &mut self.entries {
            let through = frozen && e.through_hitstop;
            e.clock.update(if through { dt } else { run });
            for c in e.clock.take_cues() {
                let c = on_stage(e, c, self.screen_w);
                // an effect playing through a stop cannot renew it, or a looping one would freeze the game for good
                if !(through && matches!(c.cue, Cue::Hitstop { .. })) {
                    felt.push(c);
                }
                fired.push((e.handle, c));
            }
        }
        if let Some(feel) = &mut self.feel {
            feel.take(&felt);
        }
        self.events
            .extend(fired.into_iter().map(|(handle, c)| StageEvent::Cue {
                handle,
                t: c.t,
                pan: c.pan,
                cue: c.cue,
            }));
        self.time += run;
        self.fly()?;
        self.follow();
        let (done, playing): (Vec<_>, Vec<_>) = std::mem::take(&mut self.entries)
            .into_iter()
            .partition(|e| e.clock.done() && !e.ordered && e.trail.is_none());
        self.entries = playing;
        for e in done {
            self.ended.push(e.handle);
            self.events.push(StageEvent::End { handle: e.handle });
            self.released
                .extend(e.own.into_iter().flatten().map(|o| o.surface));
        }
        Ok(run)
    }

    /// Draw at most `layers` layers per host frame (each [`Scene::draw_layer`], or each [`Scene::end_frame`] after
    /// [`Scene::draw_handle`]), or, with `None` (the default), every layer whose frame changed. A layer asked of a
    /// painter that paints elsewhere counts as drawn. An effect left over shows the last frame it showed (nothing,
    /// if it has not been drawn yet or that frame was dropped) and is drawn in a later host frame, the one waiting
    /// longest first, so none starves. It skips to the frame its clock is on then: past the budget an effect plays
    /// at fewer frames a second, and the cost of a host frame stays capped. [`Scene::draw_handle`] takes effects as
    /// the host shows them, first come first drawn.
    pub fn set_draw_budget(&mut self, layers: Option<NonZeroU32>) {
        self.draw_budget = layers;
    }

    /// Draw the back layer of every instance, then the front layer of every instance, oldest first
    /// ([`Scene::draw_layer`]).
    pub fn draw<H: Shell<Surface = S>, P: Painter>(
        &mut self,
        frames: &mut Frames<S, P>,
        shell: &mut H,
    ) -> Result<(), H::Error> {
        self.draw_layer(Layer::Back, frames, shell)?;
        self.draw_layer(Layer::Front, frames, shell)
    }

    /// Show `layer` of every instance, oldest first: what [`Scene::draw_handle`] does for each, then
    /// [`Scene::end_frame`]. A running effect shows a shared frame from `frames`: the first effect of any scene to
    /// reach it has the painter paint it into a new surface, the rest show that surface. A fading effect shows its
    /// own surface, painted again only when its clock frame or placement changes. A frame painted elsewhere shows
    /// once it is delivered; until then the effect shows what it showed before. Stops at the first error the shell
    /// returns.
    pub fn draw_layer<H: Shell<Surface = S>, P: Painter>(
        &mut self,
        layer: Layer,
        frames: &mut Frames<S, P>,
        shell: &mut H,
    ) -> Result<(), H::Error> {
        let now = self.draws + 1;
        let turns = self.turns(layer, now, frames);
        for i in 0..self.entries.len() {
            let may_draw = turns.as_ref().is_none_or(|t| t.contains(&i));
            self.show_entry(i, layer, now, may_draw, frames, shell)?;
        }
        self.end_frame(frames, shell)
    }

    /// Under a draw budget, the entries that may draw `layer` in host frame `now`: of those whose frame is not
    /// held or on its way, the ones waiting longest, as many as the budget has left. `None` without a budget:
    /// every one may.
    fn turns<P: Painter>(
        &mut self,
        layer: Layer,
        now: u64,
        frames: &mut Frames<S, P>,
    ) -> Option<Vec<usize>> {
        let left = self.draw_budget?.get().saturating_sub(self.drawn) as usize;
        let slot = own_slot(layer);
        let shown_at = frames.now();
        let mut due: Vec<(u64, usize)> = Vec::new();
        for (i, e) in self.entries.iter().enumerate() {
            // an effect the host cannot see is not shown, so it takes no turn from one it can
            if !self.in_view(e) {
                continue;
            }
            let needs = match shown(e) {
                // a held frame is marked shown now, which this host frame does anyway
                Some(id) => {
                    !frames.pending.contains(&(id.clone(), layer))
                        && frames
                            .store
                            .show(&id, layer, shown_at, || Err::<(S, u64), ()>(()))
                            .is_err()
                }
                None => {
                    let drawn = e.own_at();
                    e.asked[slot] != Some(drawn)
                        && e.own[slot].as_ref().is_none_or(|own| own.drawn != drawn)
                }
            };
            if needs {
                due.push((e.waiting[slot].unwrap_or(now), i));
            }
        }
        due.sort_unstable();
        Some(due.into_iter().take(left).map(|(_, i)| i).collect())
    }

    /// Show `layer` of the instance `handle` within the host's current frame, for a host that paints each effect
    /// on its own (a gpui view per effect); call [`Scene::end_frame`] once every effect of the frame is shown.
    /// `Ok(false)` if no instance has this handle.
    pub fn draw_handle<H: Shell<Surface = S>, P: Painter>(
        &mut self,
        handle: u32,
        layer: Layer,
        frames: &mut Frames<S, P>,
        shell: &mut H,
    ) -> Result<bool, H::Error> {
        let Some(i) = self.entries.iter().position(|e| e.handle == handle) else {
            return Ok(false);
        };
        let may_draw = self.draw_budget.is_none_or(|b| self.drawn < b.get());
        self.show_entry(i, layer, self.draws + 1, may_draw, frames, shell)?;
        Ok(true)
    }

    /// End the scene's frame: free the surfaces of effects that ended since the last one and have the painter let
    /// go of their state. The shared frames are freed by [`Frames::end_frame`], once every scene drew.
    pub fn end_frame<H: Shell<Surface = S>, P: Painter>(
        &mut self,
        frames: &mut Frames<S, P>,
        shell: &mut H,
    ) -> Result<(), H::Error> {
        self.draws += 1;
        self.drawn = 0;
        self.end_effects(frames);
        for surface in std::mem::take(&mut self.released) {
            shell.free(surface)?;
        }
        Ok(())
    }

    /// Have the painter let go of the state of every effect gone since the last call.
    fn end_effects<P: Painter>(&mut self, frames: &mut Frames<S, P>) {
        for handle in std::mem::take(&mut self.ended) {
            frames.painter.end(EffectKey {
                scene: self.id,
                handle,
            });
        }
    }

    /// Put `frame`, painted elsewhere at `drawn` for layer `layer` of instance `handle` while it fades or wraps
    /// its actor ([`Target::Own`]), in its own surface and show it from the next draw. A frame for an instance gone
    /// since, or older than the one it shows (an earlier clock frame, or the same one with an earlier actor), is
    /// dropped. `Ok(false)` if it was dropped.
    pub fn deliver_own<D, H: Receive<D, Surface = S>>(
        &mut self,
        handle: u32,
        layer: Layer,
        drawn: OwnAt,
        frame: &D,
        shell: &mut H,
    ) -> Result<bool, H::Error> {
        let Some(e) = self.entries.iter_mut().find(|e| e.handle == handle) else {
            return Ok(false);
        };
        let slot = own_slot(layer);
        if e.asked[slot] == Some(drawn) {
            e.asked[slot] = None;
        }
        match &mut e.own[slot] {
            Some(o)
                if (o.drawn.clock, o.drawn.actor, o.drawn.trace)
                    > (drawn.clock, drawn.actor, drawn.trace) =>
            {
                return Ok(false);
            }
            Some(o) => {
                shell.refill_from(&mut o.surface, frame)?;
                o.drawn = drawn;
            }
            None => {
                let surface = shell.make_from(frame)?;
                e.own[slot] = Some(Own { surface, drawn });
            }
        }
        Ok(true)
    }

    /// Show `layer` of entry `i`, marking the frames it shows as shown now. When it needs a new frame and
    /// `may_draw` is false, or the frame is being painted elsewhere, it shows the last one it showed and waits.
    fn show_entry<H: Shell<Surface = S>, P: Painter>(
        &mut self,
        i: usize,
        layer: Layer,
        now: u64,
        may_draw: bool,
        frames: &mut Frames<S, P>,
        shell: &mut H,
    ) -> Result<(), H::Error> {
        let Some(e) = self.entries.get(i) else {
            return Ok(());
        };
        let pass = Pass {
            handle: e.handle,
            layer,
            rect: self.rect_of(e),
        };
        if !self.in_view(e) {
            return Ok(());
        }
        let scene = self.id;
        let Some(e) = self.entries.get_mut(i) else {
            return Ok(());
        };
        let slot = own_slot(layer);
        let key = EffectKey {
            scene,
            handle: e.handle,
        };
        let shown_at = frames.now();
        let mut drew = false;
        match shown(e) {
            Some(id) => {
                let Frames {
                    store,
                    painter,
                    pending,
                    ..
                } = frames;
                // a frame on its way from a painter elsewhere: wait for it, then show it once it is here
                if let Some(f) = e.awaiting[slot].clone().filter(|f| *f != id) {
                    if store.show(&f, layer, shown_at, || Err(())).is_ok() {
                        e.last[slot] = Some(f);
                        e.awaiting[slot] = None;
                    } else if pending.contains(&(f, layer)) {
                        e.waiting[slot].get_or_insert(now);
                        if let Some(last) = e.last[slot].as_ref()
                            && let Ok(surface) = store.show(last, layer, shown_at, || Err(()))
                        {
                            shell.show(pass, surface)?;
                        }
                        return Ok(());
                    } else {
                        // dropped or forgotten on its way: ask for the frame the clock is on
                        e.awaiting[slot] = None;
                    }
                }
                let awaiting = &mut e.awaiting[slot];
                let got = store.show(&id, layer, shown_at, || {
                    if !may_draw {
                        return Err(Fill::Wait);
                    }
                    if pending.contains(&(id.clone(), layer)) {
                        *awaiting = Some(id.clone());
                        return Err(Fill::Wait);
                    }
                    drew = true;
                    let job = Job {
                        effect: key,
                        clip: &e.clip,
                        frame: e.clock.frame(),
                        fade: e.clock.fade(),
                        place: e.place,
                        layer,
                        target: Target::Shared(id.clone()),
                        actor: None,
                        trace: None,
                    };
                    match painter.paint(&job) {
                        Some(frame) => {
                            Ok((shell.make(frame).map_err(Fill::Shell)?, frame_bytes(frame)))
                        }
                        None => {
                            pending.insert((id.clone(), layer));
                            *awaiting = Some(id.clone());
                            Err(Fill::Wait)
                        }
                    }
                });
                match got {
                    Ok(surface) => {
                        shell.show(pass, surface)?;
                        e.last[slot] = Some(id);
                        e.awaiting[slot] = None;
                        e.waiting[slot] = None;
                    }
                    Err(Fill::Shell(err)) => return Err(err),
                    Err(Fill::Wait) => {
                        e.waiting[slot].get_or_insert(now);
                        let last = e.last[slot]
                            .as_ref()
                            .and_then(|last| store.show(last, layer, shown_at, || Err(())).ok());
                        if let Some(surface) = last {
                            shell.show(pass, surface)?;
                        }
                    }
                }
            }
            None => {
                let drawn = e.own_at();
                let stale = e.own[slot].as_ref().is_none_or(|o| o.drawn != drawn);
                // one own frame on its way at a time, as for shared frames
                let asked = e.asked[slot].is_some();
                if stale && may_draw && !asked {
                    drew = true;
                    let job = Job {
                        effect: key,
                        clip: &e.clip,
                        frame: e.clock.frame(),
                        fade: e.clock.fade(),
                        place: e.place,
                        layer,
                        target: Target::Own(drawn),
                        actor: e.wrapped(),
                        trace: e.traced(),
                    };
                    match frames.painter.paint(&job) {
                        Some(frame) => {
                            match &mut e.own[slot] {
                                Some(o) => {
                                    shell.refill(&mut o.surface, frame)?;
                                    o.drawn = drawn;
                                }
                                None => {
                                    let surface = shell.make(frame)?;
                                    e.own[slot] = Some(Own { surface, drawn });
                                }
                            }
                            e.waiting[slot] = None;
                        }
                        None => {
                            e.asked[slot] = Some(drawn);
                            e.waiting[slot].get_or_insert(now);
                        }
                    }
                } else if stale {
                    e.waiting[slot].get_or_insert(now);
                }
                if let Some(o) = &e.own[slot] {
                    shell.show(pass, &o.surface)?;
                }
            }
        }
        if drew {
            self.drawn = self.drawn.saturating_add(1);
        }
        Ok(())
    }

    /// Free every surface the scene holds and every surface waiting to be freed, ending every instance and
    /// having the painter let go of their state: for a host shutting down whose surfaces hold host objects. The
    /// shared frames stay in `frames` for the other scenes ([`Frames::clear`] frees them). The scene is empty
    /// afterwards and can play again.
    pub fn clear<H: Shell<Surface = S>, P: Painter>(
        &mut self,
        frames: &mut Frames<S, P>,
        shell: &mut H,
    ) -> Result<(), H::Error> {
        self.flights.clear();
        for e in std::mem::take(&mut self.entries) {
            self.ended.push(e.handle);
            self.events.push(StageEvent::End { handle: e.handle });
            self.released
                .extend(e.own.into_iter().flatten().map(|o| o.surface));
        }
        self.end_effects(frames);
        for surface in std::mem::take(&mut self.released) {
            shell.free(surface)?;
        }
        Ok(())
    }
}

/// The shared frame entry `e` shows now, or none while it fades out after a stop or wraps its actor (those
/// frames are its own). Effects showing the same id draw byte-equal frames.
fn shown<S>(e: &Entry<S>) -> Option<FrameId> {
    (!e.clock.stopped() && e.wrapped().is_none() && e.trail.is_none()).then(|| FrameId {
        key: e.clip.key(e.place),
        index: e.clock.frame(),
    })
}

/// Which of an entry's own surfaces holds `layer`.
fn own_slot(layer: Layer) -> usize {
    match layer {
        Layer::Back => 0,
        Layer::Front => 1,
        Layer::Both => 2,
    }
}

/// Cue `c` of entry `e`, from its player's frame to the stage: its origin moves to where the entry sits, a flip
/// the host set since it played mirrors it again (rotation is left out), and its pan follows `screen_w`.
fn on_stage<S>(e: &Entry<S>, mut c: TimedCue, screen_w: f64) -> TimedCue {
    let origin = e.clip.placement();
    let sx = if e.place.flip == origin.flip {
        1.0
    } else {
        -1.0
    };
    let (ax, ay) = (e.at.0 as f64, e.at.1 as f64);
    let to_x = |x: f64| ax + sx * (x - origin.x);
    let mut world_x = ax;
    match &mut c.cue {
        Cue::Ripple { x, y, .. } => {
            *x = to_x(*x);
            *y = ay + (*y - origin.y);
            world_x = *x;
        }
        Cue::Sound { x: Some(x), .. } => {
            *x = to_x(*x);
            world_x = *x;
        }
        _ => {}
    }
    c.pan = (screen_w > 0.0).then(|| clamp(world_x / screen_w * 2.0 - 1.0, -1.0, 1.0));
    c
}

/// (`x`, `y`) as a whole game pixel, rounded as JS `Math.round` does; NaN or infinite is an error.
fn game_pixel(x: f64, y: f64) -> Result<(i64, i64), StageError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(StageError::NotFinite);
    }
    // `as` saturates; a position past i64 is off any canvas anyway
    Ok((js_round(x) as i64, js_round(y) as i64))
}

/// Colour words (ABGR) as straight-alpha RGBA bytes into `out` (`w * h * 4`); true if any pixel is visible.
pub fn color_rgba(frame: &Frame, out: &mut [u8]) -> bool {
    let mut any = false;
    for (&c, o) in frame.color.iter().zip(out.as_chunks_mut::<4>().0) {
        *o = c.to_le_bytes();
        any |= c >> 24 != 0;
    }
    any
}

/// Light words (xBGR) as RGBA bytes into `out` (`w * h * 4`) for drawing with `"lighter"` (dst += src): each
/// lit pixel is light over transparency as [`composite_px`] makes it (alpha its brightest channel, rgb
/// `light * 255 / alpha`), so it adds its light to an opaque canvas and shows as that light's colour on a
/// transparent one; unlit pixels are fully transparent. True if any pixel is lit.
pub fn light_rgba(frame: &Frame, out: &mut [u8]) -> bool {
    let mut any = false;
    for (&l, o) in frame.light.iter().zip(out.as_chunks_mut::<4>().0) {
        // most of a frame is unlit, and unlit is transparent: skip the blend for it
        if l & 0x00ff_ffff == 0 {
            *o = [0; 4];
            continue;
        }
        *o = composite_px(0, l, None);
        any = true;
    }
    any
}
