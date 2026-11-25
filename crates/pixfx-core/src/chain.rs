//! Moves: several effects played as one, port of pixelfx `src/core/chain.js` (`chain`). A [`Link`] says which
//! effect, when it starts, when it is cut off, where it plays (a number, or a function of the move's time, so a
//! projectile is an effect whose x moves) and with which options. A move is an ordinary [`Effect`] from the
//! outside: its `spawn` returns a [`Chain`] of its links and its `cues` returns [`chain_cues`], so it plays,
//! recolours, renders and fires cues like any other.
//!
//! ```ignore
//! fn links() -> Vec<Link> {
//!     vec![
//!         Link::new(&SPELL_CHARGE, 0.08).until(THROW).palette(&pal::FIRE),
//!         Link::new(&FIREBALL, THROW).until(HIT).x_by(|t| 28.0 + (t - THROW) * SPEED).y(-22.0),
//!         Link::new(&SMALL_EXPLOSION, HIT).x(TX),
//!     ]
//! }
//! impl Effect for FireballThrow {
//!     fn spawn(&self, _opts: &Options, seed: u64) -> Box<dyn EffectState> {
//!         Box::new(Chain::new(links(), seed))
//!     }
//!     fn cues(&self, opts: &Options) -> Vec<TimedCue> {
//!         chain_cues(&links(), opts, META.duration)
//!     }
//! }
//! ```
//!
//! Each link plays exactly as pixelfx's `createFx` instance inside the chain: created (at time 0) on the first
//! step at or after its start, stepped from the next, drawn at its own time, gone once cut off or, unless it
//! loops, once its duration has passed; link `i` takes seed `seed + i`. Where pixelfx resets the painter for a
//! link, the port keeps the move's fade (a stopped move fades as a whole), multiplies the light strength by the
//! link's `light`, and turns a link's offset with the move's rotation (identical when unrotated). The acted-out
//! character of pixelfx moves (`act`, `READY`, `shiftKeys`, `shiftHits`) is gallery staging and is not ported; a
//! link that wraps an actor gets none and draws its stand-in, as pixelfx's stock actor.

use std::sync::Arc;

use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{js_max, js_min};

use crate::cue::{Cue, TimedCue};
use crate::effect::{Effect, EffectState, Layer, OptValue, Options};

/// Simulation sub-steps per second, as [`crate::effect::Sim`] steps (pixelfx `createFx`'s 120 Hz).
const STEPS_PER_SECOND: f64 = 120.0;

/// Where a link plays along one axis of the move's space: a fixed offset, or a function of the move's time in
/// seconds (pixelfx's `x: (t) => ...`).
#[derive(Clone)]
pub enum Pos {
    At(f64),
    Along(Arc<dyn Fn(f64) -> f64 + Send + Sync>),
}

impl Pos {
    /// The offset at move time `t`.
    pub fn at(&self, t: f64) -> f64 {
        match self {
            Pos::At(v) => *v,
            Pos::Along(f) => f(t),
        }
    }
}

impl From<f64> for Pos {
    fn from(v: f64) -> Self {
        Pos::At(v)
    }
}

/// One effect in a move (a pixelfx chain cue: `{ fx, t, until, x, y, flip, ...options }`).
#[derive(Clone)]
pub struct Link {
    pub fx: &'static dyn Effect,
    /// Move time it starts at, seconds.
    pub t: f64,
    /// Move time it is cut off at (`f64::INFINITY`: when it ends by itself).
    pub until: f64,
    /// Its origin in the move's space (local px; x mirrors with the move).
    pub x: Pos,
    pub y: Pos,
    /// Mirror it within the move.
    pub flip: bool,
    /// Its own options: palette, light and named options. Its placement and seed come from the chain.
    pub opts: Options,
}

impl Link {
    /// `fx` starting at move time `t`, at the move's origin, with its default options.
    pub fn new(fx: &'static dyn Effect, t: f64) -> Self {
        Self {
            fx,
            t,
            until: f64::INFINITY,
            x: Pos::At(0.0),
            y: Pos::At(0.0),
            flip: false,
            opts: Options::default(),
        }
    }

    /// Cut it off at move time `t`.
    pub fn until(mut self, t: f64) -> Self {
        self.until = t;
        self
    }

    pub fn x(mut self, x: f64) -> Self {
        self.x = Pos::At(x);
        self
    }

    pub fn y(mut self, y: f64) -> Self {
        self.y = Pos::At(y);
        self
    }

    /// x as a function of the move's time (a projectile's flight).
    pub fn x_by(mut self, f: impl Fn(f64) -> f64 + Send + Sync + 'static) -> Self {
        self.x = Pos::Along(Arc::new(f));
        self
    }

    /// y as a function of the move's time.
    pub fn y_by(mut self, f: impl Fn(f64) -> f64 + Send + Sync + 'static) -> Self {
        self.y = Pos::Along(Arc::new(f));
        self
    }

    pub fn flip(mut self) -> Self {
        self.flip = true;
        self
    }

    pub fn palette(mut self, ramp: &[Rgba]) -> Self {
        self.opts.palette = Some(ramp.to_vec());
        self
    }

    /// A number option of the linked effect (pixelfx `{ height: 28 }`).
    pub fn num(mut self, name: &str, v: f64) -> Self {
        self.opts.extra.insert(name.to_owned(), OptValue::Num(v));
        self
    }

    /// A colour option of the linked effect.
    pub fn color(mut self, name: &str, c: Rgba) -> Self {
        self.opts.extra.insert(name.to_owned(), OptValue::Color(c));
        self
    }

    /// A ramp option of the linked effect.
    pub fn ramp(mut self, name: &str, r: &[Rgba]) -> Self {
        self.opts
            .extra
            .insert(name.to_owned(), OptValue::Ramp(r.to_vec()));
        self
    }

    /// The options link `i` of a move seeded `seed` spawns with.
    fn spawn_opts(&self, seed: u64) -> Options {
        Options {
            seed: Some(seed),
            ..self.opts.clone()
        }
    }
}

/// One link while the move plays: its spawned state (none before it starts), its own time, whether it is over.
struct Live {
    link: Link,
    seed: u64,
    state: Option<Box<dyn EffectState>>,
    t: f64,
    over: bool,
}

/// The state of a move: its links and its own time (pixelfx `chain(cues)(o)`).
pub struct Chain {
    list: Vec<Live>,
    t: f64,
}

impl Chain {
    /// The links of a move spawned with `seed`; link `i` takes seed `seed + i` (pixelfx `(o.seed ?? 1) + i`).
    pub fn new(links: Vec<Link>, seed: u64) -> Self {
        let list = links
            .into_iter()
            .enumerate()
            .map(|(i, link)| Live {
                link,
                seed: seed.wrapping_add(i as u64),
                state: None,
                t: 0.0,
                over: false,
            })
            .collect();
        Self { list, t: 0.0 }
    }

    /// The move's own time: the sum of its steps.
    pub fn t(&self) -> f64 {
        self.t
    }
}

impl EffectState for Chain {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        for c in &mut self.list {
            if c.over || t < c.link.t {
                continue;
            }
            let fx = c.link.fx;
            let meta = fx.meta();
            match &mut c.state {
                None => c.state = Some(fx.spawn(&c.link.spawn_opts(c.seed), c.seed)),
                Some(s) => {
                    // pixelfx `createFx.update`: max(1, ceil(dt * 120)) equal sub-steps
                    c.t += dt;
                    let n = js_max(1.0, libm::ceil(dt * STEPS_PER_SECOND - 1e-9));
                    for _ in 0..n as u64 {
                        s.update(dt / n);
                    }
                }
            }
            if t >= c.link.until || (!meta.looping && c.t >= meta.duration) {
                c.over = true;
            }
        }
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let saved = p.save();
        for c in &self.list {
            let Some(s) = &c.state else {
                continue;
            };
            if c.over {
                continue;
            }
            let meta = c.link.fx.meta();
            let tt = if meta.looping {
                c.t % meta.duration
            } else {
                js_min(c.t, meta.duration)
            };
            p.nest(c.link.x.at(self.t), c.link.y.at(self.t), c.link.flip);
            p.light_k *= c.link.opts.light;
            s.draw(p, tt, layer);
            // each one placed the painter for itself: put it back for whoever draws next
            p.restore(saved);
        }
    }
}

/// The cues of a move `duration` seconds long made of `links`, in the move's space and time: each link's own
/// cues ([`Effect::cues`] with its options) moved to where and when it plays, a looping link's repeated while it
/// plays, none at or after a link is cut off. `opts` are the move's options; its seed seeds the links as
/// [`Chain::new`] does.
pub fn chain_cues(links: &[Link], opts: &Options, duration: f64) -> Vec<TimedCue> {
    let seed = opts.seed.unwrap_or(1);
    let mut out = Vec::new();
    for (i, link) in links.iter().enumerate() {
        let meta = link.fx.meta();
        let end = js_min(link.until, duration);
        let sx = if link.flip { -1.0 } else { 1.0 };
        let own = link.fx.cues(&link.spawn_opts(seed.wrapping_add(i as u64)));
        let mut round = 0.0;
        loop {
            let start = link.t + round * meta.duration;
            if start >= end {
                break;
            }
            for c in &own {
                if c.t < 0.0 || (!meta.looping && c.t > meta.duration) {
                    continue;
                }
                let at = start + c.t;
                if at >= end {
                    continue;
                }
                let (lx, ly) = (link.x.at(at), link.y.at(at));
                let mut w = *c;
                w.t = at;
                match &mut w.cue {
                    Cue::Ripple { x, y, .. } => {
                        *x = lx + sx * *x;
                        *y += ly;
                    }
                    Cue::Sound { x, .. } => *x = Some(lx + sx * x.unwrap_or(0.0)),
                    _ => {}
                }
                out.push(w);
            }
            if !meta.looping || meta.duration <= 0.0 {
                break;
            }
            round += 1.0;
        }
    }
    out.sort_by(|a, b| a.t.total_cmp(&b.t));
    out
}
