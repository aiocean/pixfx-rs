//! What a frame of effects costs the engine natively, split into updating the clocks, drawing the layers and turning
//! them into RGBA, per effect id. Plays `N` instances of each id (default every effect the build has, 50 each) for
//! `S` seconds of an `H` Hz display (default 10 s at 60 Hz) and prints a markdown table. Instances are drawn as a host draws
//! them: only when their clock reaches a new frame, both layers, each cropped to its lit box as the shells send it.
//!
//!   cargo run --release -p pixfx-effects --example bench -- [--ids ID,ID...] [--n N] [--secs S] [--hz H]
//!
//! One effect alone, built with nothing else:
//!
//!   cargo run --release -p pixfx-effects --no-default-features --features fire-candle --example bench
//!
//! `cargo test -p pixfx-effects --example bench` runs its tests.

use std::time::{Duration, Instant};

use std::process::ExitCode;

use pixfx_core::{Layer, Options, Player};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args).and_then(|cfg| run(&cfg)) {
        Ok(report) => {
            println!("{}", table(&report));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

/// What to run: `n` instances of each of `ids` (seeds 1..=n), for `secs` seconds of a display refreshing `hz` times a
/// second.
#[derive(Clone, Debug, PartialEq)]
struct BenchConfig {
    ids: Vec<String>,
    n: usize,
    secs: f64,
    hz: f64,
}

/// One effect id's share: its frame (canvas plus glow margin), how many times its instances drew both layers, and
/// the mean milliseconds per display frame spent drawing and converting them.
#[derive(Clone, Debug, PartialEq)]
struct BenchRow {
    id: String,
    w: u32,
    h: u32,
    draws: u64,
    draw_ms: f64,
    rgba_ms: f64,
}

/// Display frames run, mean milliseconds per display frame spent updating every instance, and one row per id in the
/// order asked.
#[derive(Clone, Debug, PartialEq)]
struct BenchReport {
    frames: u64,
    update_ms: f64,
    rows: Vec<BenchRow>,
}

struct Group {
    id: String,
    players: Vec<Player>,
    /// The clock frame each instance last drew.
    drawn: Vec<Option<u64>>,
    draws: u64,
    draw: Duration,
    rgba: Duration,
}

/// Run the bench. An unknown id, no instances, or a time or rate that is not a positive number is an error.
fn run(cfg: &BenchConfig) -> Result<BenchReport, String> {
    if cfg.n == 0 {
        return Err("bench needs at least one instance (--n)".into());
    }
    if !(cfg.hz.is_finite() && cfg.hz > 0.0 && cfg.secs.is_finite() && cfg.secs > 0.0) {
        return Err("bench needs a positive --secs and --hz".into());
    }
    let mut groups = cfg
        .ids
        .iter()
        .map(|id| group(id, cfg.n))
        .collect::<Result<Vec<_>, _>>()?;
    let dt = 1.0 / cfg.hz;
    // whole display frames in `secs` (the cast truncates a positive number); the epsilon keeps 1.0 * 60.0 from
    // landing a hair under 60
    let frames = (cfg.secs * cfg.hz + 1e-9) as u64;
    let mut update = Duration::ZERO;
    for _ in 0..frames {
        let start = Instant::now();
        for g in &mut groups {
            for p in &mut g.players {
                p.update(dt);
            }
        }
        update += start.elapsed();
        for g in &mut groups {
            draw_new_frames(g);
        }
    }
    let per_frame = |d: Duration| d.as_secs_f64() * 1000.0 / frames.max(1) as f64;
    Ok(BenchReport {
        frames,
        update_ms: per_frame(update),
        rows: groups
            .into_iter()
            .map(|g| {
                let (w, h) = g
                    .players
                    .first()
                    .map_or((0, 0), |p| (p.frame().w, p.frame().h));
                BenchRow {
                    id: g.id,
                    w,
                    h,
                    draws: g.draws,
                    draw_ms: per_frame(g.draw),
                    rgba_ms: per_frame(g.rgba),
                }
            })
            .collect(),
    })
}

/// `[--ids a,b] [--n N] [--secs S] [--hz H]`: each flag left out takes its default (every effect the build
/// has, 50 of each, 10 s, 60 Hz).
fn parse_args(args: &[String]) -> Result<BenchConfig, String> {
    let mut cfg = BenchConfig {
        ids: pixfx_effects::EFFECTS
            .iter()
            .map(|fx| fx.meta().id.to_owned())
            .collect(),
        n: 50,
        secs: 10.0,
        hz: 60.0,
    };
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let v = it.next().ok_or(format!("{flag} needs a value"))?;
        let number = |what: &str| format!("{flag} needs {what}, got {v:?}");
        match flag.as_str() {
            "--ids" => cfg.ids = v.split(',').map(str::to_owned).collect(),
            "--n" => cfg.n = v.parse().map_err(|_| number("a whole number"))?,
            "--secs" => cfg.secs = v.parse().map_err(|_| number("a number"))?,
            "--hz" => cfg.hz = v.parse().map_err(|_| number("a number"))?,
            _ => return Err(format!("unknown bench flag {flag}")),
        }
    }
    Ok(cfg)
}

/// The report as a markdown table: one row per id with its frame size, draws and milliseconds per display frame.
fn table(r: &BenchReport) -> String {
    let mut lines = vec![
        format!(
            "{} display frames; update {:.2} ms per frame",
            r.frames, r.update_ms
        ),
        "| effect | frame | draws | draw ms | rgba ms |".to_owned(),
        "|---|---|---:|---:|---:|".to_owned(),
    ];
    lines.extend(r.rows.iter().map(|row| {
        format!(
            "| {} | {}×{} | {} | {:.2} | {:.2} |",
            row.id, row.w, row.h, row.draws, row.draw_ms, row.rgba_ms
        )
    }));
    lines.join("\n")
}

fn group(id: &str, n: usize) -> Result<Group, String> {
    let fx = pixfx_effects::effect(id).ok_or_else(|| format!("unknown effect {id}"))?;
    let players = (1..=n as u64)
        .map(|seed| {
            let opts = Options {
                seed: Some(seed),
                ..Options::default()
            };
            Player::new(fx, opts, fx.meta().glow_margin())
        })
        .collect();
    Ok(Group {
        id: id.to_owned(),
        players,
        drawn: vec![None; n],
        draws: 0,
        draw: Duration::ZERO,
        rgba: Duration::ZERO,
    })
}

/// Draw both layers of every instance whose clock reached a new frame, timing the drawing and the conversion apart.
fn draw_new_frames(g: &mut Group) {
    for (p, drawn) in g.players.iter_mut().zip(&mut g.drawn) {
        let frame = p.clock_frame();
        if *drawn == Some(frame) {
            continue;
        }
        *drawn = Some(frame);
        g.draws += 1;
        let place = p.placement();
        for layer in [Layer::Back, Layer::Front] {
            let start = Instant::now();
            p.draw(layer, place);
            let drawn_at = Instant::now();
            if let Some(b) = p.lit_box() {
                std::hint::black_box(p.color_rgba_in(b));
                std::hint::black_box(p.light_rgba_in(b));
            }
            g.draw += drawn_at - start;
            g.rgba += drawn_at.elapsed();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An effect this build has; the tests run on whatever one effect, or all, the build turned on.
    fn any() -> Result<&'static dyn pixfx_core::Effect, String> {
        pixfx_effects::EFFECTS
            .first()
            .copied()
            .ok_or_else(|| "the build has no effect".to_string())
    }

    fn config(ids: &[&str], n: usize) -> BenchConfig {
        BenchConfig {
            ids: ids.iter().map(|s| (*s).to_owned()).collect(),
            n,
            secs: 1.0,
            hz: 60.0,
        }
    }

    #[test]
    fn each_instance_draws_once_per_new_frame() -> Result<(), String> {
        let fx = any()?;
        let fps = fx.meta().fps;
        let report = run(&config(&[fx.meta().id], 3))?;
        assert_eq!(report.frames, 60);
        let row = report.rows.first().ok_or("no row")?;
        // frame 0, then one more for each authored frame the clock passes in 1 s; but it draws after each update, at
        // most once a display frame, so an effect authored at the display rate (60 fps) is past frame 0 before its
        // first draw and draws once on each of the 60
        assert_eq!(row.draws, 3 * (u64::from(fps) + 1).min(report.frames));
        Ok(())
    }

    #[test]
    fn rows_report_the_frame_size_with_its_glow_margin() -> Result<(), String> {
        let fx = any()?;
        let p = Player::new(fx, Options::default(), fx.meta().glow_margin());
        let report = run(&config(&[fx.meta().id], 1))?;
        let row = report.rows.first().ok_or("no row")?;
        assert_eq!(
            (row.id.as_str(), row.w, row.h),
            (fx.meta().id, p.frame().w, p.frame().h)
        );
        Ok(())
    }

    #[test]
    fn each_id_gets_its_own_row_in_order() -> Result<(), String> {
        // the last built effect first: rows follow the order asked, not the registry's
        let (first, last) = (
            any()?.meta().id,
            pixfx_effects::EFFECTS
                .last()
                .copied()
                .ok_or("none")?
                .meta()
                .id,
        );
        let report = run(&config(&[last, first], 2))?;
        let ids: Vec<&str> = report.rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, [last, first]);
        Ok(())
    }

    #[test]
    fn an_unknown_effect_is_an_error_naming_it() {
        let err = run(&config(&["fire/nope"], 1)).err();
        assert!(err.is_some_and(|e| e.contains("fire/nope")));
    }

    #[test]
    fn no_instances_is_an_error() -> Result<(), String> {
        assert!(run(&config(&[any()?.meta().id], 0)).is_err());
        Ok(())
    }

    #[test]
    fn a_rate_that_is_not_positive_is_an_error() -> Result<(), String> {
        let cfg = BenchConfig {
            hz: 0.0,
            ..config(&[any()?.meta().id], 1)
        };
        assert!(run(&cfg).is_err());
        Ok(())
    }

    fn words(s: &str) -> Vec<String> {
        s.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn no_flags_bench_fifty_of_each_built_effect_for_ten_seconds_at_sixty_hz() -> Result<(), String>
    {
        let cfg = parse_args(&[])?;
        let ids: Vec<&str> = pixfx_effects::EFFECTS
            .iter()
            .map(|fx| fx.meta().id)
            .collect();
        assert_eq!(cfg, config(&ids, 50).with(10.0, 60.0));
        Ok(())
    }

    #[test]
    fn flags_set_ids_count_time_and_rate() -> Result<(), String> {
        let cfg = parse_args(&words(
            "--ids fire/candle,ice/frost --n 5 --secs 2 --hz 120",
        ))?;
        assert_eq!(
            cfg,
            config(&["fire/candle", "ice/frost"], 5).with(2.0, 120.0)
        );
        Ok(())
    }

    #[test]
    fn an_unknown_flag_or_a_bad_number_is_an_error() {
        assert!(parse_args(&words("--fast")).is_err());
        assert!(parse_args(&words("--n many")).is_err());
        assert!(parse_args(&words("--hz")).is_err());
    }

    #[test]
    fn the_table_has_a_row_per_id_with_its_size_and_times() -> Result<(), String> {
        let id = any()?.meta().id;
        let report = run(&config(&[id], 1))?;
        let text = table(&report);
        let row = report.rows.first().ok_or("no row")?;
        assert!(
            text.contains(&format!("| {id} | {}×{} | {} |", row.w, row.h, row.draws)),
            "{text}"
        );
        assert!(text.contains("update"), "{text}");
        Ok(())
    }

    trait With {
        fn with(self, secs: f64, hz: f64) -> Self;
    }

    impl With for BenchConfig {
        fn with(self, secs: f64, hz: f64) -> Self {
            BenchConfig { secs, hz, ..self }
        }
    }
}
