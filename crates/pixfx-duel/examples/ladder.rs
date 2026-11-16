//! The ladder: the trained policy against the brain, the first mind, the scripted fighters and itself, printed as
//! the "Done when" table of `docs/superpowers/specs/2026-11-15-duel-rules-in-rust-design.md`.
//!
//! `cargo run --release -p pixfx-duel --example ladder -- [--seeds 150] [--rounds 3] [--limit 120] [--weights FILE]`

use pixfx_duel::duel::{Duel, DuelConfig, TICK};
use pixfx_duel::first_mind::FirstMind;
use pixfx_duel::ladder::{Match, Record, match_up};
use pixfx_duel::scripted::Scripted;
use pixfx_duel::seat::{Brain, MakeSeat, PolicySeat, policy_mind, shipped};
use pixfx_duel::sense::sense_of;
use pixfx_duel::style::Style;
use pixfx_mind::policy::Net;
use std::rc::Rc;
use std::time::Instant;

/// Makes a fresh seat for each fight.
type Maker<'a> = Box<dyn Fn() -> MakeSeat + 'a>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let seeds: u32 = arg("--seeds").map_or(Ok(150), |s| s.parse())?;
    let rounds: u32 = arg("--rounds").map_or(Ok(3), |s| s.parse())?;
    let limit: f64 = arg("--limit").map_or(Ok(120.0), |s| s.parse())?;
    let net: Rc<Net> = match arg("--weights") {
        Some(path) => Rc::new(Net::load(&std::fs::read(&path)?)?),
        None => shipped()?,
    };
    let effects: Vec<String> = pixfx_effects::EFFECTS
        .iter()
        .map(|e| e.meta().id.to_string())
        .collect();
    let m = Match {
        seeds: 1..seeds + 1,
        rounds,
        limit,
        effects: effects.clone(),
    };
    let styled = |s: Style| {
        let n = net.clone();
        move || PolicySeat::with_style(n.clone(), s)
    };
    let drawn = || PolicySeat::make(net.clone());

    println!("ladder: {seeds} seeds x both ways x {rounds} rounds, rounds cut at {limit} s\n");
    println!(
        "{:<34} {:>7} {:>9} {:>7} {:>7} {:>5} {:>6} {:>7} {:>6}",
        "match", "win", "thunder", "frost", "len s", "cut", "top", "attack", "dist"
    );
    let rows: Vec<(&str, Maker<'_>, Maker<'_>)> = vec![
        (
            "balanced vs brain (>=75%, 65% each)",
            Box::new(styled(Style::Balanced)),
            Box::new(Brain::make),
        ),
        (
            "balanced vs first mind (>=80%)",
            Box::new(styled(Style::Balanced)),
            Box::new(FirstMind::make),
        ),
        (
            "balanced vs aggressive script",
            Box::new(styled(Style::Balanced)),
            Box::new(Scripted::aggressive),
        ),
        (
            "balanced vs turtle script",
            Box::new(styled(Style::Balanced)),
            Box::new(Scripted::turtle),
        ),
        (
            "mirror, styles drawn (45-55%)",
            Box::new(drawn),
            Box::new(drawn),
        ),
        (
            "brain vs first mind (baseline)",
            Box::new(Brain::make),
            Box::new(FirstMind::make),
        ),
        (
            "brain mirror (school baseline)",
            Box::new(Brain::make),
            Box::new(Brain::make),
        ),
    ];
    for (name, a, b) in &rows {
        let r = match_up(a.as_ref(), b.as_ref(), &m);
        print_row(name, &r);
    }
    println!("\nmirrors: the win column is Thunder's share of the rounds in the thunder column\n");
    decision_cost(&net, &effects);
    Ok(())
}

fn print_row(name: &str, r: &Record) {
    let pct = |x: f64| format!("{:.1}%", x * 100.0);
    let top = r
        .choices
        .iter()
        .max_by_key(|(_, n)| **n)
        .map_or("-", |(k, _)| k.as_str());
    println!(
        "{:<34} {:>7} {:>9} {:>7} {:>7.1} {:>5} {:>6} {:>7} {:>6.0}   top: {top}",
        name,
        pct(r.win_rate()),
        pct(r.as_school(0)),
        pct(r.as_school(1)),
        r.mean_length(),
        r.timeouts,
        pct(r.most_used()),
        pct(r.attack_share()),
        r.mean_distance,
    );
}

/// The time one policy decision takes, natively, over 5,000 decisions of a real fight.
fn decision_cost(net: &Rc<Net>, effects: &[String]) {
    let config = DuelConfig {
        seed: 1,
        left: 0.0,
        right: 220.0,
        ground: 150.0,
        sky: 96.0,
        effects: effects.to_vec(),
    };
    let mut duel = Duel::new(config, [Brain::make(), Brain::make()]);
    let mut senses = Vec::new();
    while senses.len() < 5000 {
        duel.step(TICK);
        senses.push(sense_of(&duel.state, 0));
    }
    let mut mind = policy_mind(&duel.state, 0, Style::Balanced);
    // the decisions a pass makes, counted apart from the timing
    let mut counter = mind.clone();
    let decided_per_pass = senses
        .iter()
        .filter(|s| {
            let _ = counter.begin(s, TICK);
            counter.due(s).is_some()
        })
        .count() as u32;
    let start = Instant::now();
    let decided = decided_per_pass * 20;
    for _ in 0..20 {
        for s in &senses {
            let _ = mind.think(net, s, TICK);
        }
        mind.reset();
    }
    let per = start.elapsed().as_secs_f64() / f64::from(decided.max(1));
    println!(
        "decision cost (native, release): {:.3} ms over {decided} decisions",
        per * 1000.0
    );
}
