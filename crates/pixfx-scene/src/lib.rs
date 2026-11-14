//! What a stage shows and where, decided in plain Rust for every shell: which effects play, where, how much room
//! round them, at what scale, in which order their layers are drawn, when finished ones go, which frames are drawn
//! and shared, and the game's answer to the cues the effects fire (a [`Feel`]: shake, hit-stop, flash, ripples).
//! A shell (Canvas 2D, Phaser, Pixi, gpui) only makes, refills, shows and frees surfaces as [`Shell`] says.
#![forbid(unsafe_code)]

use std::fmt;
use std::num::NonZeroU32;

mod frames;

pub use frames::{EffectKey, Frames, InlinePainter, Job, Painter, Target};

use frames::frame_bytes;
use pixfx_core::cue::Cue;
use pixfx_core::{Clip, Clock, Feel, FrameId, Layer, Placement, TimedCue, valid_dt};
use pixfx_draw::math::clamp;
use pixfx_draw::math::js_round;
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl View {
    /// Whether `rect` (canvas pixels, `scale` per game pixel) overlaps this view; rectangles that only touch do not.
    fn overlaps(self, rect: Rect, scale: NonZeroU32) -> bool {
        let s = f64::from(scale.get());
        let (x0, y0) = (self.x * s, self.y * s);
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageError {
    /// A position was NaN or infinite.
    NotFinite,
    /// Every `u32` handle has been given out once.
    OutOfHandles,
    /// A time step was NaN, infinite or negative.
    BadStep,
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotFinite => "x and y must be finite numbers",
            Self::OutOfHandles => "every handle has been used",
            Self::BadStep => "dt must be a finite number of seconds, 0 or more",
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

/// A layer's own frame: the surface an effect fading after `stop()` draws into, and what it last drew there.
struct Own<S> {
    surface: S,
    /// The clock frame and placement it was drawn at: it is drawn again only when either changes.
    drawn: (u64, Placement),
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
    /// Its own frames while it fades out (back, front, both); shared frames live in [`Frames`].
    own: [Option<Own<S>>; 3],
    /// The own frame each layer asked a painter for and has not been handed yet ([`Scene::deliver_own`]).
    asked: [Option<(u64, Placement)>; 3],
    /// The shared frame each layer showed last: what it shows again while it waits for a draw.
    last: [Option<FrameId>; 3],
    /// The host frame each layer first waited for a draw in, past the draw budget; `None` while it is up to date.
    waiting: [Option<u64>; 3],
    /// Plays real time through a hit-stop instead of freezing with the rest ([`Scene::set_plays_through_hitstop`]).
    through_hitstop: bool,
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
            && ![v.x, v.y, v.w, v.h].iter().all(|n| n.is_finite())
        {
            return Err(StageError::NotFinite);
        }
        self.view = view;
        Ok(())
    }

    pub fn scale(&self) -> NonZeroU32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: NonZeroU32) {
        self.scale = scale;
    }

    /// Play `clip` from its start with its origin at game pixel (`x`, `y`), rounded as JS `Math.round` does;
    /// returns its handle.
    pub fn play(&mut self, clip: Clip, x: f64, y: f64) -> Result<u32, StageError> {
        let at = game_pixel(x, y)?;
        let handle = self.next.ok_or(StageError::OutOfHandles)?;
        self.next = handle.checked_add(1);
        self.entries.push(Entry {
            handle,
            at,
            place: clip.placement(),
            clock: clip.clock(),
            clip,
            own: [None, None, None],
            asked: [None, None, None],
            last: [None, None, None],
            waiting: [None, None, None],
            through_hitstop: false,
        });
        Ok(handle)
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
        // the player's own x/y keep the origin on its frame; the stage places the frame
        e.place = Placement {
            x: e.place.x,
            y: e.place.y,
            ..p
        };
        Ok(true)
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

    /// Whether a frame at `rect` is seen: it overlaps the view, or there is no view.
    fn in_view(&self, rect: Rect) -> bool {
        self.view.is_none_or(|v| v.overlaps(rect, self.scale))
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
        let (done, playing): (Vec<_>, Vec<_>) = std::mem::take(&mut self.entries)
            .into_iter()
            .partition(|e| e.clock.done());
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
            if !self.in_view(self.rect_of(e)) {
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
                    let drawn = (e.clock.clock_frame(), e.place);
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

    /// Put `frame`, painted elsewhere for layer `layer` of instance `handle` while it fades
    /// ([`Target::Own`]), in its own surface and show it from the next draw. A frame for an instance gone since,
    /// or older than the one it shows, is dropped. `Ok(false)` if it was dropped.
    pub fn deliver_own<H: Shell<Surface = S>>(
        &mut self,
        handle: u32,
        layer: Layer,
        clock: u64,
        place: Placement,
        frame: &Frame,
        shell: &mut H,
    ) -> Result<bool, H::Error> {
        let Some(e) = self.entries.iter_mut().find(|e| e.handle == handle) else {
            return Ok(false);
        };
        let slot = own_slot(layer);
        if e.asked[slot].is_some_and(|(c, p)| c == clock && p == place) {
            e.asked[slot] = None;
        }
        let drawn = (clock, place);
        match &mut e.own[slot] {
            Some(o) if o.drawn.0 > clock => return Ok(false),
            Some(o) => {
                shell.refill(&mut o.surface, frame)?;
                o.drawn = drawn;
            }
            None => {
                let surface = shell.make(frame)?;
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
        if !self.in_view(pass.rect) {
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
                let got = store.show(&id, layer, shown_at, || {
                    if !may_draw || pending.contains(&(id.clone(), layer)) {
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
                    };
                    match painter.paint(&job) {
                        Some(frame) => {
                            Ok((shell.make(frame).map_err(Fill::Shell)?, frame_bytes(frame)))
                        }
                        None => {
                            pending.insert((id.clone(), layer));
                            Err(Fill::Wait)
                        }
                    }
                });
                match got {
                    Ok(surface) => {
                        shell.show(pass, surface)?;
                        e.last[slot] = Some(id);
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
                let drawn = (e.clock.clock_frame(), e.place);
                let stale = e.own[slot].as_ref().is_none_or(|o| o.drawn != drawn);
                let asked = e.asked[slot] == Some(drawn);
                if stale && may_draw && !asked {
                    drew = true;
                    let job = Job {
                        effect: key,
                        clip: &e.clip,
                        frame: e.clock.frame(),
                        fade: e.clock.fade(),
                        place: e.place,
                        layer,
                        target: Target::Own {
                            clock: drawn.0,
                            place: drawn.1,
                        },
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

/// The shared frame entry `e` shows now, or none while it fades out after a stop (a fade frame is its own).
/// Effects showing the same id draw byte-equal frames.
fn shown<S>(e: &Entry<S>) -> Option<FrameId> {
    (!e.clock.stopped()).then(|| FrameId {
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
