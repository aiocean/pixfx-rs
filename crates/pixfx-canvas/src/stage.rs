//! What the canvas renderer decides, in plain Rust: which effects play, where, how much room round them, at
//! what scale, in which order their layers are drawn, when finished ones go, the pixels each layer becomes,
//! and the game's answer to the cues the effects fire (a [`Feel`]: shake, hit-stop, flash, ripples). No browser types, so it is tested natively; [`crate::Stage`] blits what this produces.

use std::fmt;
use std::num::NonZeroU32;

use pixfx_core::cue::Cue;
use pixfx_core::math::clamp;
use pixfx_core::math::js_round;
use pixfx_core::{
    Effect, Feel, Frame, Layer, Meta, Options, Placement, TimedCue, composite_px, valid_dt,
};
use pixfx_wasm::{Player, parse_options};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsError, JsValue};

/// [`parse_options`] for [`crate::Stage::play`]: the stage sizes each effect's margin itself, so a `margin`
/// option is an error.
pub fn stage_options(meta: &Meta, v: &JsValue) -> Result<Options, JsError> {
    let (opts, margin) = parse_options(meta, v)?;
    if margin.is_some() {
        return Err(JsError::new(
            "options.margin is not supported: Stage sizes the margin itself",
        ));
    }
    Ok(opts)
}

/// A change to where and how an effect draws, for [`crate::Stage::set`]: each field left out keeps its value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, tsify::Tsify)]
#[serde(deny_unknown_fields)]
pub struct PlacementChange {
    /// The effect's origin, in game pixels (rounded to a whole pixel).
    #[serde(default)]
    #[tsify(optional)]
    pub x: Option<f64>,
    #[serde(default)]
    #[tsify(optional)]
    pub y: Option<f64>,
    /// Mirror left-right.
    #[serde(default)]
    #[tsify(optional)]
    pub flip: Option<bool>,
    /// Rotation, radians.
    #[serde(default)]
    #[tsify(optional)]
    pub angle: Option<f64>,
    /// Glow strength.
    #[serde(default)]
    #[tsify(optional)]
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

/// `effect` with `opts` as the stage plays it: on its canvas plus [`Meta::glow_margin`], so nothing at the
/// canvas edge is faded or cut.
pub fn live_player(effect: &'static dyn Effect, opts: Options) -> Player {
    Player::new(effect, opts, effect.meta().glow_margin())
}

/// A target rectangle in canvas pixels (game pixels times the scale).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: u64,
    pub h: u64,
}

/// One blit: `layer` of the instance `handle`, drawn into `rect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pass {
    pub handle: u32,
    pub layer: Layer,
    pub rect: Rect,
}

/// Something that happened on the stage, queued in order until the host drains it ([`Scene::drain_events`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, tsify::Tsify)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StageEvent {
    /// The instance is gone: it finished, faded out after a stop, or was removed.
    End { handle: u32 },
    /// The instance fired a cue: `t` seconds into its playback, in game pixels on the stage (a ripple's or a
    /// sound's `x`/`y`), `pan` -1 to 1 across the stage once its width is known. The stage's [`Feel`] has already
    /// taken it; a sound is the host's to play.
    Cue {
        handle: u32,
        t: f64,
        #[tsify(type = "number | null")]
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

struct Entry<T> {
    handle: u32,
    /// Where the effect's origin sits, in whole game pixels.
    at: (i64, i64),
    /// How the player draws the effect on its own frame: `x`/`y` its origin there, the rest as the host sets.
    place: Placement,
    player: Player,
    /// What the shell keeps per instance (its scratch canvas); `()` in tests.
    surface: T,
}

/// The playing instances, in the order they were played, and the game-to-canvas scale.
pub struct Scene<T> {
    scale: NonZeroU32,
    /// The next handle to give out; handles start at 1 and are never reused.
    next: Option<u32>,
    entries: Vec<Entry<T>>,
    /// Events not yet drained, oldest first.
    events: Vec<StageEvent>,
    /// The answer to the cues the effects fire; `None` when the host answers them with its own.
    feel: Option<Feel>,
    /// The stage's width in game pixels, for a cue's pan; 0 leaves cues without pan.
    screen_w: f64,
}

impl<T> Scene<T> {
    pub fn new(scale: NonZeroU32) -> Self {
        Self {
            scale,
            next: Some(1),
            entries: Vec::new(),
            events: Vec::new(),
            feel: Some(Feel::new()),
            screen_w: 0.0,
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

    pub fn scale(&self) -> NonZeroU32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: NonZeroU32) {
        self.scale = scale;
    }

    /// Add `player` with its origin at game pixel (`x`, `y`), rounded as JS `Math.round` does; returns its
    /// handle.
    pub fn play(&mut self, player: Player, x: f64, y: f64, surface: T) -> Result<u32, StageError> {
        let at = game_pixel(x, y)?;
        let handle = self.next.ok_or(StageError::OutOfHandles)?;
        self.next = handle.checked_add(1);
        self.entries.push(Entry {
            handle,
            at,
            place: player.placement(),
            player,
            surface,
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

    /// Let the instance end (loops fade out); false if no instance has this handle.
    pub fn stop(&mut self, handle: u32) -> bool {
        self.entries
            .iter_mut()
            .find(|e| e.handle == handle)
            .map(|e| e.player.stop())
            .is_some()
    }

    /// Drop the instance now, queueing its end; false if no instance has this handle.
    pub fn remove(&mut self, handle: u32) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.handle != handle);
        let removed = self.entries.len() != before;
        if removed {
            self.events.push(StageEvent::End { handle });
        }
        removed
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

    /// The instance's frame (its canvas plus margin) placed so the effect's origin lands on `e.at`.
    fn rect_of(&self, e: &Entry<T>) -> Rect {
        let s = u64::from(self.scale.get());
        let origin = e.player.canvas().origin;
        let pad = i64::from(e.player.margin());
        let frame = e.player.frame();
        let corner = |at: i64, origin: i32| {
            at.saturating_sub(i64::from(origin))
                .saturating_sub(pad)
                .saturating_mul(s as i64)
        };
        Rect {
            x: corner(e.at.0, origin.0),
            y: corner(e.at.1, origin.1),
            w: u64::from(frame.w) * s,
            h: u64::from(frame.h) * s,
        }
    }

    /// Seconds since the instance started.
    pub fn t(&self, handle: u32) -> Option<f64> {
        self.entries
            .iter()
            .find(|e| e.handle == handle)
            .map(|e| e.player.t())
    }

    /// Let `dt` real seconds pass: the [`Feel`] says how much of it the effects play (none in a hit-stop, a
    /// fraction in slow motion), every instance advances by that, the cues they fire go to the feel and queue as
    /// events, and the instances that are done go, their ends queued. Returns the seconds played, for a host that
    /// freezes its own world with the effects. A `dt` that fails [`valid_dt`] (NaN, infinite or negative) is an
    /// error and changes nothing.
    pub fn update(&mut self, dt: f64) -> Result<f64, StageError> {
        if !valid_dt(dt) {
            return Err(StageError::BadStep);
        }
        let run = self.feel.as_mut().map_or(dt, |f| f.advance(dt));
        let mut fired = Vec::new();
        for e in &mut self.entries {
            e.player.update(run);
            for c in e.player.take_cues() {
                fired.push((e.handle, on_stage(e, c, self.screen_w)));
            }
        }
        if let Some(feel) = &mut self.feel {
            let cues: Vec<TimedCue> = fired.iter().map(|&(_, c)| c).collect();
            feel.take(&cues);
        }
        self.events
            .extend(fired.into_iter().map(|(handle, c)| StageEvent::Cue {
                handle,
                t: c.t,
                pan: c.pan,
                cue: c.cue,
            }));
        let events = &mut self.events;
        self.entries.retain(|e| {
            let done = e.player.done();
            if done {
                events.push(StageEvent::End { handle: e.handle });
            }
            !done
        });
        Ok(run)
    }

    /// Draw the back layer of every instance, then the front layer of every instance, oldest first, handing
    /// each drawn frame to `blit`. Stops at the first error `blit` returns.
    pub fn draw<E>(
        &mut self,
        mut blit: impl FnMut(Pass, &Frame, &mut T) -> Result<(), E>,
    ) -> Result<(), E> {
        self.draw_layer(Layer::Back, &mut blit)?;
        self.draw_layer(Layer::Front, &mut blit)
    }

    /// Draw only `layer` of every instance, oldest first, handing each drawn frame to `blit`. Stops at the
    /// first error `blit` returns.
    pub fn draw_layer<E>(
        &mut self,
        layer: Layer,
        mut blit: impl FnMut(Pass, &Frame, &mut T) -> Result<(), E>,
    ) -> Result<(), E> {
        for i in 0..self.entries.len() {
            let Some(e) = self.entries.get(i) else { break };
            let pass = Pass {
                handle: e.handle,
                layer,
                rect: self.rect_of(e),
            };
            let Some(e) = self.entries.get_mut(i) else {
                break;
            };
            e.player.draw(layer, e.place);
            blit(pass, e.player.frame(), &mut e.surface)?;
        }
        Ok(())
    }
}

/// Cue `c` of entry `e`, from its player's frame to the stage: its origin moves to where the entry sits, a flip
/// the host set since it played mirrors it again (rotation is left out), and its pan follows `screen_w`.
fn on_stage<T>(e: &Entry<T>, mut c: TimedCue, screen_w: f64) -> TimedCue {
    let origin = e.player.placement();
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
