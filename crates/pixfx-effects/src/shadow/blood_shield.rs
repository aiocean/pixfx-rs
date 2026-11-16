//! Blood shield: blood spreads into a pool under the character, veins grow out of it up round them, and a glassy film
//! of blood closes over them into a wide dome (dark cultivation fantasy). Made here from the reference sheets of
//! `pixfx-ref`'s `examples/specs/shield/blood.json` and `blood-detail-{a,b,c}.json` (`target/ref/shield_blood*`):
//! the dome's rim, its branching veins, the see-through film, the pool, droplets, splash and ripples, in the sheet's
//! exact colours. Its golden hashes pin it.
//!
//! The veins are a tree from `pixfx_draw::growth`, grown once by `examples/bake_blood_shield.rs` and baked as data into
//! [`super::blood_shield_veins`]: six main veins laid on set paths, side branches grown off them by space colonization
//! with Murray's-law fork angles, stroked as tubes that taper from root to tip and are lit from the upper left. Nothing
//! the game sets changes them, so a shield grows nothing when it starts. A pool spreads under the feet and the veins rise from it, the growing tips glowing; the film fills in
//! behind them. It holds with a heartbeat (the rim swells, a pulse runs out along the veins) while droplets slide down
//! its sides. A blow lands anywhere (`hitX`, `hitY`, by default the right side at chest height): the dome flashes and
//! dents in there, ripples run across it, ten droplets splatter away from it and land, and the dent springs back past
//! its shape and settles. At the end the bubble pops: the film is gone in one frame, its outline flashes, the rim
//! breaks into shards that fly out and fall, and then INIT runs in reverse (the veins draw back into the pool twig
//! first, and the pool shrinks away). Everything is drawn from `t` and the seed. The far half of the veins and the base
//! ring go behind the character; the rim, the near half and the light in front. Counts, timing and paths are set here,
//! not taken from the sheets.

use core::f64::consts::{FRAC_PI_2, PI, TAU};

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::Rgba;
use pixfx_draw::math::{clamp, rand, smooth};

use super::blood_shield_veins::{MAP_ROWS, MAP_W, VEINS};

pub struct BloodShield;

pub static BLOOD_SHIELD: BloodShield = BloodShield;

/// The dome's half width and height (engine pixels: MAIN is 58 by 44 art pixels), the power of its outline, and
/// the depth of its base ellipse.
const RX: f64 = 43.0;
const H: f64 = 64.0;
const N: f64 = 2.5;
const RY: f64 = 5.0;

const HIT_X: NumOpt = NumOpt::new("hitX", 38.0).within(-60.0, 60.0);
const HIT_Y: NumOpt = NumOpt::new("hitY", -34.0).within(-90.0, 6.0);

const META: Meta = Meta {
    id: "shadow/blood-shield",
    name: "Blood shield",
    desc: "A pool of blood spreads under the character and veins grow up out of it into a glassy crimson dome; it beats like a heart while droplets slide down it; a blow dents it, ripples run across it and droplets splatter away; then the bubble pops into shards and the veins sink back into the pool.",
    tags: &[
        "shadow",
        "blood",
        "shield",
        "barrier",
        "cultivation",
        "block",
        "character",
        "one-shot",
    ],
    size: (144, 120),
    origin: (72, 104),
    duration: 3.8,
    looping: false,
    fps: 30,
    seed: 1,
    state: None,
    options: &[FEEL.spec(), HIT_X.spec(), HIT_Y.spec()],
};

/// Read off MAIN, darkest to lightest: the trunks' cores, their sides, a branch, the deep and mid of twigs and the rim's
/// inside, bright, rim, pale, highlight.
const BLOOD: [Rgba; 12] = [
    Rgba::hex24(0x560009),
    Rgba::hex24(0x630009),
    Rgba::hex24(0x73000b),
    Rgba::hex24(0x82000d),
    Rgba::hex24(0x90010f),
    Rgba::hex24(0xa10110),
    Rgba::hex24(0xb20313),
    Rgba::hex24(0xc70818),
    Rgba::hex24(0xdf0b1c),
    Rgba::hex24(0xf7222e),
    Rgba::hex24(0xfb7376),
    Rgba::hex24(0xfee5e4),
];
const CORE: usize = 0;
const VEIN: usize = 2;
const DEEP: usize = 4;
const MID: usize = 6;
const BRIGHT: usize = 8;
const RIM: usize = 9;
const PALE: usize = 10;
const HIGH: usize = 11;
/// Toward the light, upper left (y grows down): veins, film and rim are shaded by it.
const LIGHT: (f64, f64) = (-0.6, -0.8);
/// The red the dome and the pool light with.
const GLOW: Rgba = Rgba::hex24(0xff2a36);

/// Seconds: the pool is full, the film starts to close, the dome is whole, the blow, the bubble pops, the veins are
/// back in the pool, the pool is gone.
const POOLED: f64 = 0.25;
const MET: f64 = 0.62;
const WHOLE: f64 = 0.9;
const HIT: f64 = 1.6;
const POP: f64 = 2.6;
const DRAINED: f64 = 3.3;
const GONE: f64 = 3.8;
/// The heartbeat's period while it holds.
const BEAT: f64 = 0.6;
/// The shards the rim breaks into when the bubble pops.
const SHARDS: usize = 30;
/// The droplets the blow throws.
const SPLATTER: usize = 10;
/// How deep the dent goes and how wide it is.
const DENT: f64 = 7.0;
const DENT_WIDE: f64 = 13.0;
/// The pool's half width when full.
const POOL_RX: f64 = 47.0;

impl Effect for BloodShield {
    fn meta(&self) -> &Meta {
        &META
    }
    /// A soft one as the dome closes, a firm one where the blow lands, a soft one as it falls.
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        let b = Blow::new(opts.num(&HIT_X), opts.num(&HIT_Y));
        let mut out = impact(WHOLE, 0.1, 0.0, -H * 0.6, opts);
        out.extend(impact(HIT, 0.4, b.x, b.y, opts));
        out.extend(impact(POP, 0.2, 0.0, -H * 0.5, opts));
        out
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let seed = seed as f64;
        Box::new(State {
            ramp: opts.palette_or(&BLOOD),
            seed,
            blow: Blow::new(opts.num(&HIT_X), opts.num(&HIT_Y)),
        })
    }
}

struct State {
    ramp: Vec<Rgba>,
    seed: f64,
    blow: Blow,
}

/// Where the blow lands: the point of the dome's outline nearest the one the game gave, and which way is out.
#[derive(Clone, Copy)]
struct Blow {
    x: f64,
    y: f64,
    side: f64,
}

impl Blow {
    fn new(hx: f64, hy: f64) -> Self {
        let a = libm::atan2((-hy / H).max(0.05), hx / RX);
        let c = libm::cos(a);
        let side = if hx < 0.0 { -1.0 } else { 1.0 };
        Blow {
            x: RX * pw(libm::fabs(c)) * if c < 0.0 { -1.0 } else { 1.0 },
            y: -H * pw(libm::sin(a)),
            side,
        }
    }
}

/// No vein at this pixel.
const NO_VEIN: u8 = u8::MAX;

/// The baked veins ([`VEINS`]) as a map, a pixel each (tone, when it grew), made once for every shield.
static VEIN_MAP: std::sync::OnceLock<Vec<(u8, f32)>> = std::sync::OnceLock::new();

fn vein_map() -> &'static [(u8, f32)] {
    VEIN_MAP.get_or_init(|| {
        let mut map = vec![(NO_VEIN, 1.0f32); MAP_W * MAP_ROWS];
        for &(col, row, tone, born) in &VEINS {
            if let Some(px) = map.get_mut(usize::from(row) * MAP_W + usize::from(col)) {
                *px = (tone, born);
            }
        }
        map
    })
}

/// The dome's shape at `t`: its half width, height, how high it has filled (0..1) and how much is left (1 whole, 0
/// drained).
#[derive(Clone, Copy)]
struct Dome {
    rx: f64,
    h: f64,
    fill: f64,
    left: f64,
    /// How far the veins have grown out (0..1 of their growth order): they rise from the pool before the film fills.
    grow: f64,
}

/// The dome whole and still: its full size, filled, the veins grown.
fn whole_dome() -> Dome {
    Dome {
        rx: RX,
        h: H,
        fill: 1.0,
        left: 1.0,
        grow: 1.0,
    }
}

fn dome(t: f64) -> Dome {
    // the film and rim are there until the bubble pops; the veins then draw back into the pool, INIT in reverse
    let popped = if t >= POP { 0.0 } else { 1.0 };
    Dome {
        rx: RX,
        h: H,
        fill: smooth(MET - 0.08, WHOLE, t) * popped,
        left: 1.0,
        grow: smooth(POOLED * 0.5, WHOLE - 0.05, t) * (1.0 - smooth(POP + 0.05, DRAINED, t)),
    }
}

/// The heartbeat, 0..1, a sharp swell and a slower ebb every `BEAT` while the dome holds.
fn beat(t: f64) -> f64 {
    let on = smooth(WHOLE, WHOLE + 0.1, t) * (1.0 - smooth(POP - 0.1, POP, t));
    let ph = libm::fmod((t - WHOLE).max(0.0), BEAT) / BEAT;
    let k = if ph < 0.15 {
        ph / 0.15
    } else {
        libm::exp(-(ph - 0.15) * 6.0)
    };
    on * k
}

/// `v` to the power the dome's outline takes (its sides near upright, its crown round).
fn pw(v: f64) -> f64 {
    libm::pow(v.max(0.0), 2.0 / N)
}

/// What a pixel centre of the whole, undented dome always is, worked out once for every shield: the dome keeps its
/// size, so a frame reads these instead of paying three `pow`s, an `atan2` and a few divisions a pixel. The same
/// numbers as the expressions they come from, bit for bit; a point off the table (the dent moved it, or it is outside
/// the box, or the painter is turned) is worked out as before.
struct Px {
    /// [`reach`].
    r: f64,
    /// The film's light before the beat and the flash: Fresnel toward the rim, deeper at the base.
    film: f64,
    /// The highlight crescent on the upper left, and the glint low on the right (0..1).
    arc: f64,
    glint: f64,
    /// Where the vein map is read here, or `usize::MAX` off it.
    vein: usize,
}

/// The box the table covers, in whole pixels: the rim reaches 4 past the outline.
const TABLE_X0: f64 = -RX - 6.0;
const TABLE_Y0: f64 = -H - 6.0;
const TABLE_W: usize = (2.0 * RX + 12.0) as usize;
const TABLE_H: usize = (H + 8.0) as usize;

static TABLE: std::sync::OnceLock<Vec<Px>> = std::sync::OnceLock::new();

fn table() -> &'static [Px] {
    TABLE.get_or_init(|| {
        let d = whole_dome();
        let mut v = Vec::with_capacity(TABLE_W * TABLE_H);
        for j in 0..TABLE_H {
            for i in 0..TABLE_W {
                let (x, y) = (TABLE_X0 + i as f64 + 0.5, TABLE_Y0 + j as f64 + 0.5);
                let (r, film, arc, glint) = film_at(&d, x, y);
                v.push(Px {
                    r,
                    film,
                    arc,
                    glint,
                    vein: vein_index(&d, x, y).unwrap_or(usize::MAX),
                });
            }
        }
        v
    })
}

/// The table's entry for this point, when it is one of its pixel centres on the whole dome.
fn px(d: &Dome, x: f64, y: f64) -> Option<&'static Px> {
    if d.rx != RX || d.h != H {
        return None;
    }
    let (fi, fj) = (x - 0.5 - TABLE_X0, y - 0.5 - TABLE_Y0);
    if fi >= 0.0
        && fj >= 0.0
        && fi == libm::floor(fi)
        && fj == libm::floor(fj)
        && (fi as usize) < TABLE_W
    {
        table().get(fj as usize * TABLE_W + fi as usize)
    } else {
        None
    }
}

/// The film at a point of the dome: its reach, its light before the beat and the flash (glass: clear in the middle so
/// the character shows through, thicker toward the rim where the eye looks through more of the film, a Fresnel
/// falloff, and a little deeper at the base where the blood pools), the highlight crescent on the upper left where the
/// light falls, and the glint low on the right where it leaves.
fn film_at(d: &Dome, x: f64, y: f64) -> (f64, f64, f64, f64) {
    let e = reach(d, x, y);
    let edge = e * e * e * e;
    let low = 1.0 + y / d.h.max(1.0);
    let ang = libm::atan2(-y / d.h.max(1.0), x / d.rx.max(1.0));
    (
        e,
        0.07 + 0.42 * edge + 0.08 * low * low,
        smooth(0.62 * PI, 0.72 * PI, ang) * (1.0 - smooth(0.86 * PI, 0.95 * PI, ang)),
        smooth(0.12 * PI, 0.18 * PI, ang) * (1.0 - smooth(0.24 * PI, 0.3 * PI, ang)),
    )
}

/// Whether every point within the dent's reach of a pixel lies outside the rim's band (3 pixels out to 9 in): its
/// reach is within `slack` of `r` and its distance from the dome's foot at least `dist`.
fn out_of_band(r: f64, slack: f64, dist: f64) -> bool {
    if dist <= 0.0 {
        return false;
    }
    let (lo, hi) = (r - slack, r + slack);
    // deep inside: at least 9.5 in wherever it moves
    (hi < 1.0 && (1.0 / hi - 1.0) * dist >= 9.5)
        // far outside: at least 3.5 out wherever it moves
        || (lo > 1.0 && (1.0 - 1.0 / lo) * dist >= 3.5)
}

/// [`reach`] at a point, from the table when it can be.
fn reach_px(d: &Dome, x: f64, y: f64) -> f64 {
    px(d, x, y).map_or_else(|| reach(d, x, y), |p| p.r)
}

/// Where the vein map is read for the point `(sx, sy)` of the dome as it stands, if on the map.
fn vein_index(d: &Dome, sx: f64, sy: f64) -> Option<usize> {
    let col = libm::floor(RX + sx * RX / d.rx.max(1.0));
    let row = libm::floor(H + sy * H / d.h.max(1.0));
    if col < 0.0 || row < 0.0 || col as usize >= MAP_W {
        return None;
    }
    Some(row as usize * MAP_W + col as usize)
}

/// How far out a point is, 1 on the outline: the dome is the superellipse |x/rx|^N + |y/h|^N = 1 above the base.
fn reach(d: &Dome, x: f64, y: f64) -> f64 {
    let sx = libm::fabs(x) / d.rx.max(1.0);
    let sy = (-y).max(0.0) / d.h.max(1.0);
    libm::pow(libm::pow(sx, N) + libm::pow(sy, N), 1.0 / N)
}

/// How much a point of the dome faces the light (`LIGHT`), as on a sphere: about 1 on its upper left, -0.6 on its
/// lower right.
fn sphere(d: &Dome, x: f64, y: f64) -> f64 {
    let (nx, ny) = (x / d.rx.max(1.0), y / d.h.max(1.0));
    LIGHT.0 * nx + LIGHT.1 * ny
}

/// A point on the dome at azimuth `az` and height `v` (0 base, 1 top), and how near it is (-1 far, 1 near).
fn surface(d: &Dome, az: f64, v: f64) -> (f64, f64, f64) {
    let phi = v * FRAC_PI_2;
    let r = pw(libm::cos(phi));
    (
        d.rx * r * libm::cos(az),
        -d.h * pw(libm::sin(phi)) + RY * libm::sin(az) * r,
        libm::sin(az),
    )
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        let back = layer == Layer::Back;
        let d = dome(t);
        if back {
            self.pool(p, t);
        }
        if d.grow > 0.0 && d.left > 0.0 {
            self.veins(p, &d, t, back);
        }
        if d.fill > 0.0 && d.left > 0.0 {
            self.base(p, &d, t, back);
        }
        if back {
            return;
        }
        if d.fill > 0.0 && d.left > 0.0 {
            self.film(p, &d, t);
            self.rim(p, &d, t);
            self.ripples(p, &d, t);
        }
        self.sliders(p, &d, t);
        self.pop(p, t);
        self.flash(p, t);
        self.splatter(p, t);
    }
}

impl State {
    fn c(&self, i: usize) -> Rgba {
        self.ramp
            .get(i.min(self.ramp.len().saturating_sub(1)))
            .copied()
            .unwrap_or(Rgba::NONE)
    }

    /// The dent at `t`: how far (pixels, positive in) the dome is pushed at the blow, springing back past its shape.
    fn dent(&self, t: f64) -> f64 {
        let s = t - HIT;
        if !(0.0..0.9).contains(&s) {
            return 0.0;
        }
        if s < 0.05 {
            return DENT * s / 0.05;
        }
        let s = s - 0.05;
        DENT * libm::exp(-5.0 * s) * libm::cos(13.0 * s)
    }

    /// A point of the dome moved by the dent: toward the dome's middle, most at the blow.
    fn warp(&self, d: &Dome, x: f64, y: f64, k: f64) -> (f64, f64) {
        if k == 0.0 {
            return (x, y);
        }
        let (bx, by) = (self.blow.x * d.rx / RX, self.blow.y * d.h / H);
        let g =
            libm::exp(-((x - bx) * (x - bx) + (y - by) * (y - by)) / (2.0 * DENT_WIDE * DENT_WIDE));
        let (cx, cy) = (0.0, -d.h * 0.45);
        let (dx, dy) = (cx - x, cy - y);
        let n = libm::sqrt(dx * dx + dy * dy).max(1.0);
        (x + dx / n * k * g, y + dy / n * k * g)
    }

    /// The pool under the feet: a flat ellipse, its dark middle, deep and mid rings, a bright rim and a pale front lip;
    /// it spreads first, holds, and shrinks to a stain once the dome has drained.
    fn pool(&self, p: &mut Painter, t: f64) {
        let grow = smooth(0.0, POOLED, t);
        // INIT in reverse: once the veins are back in it, the pool draws in to a spot and is gone
        let dry = smooth(DRAINED - 0.1, GONE - 0.05, t);
        let grow = grow * (1.0 - dry);
        let rx = POOL_RX * (0.15 + 0.85 * grow);
        let ry = 4.5 * (0.5 + 0.5 * grow);
        let fade = 1.0 - smooth(GONE - 0.15, GONE, t);
        if rx < 1.0 || fade <= 0.0 || t <= 0.0 {
            return;
        }
        p.each(-rx - 1.0, -ry - 1.0, rx + 1.0, ry + 1.0, |p, x, y, _, _| {
            let e = (x / rx) * (x / rx) + (y / ry) * (y / ry);
            if e > 1.0 {
                return;
            }
            let i = if e > 0.82 {
                if y > 0.0 { PALE } else { RIM }
            } else if e > 0.6 {
                BRIGHT
            } else if e > 0.38 {
                DEEP
            } else if e > 0.18 {
                VEIN
            } else {
                CORE
            };
            p.dot(x, y, self.c(i), fade);
            if e > 0.6 {
                p.light(x, y, GLOW, 0.22 * fade);
            }
        });
    }

    /// The base ring where the dome meets the ground: its far half behind the character, its near half bright.
    fn base(&self, p: &mut Painter, d: &Dome, t: f64, back: bool) {
        let n = 120;
        for k in 0..n {
            let az = k as f64 / n as f64 * TAU;
            let near = libm::sin(az);
            if (near < 0.0) != back {
                continue;
            }
            let (x, y) = self.warp(d, d.rx * libm::cos(az), RY * near, self.dent(t));
            p.dot(x, y, self.c(if back { BRIGHT } else { RIM }), d.left);
        }
    }

    /// The veins from [`grow_veins`], over the dome as it is now: they grow in as the dome fills, and while it holds a
    /// pulse runs out along them from the trunks' feet to the twigs with each heartbeat. The middle third goes behind
    /// the character.
    fn veins(&self, p: &mut Painter, d: &Dome, t: f64, back: bool) {
        let ph = libm::fmod((t - WHOLE).max(0.0), BEAT) / BEAT;
        let pulse_on = smooth(WHOLE, WHOLE + 0.1, t) * (1.0 - smooth(POP, POP + 0.2, t));
        let dent = self.dent(t);
        let (x0, x1) = if back {
            (-d.rx * 0.4, d.rx * 0.4)
        } else {
            (-d.rx, d.rx)
        };
        p.each(x0, -d.h, x1, 0.0, |p, x, y, _, _| {
            if !back && libm::fabs(x) < d.rx * 0.4 {
                return;
            }
            let Some((tone, born)) = self.vein_at(d, x, y, dent) else {
                return;
            };
            if born > d.grow {
                return;
            }
            // the dome is a lit sphere: veins on its upper left a shade lighter, on its lower right a shade darker
            let shade = libm::round(1.5 * sphere(d, x, y)) as i64;
            let tone = (tone as i64 + shade).clamp(0, BRIGHT as i64) as usize;
            // before the film is there the veins glow on their own, brightest at the tip still growing
            let tip = clamp(1.0 - (d.grow - born) / 0.08, 0.0, 1.0);
            if !back {
                p.light(
                    x,
                    y,
                    GLOW,
                    (0.12 * (1.0 - d.fill) + 0.35 * tip * (1.0 - d.fill)) * d.left,
                );
            }
            let lit = pulse_on * clamp(1.0 - libm::fabs(born - ph * 1.3) / 0.1, 0.0, 1.0);
            let tone = if lit > 0.4 && !back {
                (tone + 3).min(RIM)
            } else {
                tone
            };
            p.dot(x, y, self.c(tone), d.left);
            if lit > 0.0 && !back {
                p.light(x, y, GLOW, 0.25 * lit * d.left);
            }
        });
    }

    /// The vein at this pixel of the dome as it is now, if any (its tone and when it grew): the map read where the
    /// dent and the sag moved the pixel from.
    fn vein_at(&self, d: &Dome, x: f64, y: f64, dent: f64) -> Option<(usize, f64)> {
        let at = if dent == 0.0
            && let Some(p) = px(d, x, y)
        {
            (p.vein != usize::MAX).then_some(p.vein)
        } else {
            let (wx, wy) = self.warp(d, x, y, dent);
            vein_index(d, 2.0 * x - wx, 2.0 * y - wy)
        };
        let &(tone, born) = vein_map().get(at?)?;
        (tone != NO_VEIN).then_some((usize::from(tone), f64::from(born)))
    }

    /// The film: no paint, only light, faint in the middle and stronger toward the edge, brighter with each beat.
    fn film(&self, p: &mut Painter, d: &Dome, t: f64) {
        let b = beat(t);
        let flash = clamp(1.0 - (t - HIT) / 0.2, 0.0, 1.0) * if t >= HIT { 1.0 } else { 0.0 };
        let top = d.h * d.fill;
        let dent = self.dent(t);
        p.each(-d.rx, -d.h, d.rx, 0.0, |p, x, y, _, _| {
            if -y > top {
                return;
            }
            let (e, film, arc, glint) = match px(d, x, y) {
                Some(px) => (px.r, px.film, px.arc, px.glint),
                None => film_at(d, x, y),
            };
            if e >= 1.0 || self.vein_at(d, x, y, dent).is_some() {
                return;
            }
            p.light(x, y, GLOW, film * (1.0 + 0.4 * b + 0.5 * flash) * d.left);
            let odd = libm::fmod(libm::floor(x) + libm::floor(y), 2.0) != 0.0;
            if arc > 0.0 && (0.7..0.88).contains(&e) && (arc > 0.5 || !odd) {
                p.light(x, y, self.c(PALE), 0.55 * arc * d.left);
            }
            // its core painted in pixels, a pale streak two pixels wide, white in the middle of the arc
            if arc > 0.35 && (0.77..0.81).contains(&e) {
                let c = if arc > 0.8 { HIGH } else { PALE };
                p.dot(x, y, self.c(c), (0.6 + 0.4 * arc) * d.left);
            }
            // a small glint low on the right, where the light leaves
            if glint > 0.5 && (0.78..0.82).contains(&e) {
                p.dot(x, y, self.c(PALE), 0.8 * d.left);
            }
        });
    }

    /// The rim: a band about five pixels thick inside the outline, read off MAIN from the outside in: a bright edge
    /// flecked pale, bright red with a dashed highlight over the crown, then bright, mid and a few deep pixels. It
    /// swells with each beat and is dented by the blow. Its glow is light in steps, three pixels out and eight in.
    fn rim(&self, p: &mut Painter, d: &Dome, t: f64) {
        const OUT: [f64; 3] = [0.36, 0.22, 0.11];
        const IN: [f64; 5] = [0.3, 0.2, 0.12, 0.07, 0.04];
        let b = beat(t);
        let thick = 1.0 + 0.3 * b;
        let glow = d.left * (1.0 + 0.5 * b);
        let top = d.h * d.fill;
        let dent = self.dent(t);
        // the dent moves a point at most |k| pixels, and reach changes at most 1/43 per pixel (it is a norm of x / RX
        // and y / H), so a pixel whose undented reach is this far from the band cannot be brought into it
        let slack = libm::fabs(dent) / RX.min(H) * 1.05 + 1e-6;
        p.each(-d.rx - 4.0, -d.h - 4.0, d.rx + 4.0, 0.0, |p, x, y, _, _| {
            if dent != 0.0
                && let Some(px) = px(d, x, y)
                && out_of_band(px.r, slack, libm::sqrt(x * x + y * y) - libm::fabs(dent))
            {
                return;
            }
            // read the shape where the dent moved this pixel from
            let (wx, wy) = self.warp(d, x, y, dent);
            let (sx, sy) = (2.0 * x - wx, 2.0 * y - wy);
            if -sy > top + 1.0 {
                return;
            }
            let r = reach_px(d, sx, sy);
            if r <= 0.0 {
                return;
            }
            let inside = (1.0 - r) * libm::sqrt(sx * sx + sy * sy) / r;
            if !(-3.0..9.0).contains(&inside) {
                return;
            }
            let (fx, fy) = (libm::floor(x), libm::floor(y));
            let odd = libm::fmod(fx + fy, 2.0) != 0.0;
            let light = if inside < 0.0 {
                OUT.get(libm::floor(-inside) as usize)
            } else {
                IN.get(libm::floor(inside * 0.6) as usize)
            };
            if let Some(&k) = light
                && ((-2.0..2.0).contains(&inside) || !odd)
                && (inside < 2.0 || self.vein_at(d, x, y, dent).is_none())
            {
                p.light(x, y, GLOW, k * glow);
            }
            if inside < 0.0 {
                return;
            }
            let fleck = rand(&[self.seed, 3.0, fx, fy]);
            let crown = sy < -0.55 * d.h && libm::fabs(sx) < 0.7 * d.rx;
            let i = inside / thick;
            let c = if i < 1.0 {
                if fleck < 0.3 { PALE } else { RIM }
            } else if i < 2.5 {
                if crown && i < 2.0 && fleck < 0.55 {
                    HIGH
                } else {
                    RIM
                }
            } else if i < 3.5 {
                BRIGHT
            } else if i < 4.5 && fleck < 0.7 {
                MID
            } else if (4.5..5.5).contains(&i) && fleck < 0.35 {
                DEEP
            } else {
                return;
            };
            // lit like the sphere it bounds: paler toward the light, a shade darker away from it
            let sph = sphere(d, sx, sy);
            let c = if sph < -0.15 {
                c.saturating_sub(1)
            } else if sph > 0.55 && c == RIM && fleck < 0.45 {
                PALE
            } else {
                c
            };
            p.dot(x, y, self.c(c), d.left);
        });
    }

    /// Two droplets sliding down the dome's sides while it holds, each a bead with a highlight and a short trail.
    fn sliders(&self, p: &mut Painter, d: &Dome, t: f64) {
        if t < WHOLE + 0.1 || t > POP {
            return;
        }
        for (i, az) in [0.15 * PI, 0.88 * PI].into_iter().enumerate() {
            let period = 0.9 + 0.2 * i as f64;
            let ph = libm::fmod(t - WHOLE - 0.1 + i as f64 * 0.4, period) / period;
            let v = 0.85 * (1.0 - ph * ph);
            let (x, y, _) = surface(d, az, v);
            let (x, y) = self.warp(d, x, y, self.dent(t));
            self.droplet(p, (x, y), 2, (0.0, -1.0), 1.0);
        }
    }

    /// One droplet: a pixel, a bead of two by two with a highlight, or one of three; a trail of `trail` pixels the
    /// way it came (`tx`, `ty`).
    fn droplet(&self, p: &mut Painter, (x, y): (f64, f64), size: u8, (tx, ty): (f64, f64), a: f64) {
        let tl = libm::sqrt(tx * tx + ty * ty);
        if tl > 0.0 {
            for k in 1..=2 {
                let k = f64::from(k);
                p.dot(
                    x + tx / tl * k,
                    y + ty / tl * k,
                    self.c(MID),
                    a * (1.0 - 0.3 * k),
                );
            }
        }
        match size {
            0 | 1 => p.dot(x, y, self.c(BRIGHT), a),
            2 => {
                p.dot(x, y, self.c(DEEP), a);
                p.dot(x + 1.0, y, self.c(VEIN), a);
                p.dot(x, y + 1.0, self.c(VEIN), a);
                p.dot(x + 1.0, y + 1.0, self.c(CORE), a);
                p.dot(x, y, self.c(PALE), a);
            }
            _ => {
                p.disc(x, y, 1.6, self.c(DEEP), a);
                p.dot(x - 1.0, y - 1.0, self.c(HIGH), a);
            }
        }
        p.light(x, y, GLOW, 0.12 * a);
    }

    /// The flash where the blow lands: a pale star of four rays for a tenth of a second.
    fn flash(&self, p: &mut Painter, t: f64) {
        let s = t - HIT;
        if !(0.0..0.12).contains(&s) {
            return;
        }
        let k = 1.0 - s / 0.12;
        let (x, y) = (self.blow.x, self.blow.y);
        let reach = 2.0 + 4.0 * k;
        p.dot(x, y, self.c(HIGH), 1.0);
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            for r in 1..=libm::floor(reach) as i32 {
                let r = f64::from(r);
                let c = if r < reach * 0.5 { HIGH } else { PALE };
                p.dot(x + dx * r, y + dy * r, self.c(c), k);
            }
        }
        p.glow(x, y, 10.0, GLOW, 0.8 * k);
    }

    /// Ripples: two rings running out from the blow across the dome, pale and dashed, fading.
    fn ripples(&self, p: &mut Painter, d: &Dome, t: f64) {
        for (i, delay) in [0.04, 0.16].into_iter().enumerate() {
            let s = t - HIT - delay;
            if !(0.0..0.45).contains(&s) {
                continue;
            }
            let r = 3.0 + 60.0 * s;
            let a = 1.0 - s / 0.45;
            let n = (r * 6.0) as usize + 8;
            for k in 0..n {
                let ang = k as f64 / n as f64 * TAU;
                let (x, y) = (
                    self.blow.x + r * libm::cos(ang),
                    self.blow.y + r * 0.8 * libm::sin(ang),
                );
                let e = reach(d, x, y);
                if e >= 0.92 || y > 0.0 || (k + i) % 3 == 0 {
                    continue;
                }
                p.dot(x, y, self.c(PALE), a);
                p.light(x, y, GLOW, 0.2 * a);
            }
        }
    }

    /// Ten droplets thrown away from the blow, falling, each landing as a small splat that dries with the pool.
    fn splatter(&self, p: &mut Painter, t: f64) {
        let s = t - HIT;
        if s < 0.0 {
            return;
        }
        let fade = 1.0 - smooth(DRAINED, GONE - 0.2, t);
        for k in 0..SPLATTER {
            let r = |n: f64| rand(&[self.seed, 7.0, k as f64, n]);
            let born = 0.02 * r(0.0);
            let s = s - born;
            if s < 0.0 {
                continue;
            }
            let vx = self.blow.side * (35.0 + 85.0 * r(1.0));
            let vy = -(25.0 + 85.0 * r(2.0));
            let g = 320.0;
            // when it reaches the ground (y = 0 + a little scatter in depth)
            let ground = 2.0 + 4.0 * r(3.0);
            let (y0, x0) = (self.blow.y, self.blow.x);
            let disc = vy * vy - 2.0 * g * (y0 - ground);
            let land = (-vy + libm::sqrt(disc.max(0.0))) / g;
            if s < land {
                let (x, y) = (x0 + vx * s, y0 + vy * s + 0.5 * g * s * s);
                let size = if r(4.0) < 0.3 {
                    3
                } else if r(4.0) < 0.75 {
                    2
                } else {
                    1
                };
                self.droplet(p, (x, y), size, (-vx, -(vy + g * s)), 1.0);
            } else if fade > 0.0 {
                let x = x0 + vx * land;
                p.dot(x, ground, self.c(DEEP), fade);
                p.dot(x + 1.0, ground, self.c(VEIN), fade);
                if r(5.0) < 0.5 {
                    p.dot(x - 1.0, ground, self.c(MID), fade);
                }
            }
        }
    }

    /// The pop: the film is gone in one frame, a ring of light flashes where it was, and the rim breaks into shards
    /// that fly out from the middle, turn as they fall and fade; a few fat drops fall straight into the pool.
    fn pop(&self, p: &mut Painter, t: f64) {
        let s = t - POP;
        if !(0.0..0.7).contains(&s) {
            return;
        }
        // the flash: the outline in pale light for two frames, spreading a little
        if s < 0.08 {
            let k = 1.0 - s / 0.08;
            let n = 160;
            for i in 0..=n {
                let a = f64::from(i) / f64::from(n) * PI;
                let c = libm::cos(a);
                let grow = 1.0 + 0.15 * s / 0.08;
                let (x, y) = (
                    RX * grow * pw(libm::fabs(c)) * if c < 0.0 { -1.0 } else { 1.0 },
                    -H * grow * pw(libm::sin(a)),
                );
                p.dot(x, y, self.c(HIGH), k);
                p.light(x, y, GLOW, 0.6 * k);
            }
            p.glow(0.0, -H * 0.5, RX, GLOW, 0.5 * k);
        }
        let g = 260.0;
        for i in 0..SHARDS {
            let r = |n: f64| rand(&[self.seed, 13.0, i as f64, n]);
            // where on the rim it broke from
            let a = (i as f64 + 0.5 * r(0.0)) / SHARDS as f64 * PI;
            let c = libm::cos(a);
            let (x0, y0) = (
                RX * pw(libm::fabs(c)) * if c < 0.0 { -1.0 } else { 1.0 },
                -H * pw(libm::sin(a)),
            );
            // out from the dome's middle, faster at the crown
            let (ox, oy) = (x0, y0 + H * 0.4);
            let l = libm::sqrt(ox * ox + oy * oy).max(1.0);
            let speed = 50.0 + 70.0 * r(1.0);
            let (vx, vy) = (ox / l * speed, oy / l * speed - 30.0 * r(2.0));
            let (x, y) = (x0 + vx * s, y0 + vy * s + 0.5 * g * s * s);
            if y > 3.0 {
                continue;
            }
            let fade = 1.0 - smooth(0.25 + 0.25 * r(3.0), 0.6, s);
            if fade <= 0.0 {
                continue;
            }
            // a shard is a short curved piece of rim, two or three pixels long, turning as it flies
            let turn = a + (r(4.0) - 0.5) * 18.0 * s;
            let len = if r(5.0) < 0.4 { 3 } else { 2 };
            for k in 0..len {
                let u = f64::from(k) - f64::from(len - 1) / 2.0;
                let (px, py) = (x + u * libm::sin(turn), y - u * libm::cos(turn));
                let col = if k == 0 { PALE } else { RIM };
                p.dot(px, py, self.c(col), fade);
            }
            p.light(x, y, GLOW, 0.25 * fade);
        }
        // fat drops from the crown and the walls, falling into the pool
        for i in 0..6 {
            let r = |n: f64| rand(&[self.seed, 17.0, f64::from(i), n]);
            let x = (f64::from(i) / 5.0 - 0.5) * 1.6 * RX * (0.8 + 0.2 * r(0.0));
            let y0 = -H * pw(libm::cos(x / RX * FRAC_PI_2).max(0.0)) * (0.6 + 0.35 * r(1.0));
            let sd = s - 0.04 * r(2.0);
            if sd < 0.0 {
                continue;
            }
            let y = y0 + 0.5 * 300.0 * sd * sd;
            if y < 2.0 {
                self.droplet(p, (x, y), 2, (0.0, -1.0), 1.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_baked_veins_were_grown_for_this_dome() {
        // the map is the whole dome, so a dome of another size needs the veins grown again
        assert_eq!(
            (MAP_W, MAP_ROWS),
            (2 * RX as usize + 1, H as usize + 1),
            "run examples/bake_blood_shield.rs again"
        );
        assert!(
            VEINS
                .iter()
                .all(|&(c, r, tone, born)| usize::from(c) < MAP_W
                    && usize::from(r) < MAP_ROWS
                    && usize::from(tone) < BLOOD.len()
                    && (0.0..=1.0).contains(&born))
        );
    }
}
