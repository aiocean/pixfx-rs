# Duel auto-balance loop: design

Status: design for later. Start it only after the steps in "Before the loop" are done.

## Goal

Find the duel's rule numbers (damage, cooldowns, qi, statuses) that make Thunder and Frost even and the fights good to
watch, by letting an agent propose numbers and the trained policy play them, instead of tuning by hand.

The shape is AlphaEvolve's (DeepMind, 2025): a model proposes candidates, a scoring function people wrote judges each
one, the best candidates are kept and changed again. Here the candidate is a set of numbers in
`crates/pixfx-duel/src/data.rs` and the scoring function is the ladder (`crates/pixfx-duel/src/ladder.rs`).

## Why the policy, and why it must be fine-tuned every round

The old brain is too weak to show balance. With the brain on both sides Thunder wins 58% of the mirror; with the
mid-training policy on both sides Thunder wins 28% (20 seeds, update ~800; to be confirmed on 150 seeds). A strong
player is needed to find each school's best line.

A policy trained on old rules plays the old game. Measuring a rule change with it without retraining can call an
unbalanced change balanced, because the policy has not yet found the new strongest line. So every candidate is
fine-tuned from the base model before it is measured.

## Before the loop

1. The current run (`train/runs/v1`) finishes; its 150-seed ladder confirms or rules out the Frost advantage.
2. The skill-slot description the policy reads (`crates/pixfx-mind/src/policy/observe.rs:15`) gains status fields
   (stun, slow, knockback, their duration). Without them a skill whose value is a status looks like a weak attack, and
   balance measured through the policy is wrong for exactly the statuses this loop will tune. The input size
   changes, so this needs one full retrain (2-4 h). That run is the **base model**.
3. One or two balance rounds by hand, with the user, to learn what a good score looks like before a machine optimises it.
4. The duel.js parity gate (`crates/pixfx-duel/CLAUDE.md`, Traps) is settled: either the Rust port is the only
   source of the rules, or every number the loop changes is also changed in `site/assets/duel.js` and the dump re-run.

## One round

```
candidate numbers (data.rs)
  -> fine-tune from base:  uv run python ppo.py --run bal/<n> --resume runs/base/last.pt --updates <K>
  -> export to a scratch path: uv run python export.py runs/bal/<n>/last.pt --out /tmp/bal/<n>.bin --check ...
  -> ladder: cargo run --release -p pixfx-duel --example ladder -- --seeds 150 --weights /tmp/bal/<n>.bin
  -> score, keep or drop
```

Every candidate fine-tunes from the same base model, not from the previous candidate, so candidates are compared on
equal footing and a bad candidate cannot poison the next.

Cost: the fine-tune dominates, an estimated 20-40 min for a few hundred updates on this machine (measure on the first
round). That is a few dozen candidates a night, not thousands: the proposer must reason from the last scores, not
search blindly.

## The score

The loop optimises exactly what the score measures, so the score must include what makes a fight worth watching,
not only who wins. Every term comes from the ladder's `Record`:

| Term | From `Record` | Target |
|---|---|---|
| School balance (policy vs itself) | `as_school(0)` | 45-55% Thunder |
| Policy still strong | `win_rate()` vs brain / first mind | stays at the base model's level |
| Round length | `mean_length()`, `timeouts` | 15-45 s, no timeouts |
| No single habit | `most_used()` | under 40% |
| Fighting, not waiting | `attack_share()` | at least the base model's |
| Distance kept | `mean_distance` | in a band the user picks after the hand rounds |

Hard limits (balance, timeouts) reject a candidate; the rest rank the candidates that pass. Each ladder number at 150
seeds carries noise of a few percent, so a candidate only replaces the best when it beats it by more than that.

## What the proposer may change

A list the user approves before the loop starts: which constants in `data.rs` and which spell numbers, and the range
each may move in (for example ±20%). The proposer never changes code, only numbers in that list. Rule changes stay
the user's decision (the agreement with the training session): the loop produces a short list of best candidates
with their scores, and the user picks which one lands.

## Output

For each night: a table of every candidate (numbers changed, each score term, kept or dropped), the best three, and
their weights in scratch paths. Landing one is a normal change: numbers in `data.rs` (and duel.js if still
gated), weights exported to `policy.bin`, the fight goldens re-pinned, `cargo test -p pixfx-duel --all-targets`.

## Limits

- Balanced for this policy, not for people. It reacts with a fixed delay and makes no human mistakes. For the
  landing duel, where the policy plays both sides, that is the audience. If players pick schools later, people
  play-test the result.
- The policy has seen two schools and 14 skills. A candidate that moves numbers far outside what it trained on is
  measured by a policy guessing; the allowed ranges keep candidates near trained ground.

## Open questions

- Where the loop runs: a Temporal workflow with agents (survives a reboot overnight) or one long agent session.
- Whether `ladder.rs` gains a machine-readable output (JSON) for the score, or the loop parses its table.
