//! The effect contract and live instances: port of pixelfx `src/core/fx.js` (`resolveOptions`, `createFx`).
//!
//! An effect is a static [`Effect`]: its [`Meta`] plus a way to spawn state. Effects crates list theirs in a
//! static slice; [`find`] looks one up by id.
//! An [`Instance`] owns one spawned [`EffectState`], advances it in fixed sub-steps and draws it at the
//! instance's time, fading it out after [`Instance::stop`].

use std::collections::BTreeMap;

use crate::color::{Rgba, fit};
use crate::cue::{TimedCue, cues_between, cues_of};
use crate::frame::Frame;
use crate::math::{clamp, js_max, js_min};
use crate::painter::Painter;
use crate::state::StateMeta;

/// The longest step [`Instance::update`] takes: a live host that stalls (a background tab, a debugger)
/// resumes where it left off instead of simulating the whole gap. pixelfx has no such clamp; renders never
/// meet it (see [`crate::render::render_frames`]).
pub const MAX_DT: f64 = 0.25;

/// Seconds an instance takes to fade out after [`Instance::stop`] (pixelfx `FADE`).
pub const FADE: f64 = 0.3;

/// Simulation sub-steps per second: `update(dt)` runs at least this finely whatever the caller's frame rate.
const STEPS_PER_SECOND: f64 = 120.0;

/// What an effect is: pixelfx `meta`, minus what the Rust port types elsewhere (options live in the effect,
/// the backdrop belongs to the tools).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Meta {
    /// `category/name`, as pixelfx's file path under `src/fx` (`fire/candle`).
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub tags: &'static [&'static str],
    /// Canvas size in pixels.
    pub size: (u32, u32),
    /// Where the effect's local (0, 0) sits on its canvas.
    pub origin: (i32, i32),
    /// Seconds: one loop, or the whole one-shot.
    pub duration: f64,
    pub looping: bool,
    pub fps: u32,
    /// The seed used when the caller gives none (pixelfx `meta.options.seed`; every pixelfx effect has one).
    pub seed: u64,
    /// Set when the effect is one state of a creature.
    pub state: Option<StateMeta>,
}

impl Meta {
    /// A full-screen effect (transitions, weather, impact frames): 16:9 and at least 320 wide
    /// (pixelfx `isScreen`). Renders of these keep their light up to the canvas edge.
    pub fn is_screen(&self) -> bool {
        let (w, h) = (f64::from(self.size.0), f64::from(self.size.1));
        w >= 320.0 && (w / h - 16.0 / 9.0).abs() < 0.05
    }

    /// Empty pixels worth giving the effect on every side of its canvas so glow that spills past it shows
    /// instead of being cut: half the canvas's longer side.
    pub fn glow_margin(&self) -> u32 {
        self.size.0.max(self.size.1) / 2
    }

    /// The canvas an instance draws on unless its effect sizes it from its options: `size` and `origin`.
    pub fn canvas(&self) -> Canvas {
        Canvas {
            size: self.size,
            origin: self.origin,
        }
    }
}

/// The canvas one instance draws on: its size in pixels and where the effect's local (0, 0) sits on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Canvas {
    pub size: (u32, u32),
    pub origin: (i32, i32),
}

/// Which part of an effect to draw: what goes behind a character, what goes in front, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Back,
    Front,
    Both,
}

/// An effect: a static, spawned into any number of instances.
pub trait Effect: Send + Sync {
    fn meta(&self) -> &Meta;
    /// The canvas an instance spawned with `opts` draws on: [`Meta::canvas`], unless the effect sizes its
    /// canvas from its options (weather's `width` and `height`).
    fn canvas(&self, _opts: &Options) -> Canvas {
        self.meta().canvas()
    }
    /// Fresh state for one instance (pixelfx `create(opts)`; a draw-only module returns state holding just
    /// its resolved options). `seed` is the caller's seed or [`Meta::seed`].
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState>;
    /// The game-feel cues this effect fires, in its own space and time (pixelfx `cues(o)`); none by default.
    fn cues(&self, _opts: &Options) -> Vec<TimedCue> {
        Vec::new()
    }
}

/// One instance's state.
pub trait EffectState: Send {
    /// Advance one simulation sub-step of `dt` seconds (pixelfx `update`; draw-only effects keep the default).
    fn update(&mut self, _dt: f64) {}
    /// Draw at time `t` (seconds into the loop or one-shot). [`Instance`] calls this with [`Layer::Back`]
    /// (pixelfx `drawBack`) and [`Layer::Front`] (pixelfx `draw`), never [`Layer::Both`]; an effect with
    /// nothing behind the character draws nothing for `Back`.
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer);
}

/// An option value beyond the common ones: a number, a colour ramp or a single colour.
#[derive(Clone, Debug, PartialEq)]
pub enum OptValue {
    Num(f64),
    Ramp(Vec<Rgba>),
    Color(Rgba),
}

/// What the caller sets on an instance; anything left unset takes the effect's default.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Where the effect's origin goes, in frame space.
    pub x: f64,
    pub y: f64,
    /// Mirror left-right.
    pub flip: bool,
    /// Rotation, radians.
    pub angle: f64,
    pub seed: Option<u64>,
    /// Glow strength.
    pub light: f64,
    pub palette: Option<Vec<Rgba>>,
    /// The effect's own options by name (pixelfx `height`, `wood`, ...).
    pub extra: BTreeMap<String, OptValue>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            flip: false,
            angle: 0.0,
            seed: None,
            light: 1.0,
            palette: None,
            extra: BTreeMap::new(),
        }
    }
}

/// pixelfx `resolveOptions` for one ramp: the caller's ramp, resampled to the default's length when it is
/// a non-empty ramp of another length, else the default.
fn resolve_ramp(given: Option<&[Rgba]>, default: &[Rgba]) -> Vec<Rgba> {
    match given {
        Some(r) if !r.is_empty() && r.len() != default.len() => fit(r, default.len()),
        Some(r) => r.to_vec(),
        None => default.to_vec(),
    }
}

impl Options {
    /// The palette, resolved against the effect's default as pixelfx `resolveOptions` does.
    pub fn palette_or(&self, default: &[Rgba]) -> Vec<Rgba> {
        resolve_ramp(self.palette.as_deref(), default)
    }

    /// A ramp option, resolved against the effect's default as pixelfx `resolveOptions` does. A number given
    /// for a ramp is ignored (pixelfx would pass it through and draw garbage).
    pub fn ramp_or(&self, key: &str, default: &[Rgba]) -> Vec<Rgba> {
        let given = match self.extra.get(key) {
            Some(OptValue::Ramp(r)) => Some(r.as_slice()),
            _ => None,
        };
        resolve_ramp(given, default)
    }

    /// A single-colour option (pixelfx `stone: "#4a3f69"`), or the effect's default when it is unset or not a
    /// colour (a ramp given for it is not a colour).
    pub fn color_or(&self, key: &str, default: Rgba) -> Rgba {
        match self.extra.get(key) {
            Some(OptValue::Color(c)) => *c,
            _ => default,
        }
    }

    /// A numeric option, or the effect's default when it is unset or not a number.
    pub fn num_or(&self, key: &str, default: f64) -> f64 {
        match self.extra.get(key) {
            Some(OptValue::Num(v)) => *v,
            _ => default,
        }
    }

    /// [`Options::num_or`] held to `lo..=hi`, for an option that sizes a loop: an effect names the largest value
    /// it draws in reasonable time, so a huge number cannot hang it. NaN stays NaN, which draws nothing.
    pub fn num_in(&self, key: &str, default: f64, lo: f64, hi: f64) -> f64 {
        clamp(self.num_or(key, default), lo, hi)
    }
}

/// A live effect: pixelfx `createFx`.
pub struct Instance {
    effect: &'static dyn Effect,
    state: Box<dyn EffectState>,
    opts: Options,
    t: f64,
    stop_at: Option<f64>,
    cues: Vec<TimedCue>,
    fired: Vec<TimedCue>,
    screen_w: f64,
}

impl Instance {
    pub fn new(effect: &'static dyn Effect, opts: Options) -> Self {
        let seed = opts.seed.unwrap_or(effect.meta().seed);
        let state = effect.spawn(&opts, seed);
        let cues = cues_of(effect, &opts);
        Self {
            effect,
            state,
            opts,
            t: 0.0,
            stop_at: None,
            cues,
            fired: Vec::new(),
            screen_w: 0.0,
        }
    }

    /// The screen width that sets the stereo pan of fired cues; 0 (the default) leaves them without pan.
    pub fn set_screen_w(&mut self, screen_w: f64) {
        self.screen_w = screen_w;
    }

    /// The cues playback has passed since the last call, in world space and time order; clears them.
    pub fn take_cues(&mut self) -> Vec<TimedCue> {
        std::mem::take(&mut self.fired)
    }

    /// Advance by `dt` seconds, at most [`MAX_DT`]. A step that fails [`valid_dt`] (NaN, infinite or
    /// negative) is ignored and leaves the instance as it was: it would make the time NaN for good or run it
    /// backwards. Shells that report such a step to their caller check [`valid_dt`] themselves first.
    pub fn update(&mut self, dt: f64) {
        if valid_dt(dt) {
            let dt = js_min(dt, MAX_DT);
            if self.stop_at.is_none() {
                self.fire_cues(self.t, self.t + dt);
            }
            self.advance(dt);
        }
    }

    /// Record the cues in `(t0, t1]`; a cue at time 0 fires on the first update (pixelfx `fire`).
    fn fire_cues(&mut self, t0: f64, t1: f64) {
        let from = if t0 == 0.0 { -1e-9 } else { t0 };
        let meta = self.effect.meta();
        let hit = cues_between(&self.cues, from, t1, meta, &self.opts, self.screen_w);
        self.fired.extend(hit);
    }

    /// Advance by `dt` seconds in `max(1, ceil(dt * 120 - 1e-9))` equal sub-steps (pixelfx `update`).
    pub(crate) fn advance(&mut self, dt: f64) {
        self.t += dt;
        // NaN steps run no sub-steps, as JS's `i < NaN` loop
        let n = js_max(1.0, libm::ceil(dt * STEPS_PER_SECOND - 1e-9));
        for _ in 0..n as u64 {
            self.state.update(dt / n);
        }
    }

    /// Draw into `frame` at the current time: loops wrap, one-shots hold their last moment.
    pub fn draw(&self, frame: &mut Frame, layer: Layer) {
        let meta = self.effect.meta();
        let mut p = Painter::new(frame);
        p.place(self.opts.x, self.opts.y, self.opts.flip, self.opts.angle);
        p.fade = match self.stop_at {
            None => 1.0,
            Some(s) => clamp(1.0 - (self.t - s) / FADE, 0.0, 1.0),
        };
        p.light_k = self.opts.light;
        let t = if meta.looping {
            self.t % meta.duration
        } else {
            js_min(self.t, meta.duration)
        };
        if layer != Layer::Front {
            self.state.draw(&mut p, t, Layer::Back);
        }
        if layer != Layer::Back {
            self.state.draw(&mut p, t, Layer::Front);
        }
    }

    /// Let it end: loops fade out over [`FADE`], one-shots are cut short. Later calls keep the first stop.
    pub fn stop(&mut self) {
        if self.stop_at.is_none() {
            self.stop_at = Some(self.t);
        }
    }

    pub fn done(&self) -> bool {
        let meta = self.effect.meta();
        self.stop_at.is_some_and(|s| self.t - s >= FADE)
            || (!meta.looping && self.t >= meta.duration)
    }

    /// Seconds since the instance started.
    pub fn t(&self) -> f64 {
        self.t
    }
}

/// Whether `dt` is a time step [`Instance::update`] takes: a finite number of seconds, 0 or more.
pub fn valid_dt(dt: f64) -> bool {
    dt.is_finite() && dt >= 0.0
}

/// The effect with this id in `list`.
pub fn find(list: &[&'static dyn Effect], id: &str) -> Option<&'static dyn Effect> {
    list.iter().copied().find(|e| e.meta().id == id)
}
