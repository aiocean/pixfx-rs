//! A move ([`Chain`]) plays its links as pixelfx `chain` does: the expected frames are pixelfx's own, the same toy
//! effects chained and stepped in node (FNV-1a over the colour then light words of each frame).

use pixfx_core::cue::{Cue, TimedCue};
use pixfx_core::effect::{Effect, EffectState, Layer, Meta, Options, Sim};
use pixfx_core::particles::fly_v;
use pixfx_core::{Chain, Link, chain_cues};
use pixfx_draw::color::{Rgba, WHITE};
use pixfx_draw::{Frame, Painter};

const B: Rgba = Rgba::hex24(0x3cc8ff);

const fn meta(id: &'static str, duration: f64, looping: bool) -> Meta {
    Meta {
        id,
        name: id,
        desc: "",
        tags: &[],
        size: (96, 64),
        origin: (48, 32),
        duration,
        looping,
        fps: 30,
        seed: 1,
        state: None,
        options: &[],
    }
}

/// Draws a pixel, and light under it, at x = its time * 20.
struct Toy(Meta);
struct ToyState;

impl Effect for Toy {
    fn meta(&self) -> &Meta {
        &self.0
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(ToyState)
    }
    fn cues(&self, _opts: &Options) -> Vec<TimedCue> {
        vec![TimedCue {
            t: 0.1,
            cue: Cue::Ripple {
                x: 4.0,
                y: -2.0,
                r0: 0.0,
                r1: 10.0,
                dur: 0.2,
                amp: 1.0,
            },
            pan: None,
        }]
    }
}

impl EffectState for ToyState {
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Front {
            let x = libm::floor(t * 20.0);
            p.px(x, 0.0, WHITE);
            p.light(x, 1.0, B, 0.5);
        }
    }
}

static ONCE: Toy = Toy(meta("toy/once", 0.5, false));
static LOOP: Toy = Toy(meta("toy/loop", 0.2, true));

fn links() -> Vec<Link> {
    vec![
        Link::new(&ONCE, 0.1).x(5.0),
        Link::new(&LOOP, 0.3)
            .until(0.75)
            .x_by(|t| -20.0 + t * 10.0)
            .y(3.0)
            .flip(),
    ]
}

struct Move;

impl Effect for Move {
    fn meta(&self) -> &Meta {
        static META: Meta = meta("toy/move", 1.2, false);
        &META
    }
    fn spawn(&self, _opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(Chain::new(links(), seed))
    }
    fn cues(&self, opts: &Options) -> Vec<TimedCue> {
        chain_cues(&links(), opts, self.meta().duration)
    }
}

static MOVE: Move = Move;

fn fnv(f: &Frame) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for w in f.color.iter().chain(f.light.iter()) {
        for s in [0, 8, 16, 24] {
            h ^= (w >> s) & 255;
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    h
}

#[test]
fn a_move_plays_its_links_as_pixelfx() {
    #[rustfmt::skip]
    let js: [u32; 37] = [
        2802032069, 2802032069, 2802032069, 2802032069, 1709585161, 2740897961, 2740897961, 35325001, 2251582953,
        4068342013, 15718109, 298558141, 298558141, 4071859709, 1662103773, 1662103773, 1741186461, 3372311933,
        970153181, 937540937, 937540937, 846140201, 846140201, 2802032069, 2802032069, 2802032069, 2802032069,
        2802032069, 2802032069, 2802032069, 2802032069, 2802032069, 2802032069, 2802032069, 2802032069, 2802032069,
        2802032069,
    ];
    let opts = Options {
        x: 2.0,
        y: -1.0,
        seed: Some(1),
        ..Options::default()
    };
    let mut sim = Sim::new(&MOVE, opts);
    for (k, want) in js.iter().enumerate() {
        sim.seek(k as u64);
        let mut f = Frame::new(96, 64, -48, -32);
        sim.draw(&mut f, Layer::Both, sim.placement(), 1.0);
        assert_eq!(fnv(&f), *want, "frame {k}");
    }
}

#[test]
fn a_moves_cues_are_its_links_moved_to_where_and_when_they_play() {
    let cues = MOVE.cues(&Options::default());
    let at: Vec<(f64, f64, f64)> = cues
        .iter()
        .filter_map(|c| match c.cue {
            Cue::Ripple { x, y, .. } => Some((c.t, x, y)),
            _ => None,
        })
        .collect();
    // the one-shot at 0.1 + 0.1; the loop (flipped, every 0.2 s from 0.3, cut off at 0.75) at 0.4, 0.6
    let want = [
        (0.2, 9.0, -2.0),
        (0.4, -20.0 + 0.4 * 10.0 - 4.0, 1.0),
        (
            0.6000000000000001,
            -20.0 + 0.6000000000000001 * 10.0 - 4.0,
            1.0,
        ),
    ];
    assert_eq!(at.len(), want.len());
    for ((t, x, y), (wt, wx, wy)) in at.iter().zip(want) {
        assert!(
            (t - wt).abs() < 1e-12 && (x - wx).abs() < 1e-12 && (y - wy).abs() < 1e-12,
            "{at:?}"
        );
    }
}

#[test]
fn fly_v_matches_pixelfx() {
    // node: flyV(40, -60, 0.7, 950, 2.2), flyV(40, -60, 0.7, 950, 0)
    assert_eq!(
        fly_v(40.0, -60.0, 0.7, 950.0, 2.2),
        (8.575244057079118, 326.38165829818627)
    );
    assert_eq!(fly_v(40.0, -60.0, 0.7, 950.0, 0.0), (40.0, 605.0));
}
