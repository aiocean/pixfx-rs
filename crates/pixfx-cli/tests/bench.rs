use pixfx_cli::bench::{BenchConfig, run};
use pixfx_core::{Options, Player};

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
    let fps = pixfx_effects::effect("fire/candle")
        .ok_or("no candle")?
        .meta()
        .fps;
    let report = run(&config(&["fire/candle"], 3))?;
    assert_eq!(report.frames, 60);
    let row = report.rows.first().ok_or("no row")?;
    // frame 0, then one more for each authored frame the clock passes in 1 s
    assert_eq!(row.draws, 3 * (u64::from(fps) + 1));
    Ok(())
}

#[test]
fn rows_report_the_frame_size_with_its_glow_margin() -> Result<(), String> {
    let fx = pixfx_effects::effect("fire/candle").ok_or("no candle")?;
    let p = Player::new(fx, Options::default(), fx.meta().glow_margin());
    let report = run(&config(&["fire/candle"], 1))?;
    let row = report.rows.first().ok_or("no row")?;
    assert_eq!(
        (row.id.as_str(), row.w, row.h),
        ("fire/candle", p.frame().w, p.frame().h)
    );
    Ok(())
}

#[test]
fn each_id_gets_its_own_row_in_order() -> Result<(), String> {
    let report = run(&config(&["fire/candle", "fire/campfire"], 2))?;
    let ids: Vec<&str> = report.rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["fire/candle", "fire/campfire"]);
    Ok(())
}

#[test]
fn an_unknown_effect_is_an_error_naming_it() {
    let err = run(&config(&["fire/nope"], 1)).err();
    assert!(err.is_some_and(|e| e.contains("fire/nope")));
}

#[test]
fn no_instances_is_an_error() {
    assert!(run(&config(&["fire/candle"], 0)).is_err());
}

#[test]
fn a_rate_that_is_not_positive_is_an_error() {
    let cfg = BenchConfig {
        hz: 0.0,
        ..config(&["fire/candle"], 1)
    };
    assert!(run(&cfg).is_err());
}

fn words(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn no_flags_bench_fifty_of_each_fire_for_ten_seconds_at_sixty_hz() -> Result<(), String> {
    let cfg = pixfx_cli::bench::parse_args(&[])?;
    assert_eq!(
        cfg,
        config(&["fire/candle", "fire/campfire", "fire/brazier"], 50).with(10.0, 60.0)
    );
    Ok(())
}

#[test]
fn flags_set_ids_count_time_and_rate() -> Result<(), String> {
    let cfg = pixfx_cli::bench::parse_args(&words(
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
    assert!(pixfx_cli::bench::parse_args(&words("--fast")).is_err());
    assert!(pixfx_cli::bench::parse_args(&words("--n many")).is_err());
    assert!(pixfx_cli::bench::parse_args(&words("--hz")).is_err());
}

#[test]
fn the_table_has_a_row_per_id_with_its_size_and_times() -> Result<(), String> {
    let report = run(&config(&["fire/candle"], 1))?;
    let text = pixfx_cli::bench::table(&report);
    let row = report.rows.first().ok_or("no row")?;
    assert!(
        text.contains(&format!(
            "| fire/candle | {}×{} | {} |",
            row.w, row.h, row.draws
        )),
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
