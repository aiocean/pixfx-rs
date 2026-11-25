//! Storm cloud: port of pixelfx `src/fx/lightning/storm-cloud.js`.
//!
//! The bolt's forked shape, its halo-body-core inking and its flicker follow Sprite Fusion's open-source
//! lightning (https://www.spritefusion.com/pixel-art-vfx/lightning), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::particles::cycle;
use pixfx_core::{Effect, EffectState, Layer, Meta, Options, RampOpt};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, ramp};
use pixfx_draw::math::{
    clamp, js_max, js_min, js_round, loop_noise, or_one, rng, smooth, tile_noise,
};

use super::lib::{PLASMA, StrokeOpts, VOLT, crackle, flight, spark, stroke, zigzag};

pub struct StormCloud;

pub static STORM_CLOUD: StormCloud = StormCloud;

const META: Meta = Meta {
    id: "lightning/storm-cloud",
    name: "Storm cloud",
    desc: "A storm cloud boils overhead, billows lit from inside by flickering discharges, ragged scud under its flat dark base; rain slants down and once a loop a bolt cracks to the floor.",
    tags: &[
        "lightning",
        "storm",
        "weather",
        "cloud",
        "rain",
        "ambient",
        "loop",
    ],
    size: (168, 176),
    origin: (84, 164),
    duration: 3.0,
    looping: true,
    fps: 30,
    seed: 8,
    state: None,
    options: &[SPARKS.spec(), CLOUD.spec(), RAIN.spec()],
};

const SPARKS: RampOpt = RampOpt::new("sparks", &PLASMA);
const CLOUD: RampOpt = RampOpt::new(
    "cloud",
    &[
        Rgba::hex24(0x09070f),
        Rgba::hex24(0x120f1e),
        Rgba::hex24(0x1b182e),
        Rgba::hex24(0x252640),
        Rgba::hex24(0x323758),
        Rgba::hex24(0x464d78),
        Rgba::hex24(0x646b9c),
    ],
);
const RAIN: RampOpt = RampOpt::new(
    "rain",
    &[
        Rgba::hex24(0x2b3250),
        Rgba::hex24(0x46527a),
        Rgba::hex24(0x6d7cab),
        Rgba::hex24(0xa5b5e0),
    ],
);

const FPS: f64 = 30.0;
const N: f64 = 90.0;
const P: f64 = 3.0;
const BASE: f64 = -100.0;
const TOP: f64 = -156.0;
const TH: f64 = 0.35;
// the billows: [x, y, visible radius, squash]; a tower leaning into a flat anvil sheared off to the right, big
// billows of all sizes below it, smaller shoulders, a flat run along the base
const BILLOWS: [(f64, f64, f64, f64); 17] = [
    (-6.0, -132.0, 17.0, 1.0),
    (18.0, -127.0, 14.0, 1.0),
    (-28.0, -122.0, 12.0, 1.0),
    (38.0, -118.0, 10.0, 1.0),
    (0.0, -142.0, 12.0, 1.0),
    (14.0, -149.0, 19.0, 0.38),
    (40.0, -148.0, 14.0, 0.34),
    (-48.0, -113.0, 10.0, 1.0),
    (55.0, -110.0, 9.0, 1.0),
    (-17.0, -115.0, 13.0, 1.0),
    (22.0, -113.0, 12.0, 1.0),
    (-42.0, -104.0, 11.0, 1.0),
    (-10.0, -104.0, 12.0, 1.0),
    (20.0, -104.0, 11.0, 1.0),
    (48.0, -103.0, 9.0, 1.0),
    (-63.0, -104.0, 7.0, 1.0),
    (66.0, -103.0, 6.0, 1.0),
];
// discharges inside: [frame, billow, strength]; the bolt to the floor at frame BOLT, out of billow BOLT_B
const FLASHES: [(f64, usize, f64); 8] = [
    (7.0, 2, 0.8),
    (8.0, 9, 0.55),
    (29.0, 1, 0.75),
    (31.0, 10, 1.0),
    (33.0, 3, 0.5),
    (70.0, 0, 0.8),
    (71.0, 4, 0.55),
    (72.0, 5, 0.35),
];
const BOLT: f64 = 52.0;
const BOLT_B: usize = 12;
const BK: [f64; 7] = [1.0, 0.7, 0.3, 0.95, 0.5, 0.75, 0.2];
const W0: f64 = -84.0;
const H0: f64 = TOP - 6.0;
const GW: usize = 168;
const GH: usize = (BASE + 8.0 - H0) as usize;

impl Effect for StormCloud {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            c: ramp(&opts.palette_or(&VOLT)),
            k: ramp(&opts.ramp(&CLOUD)),
            rn: ramp(&opts.ramp(&RAIN)),
            s: opts.ramp(&SPARKS),
            seed: seed as f64,
        })
    }
}

struct State {
    c: [Rgba; 8],
    k: [Rgba; 7],
    rn: [Rgba; 4],
    s: Vec<Rgba>,
    seed: f64,
}

/// `VIS` of the source: how far out a billow shows, as a share of its field radius.
fn vis() -> f64 {
    libm::sqrt(1.0 - libm::sqrt(TH))
}

/// Where a billow is this frame.
#[derive(Clone, Copy)]
struct Billow {
    sq: f64,
    x: f64,
    y: f64,
    r: f64,
}

/// Where each billow is this frame: drifting, breathing and boiling up on its own clock, all seamless over the
/// loop.
fn billows(seed: f64, ph: f64) -> Vec<Billow> {
    BILLOWS
        .iter()
        .enumerate()
        .map(|(i, &(x, y, v, sq))| {
            let i = i as f64;
            let top = y < -112.0 && sq == 1.0;
            let boil = 0.82
                + 0.3
                    * libm::pow(
                        libm::sin(core::f64::consts::PI * ((ph + i * 0.37) % 1.0)),
                        2.0,
                    );
            Billow {
                sq,
                x: x + 3.5 * loop_noise(ph, 0.7, seed + i)
                    + if sq < 1.0 {
                        4.0 * loop_noise(ph, 0.5, seed + i + 90.0)
                    } else {
                        0.0
                    },
                y: y + 2.0 * loop_noise(ph, 0.7, seed + i + 30.0)
                    - if top { 4.0 * (boil - 0.97) } else { 0.0 },
                r: (v * if top {
                    boil
                } else {
                    1.0 + 0.08 * loop_noise(ph, 0.6, seed + i + 60.0)
                }) / vis(),
            }
        })
        .collect()
}

/// pixelfx's `Float32Array` `+=`: the sum rounded to single precision.
fn add32(cell: &mut f32, v: f64) {
    *cell = (f64::from(*cell) + v) as f32;
}

/// The cloud as one field: billows summed so they fuse, clipped flat at the base, its edge ragged with drifting
/// noise; shaded as one mass (a lit top, a cool middle, a dark belly), discharges lighting whole billows from
/// inside.
fn cloud(
    p: &mut Painter,
    b: &[Billow],
    ph: f64,
    seed: f64,
    k: &[Rgba; 7],
    c: &[Rgba; 8],
    flashes: &[(usize, f64)],
) {
    let mut field = vec![0f32; GW * GH];
    let mut gx = vec![0f32; GW * GH];
    let mut gy = vec![0f32; GW * GH];
    let mut lit = vec![0f32; GW * GH];
    for (i, bb) in b.iter().enumerate() {
        let kk = flashes
            .iter()
            .find(|&&(fb, _)| fb == i)
            .map_or(0.0, |&(_, k)| k);
        let r2 = bb.r * bb.r;
        let ry = bb.r * bb.sq;
        let y0 = js_max(0.0, libm::floor(bb.y - ry - H0));
        let y1 = js_min(GH as f64, libm::ceil(bb.y + ry - H0));
        let x0 = js_max(0.0, libm::floor(bb.x - bb.r - W0));
        let x1 = js_min(GW as f64, libm::ceil(bb.x + bb.r - W0));
        let mut y = y0;
        while y < y1 {
            let mut x = x0;
            while x < x1 {
                let dx = x + W0 + 0.5 - bb.x;
                let dy = (y + H0 + 0.5 - bb.y) / bb.sq;
                let d2 = (dx * dx + dy * dy) / r2;
                if d2 < 1.0 {
                    let j = y as usize * GW + x as usize;
                    let u = 1.0 - d2;
                    let cc = u * u;
                    add32(&mut field[j], cc);
                    add32(&mut gx[j], (-4.0 * u * dx) / r2);
                    add32(&mut gy[j], (-4.0 * u * dy) / (r2 * bb.sq));
                    if kk != 0.0 {
                        add32(&mut lit[j], kk * (0.25 + 0.75 * cc));
                    }
                }
                x += 1.0;
            }
            y += 1.0;
        }
    }
    let rmp = [
        k[0], k[1], k[2], k[3], k[4], k[5], k[6], c[3], c[4], c[5], c[6],
    ];
    let n = k.len() as f64;
    for y in 0..GH {
        let cy = y as f64 + H0 + 0.5;
        for x in 0..GW {
            let j = y * GW + x;
            let fj = f64::from(field[j]);
            if fj < TH - 0.06 {
                continue;
            }
            let cx = x as f64 + W0 + 0.5;
            let base = BASE + 1.5 * tile_noise(cx * 0.12 + 9.0 * ph, 3.0, 9.0, 0.0, seed + 5.0);
            let ff = fj * (1.0 - smooth(base - 3.0, base, cy))
                + 0.05 * tile_noise(cx * 0.21 + 18.0 * ph, cy * 0.21, 18.0, 0.0, seed);
            if ff < TH {
                continue;
            }
            // the field's slope rounds each billow's top; height does the rest: lit above, cool in the middle,
            // dark below
            let (gxj, gyj) = (f64::from(gx[j]), f64::from(gy[j]));
            let gl = or_one(libm::hypot(gxj, gyj));
            let lam = -(gxj * 0.42 + gyj * 0.9) / gl;
            let edge = js_min(1.0, gl * 5.0) * if ff < TH + 0.08 { 1.0 } else { 0.55 };
            let h = 1.0 - 2.0 * clamp((cy - TOP) / (BASE - TOP), 0.0, 1.0);
            let mut i = 2.7
                + 2.2 * h
                + 1.3 * lam * edge
                + 0.4 * tile_noise(cx * 0.16 + 12.0 * ph, cy * 0.16, 12.0, 0.0, seed + 9.0);
            if ff < TH + 0.05 && lam > 0.3 && h > -0.5 {
                i += 1.0;
            }
            if cy > base - 6.0 {
                i -= 1.0;
            }
            i = clamp(js_round(i), 0.0, n - 1.0);
            // lit from inside: brightest deep in the billow, breaking up through the cloud's own lumps toward
            // its edges
            let l = f64::from(lit[j])
                * (0.7 + 0.6 * tile_noise(cx * 0.2 - 14.0 * ph, cy * 0.2, 14.0, 0.0, seed + 3.0));
            if l > 0.12 {
                i = js_min((rmp.len() - 1) as f64, i + js_round(l * 5.5));
            }
            p.px(cx, cy, rmp[i as usize]);
        }
    }
}

/// Scud: ragged wisps that form under the base, drift off on the wind and fray away.
fn scud(p: &mut Painter, ph: f64, seed: f64, k: &[Rgba; 7]) {
    let mut r = rng(&[seed, 13.0]);
    for w in 0..6 {
        let w = f64::from(w);
        let x0 = r.range(-60.0, 60.0);
        let y0 = BASE + r.range(3.0, 9.0);
        let rx = r.range(10.0, 20.0);
        let ry = r.range(1.3, 2.2);
        let off = r.next();
        let u = (ph + off) % 1.0;
        let a = libm::pow(libm::sin(core::f64::consts::PI * u), 0.8);
        let xc = x0 + 16.0 * (u - 0.5);
        let mut y = libm::floor(y0 - ry - 1.0);
        while y <= y0 + ry + 1.0 {
            let mut x = libm::floor(xc - rx - 1.0);
            while x <= xc + rx + 1.0 {
                let dx = (x + 0.5 - xc) / rx;
                let dy = (y + 0.5 - y0) / ry;
                let nz = tile_noise(
                    (x + 0.5) * 0.35 + 10.0 * ph + w * 7.0,
                    (y + 0.5) * 0.5,
                    10.0,
                    0.0,
                    seed + 20.0 + w,
                );
                let v = (1.0 - dx * dx - dy * dy) * a + 0.6 * nz - 0.25;
                if v > 0.0 {
                    p.blend(
                        x + 0.5,
                        y + 0.5,
                        if v > 0.4 { k[2] } else { k[3] },
                        js_min(0.6, 0.2 + v * 0.7),
                    );
                }
                x += 1.0;
            }
            y += 1.0;
        }
    }
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let (c, kc, rn, seed) = (&self.c, &self.k, &self.rn, self.seed);
        let f = libm::floor(t * FPS + 1e-6) % N;
        let ph = f / N;
        let b = billows(seed, ph);
        let mut br = rng(&[seed, 5.0, BOLT]);
        let ba = f - BOLT;
        let bx0 = BILLOWS[BOLT_B].0 + br.range(-4.0, 4.0);
        let bx1 = br.range(-24.0, 24.0);
        let striking = (0.0..BK.len() as f64).contains(&ba);
        // what's lighting the cloud this frame: discharges inside billows, and the billow the bolt leaves from
        // (a JS Map: a key keeps its first place, a later set replaces its value)
        let mut flashes: Vec<(usize, f64)> = Vec::new();
        let set =
            |fl: &mut Vec<(usize, f64)>, bi: usize, k: f64| match fl.iter_mut().find(|e| e.0 == bi)
            {
                Some(e) => e.1 = k,
                None => fl.push((bi, k)),
            };
        for &(ff, bi, k) in &FLASHES {
            if ff == f {
                set(&mut flashes, bi, k);
            }
        }
        if striking {
            let prev = flashes.iter().find(|e| e.0 == BOLT_B).map_or(0.0, |e| e.1);
            set(&mut flashes, BOLT_B, js_max(prev, BK[ba as usize]));
        }
        let lights: Vec<(f64, f64, f64, f64)> = flashes
            .iter()
            .map(|&(bi, k)| {
                let bb = b[bi];
                (
                    bb.x,
                    bb.y,
                    k,
                    bb.r * vis() * if bb.sq < 1.0 { 1.6 } else { 2.2 },
                )
            })
            .collect();
        let flash = lights.iter().fold(0.0, |s, l| js_max(s, l.2));

        // rain: slanting streaks from the belly, splashing on the floor, lit when the sky flashes
        cycle(
            t,
            P,
            150,
            seed + 11.0,
            |g| g.range(0.5, 0.62),
            |age, _life, r, _| {
                let x0 = r.range(-62.0, 62.0);
                let vy = r.range(190.0, 240.0);
                let vx = -vy * 0.18;
                let tf = (0.0 - (BASE + 4.0)) / vy;
                if age < tf {
                    let x = x0 + vx * age;
                    let y = BASE + 4.0 + vy * age;
                    let l = r.range(4.0, 6.5);
                    p.line(
                        x - vx * 0.018 * (l / 4.0),
                        y - l,
                        x,
                        y,
                        |u: f64| {
                            if u > 0.7 {
                                if flash > 0.5 { c[5] } else { rn[3] }
                            } else if flash > 0.5 {
                                c[3]
                            } else {
                                rn[if u > 0.35 { 2 } else { 1 }]
                            }
                        },
                        1.0,
                    );
                } else if age < tf + 0.12 {
                    let x = x0 + vx * tf;
                    let a = (age - tf) / 0.12;
                    for s in [-1.0, 1.0] {
                        p.px(
                            x + s * (1.0 + a * 2.5),
                            -1.0 - libm::sin(a * core::f64::consts::PI) * 2.5,
                            if a < 0.5 { rn[3] } else { rn[2] },
                        );
                    }
                    if a < 0.4 {
                        p.px(x, 0.0, rn[3]);
                    }
                }
            },
        );
        // the scud hanging under the base, the cloud, then the light of whatever burns inside it
        scud(p, ph, seed, kc);
        cloud(p, &b, ph, seed, kc, c, &flashes);
        for &(x, y, k, r) in &lights {
            p.glow(x, y, r, c[4], 0.4 * k);
        }
        p.glow(0.0, -122.0, 80.0, c[2], 0.05 + 0.22 * flash);
        // a discharge crawls along the belly now and then
        for &(ff, bi, k) in &FLASHES {
            if ff == f && k > 0.6 {
                let mut r = rng(&[seed, 7.0, ff]);
                let x = b[bi].x;
                let xa = x - r.range(12.0, 22.0);
                let ya = BASE - r.range(2.0, 6.0);
                let xb = x + r.range(12.0, 22.0);
                let yb = BASE - r.range(1.0, 5.0);
                let bolt = zigzag(&mut r, xa, ya, xb, yb, 0.25, 0.5);
                crackle(p, &bolt, 0.6, c, 0.4, 1.0);
            }
        }
        // the bolt: struck once a loop, forked, flickering, the floor lit and sparks thrown where it lands
        if striking {
            let k = BK[ba as usize];
            let bolt = zigzag(
                &mut rng(&[seed, 9.0, BOLT]),
                bx0,
                BASE - 3.0,
                bx1,
                0.0,
                0.22,
                0.7,
            );
            stroke(
                p,
                &bolt,
                k,
                c,
                if ba == 0.0 { 1.15 } else { 1.0 },
                StrokeOpts {
                    hero: ba == 0.0,
                    ..StrokeOpts::default()
                },
            );
            p.circle(bx1, -1.0, 3.0 * k + 1.5, c[2], 1.0);
            p.circle(bx1, -1.0, 3.0 * k + 0.5, c[5], 1.0);
            p.circle(bx1, -1.0, 2.0 * k, WHITE, 1.0);
            p.glow(bx1, -22.0, 78.0, c[3], 0.5 * k);
            p.glow(bx1, -3.0, 18.0, c[5], 0.45 * k);
            p.glow(bx0, BASE, 84.0, c[4], 0.3 * k);
        }
        let sa = (((t - BOLT / FPS) % P) + P) % P;
        for j in 0..16 {
            let mut q = rng(&[seed, 31.0, f64::from(j)]);
            let life = q.range(0.25, 0.6);
            if sa >= life {
                continue;
            }
            let a = -core::f64::consts::FRAC_PI_2 + q.range(-1.3, 1.3);
            let sp = q.range(60.0, 200.0);
            let (x, y, vx, vy) = flight(bx1, -1.0, libm::cos(a) * sp, libm::sin(a) * sp, sa, 0.0);
            spark(p, x, y, vx, vy, 1.0 - sa / life, &self.s);
        }
    }
}
