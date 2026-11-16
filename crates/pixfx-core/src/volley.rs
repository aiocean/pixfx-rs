//! An effect that takes an order: a game tells a playing instance to attack toward a point ([`Order`]), and the
//! effect answers with a volley ([`Launch`] each): other effects the scene plays and flies, turned to face where
//! they go, along a [`Path`] onto the line through the point and on past it. A piece strikes what it meets on
//! the way: a body the game declared (`pixfx_scene::Scene::set_body`), or what the game says it hit
//! (`pixfx_scene::Scene::burst`); a volley that `lands` (a thrown thing) also strikes the point itself. The
//! effect decides the volley from where it stands and its time; the scene carries it out (`pixfx_scene::Scene::order`),
//! so every shell gets the same flight.

use crate::effect::{Effect, Options};
use pixfx_draw::math::{clamp, rand};

/// How a volley flies toward its point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Aim {
    /// Every piece curves onto the line through the point and flies on along it, bursting on the first body it
    /// meets.
    Converge,
    /// Every piece flies straight at the point and on past it, through every body it meets, striking each once.
    Pierce,
}

impl Aim {
    /// `"converge"` or `"pierce"`; none for anything else.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "converge" => Some(Self::Converge),
            "pierce" => Some(Self::Pierce),
            _ => None,
        }
    }
}

/// Attack toward (`x`, `y`), in game pixels on the stage, as `aim` says; the body `caster` (the one attacking,
/// if it has one) is never struck by its own volley.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Order {
    pub x: f64,
    pub y: f64,
    pub aim: Aim,
    pub caster: Option<u32>,
}

/// A quadratic curve in stage pixels: from `from`, bent toward `via`, to `to`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Path {
    pub from: (f64, f64),
    pub via: (f64, f64),
    pub to: (f64, f64),
}

impl Path {
    /// A straight path.
    pub fn line(from: (f64, f64), to: (f64, f64)) -> Self {
        Self {
            from,
            via: ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0),
            to,
        }
    }

    /// The point `u` (0 to 1) along it.
    pub fn at(&self, u: f64) -> (f64, f64) {
        let u = clamp(u, 0.0, 1.0);
        let v = 1.0 - u;
        (
            v * v * self.from.0 + 2.0 * v * u * self.via.0 + u * u * self.to.0,
            v * v * self.from.1 + 2.0 * v * u * self.via.1 + u * u * self.to.1,
        )
    }

    /// The way it heads at `u`, radians (0 along +x, y down); its overall way where it has none there.
    pub fn heading(&self, u: f64) -> f64 {
        let u = clamp(u, 0.0, 1.0);
        let dx = 2.0 * (1.0 - u) * (self.via.0 - self.from.0) + 2.0 * u * (self.to.0 - self.via.0);
        let dy = 2.0 * (1.0 - u) * (self.via.1 - self.from.1) + 2.0 * u * (self.to.1 - self.via.1);
        if dx * dx + dy * dy > 1e-12 {
            libm::atan2(dy, dx)
        } else {
            libm::atan2(self.to.1 - self.from.1, self.to.0 - self.from.0)
        }
    }

    /// Its length in pixels, near enough (`PIECES` straight pieces).
    pub fn length(&self) -> f64 {
        self.u_at(f64::INFINITY).1
    }

    /// The `u` (0 to 1) `dist` pixels along it, near enough (`PIECES` straight pieces), and its whole length.
    pub fn u_at(&self, dist: f64) -> (f64, f64) {
        const PIECES: u32 = 32;
        let mut len = 0.0;
        let mut last = self.from;
        let mut found = None;
        for i in 1..=PIECES {
            let u = f64::from(i) / f64::from(PIECES);
            let p = self.at(u);
            let step = libm::hypot(p.0 - last.0, p.1 - last.1);
            if found.is_none() && len + step >= dist {
                let k = if step > 1e-12 {
                    (dist - len) / step
                } else {
                    0.0
                };
                found = Some((f64::from(i - 1) + clamp(k, 0.0, 1.0)) / f64::from(PIECES));
            }
            len += step;
            last = p;
        }
        (found.unwrap_or(1.0), len)
    }
}

/// One piece of a volley. While it waits `after` seconds, the scene shows `held` (else `effect`) where `path` starts,
/// turning from `rest` (the way the piece pointed in the effect that launched it) to its path; then it plays
/// `effect` with `opts` there, with `tail` trailing it (`pixfx_scene::Scene::trail`), and flies it along the
/// path and `on` pixels straight on past its end in `secs` (`u = s^ease` of the whole way at time share `s`).
/// Its tip, `nose` pixels ahead of its origin, strikes: a body it meets (it goes then, unless it flies `through`
/// them), and the point `mark` pixels along the way, for a piece that lands there. Where it strikes the scene sends
/// a strike event and plays `hit` (with `hit_opts`) turned to its heading. A piece that strikes nothing goes at
/// the end of its way.
#[derive(Clone)]
pub struct Launch {
    pub effect: &'static dyn Effect,
    pub opts: Options,
    pub held: Option<&'static dyn Effect>,
    /// What trails it while it flies (`pixfx_scene::Scene::trail`), with its options.
    pub tail: Option<(&'static dyn Effect, Options)>,
    pub rest: f64,
    pub after: f64,
    pub path: Path,
    pub on: f64,
    pub secs: f64,
    pub ease: f64,
    pub nose: f64,
    pub through: bool,
    pub mark: Option<f64>,
    pub hit: Option<&'static dyn Effect>,
    pub hit_opts: Options,
}

impl Launch {
    /// The length of its path, before it flies straight on.
    pub fn reach(&self) -> f64 {
        self.path.length()
    }

    /// Where its origin is and which way it heads `dist` pixels along its way: along the path, then straight on
    /// the way the path ends.
    pub fn point(&self, dist: f64) -> ((f64, f64), f64) {
        let reach = self.reach();
        if dist <= reach && reach > 1e-9 {
            let (u, _) = self.path.u_at(dist.max(0.0));
            return (self.path.at(u), self.path.heading(u));
        }
        let a = self.path.heading(1.0);
        let more = dist - reach;
        (
            (
                self.path.to.0 + libm::cos(a) * more,
                self.path.to.1 + libm::sin(a) * more,
            ),
            a,
        )
    }

    /// How far along its way it is at time share `s` (0 to 1) of its flight.
    pub fn along(&self, s: f64) -> f64 {
        libm::pow(clamp(s, 0.0, 1.0), self.ease.max(0.01)) * (self.reach() + self.on)
    }

    /// Where its origin is and which way it heads at time share `s` of its flight.
    pub fn at(&self, s: f64) -> ((f64, f64), f64) {
        self.point(self.along(s))
    }

    /// Where its tip is when its origin is at `at` heading `heading`.
    pub fn tip(&self, at: (f64, f64), heading: f64) -> (f64, f64) {
        (
            at.0 + libm::cos(heading) * self.nose,
            at.1 + libm::sin(heading) * self.nose,
        )
    }
}

/// What trails a volley's pieces: `effect` played with the options `opts` gives.
#[derive(Clone, Copy)]
pub struct Tail {
    pub effect: &'static dyn Effect,
    pub opts: fn() -> Options,
}

/// Where one shot of a volley starts, in stage pixels, and the way it points (radians, 0 along +x, y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub at: (f64, f64),
    pub heading: f64,
}

/// How a volley of like pieces flies at an order, whatever the pieces are (swords, talismans, arrows, fireballs):
/// the effect that owns them says where each piece is ([`Shot`]); [`Volley::aim`] turns them into launches.
/// Every piece turns to its way over `poise` seconds; the one nearest the point goes first, the rest `gap` seconds
/// apart. [`Aim::Converge`]: each flies out the way it points, then curves onto the point (spread by up to
/// `spread` pixels), easing in by `ease`, so its tip (`nose` pixels ahead of its origin) would sink `sink`
/// pixels into it, and flies on `beyond` pixels past. [`Aim::Pierce`]: each flies straight at the point and
/// `beyond` pixels on. A volley that `lands` strikes the point itself (a thrown thing); one that does not strikes
/// only what it meets.
#[derive(Clone, Copy)]
pub struct Volley {
    /// What flies, drawn pointing along +x from its origin, and what stands in its place while it waits to leave
    /// (none: the piece itself, still).
    pub piece: &'static dyn Effect,
    pub held: Option<&'static dyn Effect>,
    /// What trails each piece in flight (an effect that [`Effect::traces`], as `projectiles/trail`), or none.
    pub tail: Option<Tail>,
    /// What plays where a piece strikes, turned to its heading.
    pub hit: Option<&'static dyn Effect>,
    /// Seconds every shot takes to turn to its path before the first leaves, and between launches.
    pub poise: f64,
    pub gap: f64,
    /// Pixels a second, and the shortest and longest flight in seconds.
    pub speed: f64,
    pub min_secs: f64,
    pub max_secs: f64,
    /// How far ahead of its origin a piece strikes with, and how deep it sinks into a converging point.
    pub nose: f64,
    pub sink: f64,
    /// How far a converging piece first flies out the way it points: this share of the distance, held to
    /// `bend_min`..`bend_max` pixels.
    pub bend: f64,
    pub bend_min: f64,
    pub bend_max: f64,
    /// How far apart converging pieces aim, across (`x`) and up (`y`), in pixels.
    pub spread: (f64, f64),
    /// `u = s^ease` of a converging flight: above 1 it starts slow and lands fast.
    pub ease: f64,
    /// How far a piece flies on past the point.
    pub beyond: f64,
    /// It strikes the point itself, and a converging piece goes there.
    pub lands: bool,
}

impl Volley {
    /// The launches that carry out `order` with `shots`, seeded by `seed` (each piece's spread).
    pub fn aim(&self, shots: &[Shot], order: &Order, seed: f64) -> Vec<Launch> {
        let target = (order.x, order.y);
        let dist = |p: (f64, f64)| libm::hypot(target.0 - p.0, target.1 - p.1);
        let mut turn: Vec<(usize, Shot)> = shots.iter().copied().enumerate().collect();
        turn.sort_by(|a, b| dist(a.1.at).total_cmp(&dist(b.1.at)).then(a.0.cmp(&b.0)));
        turn.into_iter()
            .enumerate()
            .map(|(n, (k, p))| {
                let after = self.poise + n as f64 * self.gap;
                match order.aim {
                    Aim::Converge => self.converge(p, target, k as f64, seed, after),
                    Aim::Pierce => self.pierce(p, target, after),
                }
            })
            .collect()
    }

    /// A launch along `path` and `on` pixels on, striking the point `mark` along it if it lands there.
    #[allow(clippy::too_many_arguments)]
    fn launch(
        &self,
        rest: f64,
        path: Path,
        on: f64,
        after: f64,
        ease: f64,
        through: bool,
        mark: Option<f64>,
    ) -> Launch {
        let len = path.length() + on;
        Launch {
            effect: self.piece,
            opts: Options::default(),
            held: self.held,
            tail: self.tail.map(|t| (t.effect, (t.opts)())),
            rest,
            after,
            secs: clamp(len / self.speed.max(1e-6), self.min_secs, self.max_secs),
            path,
            on,
            ease,
            nose: self.nose,
            through,
            mark: mark.filter(|_| self.lands),
            hit: self.hit,
            hit_opts: Options::default(),
        }
    }

    fn converge(&self, p: Shot, target: (f64, f64), k: f64, seed: f64, after: f64) -> Launch {
        let hit = (
            target.0 + (rand(&[k, seed, 3.0]) - 0.5) * self.spread.0,
            target.1 + (rand(&[k, seed, 5.0]) - 0.5) * self.spread.1,
        );
        let from = p.at;
        let d = libm::hypot(hit.0 - from.0, hit.1 - from.1).max(1.0);
        // out the way it points first, then curving in
        let reach = clamp(d * self.bend, self.bend_min, self.bend_max);
        let via = (
            from.0 + libm::cos(p.heading) * reach,
            from.1 + libm::sin(p.heading) * reach,
        );
        let (dx, dy) = (hit.0 - via.0, hit.1 - via.1);
        let len = libm::hypot(dx, dy).max(1e-6);
        // its origin ends the curve short of the point by its nose, sunk in
        let short = self.nose - self.sink;
        let to = (hit.0 - dx / len * short, hit.1 - dy / len * short);
        let path = Path { from, via, to };
        let on = if self.lands { 0.0 } else { self.beyond };
        // where its tip reaches the point, before it sinks in
        let mark = (path.length() - self.sink).max(0.0);
        self.launch(p.heading, path, on, after, self.ease, false, Some(mark))
    }

    fn pierce(&self, p: Shot, target: (f64, f64), after: f64) -> Launch {
        let from = p.at;
        let d = libm::hypot(target.0 - from.0, target.1 - from.1).max(1e-6);
        // its tip on the point at the path's end
        let short = (d - self.nose).max(0.0);
        let to = (
            from.0 + (target.0 - from.0) / d * short,
            from.1 + (target.1 - from.1) / d * short,
        );
        self.launch(
            p.heading,
            Path::line(from, to),
            self.beyond,
            after,
            1.0,
            true,
            Some(short),
        )
    }
}
