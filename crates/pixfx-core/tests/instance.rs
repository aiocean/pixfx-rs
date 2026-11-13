//! Effect instances and frame rendering against pixelfx `src/core/fx.js` (`createFx`, `resolveOptions`) and
//! `tools/lib/fxlib.mjs` (`renderFrames`, `edgeFade`).

use pixfx_core::color::{Rgba, WHITE, fit};
use pixfx_core::effect::{Effect, EffectState, Instance, Layer, Meta, OptValue, Options, find};
use pixfx_core::frame::Frame;
use pixfx_core::painter::Painter;
use pixfx_core::render::render_frames;

/// A test effect: a dot that moves 1 px right per simulation sub-step (row 0), and a dot at x = floor(t * 10)
/// (row 1) showing the time the instance hands to `draw`.
struct Dot {
    meta: Meta,
}

struct DotState {
    steps: f64,
}

impl Effect for Dot {
    fn meta(&self) -> &Meta {
        &self.meta
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(DotState { steps: 0.0 })
    }
}

impl EffectState for DotState {
    fn update(&mut self, _dt: f64) {
        self.steps += 1.0;
    }
    fn draw(&self, p: &mut Painter, t: f64, layer: Layer) {
        if layer == Layer::Front {
            p.px(self.steps, 0.0, WHITE);
            p.px(libm::floor(t * 10.0), 1.0, WHITE);
        }
    }
}

const fn dot_meta(id: &'static str, looping: bool) -> Meta {
    Meta {
        id,
        name: "Dot",
        desc: "A dot that moves one pixel per simulation step.",
        tags: &["test"],
        size: (64, 4),
        origin: (0, 0),
        duration: 1.0,
        looping,
        fps: 30,
        seed: 5,
        state: None,
    }
}

static DOT: Dot = Dot {
    meta: dot_meta("test/dot", false),
};
static DOT_LOOP: Dot = Dot {
    meta: dot_meta("test/dot-loop", true),
};

/// Where the lit pixel of `row` is, if any.
fn lit(frame: &Frame, row: u32) -> Option<u32> {
    (0..frame.w).find(|&x| frame.color[(row * frame.w + x) as usize] != 0)
}

fn drawn(inst: &Instance) -> Frame {
    let mut frame = Frame::new(64, 4, 0, 0);
    inst.draw(&mut frame, Layer::Both);
    frame
}

#[test]
fn update_substeps() {
    let mut inst = Instance::new(&DOT_LOOP, Options::default());
    inst.update(1.0 / 30.0);
    // max(1, ceil(1/30 * 120 - 1e-9)) = 4 sub-steps
    assert_eq!(lit(&drawn(&inst), 0), Some(4));
    assert_eq!(inst.t(), 1.0 / 30.0);
    inst.update(0.0);
    assert_eq!(
        lit(&drawn(&inst), 0),
        Some(5),
        "a zero step still runs one sub-step"
    );
}

#[test]
fn huge_dt_is_clamped() {
    let mut inst = Instance::new(&DOT_LOOP, Options::default());
    inst.update(30.0);
    // clamped to MAX_DT 0.25: 30 sub-steps, not 3600
    assert_eq!(lit(&drawn(&inst), 0), Some(30));
    assert_eq!(inst.t(), 0.25);
}

#[test]
fn bad_steps_are_ignored() {
    use pixfx_core::effect::valid_dt;
    let mut inst = Instance::new(&DOT_LOOP, Options::default());
    inst.update(0.1);
    let before = drawn(&inst);
    for dt in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.01,
        -f64::MIN_POSITIVE,
    ] {
        assert!(!valid_dt(dt), "dt {dt}");
        inst.update(dt);
        assert_eq!(inst.t(), 0.1, "dt {dt} moved time");
        assert_eq!(drawn(&inst), before, "dt {dt} ran a sub-step");
    }
    assert!(valid_dt(0.0) && valid_dt(0.25) && valid_dt(1e9));
    inst.update(0.1);
    assert_eq!(inst.t(), 0.2, "a good step after bad ones still advances");
}

#[test]
fn non_loop_done_after_duration() {
    let mut inst = Instance::new(&DOT, Options::default());
    for _ in 0..3 {
        inst.update(0.25);
    }
    assert!(!inst.done());
    inst.update(0.25);
    assert!(inst.done());
    inst.update(0.25);
    // a one-shot holds its last moment: time handed to draw stops at the duration
    assert_eq!(lit(&drawn(&inst), 1), Some(10));
}

#[test]
fn loop_wraps_time() {
    let mut inst = Instance::new(&DOT_LOOP, Options::default());
    for _ in 0..5 {
        inst.update(0.25);
    }
    assert!(!inst.done(), "a loop never ends on its own");
    assert_eq!(inst.t(), 1.25);
    // draw sees t % duration = 0.25
    assert_eq!(lit(&drawn(&inst), 1), Some(2));
}

#[test]
fn stop_fades_out_over_fade_time() {
    let mut inst = Instance::new(&DOT_LOOP, Options::default());
    inst.update(0.25);
    inst.stop();
    inst.update(0.25);
    assert!(!inst.done());
    inst.stop(); // a second stop keeps the first stop time
    inst.update(0.05);
    assert!(inst.done(), "done 0.3 s after stop");
    assert_eq!(lit(&drawn(&inst), 0), None, "faded out completely");
}

#[test]
fn layers_split_back_and_front() {
    let mut inst = Instance::new(&DOT, Options::default());
    inst.update(0.25);
    let mut back = Frame::new(64, 4, 0, 0);
    inst.draw(&mut back, Layer::Back);
    assert_eq!(lit(&back, 0), None, "the dot draws only in front");
    let mut front = Frame::new(64, 4, 0, 0);
    inst.draw(&mut front, Layer::Front);
    assert_eq!(front, drawn(&inst));
}

#[test]
fn render_frame_count() {
    // pixelfx n = max(1, round(duration * fps) + (loop ? 0 : 1))
    let one_shot = render_frames(&DOT, &Options::default(), 0);
    assert_eq!(one_shot.frames.len(), 31);
    let looped = render_frames(&DOT_LOOP, &Options::default(), 0);
    assert_eq!(looped.frames.len(), 30);
    assert_eq!(
        (looped.w, looped.h, looped.fps, looped.margin),
        (64, 4, 30, 0)
    );
    // frame i is drawn after i steps of 1/fps: 4 sub-steps each
    assert_eq!(lit(&looped.frames[0], 0), Some(0));
    assert_eq!(lit(&looped.frames[3], 0), Some(12));
}

#[test]
fn render_margin_grows_the_frame() {
    let r = render_frames(&DOT, &Options::default(), 3);
    assert_eq!((r.w, r.h, r.margin), (70, 10, 3));
    let f = &r.frames[1];
    assert_eq!((f.ox, f.oy), (-3, -3));
    // origin (0, 0) sits 3 px in; after one frame the dot has moved 4 px
    assert_eq!(lit(f, 3), Some(7));
}

/// Light everywhere, to see the edge fade.
struct Glow {
    meta: Meta,
}

struct GlowState;

impl Effect for Glow {
    fn meta(&self) -> &Meta {
        &self.meta
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(GlowState)
    }
}

impl EffectState for GlowState {
    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Front {
            for y in -4..184 {
                for x in -4..324 {
                    p.add(f64::from(x), f64::from(y), Rgba(200, 100, 50, 255), 1.0);
                }
            }
        }
    }
}

const fn glow_meta(size: (u32, u32)) -> Meta {
    Meta {
        id: "test/glow",
        name: "Glow",
        desc: "Light over the whole canvas.",
        tags: &["test"],
        size,
        origin: (0, 0),
        duration: 1.0,
        looping: true,
        fps: 2,
        seed: 1,
        state: None,
    }
}

static GLOW: Glow = Glow {
    meta: glow_meta((24, 20)),
};
static GLOW_SCREEN: Glow = Glow {
    meta: glow_meta((320, 180)),
};

const FULL: u32 = 0xff3264c8;

#[test]
fn render_fades_light_at_the_edges() {
    let r = render_frames(&GLOW, &Options::default(), 0);
    let f = &r.frames[0];
    // (x, y, light) from pixelfx edgeFade (E = 8) on a 24 x 20 frame of light 0xff3264c8
    let expected = [
        (0, 0, 0xff010102),
        (3, 0, 0xff010102),
        (5, 4, 0xff1e3b77),
        (9, 9, 0xff3264c8),
        (10, 9, 0xff3264c8),
        (7, 9, 0xff3163c6),
        (8, 8, 0xff3264c8),
        (16, 10, 0xff3163c6),
        (15, 10, 0xff3264c8),
        (23, 19, 0xff010102),
        (12, 2, 0xff0c172e),
    ];
    for (x, y, l) in expected {
        assert_eq!(f.light[y * 24 + x], l, "light at ({x}, {y})");
    }
    // no fade with a margin or for a full-screen effect
    assert!(
        render_frames(&GLOW, &Options::default(), 1).frames[0]
            .light
            .iter()
            .all(|&l| l == FULL)
    );
    assert!(
        render_frames(&GLOW_SCREEN, &Options::default(), 0).frames[0]
            .light
            .iter()
            .all(|&l| l == FULL)
    );
}

#[test]
fn render_places_the_effect_at_its_origin() {
    let opts = Options {
        x: 20.0,
        y: 2.0,
        ..Options::default()
    };
    // pixelfx renderFrames sets x, y to the origin whatever the instance options say
    let r = render_frames(&DOT, &opts, 0);
    assert_eq!(lit(&r.frames[0], 0), Some(0));
}

/// The seed an effect is spawned with: the caller's, else the effect's default.
struct SeedProbe {
    meta: Meta,
}

struct SeedState {
    seed: u64,
}

impl Effect for SeedProbe {
    fn meta(&self) -> &Meta {
        &self.meta
    }
    fn spawn(&self, _opts: &Options, seed: u64) -> Box<dyn EffectState> {
        Box::new(SeedState { seed })
    }
}

impl EffectState for SeedState {
    fn draw(&self, p: &mut Painter, _t: f64, _layer: Layer) {
        p.px(self.seed as f64, 0.0, WHITE);
    }
}

static SEED_PROBE: SeedProbe = SeedProbe {
    meta: dot_meta("test/seed", false),
};

#[test]
fn seed_defaults_to_the_effects_own() {
    let inst = Instance::new(&SEED_PROBE, Options::default());
    assert_eq!(lit(&drawn(&inst), 0), Some(5));
    let opts = Options {
        seed: Some(9),
        ..Options::default()
    };
    assert_eq!(lit(&drawn(&Instance::new(&SEED_PROBE, opts)), 0), Some(9));
}

#[test]
fn options_resolve_like_pixelfx() {
    let def = [
        Rgba(1, 0, 0, 255),
        Rgba(2, 0, 0, 255),
        Rgba(3, 0, 0, 255),
        Rgba(4, 0, 0, 255),
    ];
    let two = vec![Rgba(10, 0, 0, 255), Rgba(20, 0, 0, 255)];
    let four = vec![Rgba(9, 9, 9, 255); 4];
    let mut opts = Options {
        palette: Some(two.clone()),
        ..Options::default()
    };
    opts.extra
        .insert("wood".into(), OptValue::Ramp(four.clone()));
    opts.extra
        .insert("short".into(), OptValue::Ramp(two.clone()));
    opts.extra.insert("height".into(), OptValue::Num(30.0));
    // a ramp of another length is resampled to the default's length
    assert_eq!(opts.palette_or(&def), fit(&two, 4));
    assert_eq!(opts.ramp_or("short", &def), fit(&two, 4));
    assert_eq!(opts.ramp_or("wood", &def), four);
    assert_eq!(opts.ramp_or("iron", &def), def);
    assert_eq!(Options::default().palette_or(&def), def);
    assert_eq!(opts.num_or("height", 22.0), 30.0);
    assert_eq!(opts.num_or("depth", 22.0), 22.0);
    assert_eq!(Options::default().light, 1.0);
}

#[test]
fn color_option_resolves_to_the_set_colour_or_the_default() {
    let def = Rgba(74, 63, 105, 255);
    let red = Rgba(255, 0, 0, 255);
    let mut opts = Options::default();
    opts.extra.insert("stone".into(), OptValue::Color(red));
    opts.extra.insert(
        "ramp".into(),
        OptValue::Ramp(vec![red, Rgba(0, 0, 255, 255)]),
    );
    opts.extra.insert("one".into(), OptValue::Ramp(vec![red]));
    opts.extra.insert("num".into(), OptValue::Num(3.0));
    assert_eq!(opts.color_or("stone", def), red);
    // unset, or set to another kind of value (a ramp of any length is not a colour): the default
    assert_eq!(opts.color_or("unset", def), def);
    assert_eq!(opts.color_or("ramp", def), def);
    assert_eq!(opts.color_or("one", def), def);
    assert_eq!(opts.color_or("num", def), def);
}

#[test]
fn screen_is_16_by_9_and_at_least_320_wide() {
    assert!(GLOW_SCREEN.meta().is_screen());
    assert!(!GLOW.meta().is_screen());
    assert!(!glow_meta((160, 90)).is_screen(), "16:9 but too small");
}

#[test]
fn find_looks_effects_up_by_id() {
    let list: &[&'static dyn Effect] = &[&DOT, &DOT_LOOP];
    assert_eq!(
        find(list, "test/dot-loop").map(|e| e.meta().id),
        Some("test/dot-loop")
    );
    assert!(find(list, "test/nope").is_none());
    assert!(find(&[], "test/dot").is_none());
}

#[test]
fn glow_margin_is_half_the_longer_side() {
    // 64 x 4 canvas: the longer side decides
    assert_eq!(dot_meta("test/dot", true).glow_margin(), 32);
    let tall = Meta {
        size: (30, 129),
        ..dot_meta("test/tall", true)
    };
    assert_eq!(tall.glow_margin(), 64);
}
