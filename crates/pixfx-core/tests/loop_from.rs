//! A looping effect that opens once (`Effect::loop_from`): its opening frames play once, then only the frames
//! after it repeat.

use pixfx_core::{Clock, Effect, EffectState, Layer, Meta, Options};
use pixfx_draw::Painter;

struct Opens;

static OPENS: Opens = Opens;

const META: Meta = Meta {
    id: "test/opens",
    name: "Opens",
    desc: "Opens for 0.3 s, then loops 0.2 s.",
    tags: &[],
    size: (4, 4),
    origin: (2, 2),
    duration: 0.5,
    looping: true,
    fps: 10,
    seed: 1,
    state: None,
    options: &[],
};

struct Nothing;

impl EffectState for Nothing {
    fn draw(&self, _p: &mut Painter, _t: f64, _layer: Layer) {}
}

impl Effect for Opens {
    fn meta(&self) -> &Meta {
        &META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(Nothing)
    }
    fn loop_from(&self) -> Option<f64> {
        Some(0.3)
    }
}

#[test]
fn the_opening_plays_once_and_only_the_rest_loops() {
    let mut clock = Clock::new(&OPENS, &Options::default());
    let mut shown = vec![clock.frame()];
    for _ in 0..11 {
        clock.update(0.1);
        shown.push(clock.frame());
    }
    // frames 0 to 2 open; 3 and 4 loop
    assert_eq!(shown, [0, 1, 2, 3, 4, 3, 4, 3, 4, 3, 4, 3]);
    assert!(!clock.done());
}
