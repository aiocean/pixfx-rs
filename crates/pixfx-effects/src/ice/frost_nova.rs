//! Frost nova: port of pixelfx `src/fx/ice/frost-nova.js`.
//!
//! pixelfx acts it out on a stand-in character (`act`: poses, the slam, the hits); that staging is dropped here.
//! Beyond pixelfx: the slam that releases the nova fires impact cues ([`pixfx_core::cue::impact`]) for the game
//! to answer; the `feel` option scales them, 0 for none.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::sim::{
    BitKind, Bits, CrystalOpts, Kind, Lights, Particles, ParticlesOpts, band, crystal, flare,
};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{Rng, TAU, clamp, ease, js_max, js_min, js_round, rand, smooth, to_uint32};

use super::lib::{PALETTE, bit, emit};

pub struct FrostNova;

pub static FROST_NOVA: FrostNova = FrostNova;

const META: Meta = Meta {
    id: "ice/frost-nova",
    name: "Frost nova",
    desc: "Frost spirals into a spinning snowflake, then a wave blooms three rings of ice crystals round the caster; snow drifts, frost creeps, and the ice melts back into the floor.",
    tags: &["ice", "nova", "aoe", "frost", "magic", "one-shot"],
    size: (288, 192),
    origin: (144, 150),
    duration: 3.4,
    looping: false,
    fps: 30,
    seed: 4,
    state: None,
    options: &[REACH.spec(), FEEL.spec()],
};

const REACH: NumOpt = NumOpt::new("reach", 104.0);

const CHARGE: f64 = 0.42;
const SPREAD: f64 = 0.62;
const HOLD: f64 = 1.95;
const MELT: f64 = 0.7;
const CORE_Y: f64 = -24.0;

/// A ring of crystals: where along the reach (0..1), heights, widths, how many a side, how many small crystals
/// lean off each.
struct Ring {
    q: (f64, f64),
    h: (f64, f64),
    w: (f64, f64),
    n: u32,
    kids: u32,
}

/// Three rings of crystals: tall ones hugging the caster, smaller ones further out.
const RINGS: [Ring; 3] = [
    Ring {
        q: (0.15, 0.24),
        h: (24.0, 32.0),
        w: (10.0, 13.0),
        n: 2,
        kids: 2,
    },
    Ring {
        q: (0.36, 0.52),
        h: (13.0, 19.0),
        w: (7.0, 9.0),
        n: 3,
        kids: 1,
    },
    Ring {
        q: (0.62, 0.9),
        h: (5.0, 10.0),
        w: (4.0, 6.0),
        n: 4,
        kids: 0,
    },
];

impl Effect for FrostNova {
    fn meta(&self) -> &Meta {
        &META
    }
    /// Beyond pixelfx: the slam that releases the wave asks the game for an impact's answer.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(CHARGE, 0.6, 0.0, CORE_Y, opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let c: [Rgba; 8] = ramp(&opts.palette_or(&PALETTE));
        let dim = [c[0], c[0], c[1], c[2], c[3], c[4], c[5], c[6]];
        let reach = opts.num(&REACH);
        let mut r = Rng::new(to_uint32(seed as f64));
        // lay out the crystals: both sides, three rings, alternating back and front rows
        let mut sites = Vec::new();
        for side in [-1.0, 1.0] {
            for (ri, ring) in RINGS.iter().enumerate() {
                for i in 0..ring.n {
                    let fi = f64::from(i);
                    let q = ring.q.0
                        + ((ring.q.1 - ring.q.0) * (fi + r.range(0.2, 0.8))) / f64::from(ring.n);
                    let h = r.range(ring.h.0, ring.h.1);
                    let w = r.range(ring.w.0, ring.w.1);
                    let mut parts = vec![Part {
                        dx: 0.0,
                        h,
                        w,
                        lean: side * h * r.range(0.1, 0.28),
                    }];
                    for k in 0..ring.kids {
                        let s = if k % 2 == 1 { -side } else { side };
                        let kh = h * r.range(0.4, 0.62);
                        let dx = s * r.range(0.3, 0.5) * w;
                        let lean = s * kh * r.range(0.35, 0.6);
                        parts.insert(
                            0,
                            Part {
                                dx,
                                h: kh,
                                w: w * 0.62,
                                lean,
                            },
                        );
                    }
                    let t0 = reached_at(q * reach, reach) + r.range(0.0, 0.03);
                    let phase = r.next();
                    sites.push(Site {
                        x: side * q * reach,
                        h,
                        back: (i as usize + ri + usize::from(side > 0.0)) % 2 == 1,
                        t0,
                        up: false,
                        parts,
                        phase,
                    });
                }
            }
        }
        Box::new(State {
            c,
            dim,
            reach,
            r,
            ps: Particles::new(ParticlesOpts {
                ground: 0.0,
                sparks: vec![c[7], c[6], c[5], c[4]],
                haze: Some(c[5]),
            }),
            bits: Bits::new(0.0),
            lights: Lights::new(),
            sites,
            t: 0.0,
            released: false,
        })
    }
}

/// When the wave's front reaches `d` px out.
fn reached_at(d: f64, reach: f64) -> f64 {
    CHARGE + SPREAD * (1.0 - libm::cbrt(1.0 - js_min(1.0, d / reach)))
}

/// One crystal of a site: offset, height, width, lean at the tip.
struct Part {
    dx: f64,
    h: f64,
    w: f64,
    lean: f64,
}

struct Site {
    x: f64,
    h: f64,
    back: bool,
    t0: f64,
    up: bool,
    parts: Vec<Part>,
    phase: f64,
}

struct State {
    c: [Rgba; 8],
    dim: [Rgba; 8],
    reach: f64,
    r: Rng,
    ps: Particles,
    bits: Bits,
    lights: Lights,
    sites: Vec<Site>,
    t: f64,
    released: bool,
}

fn sink(tt: f64) -> f64 {
    smooth(HOLD, HOLD + MELT, tt)
}

impl State {
    /// The wave's front, px out from the caster.
    fn front(&self, t: f64) -> f64 {
        if t < CHARGE {
            0.0
        } else {
            self.reach * (1.0 - libm::pow(1.0 - clamp((t - CHARGE) / SPREAD, 0.0, 1.0), 3.0))
        }
    }

    fn draw_crystals(&self, p: &mut Painter, back: bool) {
        let t = self.t;
        let m = sink(t);
        for s in &self.sites {
            if !s.up || s.back != back || m >= 1.0 {
                continue;
            }
            let g = ease::out_back(clamp((t - s.t0) / 0.1, 0.0, 1.0), 2.6);
            let white = t < s.t0 + 0.03;
            let sw = (((t - s.t0) * 0.7 + s.phase * 3.0) % 1.6) / 0.5;
            for q in &s.parts {
                crystal(
                    p,
                    s.x + q.dx,
                    if back { -1.0 } else { 0.0 },
                    q.h * g * (1.0 - m),
                    q.w * (1.0 - m * 0.3),
                    q.lean * g * (1.0 - m),
                    if back { &self.dim } else { &self.c },
                    CrystalOpts {
                        a: 1.0,
                        white,
                        glint: if sw <= 1.0 { sw } else { -1.0 },
                    },
                );
            }
        }
    }

    fn ring(&self, p: &mut Painter, back: bool) {
        let t = self.t;
        if t < CHARGE {
            return;
        }
        let u = clamp((t - CHARGE) / (SPREAD + 0.15), 0.0, 1.0);
        if u >= 1.0 {
            return;
        }
        let fr = self.front(t);
        band(
            p,
            0.0,
            0.0,
            fr + 2.0,
            0.16,
            js_max(2.0, 10.0 * libm::pow(1.0 - u, 0.7)),
            if u < 0.75 { 1.0 } else { (1.0 - u) / 0.25 },
            &self.c,
            |g| (libm::sin(g) < 0.0) == back,
            t - CHARGE < 0.1,
        );
        if !back {
            for s in [-1.0, 1.0] {
                p.glow(s * (fr + 2.0), -2.0, 22.0, self.c[5], 0.55 * (1.0 - u));
            }
        }
    }
}

/// Frost on the floor: a white crust, little tufts, a dithered bloom under the surface.
fn frost(p: &mut Painter, c: &[Rgba; 8], x0: f64, x1: f64, reach: f64, a: f64, edge: f64) {
    let mut x = libm::floor(x0);
    while x <= libm::ceil(x1) {
        let xx = x;
        x += 1.0;
        let d = libm::fabs(xx + 0.5);
        let q = d / reach;
        let k = clamp((a * 1.15 - q) / 0.15, 0.0, 1.0);
        if k <= 0.0 {
            continue;
        }
        let h = rand(&[xx, 17.0]);
        let fresh = edge > 0.0 && d > edge - 3.0;
        let tuft = if h < 0.45 {
            js_round((1.0 + rand(&[xx, 5.0]) * 3.0) * (1.0 - q * 0.6))
        } else {
            0.0
        };
        let dd = (4.0 + rand(&[xx, 11.0]) * 4.0) * (1.0 - q * 0.5);
        p.dot(xx, 0.0, if fresh || h < 0.25 { WHITE } else { c[6] }, k);
        let mut j = 1.0;
        while j <= tuft {
            p.dot(xx, -j, if j == tuft { WHITE } else { c[5] }, k);
            j += 1.0;
        }
        if tuft > 1.0 && rand(&[xx, 9.0]) < 0.5 {
            let side = if rand(&[xx, 3.0]) < 0.5 { -1.0 } else { 1.0 };
            p.dot(xx + side, -tuft + 1.0, c[6], k);
        }
        let mut j = 1.0;
        while j <= dd {
            let v = j / dd;
            let col = if rand(&[xx * 31.0 + j, 7.0]) < 0.06 {
                WHITE
            } else if v < 0.25 {
                c[6]
            } else if v < 0.55 {
                c[5]
            } else {
                c[4]
            };
            p.dot(xx, j, col, k * (1.1 - v) * if fresh { 1.3 } else { 1.0 });
            j += 1.0;
        }
    }
}

/// The snowflake the cold gathers into: six arms with side branches, spinning, pulsing.
fn snowflake(p: &mut Painter, c: &[Rgba; 8], x: f64, y: f64, l: f64, spin: f64, k: f64) {
    if l < 0.8 {
        return;
    }
    for i in 0..6 {
        let a = spin + (f64::from(i) * TAU) / 6.0;
        let (ca, sa) = (libm::cos(a), libm::sin(a));
        let mut d = 0.0;
        while d <= l {
            p.dot(
                x + ca * d,
                y + sa * d,
                if d < l * 0.4 { WHITE } else { c[6] },
                k + 0.2,
            );
            d += 0.5;
        }
        if l > 3.0 {
            for s in [-1.0, 1.0] {
                let (bx, by, ba) = (x + ca * l * 0.6, y + sa * l * 0.6, a + s * 0.9);
                let mut d = 0.0;
                while d <= l * 0.35 {
                    p.dot(bx + libm::cos(ba) * d, by + libm::sin(ba) * d, c[5], k);
                    d += 0.5;
                }
            }
        }
    }
    p.px(x, y, WHITE);
    p.glow(x, y, 10.0 + l * 3.0, c[5], 0.35 + 0.5 * k);
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        let fr = self.front(t);
        let (reach, sink_t) = (self.reach, sink(t));
        let State {
            c,
            r,
            ps,
            bits,
            lights,
            sites,
            released,
            ..
        } = self;
        let c = *c;
        if t < CHARGE - 0.06 {
            // the cold gathers: streaks spiral in to the core, vapour slides in along the floor
            if r.chance(0.9) {
                let a = r.range(0.0, TAU);
                let d = r.range(26.0, 58.0);
                let tt = r.range(0.2, 0.3);
                let v = (d * 3.0) / (libm::exp(3.0 * tt) - 1.0);
                let x = libm::cos(a) * d;
                let y = js_min(CORE_Y + libm::sin(a) * d * 0.8, -2.0);
                let l = libm::hypot(x, y - CORE_Y);
                let l = if l == 0.0 { 1.0 } else { l };
                emit!(
                    ps,
                    r,
                    Kind::Streak,
                    x,
                    y,
                    (-x / l) * v,
                    ((CORE_Y - y) / l) * v,
                    tt,
                    if r.chance(0.5) { c[6] } else { c[5] }
                );
            }
            if r.chance(0.14) {
                let s = r.sign();
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    s * r.range(28.0, 60.0),
                    -r.range(0.0, 2.0),
                    -s * r.range(35.0, 70.0),
                    -r.range(0.0, 3.0),
                    r.range(0.35, 0.5),
                    c[6],
                    r.range(2.0, 3.0)
                );
            }
        }
        if t >= CHARGE && !*released {
            *released = true;
            lights.add(0.0, CORE_Y, 84.0, c[4], 0.8, 0.55);
            lights.add(0.0, CORE_Y, 45.0, WHITE, 0.7, 0.12);
            for _ in 0..22 {
                let a = r.range(-core::f64::consts::PI, 0.25);
                let sp = r.range(110.0, 280.0);
                bit!(bits, BitKind::Shard;
                    x = r.range(-3.0, 3.0),
                    y = CORE_Y,
                    vx = libm::cos(a) * sp,
                    vy = libm::sin(a) * sp,
                    g = 0.7,
                    drag = 0.6,
                    bounce = 0.3,
                    life = r.range(0.9, 1.5),
                    a = r.range(0.0, TAU),
                    va = r.range(-20.0, 20.0),
                    s = r.range(1.5, 3.5),
                    c = *r.pick(&[c[7], c[6], c[5]]).unwrap_or(&c[7]),
                );
            }
            for _ in 0..14 {
                let a = r.range(-core::f64::consts::PI, 0.0);
                let sp = r.range(40.0, 160.0);
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    0.0,
                    CORE_Y,
                    libm::cos(a) * sp,
                    libm::sin(a) * sp,
                    r.range(1.2, 1.8),
                    if r.chance(0.6) { WHITE } else { c[6] },
                    r.range(0.5, 1.6)
                );
            }
        }
        if t > CHARGE && t < CHARGE + SPREAD {
            for s in [-1.0, 1.0] {
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    s * fr + r.range(-2.0, 2.0),
                    -r.range(0.0, 5.0),
                    s * r.range(15.0, 75.0),
                    -r.range(2.0, 15.0),
                    r.range(0.6, 1.0),
                    if r.chance(0.7) { c[6] } else { c[5] },
                    r.range(3.0, 5.5) * (1.0 - 0.4 * (fr / reach))
                );
                if r.chance(0.45) {
                    emit!(
                        ps,
                        r,
                        Kind::Flake,
                        s * fr,
                        -2.0,
                        s * r.range(10.0, 45.0),
                        -r.range(30.0, 85.0),
                        r.range(1.1, 1.9),
                        if r.chance(0.5) { WHITE } else { c[6] },
                        r.range(0.5, 1.2)
                    );
                }
            }
        }
        for s in sites.iter_mut() {
            if !s.up && t >= s.t0 {
                s.up = true;
                // each crystal punches up with a spray of shards, a puff and a glint
                let n = 2.0 + js_round(s.h / 8.0);
                let mut i = 0.0;
                while i < n {
                    i += 1.0;
                    let a = -core::f64::consts::FRAC_PI_2 + r.range(-1.3, 1.3);
                    let sp = r.range(60.0, 200.0) * (0.5 + s.h / 40.0);
                    bit!(bits, BitKind::Shard;
                        x = s.x + r.range(-3.0, 3.0),
                        y = 0.0,
                        vx = libm::cos(a) * sp,
                        vy = libm::sin(a) * sp,
                        g = 0.8,
                        drag = 0.6,
                        bounce = 0.3,
                        life = r.range(0.9, 1.6),
                        a = r.range(0.0, TAU),
                        va = r.range(-18.0, 18.0),
                        s = r.range(1.5, 3.0),
                        c = *r.pick(&[c[7], c[6], c[5]]).unwrap_or(&c[7]),
                    );
                }
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    s.x,
                    -1.0,
                    r.range(-30.0, 30.0),
                    -r.range(2.0, 8.0),
                    r.range(0.5, 0.9),
                    c[6],
                    r.range(2.5, 4.0)
                );
                if s.h > 10.0 {
                    emit!(
                        ps,
                        r,
                        Kind::Star,
                        s.x + r.range(-3.0, 3.0),
                        -r.range(4.0, s.h * 0.8),
                        0.0,
                        -3.0,
                        r.range(0.3, 0.6),
                        c[6],
                        r.range(1.0, 2.4)
                    );
                }
            }
        }
        // while it holds: glints run over the crystals, snow drifts down
        if t > CHARGE + 0.2 && t < HOLD + 0.2 {
            if r.chance(0.5)
                && let Some(i) = pick_index(r, sites.len())
                && let Some(s) = sites.get(i)
                && s.up
            {
                emit!(
                    ps,
                    r,
                    Kind::Star,
                    s.x + r.range(-3.0, 3.0),
                    -r.range(2.0, s.h * 0.9),
                    0.0,
                    -3.0,
                    r.range(0.3, 0.6),
                    if r.chance(0.6) { c[6] } else { WHITE },
                    r.range(0.6, 2.2)
                );
            }
            if t < HOLD - 0.3 && r.chance(0.16) {
                emit!(
                    ps,
                    r,
                    Kind::Flake,
                    r.range(-1.0, 1.0) * fr,
                    -r.range(40.0, 90.0),
                    r.range(-5.0, 5.0),
                    r.range(3.0, 10.0),
                    r.range(1.1, 1.5),
                    if r.chance(0.5) { WHITE } else { c[6] },
                    r.range(0.5, 1.4)
                );
            }
        }
        // melting: drips run off the crystals, vapour lifts off the floor
        if t > HOLD && t < HOLD + MELT {
            if r.chance(0.5)
                && let Some(i) = pick_index(r, sites.len())
                && let Some(s) = sites.get(i)
            {
                bit!(bits, BitKind::Drip;
                    x = s.x + r.range(-3.0, 3.0),
                    y = -r.range(3.0, s.h * 0.6 * (1.0 - sink_t)),
                    vy = 10.0,
                    g = 0.6,
                    life = 1.0,
                    c = c[5],
                );
            }
            if r.chance(0.25) {
                emit!(
                    ps,
                    r,
                    Kind::Vapour,
                    r.range(-1.0, 1.0) * reach * 0.8,
                    -r.range(0.0, 3.0),
                    r.range(-10.0, 10.0),
                    -r.range(6.0, 16.0),
                    r.range(0.6, 1.0),
                    c[5],
                    r.range(3.0, 5.0)
                );
            }
        }
        ps.update(r, dt);
        bits.update(dt);
        lights.update(dt);
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        let (c, t) = (&self.c, self.t);
        if layer != Layer::Front {
            let out = 1.0 - smooth(HOLD + 0.2, HOLD + MELT + 0.6, t);
            let fr = self.front(t);
            if out > 0.0 && t > CHARGE {
                let half = js_max(12.0, fr + 1.0);
                frost(p, c, -half, half, self.reach, out, fr);
            }
            self.ring(p, true);
            self.draw_crystals(p, true);
            self.ps.draw(p, Layer::Back);
        }
        if layer == Layer::Back {
            return;
        }
        self.ring(p, false);
        self.draw_crystals(p, false);
        if t < CHARGE {
            let k = t / CHARGE;
            let beat = 1.0
                + 0.25 * libm::exp(-libm::pow(t - CHARGE * 0.55, 2.0) / 0.002)
                + 0.3 * libm::exp(-libm::pow(t - CHARGE * 0.85, 2.0) / 0.002);
            snowflake(p, c, 0.0, CORE_Y, (2.0 + 5.5 * k * k) * beat, t * 4.0, k);
        } else if t < CHARGE + 0.22 {
            let a = (t - CHARGE) / 0.22;
            flare(p, 0.0, CORE_Y, 36.0 * libm::pow(1.0 - a, 1.5), 1.0 - a, c);
            p.glow(0.0, CORE_Y, 46.0, WHITE, 0.6 * (1.0 - a));
        }
        // the frosted floor gives off a cold light while the ice stands
        if t > CHARGE {
            let k = (1.0 - smooth(HOLD, HOLD + MELT + 0.3, t)) * (0.85 + 0.15 * libm::sin(t * 2.4));
            let fr = self.front(t);
            for i in -3..=3 {
                p.glow((f64::from(i) / 3.5) * fr, -4.0, 26.0, c[4], 0.16 * k);
            }
        }
        self.bits.draw(p);
        self.ps.draw(p, Layer::Front);
        self.lights.draw(p);
    }
}

/// The index `Rng::pick` would take from a list of `n` (one draw, as pixelfx `R.pick`).
fn pick_index(r: &mut Rng, n: usize) -> Option<usize> {
    let i = libm::floor(r.next() * n as f64) as usize;
    (i < n).then_some(i)
}
