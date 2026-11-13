//! `pixfx bench`: what a frame of effects costs the engine natively, split into updating the clocks, drawing the
//! layers and turning them into RGBA, per effect id. Instances are drawn as a host draws them: only when their clock
//! reaches a new frame, both layers, each cropped to its lit box as the Phaser plugin sends it.

use std::time::{Duration, Instant};

use pixfx_core::{Layer, Options, Player};

/// What to run: `n` instances of each of `ids` (seeds 1..=n), for `secs` seconds of a display refreshing `hz` times a
/// second.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchConfig {
    pub ids: Vec<String>,
    pub n: usize,
    pub secs: f64,
    pub hz: f64,
}

/// One effect id's share: its frame (canvas plus glow margin), how many times its instances drew both layers, and
/// the mean milliseconds per display frame spent drawing and converting them.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchRow {
    pub id: String,
    pub w: u32,
    pub h: u32,
    pub draws: u64,
    pub draw_ms: f64,
    pub rgba_ms: f64,
}

/// Display frames run, mean milliseconds per display frame spent updating every instance, and one row per id in the
/// order asked.
#[derive(Clone, Debug, PartialEq)]
pub struct BenchReport {
    pub frames: u64,
    pub update_ms: f64,
    pub rows: Vec<BenchRow>,
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
pub fn run(cfg: &BenchConfig) -> Result<BenchReport, String> {
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

/// `pixfx bench [--ids a,b] [--n N] [--secs S] [--hz H]`: each flag left out takes its default (the three fires,
/// 50 of each, 10 s, 60 Hz).
pub fn parse_args(args: &[String]) -> Result<BenchConfig, String> {
    let mut cfg = BenchConfig {
        ids: ["fire/candle", "fire/campfire", "fire/brazier"]
            .map(str::to_owned)
            .to_vec(),
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
pub fn table(r: &BenchReport) -> String {
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
