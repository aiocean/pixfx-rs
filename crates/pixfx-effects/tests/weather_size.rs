//! Weather sized by its `width` and `height` options: the canvas they give, the frames it renders, and the
//! scene it covers. One instance as big as a scene lights it without the straight cuts that 320-pixel tiles
//! leave where a firefly's glow crosses from one tile into the next.

use std::collections::BTreeMap;

use pixfx_core::{Canvas, Effect, Frame, Instance, Layer, OptValue, Options};
use pixfx_effects::weather::{FIREFLIES, SNOW};

fn sized(width: f64, height: f64) -> Options {
    Options {
        extra: BTreeMap::from([
            ("width".to_string(), OptValue::Num(width)),
            ("height".to_string(), OptValue::Num(height)),
        ]),
        ..Options::default()
    }
}

fn weather() -> [&'static dyn Effect; 2] {
    [&FIREFLIES, &SNOW]
}

#[test]
fn without_size_options_weather_keeps_its_own_canvas() {
    for fx in weather() {
        assert_eq!(
            fx.canvas(&Options::default()),
            fx.meta().canvas(),
            "{}",
            fx.meta().id
        );
    }
}

#[test]
fn width_and_height_size_the_canvas_with_the_ground_at_its_foot() {
    for fx in weather() {
        assert_eq!(
            fx.canvas(&sized(637.0, 148.0)),
            Canvas {
                size: (637, 148),
                origin: (318, 138)
            },
            "{}",
            fx.meta().id
        );
    }
}

#[test]
fn size_options_are_whole_pixels_from_1_to_2048() {
    let width = |w: f64| FIREFLIES.canvas(&sized(w, 180.0)).size.0;
    assert_eq!(width(300.5), 301, "rounded as JS Math.round");
    assert_eq!(width(0.2), 1);
    assert_eq!(width(-50.0), 1);
    assert_eq!(width(1e9), 2048);
    assert_eq!(width(f64::NAN), 320, "not a number keeps the effect's own");
    assert_eq!(width(f64::INFINITY), 320);
}

#[test]
fn a_taller_canvas_is_hazed_to_its_top() {
    for fx in weather() {
        let opts = sized(64.0, 300.0);
        let cv = fx.canvas(&opts);
        let inst = Instance::new(
            fx,
            Options {
                x: f64::from(cv.origin.0),
                y: f64::from(cv.origin.1),
                ..opts
            },
        );
        let mut f = Frame::new(cv.size.0, cv.size.1, 0, 0);
        inst.draw(&mut f, Layer::Back, inst.placement());
        assert!(
            f.color[..64].iter().all(|c| c >> 24 != 0),
            "{}: the top row has no haze",
            fx.meta().id
        );
    }
}

/// The scene the tiles and the wide instance cover: 640 x 180, the ground 10 above the foot.
const W: u32 = 640;
const H: u32 = 180;
const GROUND: f64 = 170.0;

/// Draw `insts` (both layers) into one scene frame.
fn scene(insts: &[Instance]) -> Frame {
    let mut f = Frame::new(W, H, 0, 0);
    for i in insts {
        i.draw(&mut f, Layer::Both, i.placement());
    }
    f
}

/// The frame's light as levels, the brightest channel of each pixel.
fn levels(f: &Frame) -> Vec<i32> {
    f.light
        .iter()
        .map(|l| (l & 0xff).max((l >> 8) & 0xff).max((l >> 16) & 0xff) as i32)
        .collect()
}

/// The longest run of rows over which the light breaks between columns `x - 1` and `x`: it steps by
/// [`STEP`] levels or more there, and by at least twice the steps on either side, so it is not a glow's own
/// falloff, which changes from pixel to pixel smoothly. A glow cut along a straight vertical line breaks like
/// that down every row it lights.
fn cut(l: &[i32], x: u32) -> u32 {
    let (mut run, mut longest) = (0, 0);
    for y in 0..H {
        let step = |a: u32| {
            let i = (y * W + a) as usize;
            match (l.get(i), l.get(i - 1)) {
                (Some(p), Some(q)) => (p - q).abs(),
                _ => 0,
            }
        };
        let here = step(x);
        let breaks = here >= STEP && here >= 2 * step(x - 1).max(step(x + 1));
        run = if breaks { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    longest
}

const STEP: i32 = 6;
/// Rows of break that read as a seam.
const SEAM: u32 = 12;

#[test]
fn one_wide_instance_lights_the_scene_without_the_cuts_tiles_leave() {
    // what the landing page did: a 320-wide tile every 320 pixels, a seed of its own each
    let place = |x: f64, opts: Options, seed: u64| {
        Instance::new(
            &FIREFLIES,
            Options {
                x,
                y: GROUND,
                seed: Some(seed),
                ..opts
            },
        )
    };
    let mut tiles = [
        place(160.0, Options::default(), 160),
        place(480.0, Options::default(), 480),
    ];
    let mut wide = [place(320.0, sized(f64::from(W), H.into()), 1)];
    let (mut tile_seam, mut wide_seam) = (0, (0, 0));
    // a half-second at a time through one loop
    for _ in 0..24 {
        for i in tiles.iter_mut().chain(wide.iter_mut()) {
            i.update(0.25);
            i.update(0.25);
        }
        tile_seam = tile_seam.max(cut(&levels(&scene(&tiles)), 320));
        let l = levels(&scene(&wide));
        for x in 2..W - 1 {
            let run = cut(&l, x);
            if run > wide_seam.0 {
                wide_seam = (run, x);
            }
        }
    }
    assert!(
        tile_seam >= SEAM,
        "the tiles' join shows no cut ({tile_seam} rows), so this test cannot see one"
    );
    assert!(
        wide_seam.0 < SEAM,
        "the wide instance's light is cut over {} rows at x {}",
        wide_seam.0,
        wide_seam.1
    );
}
