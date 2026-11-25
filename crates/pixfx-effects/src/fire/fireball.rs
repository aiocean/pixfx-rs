//! Fireball: port of pixelfx `src/fx/fire/fireball.js`.
//!
//! The flame-puff, smoke and spark recipe is adapted from Sprite Fusion's open-source explosion
//! (https://www.spritefusion.com/pixel-art-vfx/explosion), MIT License, Copyright (c) 2026 Hugo Duprez.

use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::color::{Rgba, WHITE, mix};
use pixfx_draw::math::{TAU, js_max, js_min, js_sign};

use super::lib::{
    Hot, LAYERS, SMOKE1, drift, ember, fire_puff, flame_blade, hot_first, looped, nl, pnoise,
    smoke_puff, spark,
};

pub struct Fireball;

pub static FIREBALL: Fireball = Fireball;

const RADIUS: NumOpt = NumOpt::new("radius", 5.0);
const SPEED: NumOpt = NumOpt::new("speed", 120.0);
const META: Meta = Meta {
    id: "fire/fireball",
    name: "Fireball",
    desc: "A fireball flying right: a churning white-hot head with a corona of flame, a long torn tail of flame puffs cooling into smoke, embers shed behind.",
    tags: &[
        "fire",
        "fireball",
        "projectile",
        "magic",
        "directional",
        "loop",
    ],
    size: (136, 56),
    origin: (112, 28),
    duration: 1.2,
    looping: true,
    fps: 30,
    seed: 5,
    state: None,
    options: &[RADIUS.spec(), SPEED.spec()],
};

const PALETTE: [Rgba; 9] = [
    Rgba::hex24(0x4a1712),
    Rgba::hex24(0x8a1f17),
    Rgba::hex24(0xc9281f),
    Rgba::hex24(0xf2561d),
    Rgba::hex24(0xff8a2a),
    Rgba::hex24(0xffc23d),
    Rgba::hex24(0xffe26b),
    Rgba::hex24(0xfff3a6),
    Rgba::hex24(0xffffff),
];

const D: f64 = 1.2;
// the comet body: tongues rooted in the head streaming back, the longest behind (length, half-width, y offset)
const BODY: [(f64, f64, f64); 4] = [
    (30.0, 4.2, 0.0),
    (23.0, 4.4, -1.6),
    (22.0, 4.4, 1.6),
    (15.0, 4.6, 0.0),
];
// the corona: thin licks peeling back off the head (angle on the head (0 is the nose), length, half-width)
const CORONA: [(f64, f64, f64); 6] = [
    (-1.3, 6.0, 1.1),
    (-1.8, 8.0, 1.2),
    (-2.3, 7.0, 1.1),
    (1.3, 6.0, 1.1),
    (1.8, 8.0, 1.2),
    (2.3, 7.0, 1.1),
];

impl Effect for Fireball {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(State {
            h: hot_first(&opts.palette_or(&PALETTE)),
            radius: opts.num(&RADIUS),
            speed: opts.num(&SPEED),
            seed: seed as f64,
        })
    }
}

struct State {
    h: Hot,
    radius: f64,
    speed: f64,
    seed: f64,
}

impl EffectState for State {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer != Layer::Back {
            draw(p, t, self);
        }
    }
}

// the head: a ball whose outline churns and whose heat swirls in a spiral, white at the heart, swept back
fn head(p: &mut Painter, h: &Hot, t: f64, r0: f64, s: f64, hx: f64, hy: f64) {
    let ph = t / D;
    p.shade(
        hx - r0 * 2.4,
        hy - r0 * 1.7,
        hx + r0 * 1.5,
        hy + r0 * 1.7,
        |x, y, _| {
            let (x, y) = (x - hx, y - hy);
            let d = libm::hypot(x, y);
            let th = libm::atan2(y, x);
            let back = js_max(0.0, -libm::cos(th));
            let a = th / TAU;
            let bump = 0.55 * pnoise(a * 9.0 + ph * 9.0, 9.0, s)
                + 0.35 * pnoise(a * 5.0 - ph * 10.0, 5.0, s + 1.0);
            let rim = r0 * (1.0 + 0.55 * back * back) + bump;
            if d > rim {
                return None;
            }
            let swirl = pnoise(a * 5.0 - ph * 10.0 + d * 0.45, 5.0, s + 2.0);
            let hv = 1.0 - d / rim + 0.26 * swirl + 0.1 * (1.0 - back);
            // the leading edge: a crisp hot rim where it cuts the air
            if rim - d < 1.0 && back < 0.3 {
                return Some(if hv > 0.3 { h[3] } else { h[5] });
            }
            Some(if hv > 0.74 {
                h[0]
            } else if hv > 0.58 {
                h[1]
            } else if hv > 0.42 {
                h[2]
            } else if hv > 0.27 {
                h[4]
            } else if hv > 0.13 {
                h[5]
            } else {
                h[6]
            })
        },
    );
}

struct Puff {
    x: f64,
    y: f64,
    v: f64,
    vy: f64,
    fl: f64,
    sm: bool,
    sl: f64,
    s: f64,
    f: f64,
}

fn draw(p: &mut Painter, t: f64, o: &State) {
    let (h, s, r0, v) = (&o.h, o.seed, o.radius, o.speed);
    let n = |rate: f64, seed: f64, off: f64| nl(D, t, rate, seed, off);
    let glow = mix(mix(h[3], h[4], 0.5), WHITE, 0.25);
    let smoke = mix(SMOKE1, h[6], 0.3);
    let sparks = [h[1], h[2], h[4], h[5]];
    let emb = [h[1], h[2], h[4], h[6]];
    // the head bobs a hair as it flies
    let hx = n(3.0, s + 50.0, 0.0) * 0.5;
    let hy = n(2.5, s + 51.0, 0.0) * 0.7;

    // the tail: flame puffs thrown off the back that the head leaves behind; they cool, swell, rise and some
    // burn out into smoke. Positions are relative to the head, which outruns them.
    let mut fire: Vec<(f64, f64, f64, f64, f64, f64)> = Vec::new();
    let mut smokes: Vec<(f64, f64, f64, f64, f64)> = Vec::new();
    looped(
        t,
        D,
        130.0,
        s + 100.0,
        |r, _| {
            let fl = r.range(0.22, 0.55);
            let sm = r.chance(0.55);
            let x = -r.range(2.0, 24.0);
            let q = Puff {
                x,
                y: r.range(-1.0, 1.0) * (r0 - 0.1 * -x),
                v: v * r.range(0.3, 0.6),
                vy: r.range(-16.0, 16.0),
                fl,
                sm,
                sl: r.range(0.25, 0.45),
                s: r.range(1.3, 2.9),
                f: r.next() * 0.5 + 0.5 * (-x / 24.0),
            };
            (fl + if sm { 0.45 } else { 0.0 }, q)
        },
        |q, age, _| {
            let ta = js_min(age, q.fl);
            let (wx, y, wvx, vy) = drift(0.0, q.y, q.v, q.vy, ta, 0.0, -70.0, 4.5, 3.0);
            let x = q.x + wx - v * ta;
            if age < q.fl {
                fire.push((age, x, y, q.fl, q.s, q.f));
                return;
            }
            if !q.sm || age - q.fl > q.sl {
                return;
            }
            let sa = age - q.fl;
            let (sx, sy, _, _) = drift(x, y, wvx - v, vy * 0.6 - 6.0, sa, v * 2.0, -30.0, 2.0, 1.2);
            smokes.push((sa, sx, sy, q.sl, q.s));
        },
    );
    for &(sa, x, y, sl, sz) in &smokes {
        smoke_puff(p, x, y, 1.0 - sa / sl, sz * 0.9, Some(smoke), false, 1.0);
    }
    // oldest first (JS's stable sort by age, descending)
    fire.sort_by(|a, b| b.0.total_cmp(&a.0));
    for &(age, x, y, fl, sz, f) in &fire {
        let u = age / fl;
        fire_puff(
            p,
            x,
            y,
            1.0 - u,
            sz * (1.0 + 1.3 * u),
            f,
            h,
            0.07,
            -0.86,
            -0.5,
        );
    }

    // the body: tongues of flame rooted in the head streaming back, and a corona licking round it
    for (i, &(len, w, dy)) in BODY.iter().enumerate() {
        let nn = (s * 5.0 + i as f64) * 17.0;
        let l = len
            * (0.82 + 0.25 * n(4.0, nn, 0.0))
            * (1.0 + 0.3 * js_max(0.0, n(2.4, nn + 3.0, 0.0)));
        flame_blade(
            p,
            hx + 1.0,
            hy + dy * 0.5,
            -1.0,
            dy * 0.02,
            l,
            w * r0 / 5.0,
            n(3.0, nn + 5.0, 0.0) * 3.0 + dy,
            &n,
            nn,
            h,
            &LAYERS,
            0.5,
            1.1 + l * 0.05,
        );
    }
    for (i, &(a, len, w)) in CORONA.iter().enumerate() {
        let nn = (s * 5.0 + i as f64 + 20.0) * 17.0;
        let l = len * (0.7 + 0.4 * (0.5 + 0.5 * n(6.0, nn, 0.0)));
        let ox = libm::cos(a) * r0 * 0.85;
        let oy = libm::sin(a) * r0 * 0.85;
        let dx = libm::cos(a) * 0.55 - 1.0;
        let dy = libm::sin(a) * 0.55;
        let ln = libm::hypot(dx, dy);
        flame_blade(
            p,
            hx + ox,
            hy + oy,
            dx / ln,
            dy / ln,
            l,
            w,
            -js_sign(a) * 1.5,
            &n,
            nn,
            h,
            &LAYERS,
            0.5,
            1.1,
        );
    }
    head(p, h, t, r0, s, hx, hy);

    // motes flicked off the head's skin: short hot streaks that die at once
    looped(
        t,
        D,
        26.0,
        s + 200.0,
        |r, _| {
            let a = r.range(-2.4, 2.4);
            let vm = r.range(30.0, 70.0);
            let life = r.range(0.08, 0.18);
            (life, (a, vm, life))
        },
        |&(a, vm, life), age, _| {
            let (ca, sa) = (libm::cos(a), libm::sin(a));
            let d = r0 * 0.9 + vm * age;
            let x = hx + ca * d - v * 0.4 * age;
            let y = hy + sa * d;
            let k = 1.0 - age / life;
            let l = 1.0 + 2.0 * k;
            p.line(
                x - ca * l + v * 0.02,
                y - sa * l,
                x,
                y,
                if k > 0.5 { h[1] } else { h[3] },
                1.0,
            );
            p.glow(x, y, 3.0, h[2], 0.4 * k);
        },
    );
    // embers left behind: streaks that the head outruns, falling as they cool
    looped(
        t,
        D,
        30.0,
        s + 300.0,
        |r, _| {
            let x = -r.range(2.0, 16.0);
            let y = r.range(-3.0, 3.0);
            let vm = v * r.range(0.3, 0.8);
            let vy = r.range(-20.0, 20.0);
            let life = r.range(0.25, 0.55);
            let big = r.chance(0.25);
            let ph = r.range(0.0, 6.3);
            (life, (x, y, vm, vy, life, big, ph))
        },
        |&(qx, qy, vm, qvy, life, big, ph), age, _| {
            let (wx, y, wvx, vy) = drift(0.0, qy, vm, qvy, age, 0.0, 50.0, 3.0, 2.0);
            let x = qx + wx - v * age;
            let k = 1.0 - age / life;
            if big {
                ember(p, x, y, k, 2.0, &emb, age * 40.0 + ph);
            } else {
                spark(p, x, y, wvx - v, vy, k, &sparks);
            }
        },
    );

    // light: a hot heart, a warm halo, light running down the tail
    let fl = 0.85 + 0.15 * n(9.0, s + 7.0, 0.0);
    p.glow_line(hx - 6.0, hy, -50.0, -2.0, 12.0, glow, 0.2 * fl);
    p.glow(hx - 2.0, hy, 42.0, glow, 0.42 * fl);
    p.glow(hx, hy, 13.0, h[1], 0.3 * fl);
}
