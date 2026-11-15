//! The effect contract and live instances: port of pixelfx `src/core/fx.js` (`resolveOptions`, `createFx`).
//!
//! An effect is a static [`Effect`]: its [`Meta`] plus a way to spawn state. Effects crates list theirs in a
//! static slice; [`find`] looks one up by id.
//! An [`Instance`] owns one spawned [`EffectState`] and shows it frame by frame at the effect's fps: frame k is
//! the state after k ticks of `1 / fps`, whatever steps the host's clock takes, so live play draws exactly the
//! frames [`crate::render::render_frames`] renders. It fades the effect out after [`Instance::stop`].

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::actor::Actor;
use crate::cue::{TimedCue, cues_between, cues_of};
use crate::state::StateMeta;
use pixfx_draw::color::{Rgba, fit};
use pixfx_draw::frame::Frame;
use pixfx_draw::math::{clamp, js_max, js_min, js_round};
use pixfx_draw::painter::Painter;

/// The longest step [`Instance::update`] takes: a live host that stalls (a background tab, a debugger)
/// resumes where it left off instead of simulating the whole gap. pixelfx has no such clamp; renders never
/// meet it (see [`crate::render::render_frames`]).
pub const MAX_DT: f64 = 0.25;

/// Seconds an instance takes to fade out after [`Instance::stop`] (pixelfx `FADE`).
pub const FADE: f64 = 0.3;

/// Simulation sub-steps per second: a tick of `1 / fps` runs in sub-steps at least this fine.
const STEPS_PER_SECOND: f64 = 120.0;

/// Slack when turning a time into a frame index, so a clock that adds up `1 / fps` k times reaches frame k
/// despite rounding (k ticks of 1/30 can sum to a hair under k/30).
const FRAME_SLACK: f64 = 1e-6;

/// What an effect is: pixelfx `meta`, minus what the Rust port types elsewhere (the palette's default lives in
/// the effect, the backdrop belongs to the tools).
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
    /// The effect's own options beyond the ones every effect takes (pixelfx `meta.options` minus `palette` and
    /// `seed`), each declared once with its default. The effect reads them through the same declarations
    /// ([`Options::num`], [`Options::color`], [`Options::ramp`]); [`check_options`] refuses any other key.
    pub options: &'static [OptSpec],
}

impl Meta {
    /// A full-screen effect (transitions, weather, impact frames): 16:9 and at least 320 wide
    /// (pixelfx `isScreen`). Renders of these keep their light up to the canvas edge.
    pub fn is_screen(&self) -> bool {
        let (w, h) = (f64::from(self.size.0), f64::from(self.size.1));
        w >= 320.0 && (w / h - 16.0 / 9.0).abs() < 0.05
    }

    /// Frames in one loop or one-shot at the effect's fps: `max(1, round(duration * fps))` for a loop, one more
    /// for a one-shot (its last moment). Frame k is the effect after k ticks of `1 / fps`.
    pub fn frames(&self) -> u64 {
        let last = if self.looping { 0.0 } else { 1.0 };
        js_max(1.0, js_round(self.duration * f64::from(self.fps)) + last) as u64
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// It grips the character it plays on ([`EffectState::set_actor`]), so its frames depend on that character's
    /// sprite and are its instance's own, never shared. False by default.
    fn wraps_actor(&self) -> bool {
        false
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
    /// The character it plays on, as its sprite shows now, or none; [`Sim`] hands it over before every step and
    /// again after a respawn. Only an effect that [`Effect::wraps_actor`] reads it.
    fn set_actor(&mut self, _actor: Option<Arc<Actor>>) {}
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

/// One option an effect declares in [`Meta::options`]: its name and what it holds, with the default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptSpec {
    pub name: &'static str,
    pub kind: OptKind,
}

/// What an option holds, and its default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptKind {
    /// A number. `range` holds it to `lo..=hi` where it sizes a loop, so a huge number cannot hang the effect.
    /// A NaN default means the effect picks the value itself (burn-away's seeded hole).
    Num {
        default: f64,
        range: Option<(f64, f64)>,
    },
    /// One colour (pixelfx `stone: "#4a3f69"`).
    Color { default: Rgba },
    /// A colour ramp; a given ramp of another length is resampled to the default's.
    Ramp { default: &'static [Rgba] },
}

impl OptKind {
    /// What the kind holds, in words, for errors.
    pub fn what(&self) -> &'static str {
        match self {
            OptKind::Num { .. } => "a number",
            OptKind::Color { .. } => "a colour",
            OptKind::Ramp { .. } => "a list of colours",
        }
    }

    fn holds(&self, v: &OptValue) -> bool {
        matches!(
            (self, v),
            (OptKind::Num { .. }, OptValue::Num(_))
                | (OptKind::Color { .. }, OptValue::Color(_))
                | (OptKind::Ramp { .. }, OptValue::Ramp(_))
        )
    }
}

/// A number option as the effect declares and reads it: `const HEIGHT: NumOpt = NumOpt::new("height", 22.0);`,
/// listed in [`Meta::options`] as `HEIGHT.spec()`, read with [`Options::num`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumOpt {
    pub name: &'static str,
    pub default: f64,
    pub range: Option<(f64, f64)>,
}

impl NumOpt {
    pub const fn new(name: &'static str, default: f64) -> Self {
        Self {
            name,
            default,
            range: None,
        }
    }

    /// Held to `lo..=hi`, for an option that sizes a loop: the effect names the largest value it draws in
    /// reasonable time.
    pub const fn within(self, lo: f64, hi: f64) -> Self {
        Self {
            range: Some((lo, hi)),
            ..self
        }
    }

    pub const fn spec(&self) -> OptSpec {
        OptSpec {
            name: self.name,
            kind: OptKind::Num {
                default: self.default,
                range: self.range,
            },
        }
    }
}

/// A single-colour option, declared and read like [`NumOpt`] ([`Options::color`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorOpt {
    pub name: &'static str,
    pub default: Rgba,
}

impl ColorOpt {
    pub const fn new(name: &'static str, default: Rgba) -> Self {
        Self { name, default }
    }

    pub const fn spec(&self) -> OptSpec {
        OptSpec {
            name: self.name,
            kind: OptKind::Color {
                default: self.default,
            },
        }
    }
}

/// A colour-ramp option, declared and read like [`NumOpt`] ([`Options::ramp`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RampOpt {
    pub name: &'static str,
    pub default: &'static [Rgba],
}

impl RampOpt {
    pub const fn new(name: &'static str, default: &'static [Rgba]) -> Self {
        Self { name, default }
    }

    pub const fn spec(&self) -> OptSpec {
        OptSpec {
            name: self.name,
            kind: OptKind::Ramp {
                default: self.default,
            },
        }
    }
}

/// An option the effect does not take, or a value of the wrong kind for one it does.
#[derive(Clone, Debug, PartialEq)]
pub enum OptionError {
    Unknown {
        key: String,
        effect: &'static str,
        known: &'static [OptSpec],
    },
    Kind {
        key: String,
        effect: &'static str,
        want: &'static str,
    },
}

impl fmt::Display for OptionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OptionError::Unknown { key, effect, known } => {
                write!(f, "options.{key}: {effect} has no option {key:?}")?;
                let mut names = known.iter().map(|o| o.name);
                match names.next() {
                    None => f.write_str("; it takes none of its own"),
                    Some(first) => {
                        write!(f, "; its own options are {first}")?;
                        names.try_for_each(|n| write!(f, ", {n}"))
                    }
                }
            }
            OptionError::Kind { key, effect, want } => {
                write!(f, "options.{key}: {effect}'s {key} is {want}")
            }
        }
    }
}

/// Every one of `opts`' own options is one `meta` declares, holding the declared kind: what a host checks once
/// where options arrive from outside (JS, a file), so a misspelt or foreign option is an error instead of
/// being dropped.
pub fn check_options(meta: &Meta, opts: &Options) -> Result<(), OptionError> {
    for (key, v) in &opts.extra {
        let Some(spec) = meta.options.iter().find(|o| o.name == key) else {
            return Err(OptionError::Unknown {
                key: key.clone(),
                effect: meta.id,
                known: meta.options,
            });
        };
        if !spec.kind.holds(v) {
            return Err(OptionError::Kind {
                key: key.clone(),
                effect: meta.id,
                want: spec.kind.what(),
            });
        }
    }
    Ok(())
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

    /// A ramp option, resolved against its default as pixelfx `resolveOptions` does. A value of another kind is
    /// ignored (pixelfx would pass a number through and draw garbage).
    pub fn ramp(&self, o: &RampOpt) -> Vec<Rgba> {
        let given = match self.extra.get(o.name) {
            Some(OptValue::Ramp(r)) => Some(r.as_slice()),
            _ => None,
        };
        resolve_ramp(given, o.default)
    }

    /// A single-colour option, or its default when it is unset or not a colour (a ramp is not a colour).
    pub fn color(&self, o: &ColorOpt) -> Rgba {
        match self.extra.get(o.name) {
            Some(OptValue::Color(c)) => *c,
            _ => o.default,
        }
    }

    /// A number option, or its default when it is unset or not a number; held to its range if it has one. NaN
    /// stays NaN, which draws nothing.
    pub fn num(&self, o: &NumOpt) -> f64 {
        let v = match self.extra.get(o.name) {
            Some(OptValue::Num(v)) => *v,
            _ => o.default,
        };
        match o.range {
            Some((lo, hi)) => clamp(v, lo, hi),
            None => v,
        }
    }
}

/// Where and how an instance is drawn: the part of its [`Options`] read only at draw time, passed to
/// [`Instance::draw`] each frame. The host keeps it as data and may change it while the effect plays; start from
/// [`Instance::placement`] and change fields with struct update: `Placement { flip: true, ..inst.placement() }`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// Where the effect's origin goes, in frame space.
    pub x: f64,
    pub y: f64,
    /// Mirror left-right.
    pub flip: bool,
    /// Rotation, radians.
    pub angle: f64,
    /// Glow strength.
    pub light: f64,
}

/// An instance's clock: its time, its stop, the cues playback passes and when it ends. Everything a host decides
/// (which frame shows, when to redraw, the fade, the end) reads the clock alone; only drawing needs the effect's
/// state ([`Sim`]), so a host can keep the clock on one thread and draw elsewhere.
pub struct Clock {
    effect: &'static dyn Effect,
    /// Where cues land: the options' origin and flip.
    opts: Options,
    /// Seconds since the start. Cues, stop and the end follow it.
    t: f64,
    stop_at: Option<f64>,
    cues: Vec<TimedCue>,
    fired: Vec<TimedCue>,
    screen_w: f64,
}

impl Clock {
    pub fn new(effect: &'static dyn Effect, opts: &Options) -> Self {
        Self {
            effect,
            opts: opts.clone(),
            t: 0.0,
            stop_at: None,
            cues: cues_of(effect, opts),
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

    /// Advance by `dt` seconds, at most [`MAX_DT`], firing the cues it passes. A step that fails [`valid_dt`]
    /// (NaN, infinite or negative) is ignored and leaves the clock as it was.
    pub fn update(&mut self, dt: f64) {
        if valid_dt(dt) {
            let dt = js_min(dt, MAX_DT);
            // a step that passes no time (a host's hit-stop) fires nothing: a cue at time 0 would fire again
            if self.stop_at.is_none() && dt > 0.0 {
                self.fire_cues(self.t, self.t + dt);
            }
            self.t += dt;
        }
    }

    /// Record the cues in `(t0, t1]`; a cue at time 0 fires on the first update that passes time (pixelfx
    /// `fire`).
    fn fire_cues(&mut self, t0: f64, t1: f64) {
        let from = if t0 == 0.0 { -1e-9 } else { t0 };
        let meta = self.effect.meta();
        let hit = cues_between(&self.cues, from, t1, meta, &self.opts, self.screen_w);
        self.fired.extend(hit);
    }

    /// The frame the clock is on: `floor(t * fps)`, wrapped round a loop, held at a one-shot's last frame. The
    /// effect's state shows it once brought there ([`Sim::seek`]).
    pub fn frame(&self) -> u64 {
        let n = self.effect.meta().frames();
        let k = self.clock_frame();
        if self.effect.meta().looping {
            k % n
        } else {
            k.min(n - 1)
        }
    }

    /// Frames of `1 / fps` the clock has passed, unwrapped: it changes exactly when anything drawn can change
    /// (the frame, or the fade after [`Clock::stop`]). A host redraws when it changes.
    pub fn clock_frame(&self) -> u64 {
        libm::floor(self.t * f64::from(self.effect.meta().fps) + FRAME_SLACK) as u64
    }

    /// How opaque a frame drawn now is: 1 until [`Clock::stop`], then down to 0 over [`FADE`]. It follows the
    /// clock snapped down to its frame, so it changes once a frame like everything else drawn and does not
    /// depend on when the host draws.
    pub fn fade(&self) -> f64 {
        match self.stop_at {
            None => 1.0,
            Some(s) => {
                let shown_t = self.clock_frame() as f64 / f64::from(self.effect.meta().fps);
                clamp(1.0 - (shown_t - s) / FADE, 0.0, 1.0)
            }
        }
    }

    /// Let it end: loops fade out over [`FADE`], one-shots are cut short. Later calls keep the first stop.
    pub fn stop(&mut self) {
        if self.stop_at.is_none() {
            self.stop_at = Some(self.t);
        }
    }

    /// Whether [`Clock::stop`] was called: from then on it fades out.
    pub fn stopped(&self) -> bool {
        self.stop_at.is_some()
    }

    pub fn done(&self) -> bool {
        let meta = self.effect.meta();
        self.stop_at.is_some_and(|s| self.t - s >= FADE)
            || (!meta.looping && self.t >= meta.duration)
    }

    /// Seconds since the start.
    pub fn t(&self) -> f64 {
        self.t
    }
}

/// What an instance draws: its effect's state, brought to a frame on demand ([`Sim::seek`]). The same effect,
/// options and seed brought to the same frame draw byte-equal frames, whatever frames it was brought to before.
pub struct Sim {
    effect: &'static dyn Effect,
    state: Box<dyn EffectState>,
    opts: Options,
    seed: u64,
    /// Ticks of `1 / fps` the state has taken since it was spawned: the frame it shows.
    tick: u64,
    /// The state's own time: `1 / fps` added once per tick, as the render adds it.
    tick_t: f64,
    /// The character it plays on, handed to the state again whenever it is respawned.
    actor: Option<Arc<Actor>>,
}

impl Sim {
    pub fn new(effect: &'static dyn Effect, opts: Options) -> Self {
        let seed = opts.seed.unwrap_or(effect.meta().seed);
        Self {
            effect,
            state: effect.spawn(&opts, seed),
            opts,
            seed,
            tick: 0,
            tick_t: 0.0,
            actor: None,
        }
    }

    /// The character it plays on from the next step, or none ([`EffectState::set_actor`]). Handing over the same
    /// actor again changes nothing.
    pub fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        let same = match (&self.actor, &actor) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.actor = actor;
            self.state.set_actor(self.actor.clone());
        }
    }

    /// The frame the state shows: ticks of `1 / fps` since it was spawned (a loop's restart from frame 0
    /// respawns it).
    pub fn frame(&self) -> u64 {
        self.tick
    }

    /// Bring the state to frame `k`: tick forward, or respawn and tick from 0 when `k` is behind (a loop that
    /// came round). The same seed gives the same frames each time round.
    pub fn seek(&mut self, k: u64) {
        if k < self.tick {
            self.state = self.effect.spawn(&self.opts, self.seed);
            self.state.set_actor(self.actor.clone());
            self.tick = 0;
            self.tick_t = 0.0;
        }
        let dt = 1.0 / f64::from(self.effect.meta().fps);
        while self.tick < k {
            self.tick_t += dt;
            self.tick += 1;
            // pixelfx `update`: max(1, ceil(dt * 120)) equal sub-steps
            let n = js_max(1.0, libm::ceil(dt * STEPS_PER_SECOND - 1e-9));
            for _ in 0..n as u64 {
                self.state.update(dt / n);
            }
        }
    }

    /// Draw the current frame ([`Sim::frame`]) into `frame`, placed as `place` says and at opacity `fade`
    /// ([`Clock::fade`]).
    pub fn draw(&self, frame: &mut Frame, layer: Layer, place: Placement, fade: f64) {
        let meta = self.effect.meta();
        let mut p = Painter::new(frame);
        p.place(place.x, place.y, place.flip, place.angle);
        p.fade = fade;
        p.light_k = place.light;
        let t = if meta.looping {
            self.tick_t % meta.duration
        } else {
            js_min(self.tick_t, meta.duration)
        };
        if layer != Layer::Front {
            self.state.draw(&mut p, t, Layer::Back);
        }
        if layer != Layer::Back {
            self.state.draw(&mut p, t, Layer::Front);
        }
    }

    /// Where and how its options place it.
    pub fn placement(&self) -> Placement {
        let o = &self.opts;
        Placement {
            x: o.x,
            y: o.y,
            flip: o.flip,
            angle: o.angle,
            light: o.light,
        }
    }
}

/// A live effect: pixelfx `createFx`. Its [`Clock`] and its [`Sim`] kept in step: each update brings the state to
/// the frame the clock is on.
pub struct Instance {
    clock: Clock,
    sim: Sim,
}

impl Instance {
    pub fn new(effect: &'static dyn Effect, opts: Options) -> Self {
        Self {
            clock: Clock::new(effect, &opts),
            sim: Sim::new(effect, opts),
        }
    }

    /// The screen width that sets the stereo pan of fired cues; 0 (the default) leaves them without pan.
    pub fn set_screen_w(&mut self, screen_w: f64) {
        self.clock.set_screen_w(screen_w);
    }

    /// The cues playback has passed since the last call, in world space and time order; clears them.
    pub fn take_cues(&mut self) -> Vec<TimedCue> {
        self.clock.take_cues()
    }

    /// Advance the clock by `dt` seconds ([`Clock::update`]) and bring the state to the frame the clock is on
    /// ([`Instance::frame`]). Shells that report a bad step to their caller check [`valid_dt`] themselves first.
    pub fn update(&mut self, dt: f64) {
        if valid_dt(dt) {
            self.clock.update(dt);
            self.sim.seek(self.clock.frame());
        }
    }

    /// The frame the state shows.
    pub fn frame(&self) -> u64 {
        self.sim.frame()
    }

    /// The character it plays on, or none ([`Sim::set_actor`]).
    pub fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        self.sim.set_actor(actor);
    }

    /// Bring the state to frame `k` ([`Sim::seek`]).
    pub(crate) fn seek(&mut self, k: u64) {
        self.sim.seek(k);
    }

    /// Draw the current frame ([`Instance::frame`]) into `frame`, placed as `place` says, faded by the clock's
    /// time since [`Instance::stop`] ([`Sim::draw`]).
    pub fn draw(&self, frame: &mut Frame, layer: Layer, place: Placement) {
        self.sim.draw(frame, layer, place, self.clock.fade());
    }

    /// Let it end: loops fade out over [`FADE`], one-shots are cut short. Later calls keep the first stop.
    pub fn stop(&mut self) {
        self.clock.stop();
    }

    /// Frames of `1 / fps` the clock has passed, unwrapped ([`Clock::clock_frame`]).
    pub fn clock_frame(&self) -> u64 {
        self.clock.clock_frame()
    }

    /// Whether [`Instance::stop`] was called: from then on it fades out.
    pub fn stopped(&self) -> bool {
        self.clock.stopped()
    }

    pub fn done(&self) -> bool {
        self.clock.done()
    }

    /// Seconds since the instance started.
    pub fn t(&self) -> f64 {
        self.clock.t()
    }

    /// Where and how its options place it.
    pub fn placement(&self) -> Placement {
        self.sim.placement()
    }

    /// Its clock.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Its state.
    pub fn sim(&self) -> &Sim {
        &self.sim
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
