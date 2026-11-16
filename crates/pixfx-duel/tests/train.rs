//! The training environment: one round a episode, decisions in, rewards out, the same every time.

mod common;

use common::all_effects;
use pixfx_duel::style::Style;
use pixfx_duel::train::{Env, EnvConfig, Opponent, Step};
use pixfx_mind::policy::{ACTIONS, Act};
use pixfx_mind::Rng;

fn cfg(seed: u32, learner: usize, opponent: Opponent) -> EnvConfig {
    EnvConfig { seed, learner, style: Style::Balanced, foe_style: Style::Balanced, opponent, limit: 90.0, effects: all_effects() }
}

/// A random legal action from the env's mask.
fn random_legal(env: &Env, rng: &mut Rng) -> usize {
    let (_, legal) = env.observe();
    let ok: Vec<usize> = (0..ACTIONS).filter(|&i| legal[i]).collect();
    ok.get((rng.next() * ok.len() as f64) as usize).copied().unwrap_or(0)
}

/// Play `env` to its end with `pick`; every step's result.
fn play(env: &mut Env, shaping: f32, mut pick: impl FnMut(&Env) -> usize) -> Vec<Step> {
    let mut steps = vec![];
    for _ in 0..5000 {
        let a = pick(env);
        let s = env.step(a, shaping);
        steps.push(s);
        if s.done {
            break;
        }
    }
    steps
}

#[test]
fn an_episode_pays_plus_or_minus_one_at_its_end() {
    let mut rng = Rng::new(1);
    for (seed, learner, opp) in [(1, 0, Opponent::Brain), (2, 1, Opponent::First), (3, 0, Opponent::Aggressive)] {
        let mut env = Env::new(cfg(seed, learner, opp));
        let steps = play(&mut env, 0.0, |e| random_legal(e, &mut rng));
        let last = steps.last().copied().filter(|s| s.done);
        assert!(last.is_some(), "seed {seed}: no end in 5000 steps");
        let Some(last) = last else { return };
        let won = last.won.unwrap_or(false);
        let want = if won { 1.0 } else { -1.0 };
        assert!(last.won.is_some(), "seed {seed}: a knockout, not a cut");
        assert!((last.reward - want).abs() < 0.05, "seed {seed}: {} for won {won}", last.reward);
        assert!(last.length > 0.0 && last.length < 90.0);
    }
}

#[test]
fn an_episode_cut_at_its_limit_is_a_loss() {
    let mut env = Env::new(EnvConfig { limit: 3.0, ..cfg(4, 0, Opponent::Turtle) });
    let steps = play(&mut env, 0.0, |_| 0);
    let last = steps.last().copied();
    assert!(last.is_some_and(|s| s.done && s.won.is_none() && (s.reward + 1.0).abs() < 0.05), "{last:?}");
    assert!(steps.len() > 8, "it decided along the way: {}", steps.len());
}

#[test]
fn the_same_config_and_actions_replay_the_same_episode() {
    let run = || {
        let mut env = Env::new(cfg(5, 1, Opponent::Brain));
        let mut rng = Rng::new(9);
        let mut seen = vec![];
        let steps = play(&mut env, 1.0, |e| {
            seen.push(e.observe().0.to_vec());
            random_legal(e, &mut rng)
        });
        (steps, seen)
    };
    assert_eq!(run(), run());
}

#[test]
fn an_action_outside_the_mask_is_a_wait() {
    let run = |illegal: bool| {
        let mut env = Env::new(cfg(6, 0, Opponent::Aggressive));
        play(&mut env, 1.0, |e| {
            let (_, legal) = e.observe();
            if illegal { (0..ACTIONS).rev().find(|&i| !legal[i]).unwrap_or(0) } else { 0 }
        })
    };
    assert_eq!(run(true), run(false));
}

#[test]
fn without_shaping_only_the_outcome_and_small_penalties_pay() {
    let mut rng = Rng::new(3);
    let mut env = Env::new(cfg(7, 0, Opponent::Brain));
    let steps = play(&mut env, 0.0, |e| random_legal(e, &mut rng));
    for s in steps.iter().filter(|s| !s.done) {
        assert!(s.reward <= 0.0 && s.reward >= -0.004 - 1e-6, "{s:?}");
    }
}

#[test]
fn shaping_pays_for_damage() {
    // the same fight paid with and without shaping: the damage either way is paid only when shaping is on
    let paid = |shaping| {
        let mut rng = Rng::new(4);
        let mut env = Env::new(cfg(8, 0, Opponent::Turtle));
        play(&mut env, shaping, |e| random_legal(e, &mut rng)).iter().map(|s| s.reward).sum::<f32>()
    };
    assert_ne!(paid(1.0), paid(0.0));
}

#[test]
fn a_policy_opponent_plays_from_its_weights() {
    let net = pixfx_duel::seat::shipped();
    assert!(net.is_ok(), "weights load");
    let Ok(net) = net else { return };
    let mut rng = Rng::new(5);
    let mut env = Env::new(cfg(9, 1, Opponent::Policy(net)));
    let steps = play(&mut env, 0.0, |e| random_legal(e, &mut rng));
    assert!(steps.last().is_some_and(|s| s.done));
}

#[test]
fn an_act_that_fills_its_recent_decisions_pays_for_the_habit() {
    // meditate and guard in turn never repeat one act six times running, yet meditate is half of what it does
    let mut env = Env::new(cfg(9, 0, Opponent::Turtle));
    let mut turn = 0;
    let steps = play(&mut env, 0.0, |e| {
        turn += 1;
        let want = if turn % 2 == 0 {
            Act::Meditate
        } else {
            Act::Guard
        }
        .index();
        if e.observe().1[want] { want } else { 0 }
    });
    let worst = steps
        .iter()
        .filter(|s| !s.done)
        .map(|s| s.reward)
        .fold(0.0_f32, f32::min);
    assert!(
        worst <= -0.004,
        "the worst of {} steps paid {worst}",
        steps.len()
    );
}
