# The tournament

Players train bots, submit them as weights files, and the arena plays them against each other to find the strongest.
This is the design; what is not built yet is listed in `docs/roadmap.md`.

## Seasons

A season freezes everything a bot depends on:

- the rules: every number in `crates/pixfx-duel/src/data.rs`, the frame data, the bar;
- what a bot sees and does: the 354 inputs, the 30 acts, the legality rules, the timing (`docs/bot.md`);
- the network's shape and the weights file format.

A season is a tag on this repo (`arena-s1`, ...). A balance change or a new input waits for the next season, because
it would make every submitted bot stale. A bot from one season does not play in another.

## What a player submits

One weights file (`docs/bot.md`, "The weights file"), plus a name. Nothing else runs: the arena runs the file in its
own copy of the rules, on its own machines. A player never submits code.

Why a weights file: it cannot do anything but compute logits, it runs the same bits on every machine, every bot costs
the same to run, and the engine already seats a different file in each corner (`Duel.withPolicies(config, seats,
left, right)` in the wasm). The ladder measures one file against fixed opponents (`--weights`); playing two files
against each other natively is part of the runner (`docs/roadmap.md`).

## Checks on submission

A file enters the season only if:

1. it loads: right magic, every tensor present with the right shape (`Net::load` names what is wrong);
2. it computes: every output finite on a fixed set of observations;
3. it is good enough to be worth a slot: over a fixed set of seeds, both ways, it wins at least 75% against the
   brain (65% as each school) and at least 80% against the first mind, as the ladder measures.

A file identical to one already entered (same bytes) is refused.

## How a match is played

- A **match** between bots A and B is a set of seeds; each seed is played **both ways**: A as Thunder and B as
  Frost, then swapped, 3 rounds each. So a school's edge cancels out, and the bot that wins is the bot that plays both
  schools better.
- Round limit 120 s; a cut round is a loss for both.
- The seeds are drawn by the arena and published only after the season ends, so nobody trains against them. They
  never overlap the training seeds (1,000,000 and up) or the ladder's (1-150).
- Each bot thinks with its own seeded dice, so a match is fully decided by: the seed list, file A, file B and the
  season's rules.

## Ranking

1. **Round robin.** Every pair plays a match. A bot's score is its share of rounds won. With many entries, a Swiss
   system pairs bots of similar score instead, a fixed number of rounds.
2. **Rating.** Elo (or TrueSkill) over every match, shown beside the score.
3. **Final.** The top 8 play a single-elimination bracket on a fresh, larger seed set, both ways.

The season's winner is the final's winner.

## Replays anyone can check

A fight is three things: a seed, two weights files and a season tag. Anyone with the repo at that tag can play it
again and get the same fight, frame for frame, natively or in the browser. The arena publishes, for each match, the
seeds and the hashes of both files; the results are checkable without trusting the arena.

## What a player sees

- Their bot's results per opponent and per school, with the ladder's telemetry (what each skill did, punishes,
  counters).
- Any fight replayed on the page, both bots seated by their files.
