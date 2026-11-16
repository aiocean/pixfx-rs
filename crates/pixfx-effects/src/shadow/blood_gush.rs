//! Blood gush: a wound spurts blood in three heartbeats; each spurt arcs out, falls, splatters and runs together
//! into a thick pool that spreads slowly, darkens and soaks into the ground. Made here, on the MLS-MPM liquid
//! (`pixfx_core::fluid::Mpm`) with a high viscosity, so the blood strings, clings and pools instead of bouncing
//! like water. Painted as one body with `pixfx_draw::blob`: a wet highlight on top, a near-black outline, the
//! body, darker where it is thick. Its golden hashes pin it.

use pixfx_core::cue::{FEEL, TimedCue, impact};
use pixfx_core::fluid::{Floor, Mpm, MpmOpts};
use pixfx_core::{Effect, EffectState, Layer, Meta, NumOpt, Options};
use pixfx_draw::Painter;
use pixfx_draw::blob::{Blob, BlobLook};
use pixfx_draw::color::{Rgba, mix};
use pixfx_draw::math::{Rng, rand, smooth, to_uint32};

pub struct BloodGush;

pub static BLOOD_GUSH: BloodGush = BloodGush;

/// Where the wound is, in local pixels from the feet: by default the right side at chest height (the fighter is 40
/// rows tall, `RULER.height`; the chest is about 26 up).
const HIT_X: NumOpt = NumOpt::new("hitX", 6.0).within(-40.0, 40.0);
const HIT_Y: NumOpt = NumOpt::new("hitY", -26.0).within(-60.0, 0.0);
/// How much blood: 1 is three spurts of about a hundred particles each.
const AMOUNT: NumOpt = NumOpt::new("amount", 1.0).within(0.3, 2.0);

const META: Meta = Meta {
    id: "shadow/blood-gush",
    name: "Blood gush",
    desc: "A wound spurts blood in three heartbeats; it arcs out, splatters and runs together into a thick pool that spreads, darkens and soaks into the ground. Simulated viscous liquid.",
    tags: &["blood", "liquid", "hit", "gore", "one-shot"],
    size: (200, 110),
    origin: (60, 100),
    duration: 2.6,
    looping: false,
    fps: 30,
    seed: 5,
    state: None,
    options: &[HIT_X.spec(), HIT_Y.spec(), AMOUNT.spec(), FEEL.spec()],
};

/// Deep to light: outline, thick, body, wet highlight, then the dried stain it darkens toward.
const BLOOD: [Rgba; 5] = [
    Rgba::hex24(0x22030a),
    Rgba::hex24(0x5c0a14),
    Rgba::hex24(0x9a1424),
    Rgba::hex24(0xe8505a),
    Rgba::hex24(0x2e0910),
];

/// The spurts: start, length (s), speed (px/s). Each heartbeat is weaker than the last.
const SPURTS: [(f64, f64, f64); 3] = [(0.0, 0.14, 105.0), (0.26, 0.12, 88.0), (0.5, 0.1, 70.0)];
/// Particles a second while a spurt runs, at amount 1.
const RATE: f64 = 2200.0;
/// Upward tilt of a spurt (radians above the horizontal).
const LIFT: f64 = 0.7;
const SOAK_AT: f64 = 1.7;
const SOAK_FOR: f64 = 0.85;

impl Effect for BloodGush {
    fn meta(&self) -> &Meta {
        &META
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        impact(0.0, 0.5, opts.num(&HIT_X), opts.num(&HIT_Y), opts)
    }
    fn spawn(&self, opts: &Options, seed: u64) -> Box<dyn EffectState> {
        let (ox, oy) = (f64::from(META.origin.0), f64::from(META.origin.1));
        let cell = 2.0;
        let mpm = Mpm::new(MpmOpts {
            origin: (-ox, -oy),
            cell,
            cols: (f64::from(META.size.0) / cell) as usize,
            rows: ((oy + 6.0) / cell) as usize,
            bulk: 30_000.0,
            viscosity: 10.0,
            gravity: 340.0,
            floor: Some(Floor {
                y: 0.0,
                restitution: 0.0,
                friction: 0.6,
            }),
            ..MpmOpts::default()
        });
        let ramp = opts.palette_or(&BLOOD);
        let at = |i: usize| {
            ramp.get(i.min(ramp.len().saturating_sub(1)))
                .copied()
                .unwrap_or(BLOOD[2])
        };
        Box::new(State {
            mpm,
            t: 0.0,
            owed: 0.0,
            next: 0,
            rng: Rng::new(to_uint32(seed as f64)),
            seed: seed as f64,
            wound: (opts.num(&HIT_X), opts.num(&HIT_Y)),
            amount: opts.num(&AMOUNT),
            fresh: [at(0), at(1), at(2), at(3)],
            dry: at(4),
        })
    }
}

struct State {
    mpm: Mpm,
    t: f64,
    owed: f64,
    next: u32,
    rng: Rng,
    seed: f64,
    wound: (f64, f64),
    amount: f64,
    fresh: [Rgba; 4],
    dry: Rgba,
}

impl State {
    /// The spurt running at `t`, as its speed and how far into it (0..1), or none between heartbeats.
    fn spurt(t: f64) -> Option<(f64, f64)> {
        SPURTS
            .iter()
            .find(|(at, len, _)| t >= *at && t < at + len)
            .map(|(at, len, speed)| (*speed, (t - at) / len))
    }
}

impl EffectState for State {
    fn update(&mut self, dt: f64) {
        if dt.is_nan() || dt <= 0.0 {
            return;
        }
        if let Some((speed, k)) = Self::spurt(self.t) {
            self.owed += RATE * self.amount * dt;
            // a spurt opens hard and fades
            let push = speed * (1.0 - 0.35 * k);
            while self.owed >= 1.0 {
                self.owed -= 1.0;
                let a = -LIFT + self.rng.range(-0.14, 0.14);
                let v = push * self.rng.range(0.85, 1.1);
                let (jx, jy) = (self.rng.range(-1.2, 1.2), self.rng.range(-1.2, 1.2));
                let (sa, ca) = (libm::sin(a), libm::cos(a));
                let tag = self.next;
                self.next += 1;
                self.mpm
                    .add(self.wound.0 + jx, self.wound.1 + jy, ca * v, sa * v, tag);
            }
        }
        self.t += dt;
        self.mpm.step(dt);
        let (t, seed) = (self.t, self.seed);
        self.mpm
            .particles_mut()
            .retain(|q| t < SOAK_AT + SOAK_FOR * rand(&[seed, f64::from(q.tag)]));
    }

    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Back {
            return;
        }
        let parts = self.mpm.particles();
        let Some(first) = parts.first() else { return };
        const R: f64 = 2.4;
        let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
        for q in parts {
            x0 = x0.min(q.x);
            y0 = y0.min(q.y);
            x1 = x1.max(q.x);
            y1 = y1.max(q.y);
        }
        let mut blob = Blob::new(x0 - R, y0 - R, x1 + R + 1.0, y1 + R + 1.0);
        for q in parts {
            blob.splat(q.x, q.y, R, 1.0);
        }
        // fresh blood is wet and bright; as it pools it darkens toward a dried stain
        let dry = 0.6 * smooth(1.0, 2.4, self.t);
        let [edge, deep, body, rim] = self.fresh;
        let look = BlobLook {
            edge,
            deep: mix(deep, self.dry, dry),
            body: mix(body, self.dry, dry),
            rim: mix(rim, body, dry),
            iso: 0.6,
            deep_at: 2.6,
            shine: 0.12 * (1.0 - dry),
        };
        blob.paint(p, &look);
    }
}
