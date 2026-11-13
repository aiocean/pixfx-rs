use std::fs;
use std::path::{Path, PathBuf};

use pixfx_cli::parity::{ParityReport, compare, read_reference, write_side_gif};
use pixfx_core::color::WHITE;
use pixfx_core::effect::{Effect, EffectState, Layer, Meta, Options};
use pixfx_core::painter::Painter;
use pixfx_core::{Frame, Rendered, render_frames};

/// One 10 x 10 frame, every pixel the same colour and light.
fn flat(color: u32, light: u32) -> Rendered {
    let mut f = Frame::new(10, 10, 0, 0);
    f.color.fill(color);
    f.light.fill(light);
    Rendered {
        w: 10,
        h: 10,
        fps: 30,
        origin: (0, 0),
        margin: 0,
        frames: vec![f],
    }
}

#[test]
fn identical_renders_pass_with_nothing_differing() {
    let a = flat(0xff10_2030, 0x0040_5060);
    let r = compare(&a, &a);
    assert_eq!(r.colour_diff_frac, 0.0);
    assert_eq!(r.light_diff_frac, 0.0);
    assert!(r.pass());
}

#[test]
fn one_colour_pixel_of_a_hundred_differing_fails() {
    let a = flat(0xff10_2030, 0);
    let mut b = flat(0xff10_2030, 0);
    b.frames[0].color[42] = 0xff10_2031;
    let r = compare(&a, &b);
    assert_eq!(r.colour_diff_frac, 0.01);
    assert_eq!(r.light_diff_frac, 0.0);
    assert!(!r.pass());
}

#[test]
fn light_off_by_two_in_a_channel_is_not_counted() {
    let a = flat(0, 0x0040_5060);
    let mut b = flat(0, 0x0040_5060);
    b.frames[0].light[7] = 0x0042_5060;
    b.frames[0].light[8] = 0x0040_505e;
    assert_eq!(compare(&a, &b).light_diff_frac, 0.0);
}

#[test]
fn light_off_by_three_in_a_channel_is_counted() {
    let a = flat(0, 0x0040_5060);
    let mut b = flat(0, 0x0040_5060);
    b.frames[0].light[7] = 0x0040_5363;
    let r = compare(&a, &b);
    assert_eq!(r.light_diff_frac, 0.01);
    assert!(!r.pass());
}

#[test]
fn light_top_byte_is_ignored() {
    let a = flat(0, 0x0040_5060);
    let b = flat(0, 0xff40_5060);
    assert_eq!(compare(&a, &b).light_diff_frac, 0.0);
}

#[test]
fn half_a_percent_differing_still_passes() {
    // 2 frames of 100 pixels: one pixel differing is 0.005 of the render
    let mut a = flat(0, 0);
    a.frames.push(a.frames[0].clone());
    let mut b = a.clone();
    b.frames[1].color[0] = 1;
    b.frames[1].light[0] = 0x0000_0010;
    let r = compare(&a, &b);
    assert_eq!(r.colour_diff_frac, 0.005);
    assert_eq!(r.light_diff_frac, 0.005);
    assert!(r.pass());
}

#[test]
fn renders_of_different_shapes_differ_everywhere() {
    let a = flat(0, 0);
    let mut b = flat(0, 0);
    b.frames.push(b.frames[0].clone());
    let r = compare(&a, &b);
    assert_eq!(
        r,
        ParityReport {
            colour_diff_frac: 1.0,
            light_diff_frac: 1.0
        }
    );
    assert!(!r.pass());
}

/// A test effect: a white dot with a glow, one pixel further right each frame.
struct Walk;

struct WalkState {
    x: f64,
}

static WALK: Walk = Walk;
static WALK_META: Meta = Meta {
    id: "test/walk",
    name: "Walk",
    desc: "A glowing dot walking right.",
    tags: &["test"],
    size: (24, 12),
    origin: (4, 6),
    duration: 0.2,
    looping: true,
    fps: 30,
    seed: 1,
    state: None,
    options: &[],
};

impl Effect for Walk {
    fn meta(&self) -> &Meta {
        &WALK_META
    }
    fn spawn(&self, _opts: &Options, _seed: u64) -> Box<dyn EffectState> {
        Box::new(WalkState { x: 0.0 })
    }
}

impl EffectState for WalkState {
    fn update(&mut self, dt: f64) {
        self.x += dt * 30.0;
    }
    fn draw(&self, p: &mut Painter, _t: f64, layer: Layer) {
        if layer == Layer::Front {
            p.px(self.x, 0.0, WHITE);
            p.glow(self.x, 0.0, 3.0, WHITE, 1.0);
        }
    }
}

/// A fresh, empty directory for one test, per process so concurrent runs do not delete each other's; removed when
/// the test ends.
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("test")).ok();
    Scratch(dir)
}

/// Write `r` as `tools/pixelfx-ref.mjs` does: `<id>.json` and little-endian `<id>.{color,light}.bin`.
fn write_like_the_script(dir: &Path, id: &str, r: &Rendered) -> std::io::Result<()> {
    let words = |pick: fn(&Frame) -> &Vec<u32>| -> Vec<u8> {
        r.frames
            .iter()
            .flat_map(|f| pick(f).iter().flat_map(|v| v.to_le_bytes()))
            .collect()
    };
    fs::write(dir.join(format!("{id}.color.bin")), words(|f| &f.color))?;
    fs::write(dir.join(format!("{id}.light.bin")), words(|f| &f.light))?;
    let json = format!(
        "{{\"w\":{},\"h\":{},\"n\":{},\"fps\":{}}}\n",
        r.w,
        r.h,
        r.frames.len(),
        r.fps
    );
    fs::write(dir.join(format!("{id}.json")), json)
}

#[test]
fn a_reference_written_like_the_script_reads_back_as_the_same_render() -> std::io::Result<()> {
    let dir = scratch("read-reference");
    let port = render_frames(&WALK, &Options::default(), 0);
    assert_eq!(port.frames.len(), 6);
    write_like_the_script(&dir, "test/walk", &port)?;
    let reference = read_reference(&dir, "test/walk", (0, 0))?;
    assert_eq!(
        compare(&reference, &port),
        ParityReport {
            colour_diff_frac: 0.0,
            light_diff_frac: 0.0
        }
    );
    let lit = |r: &Rendered| {
        r.frames
            .iter()
            .map(|f| f.light.iter().filter(|&&l| l != 0).count())
            .sum::<usize>()
    };
    assert_eq!(lit(&reference), lit(&port));
    assert!(lit(&port) > 0);
    Ok(())
}

#[test]
fn a_reference_of_the_wrong_length_is_refused() -> std::io::Result<()> {
    let dir = scratch("short-reference");
    let port = render_frames(&WALK, &Options::default(), 0);
    write_like_the_script(&dir, "test/walk", &port)?;
    fs::write(dir.join("test/walk.light.bin"), [0u8; 12])?;
    let err = read_reference(&dir, "test/walk", (0, 0))
        .err()
        .map(|e| e.kind());
    assert_eq!(err, Some(std::io::ErrorKind::InvalidData));
    Ok(())
}

#[test]
fn the_side_by_side_gif_holds_both_renders_at_four_times_scale()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = scratch("side-gif");
    let port = render_frames(&WALK, &Options::default(), 0);
    let path = dir.join("walk.side.gif");
    write_side_gif(&path, &port, &port)?;
    let mut opts = gif::DecodeOptions::new();
    opts.set_color_output(gif::ColorOutput::RGBA);
    let mut dec = opts.read_info(fs::File::open(&path)?)?;
    assert_eq!((dec.width(), dec.height()), ((24 + 2 + 24) * 4, 12 * 4));
    let mut frames = 0;
    while dec.read_next_frame()?.is_some() {
        frames += 1;
    }
    assert_eq!(frames, 6);
    Ok(())
}
