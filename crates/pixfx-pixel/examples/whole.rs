//! The whole effect, start to end, from a reference drawn a sheet per moment (a spec with layers): every moment's
//! whole frames in order as one gif, and a contact strip to read them at a glance.
//!
//!   cargo run -p pixfx-pixel --example whole -- <spec.json> [--out DIR] [--scale N]
//!
//! Reads `target/ref/<slug>/<moment>/review/<moment>-<k>.rebuilt.png` (what `extract --moment` writes) for every
//! moment of the spec, in its order. Writes into `--out` (default `target/ref/<slug>`): `whole.gif` (each frame
//! drawn `--scale` times, default 4; a moment's frames at 0.12 s, its last held 0.5 s, the last of all 0.6 s, an
//! IDLE row looped twice) and `whole.png`, a row per moment.

mod common;

use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;

use common::{Args, exit_reference, home, read_image, read_spec, write_png};
use pixfx_pixel::reference::extract::name_of;
use pixfx_pixel::reference::whole::{Moment, Timing, sequence, strip};

const USAGE: &str = "usage: whole <spec.json> [--out DIR] [--scale N]";

fn main() -> std::process::ExitCode {
    exit_reference("whole", run())
}

fn run() -> Result<(), String> {
    let mut args = Args::new(USAGE);
    let out = args.value("--out")?.map(PathBuf::from);
    let scale: u32 = match args.value("--scale")? {
        Some(v) => v
            .parse()
            .map_err(|_| format!("--scale {v}: not a whole number"))?,
        None => 4,
    };
    let rest = args.rest(1..=1)?;
    let spec = read_spec(&PathBuf::from(&rest[0]))?;
    let out = out.unwrap_or_else(|| home(&spec));
    let mut moments = Vec::new();
    for (i, r) in spec.rows.iter().enumerate() {
        let key = name_of(&r.name);
        let dir = home(&spec).join(r.name.to_lowercase()).join("review");
        let frames = (1..=spec.frames_of(r))
            .map(|k| read_image(&dir.join(format!("{key}-{k}.rebuilt.png"))))
            .collect::<Result<Vec<_>, _>>()?;
        let last = i + 1 == spec.rows.len();
        let idle = key == "idle";
        let timing = Timing {
            step: 12,
            hold: if last {
                60
            } else if idle {
                12
            } else {
                50
            },
        };
        let frames = if idle {
            frames.iter().chain(frames.iter()).cloned().collect()
        } else {
            frames
        };
        moments.push(Moment {
            name: key,
            frames,
            timing,
        });
    }
    let frames = sequence(&moments);
    let (w, h) = frames.first().map_or((0, 0), |f| (f.0.w, f.0.h));
    let size = |n: u32| {
        u16::try_from(n * scale).map_err(|_| format!("{n} x {scale} is too big for a gif"))
    };
    let (gw, gh) = (size(w)?, size(h)?);
    let path = out.join("whole.gif");
    let file = fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc =
        gif::Encoder::new(BufWriter::new(file), gw, gh, &[]).map_err(|e| e.to_string())?;
    enc.set_repeat(gif::Repeat::Infinite)
        .map_err(|e| e.to_string())?;
    for (f, delay) in &frames {
        let mut rgb = Vec::with_capacity((gw as usize) * (gh as usize) * 3);
        for y in 0..u32::from(gh) {
            for x in 0..u32::from(gw) {
                let p = f.get(x / scale, y / scale).unwrap_or([0, 0, 0, 255]);
                rgb.extend_from_slice(&p[..3]);
            }
        }
        let mut frame = gif::Frame::from_rgb_speed(gw, gh, &rgb, 10);
        frame.delay = *delay;
        enc.write_frame(&frame).map_err(|e| e.to_string())?;
    }
    write_png(&out.join("whole.png"), &strip(&moments, &frames, 2, 6))?;
    println!(
        "{}: {} ({} frames), {}",
        spec.id,
        path.display(),
        frames.len(),
        out.join("whole.png").display()
    );
    Ok(())
}
