//! `explosions/body-burst`: the character it wraps blows apart into a handful of big chunks of its own sprite, flung
//! out from the body's middle. Each chunk cracks into smaller ones after a moment or when it hits the floor, and those
//! crack again, down to single pixels that bounce and heap up as dust on the floor; then the heap crumbles away. Made here, not ported: no source to check parity against; its golden
//! hashes pin it.
//!
//! It wraps its actor ([`Effect::wraps_actor`]) and draws the body itself from its first frame, so the game hides
//! the character's own sprite when it plays this (the burst takes the character's place). It reads the sprite as it
//! shows up to the burst and ignores later frames: what flies is the pose the character died in. With no actor it
//! blows apart a stand-in body (head, trunk, legs).
//!
//! The burst asks the game for shake, hit-stop, ripple and flash ([`impact`]); `feel` scales them, 0 for none.

use std::sync::Arc;

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{
    Actor, Area, Canvas, Effect, EffectState, Layer, Meta, NumOpt, Options, Silhouette,
};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::{Rng, TAU, clamp, smooth, to_uint32};

pub struct BodyBurst;

pub static BODY_BURST: BodyBurst = BodyBurst;

const META: Meta = Meta {
    id: "explosions/body-burst",
    name: "Body burst",
    desc: "The character blows apart: it swells white, bursts into big chunks of its own sprite that crack smaller and smaller as they fly and hit the floor, down to pixels that heap up as dust, then crumble away. The game hides its sprite while this plays.",
    tags: &[
        "explosion",
        "death",
        "gib",
        "disintegrate",
        "dust",
        "character",
        "impact",
        "one-shot",
    ],
    // `canvas` sizes it from `w` and `h`; this is their default
    size: (192, 144),
    origin: (96, 128),
    duration: 3.0,
    looping: false,
    fps: 60,
    seed: 1,
    state: None,
    options: &[FORCE.spec(), DUST.spec(), W.spec(), H.spec(), FEEL.spec()],
};

/// How hard the body is blown apart: the chunks' speed.
const FORCE: NumOpt = NumOpt::new("force", 1.0).within(0.25, 2.0);
/// The share of the chunks that crumble where they stand instead of flying.
const DUST: NumOpt = NumOpt::new("dust", 0.3).within(0.0, 1.0);
/// The box the pieces fly in, the feet `FLOOR_ROOM` px above its bottom middle: a piece leaving it is gone.
const W: NumOpt = NumOpt::new("w", 192.0).within(32.0, 512.0);
const H: NumOpt = NumOpt::new("h", 144.0).within(32.0, 512.0);
const FLOOR_ROOM: u32 = 16;

/// The beats, in seconds: the swell before the burst, the burst, the heap starting to crumble, all gone.
const BURST: f64 = 0.14;
const CRUMBLE: f64 = 2.0;
const GONE: f64 = 2.85;
/// Pixels per second squared.
const GRAVITY: f64 = 520.0;
/// A bit landing slower than this (px/s) stops bouncing.
const REST_SPEED: f64 = 40.0;
/// About how many body pixels make one chunk at the burst, and how few and how many chunks there are.
const CHUNK_CELLS: usize = 45;
const FIRST_CHUNKS: (usize, usize) = (4, 10);
/// A chunk landing faster than this (px/s) cracks on the floor; slower (down to `REST_SPEED`), it only bounces.
const CRACK_SPEED: f64 = 90.0;
/// A chunk this small breaks into loose pixels instead of smaller chunks.
const BIT_CELLS: usize = 4;
/// How crooked the cracks run: noise added to each squared distance when cutting.
const RAGGED: f64 = 8.0;
/// How long after the burst a piece stays white-hot.
const HEAT: f64 = 0.1;

/// The box the stand-in body is drawn in, the feet at its bottom middle.
const STAND_IN: Area = Area {
    x: -12,
    y: -44,
    w: 24,
    h: 44,
};
const SKIN: Rgba = Rgba::hex24(0xe8b796);
const TUNIC: Rgba = Rgba::hex24(0x3d6fb6);
const BOOTS: Rgba = Rgba::hex24(0x4a3426);
const HOT: Rgba = Rgba::hex24(0xfff1c2);
const FLASH: Rgba = Rgba::hex24(0xffd37a);

impl Effect for BodyBurst {
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
        impact(BURST, 0.8, 0.0, -20.0, opts)
    }

    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let canvas = self.canvas(opts);
        let w = canvas.size.0;
        let left = -canvas.origin.0;
        Box::new(State {
            force: opts.num(&FORCE),
            dust: opts.num(&DUST),
            rng: Rng::new(to_uint32(seed as f64)),
            body: body_of(None),
            mid: middle(&body_of(None)),
            chunks: Vec::new(),
            bits: Vec::new(),
            heap: vec![0; w as usize],
            left,
            top: -canvas.origin.1,
            right: left + w as i32,
            t: 0.0,
            burst: false,
        })
    }
}

/// A piece of the body flying in one block: the body's own cells, moved together by an offset.
struct Chunk {
    /// The cells as the body had them: position and colour.
    cells: Vec<(i32, i32, Rgba)>,
    /// How far the piece has moved from where it sat in the body.
    dx: f64,
    dy: f64,
    vx: f64,
    vy: f64,
    /// When it breaks into smaller pieces, in seconds, unless the floor breaks it first.
    breaks: f64,
}

/// One pixel of the body in flight or at rest: what a piece ends as.
struct Bit {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    c: Rgba,
    /// Lying in the heap.
    rest: bool,
    /// When it crumbles away, in seconds.
    gone: f64,
}

struct State {
    force: f64,
    dust: f64,
    rng: Rng,
    /// The body's pixels as the sprite shows them now, until the burst: cell and colour. Read in `set_actor`.
    body: Vec<(i32, i32, Rgba)>,
    /// The body's middle, where the burst comes from.
    mid: (f64, f64),
    chunks: Vec<Chunk>,
    bits: Vec<Bit>,
    /// How many bits lie in each column of the canvas, left to right.
    heap: Vec<u16>,
    /// The canvas's edges in the effect's space: left, top and right.
    left: i32,
    top: i32,
    right: i32,
    t: f64,
    burst: bool,
}

/// The pixels of the sprite, or of the stand-in body when there is none.
fn body_of(actor: Option<&Actor>) -> Vec<(i32, i32, Rgba)> {
    let sprite: Vec<_> = actor.map(|a| a.opaque().collect()).unwrap_or_default();
    if !sprite.is_empty() {
        return sprite;
    }
    let stand_in = Silhouette::from_fn(STAND_IN, |px, py| {
        let head = libm::hypot(px, py + 37.0) < 5.5;
        let trunk = libm::fabs(px) < 5.0 && py > -31.0 && py < -13.0;
        let legs = (-13.0..0.0).contains(&py) && libm::fabs(libm::fabs(px) - 2.5) < 1.8;
        head || trunk || legs
    });
    stand_in
        .cells()
        .map(|(x, y)| {
            let c = match y {
                ..-31 => SKIN,
                -31..-4 => TUNIC,
                _ => BOOTS,
            };
            (x, y, c)
        })
        .collect()
}

fn middle(body: &[(i32, i32, Rgba)]) -> (f64, f64) {
    if body.is_empty() {
        return (0.0, -20.0);
    }
    let n = body.len() as f64;
    let (sx, sy) = body.iter().fold((0.0, 0.0), |(sx, sy), &(x, y, _)| {
        (sx + f64::from(x) + 0.5, sy + f64::from(y) + 0.5)
    });
    (sx / n, sy / n)
}

/// Cut `cells` into `n` ragged pieces: each cell goes to the nearest of `n` cells picked at random, the distances
/// jittered so the cracks run crooked. Empty pieces are left out.
fn cut(cells: &[(i32, i32, Rgba)], n: usize, rng: &mut Rng) -> Vec<Vec<(i32, i32, Rgba)>> {
    let last = cells.len().saturating_sub(1) as f64;
    let seeds: Vec<(f64, f64)> = (0..n)
        .filter_map(|_| cells.get(rng.int(0.0, last) as usize))
        .map(|&(x, y, _)| (f64::from(x), f64::from(y)))
        .collect();
    let mut pieces = vec![Vec::new(); seeds.len()];
    for &(x, y, c) in cells {
        let (px, py) = (f64::from(x), f64::from(y));
        let nearest = seeds
            .iter()
            .enumerate()
            .map(|(i, &(sx, sy))| {
                let d = (px - sx) * (px - sx) + (py - sy) * (py - sy);
                (i, d + rng.range(0.0, RAGGED))
            })
            .fold((0, f64::INFINITY), |a, b| if b.1 < a.1 { b } else { a })
            .0;
        if let Some(piece) = pieces.get_mut(nearest) {
            piece.push((x, y, c));
        }
    }
    pieces.retain(|p| !p.is_empty());
    pieces
}

impl State {
    /// Break the body into a handful of big chunks flying out from the middle.
    fn blow_apart(&mut self) {
        let (mx, my) = self.mid;
        // the body's reach from its middle, so the outer chunks fly fastest
        let reach = self
            .body
            .iter()
            .map(|&(x, y, _)| libm::hypot(f64::from(x) + 0.5 - mx, f64::from(y) + 0.5 - my))
            .fold(1.0, f64::max);
        let n = (self.body.len() / CHUNK_CELLS).clamp(FIRST_CHUNKS.0, FIRST_CHUNKS.1);
        let pieces = cut(&self.body, n, &mut self.rng);
        self.chunks = pieces
            .into_iter()
            .map(|cells| {
                let (cx, cy) = middle(&cells);
                let (dx, dy) = (cx - mx, cy - my);
                let d = libm::hypot(dx, dy);
                let a = if d > 0.01 {
                    libm::atan2(dy, dx)
                } else {
                    self.rng.range(0.0, TAU)
                } + self.rng.range(-0.7, 0.7);
                let crumbles = self.rng.chance(self.dust);
                let out = 0.35 + 0.65 * clamp(d / reach, 0.0, 1.0);
                let speed = self.force
                    * self.rng.range(120.0, 230.0)
                    * out
                    * if crumbles { 0.12 } else { 1.0 };
                let lift = if crumbles {
                    0.0
                } else {
                    self.force * self.rng.range(60.0, 140.0)
                };
                let breaks = BURST
                    + if crumbles {
                        self.rng.range(0.1, 0.2)
                    } else {
                        self.rng.range(0.14, 0.24)
                    };
                Chunk {
                    cells,
                    dx: 0.0,
                    dy: 0.0,
                    vx: libm::cos(a) * speed,
                    vy: libm::sin(a) * speed - lift,
                    breaks,
                }
            })
            .collect();
    }

    /// Break a chunk into smaller ones kicked apart from its middle, or into loose bits once it is small or `crumbles`.
    fn shatter(&mut self, c: Chunk, crumbles: bool) {
        if crumbles || c.cells.len() <= BIT_CELLS {
            for &(x, y, col) in &c.cells {
                let (vx, vy) = (
                    c.vx + self.force * self.rng.range(-35.0, 35.0),
                    c.vy + self.force * self.rng.range(-35.0, 15.0),
                );
                let gone = self.rng.range(CRUMBLE, GONE);
                self.bits.push(Bit {
                    x: f64::from(x) + 0.5 + c.dx,
                    y: f64::from(y) + 0.5 + c.dy,
                    vx,
                    vy,
                    c: col,
                    rest: false,
                    gone,
                });
            }
            return;
        }
        let n = if c.cells.len() > 30 { 3 } else { 2 };
        let (mx, my) = middle(&c.cells);
        for cells in cut(&c.cells, n, &mut self.rng) {
            let (cx, cy) = middle(&cells);
            let (dx, dy) = (cx - mx, cy - my);
            let d = libm::hypot(dx, dy);
            let a = if d > 0.01 {
                libm::atan2(dy, dx)
            } else {
                self.rng.range(0.0, TAU)
            };
            let kick = self.force * self.rng.range(20.0, 55.0);
            let breaks = self.t + self.rng.range(0.08, 0.18);
            self.chunks.push(Chunk {
                cells,
                dx: c.dx,
                dy: c.dy,
                vx: c.vx + libm::cos(a) * kick,
                vy: c.vy + libm::sin(a) * kick,
                breaks,
            });
        }
    }

    /// Move the chunks; a chunk breaks when its time comes or when it hits the floor.
    fn step_chunks(&mut self, dt: f64) {
        let (left, right, top) = (
            f64::from(self.left),
            f64::from(self.right),
            f64::from(self.top),
        );
        let chunks = std::mem::take(&mut self.chunks);
        let mut breaking = Vec::new();
        for mut c in chunks {
            c.vy += GRAVITY * dt;
            c.dx += c.vx * dt;
            c.dy += c.vy * dt;
            let (cx, cy) = middle(&c.cells);
            let (x, y) = (cx + c.dx, cy + c.dy);
            if x < left || x >= right || y < top - 64.0 {
                // left the box: gone for good
                continue;
            }
            // how far its lowest cell sinks into the floor or the heap
            let sink = c
                .cells
                .iter()
                .filter_map(|&(cx, cy, _)| {
                    let col = self.column(f64::from(cx) + 0.5 + c.dx)?;
                    Some(f64::from(cy) + 0.5 + c.dy - self.floor_at(col))
                })
                .fold(f64::NEG_INFINITY, f64::max);
            if c.vy > 0.0 && sink >= 0.0 {
                // it lands: hard, it cracks and the pieces bounce off; barely moving, it lies down and
                // crumbles to dust; in between, it bounces whole
                c.dy -= sink;
                let (hard, lies) = (c.vy > CRACK_SPEED, c.vy < REST_SPEED);
                c.vy = -c.vy * self.rng.range(0.25, 0.45);
                c.vx *= self.rng.range(0.5, 0.8);
                if lies {
                    breaking.push((c, true));
                } else if hard || self.t >= c.breaks {
                    breaking.push((c, false));
                } else {
                    self.chunks.push(c);
                }
            } else if self.t >= c.breaks {
                breaking.push((c, false));
            } else {
                self.chunks.push(c);
            }
        }
        for (c, crumbles) in breaking {
            self.shatter(c, crumbles);
        }
    }

    /// The top of the heap in the column holding `x`: where a bit landing there comes to rest (the cell's top).
    fn floor_at(&self, col: usize) -> f64 {
        -1.0 - f64::from(self.heap.get(col).copied().unwrap_or(0))
    }

    fn column(&self, x: f64) -> Option<usize> {
        let col = libm::floor(x) as i64 - i64::from(self.left);
        usize::try_from(col).ok().filter(|&c| c < self.heap.len())
    }

    /// Lay a bit in the heap at `col`, rolling it down to a lower neighbour as sand does.
    fn settle(&mut self, i: usize, mut col: usize) {
        for _ in 0..8 {
            let here = self.heap.get(col).copied().unwrap_or(0);
            let lower = |c: Option<usize>| {
                c.and_then(|c| self.heap.get(c).map(|&h| (c, h)))
                    .filter(|&(_, h)| h + 1 < here)
            };
            let left = lower(col.checked_sub(1));
            let right = lower(Some(col + 1));
            match (left, right) {
                (Some((l, hl)), Some((r, hr))) => col = if hl <= hr { l } else { r },
                (Some((l, _)), None) => col = l,
                (None, Some((r, _))) => col = r,
                (None, None) => break,
            }
        }
        let y = self.floor_at(col);
        if let Some(h) = self.heap.get_mut(col) {
            *h = h.saturating_add(1);
        }
        let x = f64::from(self.left) + col as f64 + 0.5;
        if let Some(b) = self.bits.get_mut(i) {
            b.x = x;
            b.y = y;
            b.vx = 0.0;
            b.vy = 0.0;
            b.rest = true;
        }
    }

    fn step(&mut self, dt: f64) {
        self.t += dt;
        if !self.burst {
            if self.t < BURST {
                return;
            }
            self.burst = true;
            self.blow_apart();
        }
        self.step_chunks(dt);
        let (left, right, top) = (
            f64::from(self.left),
            f64::from(self.right),
            f64::from(self.top),
        );
        let mut landed = Vec::new();
        for (i, b) in self.bits.iter_mut().enumerate() {
            if b.rest {
                continue;
            }
            b.vy += GRAVITY * dt;
            b.x += b.vx * dt;
            b.y += b.vy * dt;
            if b.x < left || b.x >= right || b.y < top - 64.0 {
                // left the box: gone for good
                b.rest = true;
                b.gone = 0.0;
                continue;
            }
            if b.vy > 0.0 {
                landed.push(i);
            }
        }
        for i in landed {
            let Some(b) = self.bits.get(i) else {
                continue;
            };
            let Some(col) = self.column(b.x) else {
                continue;
            };
            let floor = self.floor_at(col);
            if b.y < floor {
                continue;
            }
            if b.vy < REST_SPEED {
                self.settle(i, col);
                continue;
            }
            let bounce = self.rng.range(0.25, 0.45);
            let slide = self.rng.range(0.5, 0.8);
            if let Some(b) = self.bits.get_mut(i) {
                b.y = floor;
                b.vy = -b.vy * bounce;
                b.vx *= slide;
            }
        }
    }

    fn draw_body(&self, p: &mut Painter) {
        let swell = smooth(0.0, BURST, self.t);
        // a one-pixel tremble, faster as it swells
        let shake = if (libm::floor(self.t * 60.0) as i64) % 2 == 0 {
            0.0
        } else {
            1.0
        } * swell;
        for &(x, y, c) in &self.body {
            p.px(
                f64::from(x) + libm::floor(shake),
                f64::from(y),
                mix(c, WHITE, swell * 0.6),
            );
        }
        let (mx, my) = self.mid;
        p.glow(mx, my, 10.0 + 14.0 * swell, FLASH, 0.5 * swell);
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.step(dt);
    }

    fn set_actor(&mut self, actor: Option<Arc<Actor>>) {
        // what flies is the pose the character had at the burst
        if !self.burst {
            self.body = body_of(actor.as_deref());
            self.mid = middle(&self.body);
        }
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let t = self.t;
        if !self.burst {
            self.draw_body(p);
            return;
        }
        let since = t - BURST;
        let heat = 1.0 - smooth(0.0, HEAT, since);
        let tint = |c: Rgba| {
            if heat > 0.0 {
                mix(c, HOT, heat * 0.6)
            } else {
                c
            }
        };
        for ch in &self.chunks {
            let (ox, oy) = (libm::floor(ch.dx), libm::floor(ch.dy));
            for &(x, y, c) in &ch.cells {
                let (x, y) = (f64::from(x) + ox, f64::from(y) + oy);
                if heat > 0.0 {
                    p.light(x, y, HOT, 0.4 * heat);
                }
                p.px(x, y, tint(c));
            }
        }
        for b in &self.bits {
            if t >= b.gone {
                continue;
            }
            let c = tint(b.c);
            let (x, y) = (libm::floor(b.x), libm::floor(b.y));
            if !b.rest {
                // a streak behind a fast bit
                let speed = libm::hypot(b.vx, b.vy);
                if speed > 110.0 {
                    let k = 1.5 / speed;
                    p.blend(
                        libm::floor(b.x - b.vx * k),
                        libm::floor(b.y - b.vy * k),
                        c,
                        0.5,
                    );
                }
                if heat > 0.0 {
                    p.light(x, y, HOT, 0.6 * heat);
                }
            }
            p.px(x, y, c);
        }
        // the burst's flash at the body's middle
        let flash = 1.0 - smooth(0.0, 0.3, since);
        if flash > 0.01 {
            let (mx, my) = self.mid;
            p.glow(mx, my, 30.0 * self.force.min(1.4), FLASH, 0.9 * flash);
        }
    }
}
