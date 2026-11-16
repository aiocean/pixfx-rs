//! The ladder: the trained policy against the brain, the first mind, the scripted fighters and itself, printed as
//! the "Done when" table of `games/arena/docs/specs/2026-11-15-duel-rules-in-rust-design.md`.
//!
//! `cargo run --release -p pixfx-duel --example ladder -- [--seeds 150] [--rounds 3] [--limit 120] [--weights FILE]
//! [--telemetry]`
//!
//! `--telemetry` adds, for each mirror, what each school's skills did, Thunder beside Frost.

use pixfx_duel::duel::{Duel, DuelConfig, TICK};
use pixfx_duel::first_mind::FirstMind;
use pixfx_duel::ladder::{Match, Record, SchoolLog, SkillLog, match_up};
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
    let telemetry = args.iter().any(|a| a == "--telemetry");
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
    // each row: its name, its seats, and whether it is a mirror whose schools the telemetry compares
    let rows: Vec<(&str, Maker<'_>, Maker<'_>, bool)> = vec![
        (
            "balanced vs brain (>=75%, 65% each)",
            Box::new(styled(Style::Balanced)),
            Box::new(Brain::make),
            false,
        ),
        (
            "balanced vs first mind (>=80%)",
            Box::new(styled(Style::Balanced)),
            Box::new(FirstMind::make),
            false,
        ),
        (
            "balanced vs aggressive script",
            Box::new(styled(Style::Balanced)),
            Box::new(Scripted::aggressive),
            false,
        ),
        (
            "balanced vs turtle script",
            Box::new(styled(Style::Balanced)),
            Box::new(Scripted::turtle),
            false,
        ),
        (
            "mirror, styles drawn (45-55%)",
            Box::new(drawn),
            Box::new(drawn),
            true,
        ),
        (
            "brain vs first mind (baseline)",
            Box::new(Brain::make),
            Box::new(FirstMind::make),
            false,
        ),
        (
            "brain mirror (school baseline)",
            Box::new(Brain::make),
            Box::new(Brain::make),
            true,
        ),
    ];
    let mut mirrors = Vec::new();
    for (name, a, b, mirror) in &rows {
        let r = match_up(a.as_ref(), b.as_ref(), &m);
        print_row(name, &r);
        if *mirror {
            mirrors.push((*name, r));
        }
    }
    println!("\nmirrors: the win column is Thunder's share of the rounds in the thunder column\n");
    if telemetry {
        for (name, r) in &mirrors {
            print_telemetry(name, r);
        }
    }
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

/// What each school did in a mirror, Thunder beside Frost: per skill, then per school.
fn print_telemetry(name: &str, r: &Record) {
    let [t, f] = &r.schools;
    let wins = [r.won[0] + r.lost[1], r.won[1] + r.lost[0]];
    let rounds = wins[0] + wins[1];
    println!(
        "telemetry: {name}, {rounds} rounds (Thunder won {}, Frost {})",
        wins[0], wins[1]
    );
    println!(
        "  dmg: lives of the foe taken (1.00 = one life), chip and field ticks included; dmg% of the school's total. \
         grd/bar/wrd/ddg/clh: blows the foe's guard held, its barrier turned, a ward erased, it dodged, blades met. \
         brk: casts broken while gathering. pun: struck while recovering. fin: last blow of a round it won."
    );
    let cols = [
        "cast", "land", "dmg", "dmg%", "grd", "bar", "wrd", "ddg", "clh", "brk", "pun", "fin",
    ];
    let head: String = cols
        .iter()
        .map(|c| {
            let w = if *c == "dmg" { 8 } else { 6 };
            format!("{c:>w$}")
        })
        .collect();
    println!("  {:<12} | {:^74} | {:^74}", "", "Thunder", "Frost");
    println!("  {:<12} |{head} |{head}", "skill");
    let mut names: Vec<&str> = t.skills.keys().chain(f.skills.keys()).copied().collect();
    names.sort_unstable();
    names.dedup();
    // the heaviest hitters first, by the two schools' damage together
    let dmg = |s: &SchoolLog, n: &str| s.skills.get(n).map_or(0.0, |k| k.damage);
    names.sort_by(|a, b| (dmg(t, b) + dmg(f, b)).total_cmp(&(dmg(t, a) + dmg(f, a))));
    for n in names {
        println!("  {n:<12} |{} |{}", skill_cells(t, n), skill_cells(f, n));
    }
    for status in ["burn", "shock"] {
        let cell = |s: &SchoolLog| {
            let d = s.statuses.get(status).copied().unwrap_or(0.0);
            let fin = s.finals.get(status).copied().unwrap_or(0);
            format!(
                "{:>12}{:>8.2}{:>6.1}{:>48}{fin:>6}",
                "",
                d / 100.0,
                pct(d, s.dealt()),
                ""
            )
        };
        println!("  {status:<12} |{} |{}", cell(t), cell(f));
    }
    let line = |label: &str, v: &dyn Fn(&SchoolLog) -> String| {
        println!("  {label:<34} {:>14} {:>14}", v(t), v(f));
    };
    println!("  {:<34} {:>14} {:>14}", "per school", "Thunder", "Frost");
    let per_round = |x: f64| x / f64::from(rounds.max(1));
    line("lives dealt", &|s| format!("{:.2}", s.dealt() / 100.0));
    line("lives lost", &|s| format!("{:.2}", s.life_lost / 100.0));
    line("lives healed", &|s| format!("{:.2}", s.healed / 100.0));
    line("dealt per round, % of a life", &|s| {
        format!("{:.1}", per_round(s.dealt()))
    });
    line("status share of dealt", &|s| {
        format!("{:.1}%", pct(s.statuses.values().sum(), s.dealt()))
    });
    line("chip share of dealt", &|s| {
        format!(
            "{:.1}%",
            pct(s.skills.values().map(|k| k.chip).sum(), s.dealt())
        )
    });
    line("casts per round", &|s| {
        format!("{:.1}", per_round(f64::from(sum(s, |k| k.casts))))
    });
    line("hits landed per round", &|s| {
        format!("{:.1}", per_round(f64::from(sum(s, |k| k.landed))))
    });
    line("casts broken / punished", &|s| {
        format!("{} / {}", sum(s, |k| k.broken), sum(s, |k| k.punished))
    });
    line("its blows guarded/barrier/ward/dodged", &|s| {
        format!(
            "{}/{}/{}/{}",
            sum(s, |k| k.guarded),
            sum(s, |k| k.barrier),
            sum(s, |k| k.ward),
            sum(s, |k| k.dodged)
        )
    });
    line("qi on casts per round", &|s| {
        format!("{:.0}", per_round(s.qi_spent))
    });
    line("ultimates / rage spent", &|s| {
        let u = s
            .skills
            .iter()
            .filter(|(n, _)| pixfx_duel::data::spell(n).is_some_and(|sp| sp.ultimate))
            .map(|(_, k)| k.casts)
            .sum::<u32>();
        format!("{u} / {:.0}", s.rage_spent)
    });
    line("ultimate damage, lives", &|s| {
        let d = s
            .skills
            .iter()
            .filter(|(n, _)| pixfx_duel::data::spell(n).is_some_and(|sp| sp.ultimate))
            .map(|(_, k)| k.damage)
            .sum::<f64>();
        format!("{:.2}", d / 100.0)
    });
    line("time meditating", &|s| {
        format!("{:.1}%", pct(s.meditating, s.seconds))
    });
    line("time guarding", &|s| {
        format!("{:.1}%", pct(s.guarding, s.seconds))
    });
    line("time flying", &|s| {
        format!("{:.1}%", pct(s.flying, s.seconds))
    });
    println!();
}

/// One school's cells for skill `n`.
fn skill_cells(s: &SchoolLog, n: &str) -> String {
    let k = s.skills.get(n).cloned().unwrap_or_default();
    let fin = s.finals.get(n).copied().unwrap_or(0);
    format!(
        "{:>6}{:>6}{:>8.2}{:>6.1}{:>6}{:>6}{:>6}{:>6}{:>6}{:>6}{:>6}{fin:>6}",
        k.casts,
        k.landed,
        k.damage / 100.0,
        pct(k.damage, s.dealt()),
        k.guarded,
        k.barrier,
        k.ward,
        k.dodged,
        k.clashed,
        k.broken,
        k.punished,
    )
}

fn sum(s: &SchoolLog, of: impl Fn(&SkillLog) -> u32) -> u32 {
    s.skills.values().map(of).sum()
}

fn pct(a: f64, b: f64) -> f64 {
    if b == 0.0 { 0.0 } else { a / b * 100.0 }
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
