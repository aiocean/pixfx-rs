# The bot

What a bot sees, what it can do, when it decides, and the file a player submits. The code is the source of truth:
`crates/pixfx-mind/src/policy/` (`observe.rs`, `action.rs`, `mind.rs`, `net.rs`) and
`games/arena/crates/pixfx-duel/src/seat.rs`.

## What it sees: 354 numbers

Each decision the bot gets `OBS` = 354 floats, in a frame where the foe is always ahead (positive x), so the same bot
plays from either corner. Every value is clamped to [-4, 4], anything not finite reads 0, and times are in seconds
unless a scale is given. The foe is seen 0.16 s late (the reaction delay); its own state is current.

| Range | Count | What |
|---|---|---|
| 0..18 | 18 | **Me.** Distance to the wall behind and the wall ahead (over the arena's width), height (over the ceiling), facing the foe, hp / 100, qi / 100, rage (over a full ultimate), flying, hover (over the ceiling), speed toward the foe / 200, can act, silenced, sealed, barrier, shield (0..15). Winding up, recovering, seconds left in that phase (15..18). |
| 18..56 | 38 | **Foe.** Distance (over the width), height, flying, hp / 100, qi / 100 (18..23). What it does: idle, moving toward me, moving away, guarding, meditating, flying (23..29). Winding up, recovering (29..31). The kind of skill it uses, one-hot over 12 kinds (31..43). How far into its phase as a share (43), seconds left (44), the phase's span (45), the skill's reach / 100 (46), damage / 50 (47), whether it is an ultimate (48); 29..49 are zero when it uses no skill. Out for, open in, frozen, shocked, barrier, the share of its skills ready, my seeker out (49..56). |
| 56..104 | 4 x 12 | **Threats** in flight, nearest first, zero when absent: present, shape (projectile, area, seeker, field, barrage), distance ahead / 100, height / 50, speed toward me / 200, lands in, size / 50, damage / 50. |
| 104..349 | 7 x 35 | **Slots**, zero when empty: the three basics, the three bar slots, the ultimate. Each: present, ready, kind (12), cost / 50, damage x power / 50, reach / 100, windup, recover, radius / 50, ultimate (+0..+21); then what it inflicts (+21..+35): freeze s, stun s, silence s / 2, defence-down s / 5, defence lost (share), burn s / 3, burn's share of attack a second x 5, chill stacks a hit adds now over the most it holds, slow per chill stack x 4, shock stacks a hit adds now over the most it holds, hold per shock twitch x 4, share of attack per shock twitch x 10, pull / 100, rage stolen over a full ultimate. |
| 349..354 | 5 | **Tag.** Its school (Thunder, Frost) and its style (aggressive, balanced, defensive), one-hot. |

The 12 skill kinds, in order: melee, dive, dash, blast, bolt, seeker, area, field, heal, ward, barrier, blink.

A skill is known by its kind and its numbers, never its name, so a bot reads a skill the bar has just drawn the same
way as one it has seen a thousand times.

## What it can do: 30 acts

`ACTIONS` = 30, each at a fixed index:

| Index | Act |
|---|---|
| 0 | wait |
| 1 | walk toward the foe |
| 2 | walk away |
| 3 | jump |
| 4 | duck (held 0.4 s) |
| 5 | guard (held 0.4 s) |
| 6 | fly high (to 0.6 of the ceiling) |
| 7 | land |
| 8 | meditate |
| 9 + 3 x slot + aim | cast the skill in `slot` (0..7), aimed at the foe (aim 0), 30 px past it (1) or 30 px short of it (2) |

**Legal acts.** Each decision also comes with a mask of the acts that can happen now (`legal`):

- wait is always legal;
- while the fighter cannot act (in its own cast, or held by a status), nothing else is;
- jump and fly high only on the ground, land only in the air;
- a cast only from a filled slot whose skill is ready: cooled down, qi enough to pay, full rage for the ultimate;
  a silenced fighter casts only melee, a sealed one cannot blink;
- aiming ahead or short only for areas and blinks.

An illegal act is played as a wait.

## When it decides

- The bot thinks every 0.12 s, jittered between 0.8x and 1.2x on its own seeded dice (`TEMPERAMENT.think`), about
  eight decisions a second.
- It sees the foe 0.16 s late (`TEMPERAMENT.reaction`).
- What it decides on one tick is played on the next.
- Hit-stop holds its clock still.

These are the same for every bot and every style, and the same as the hand-written brain's, so no bot is faster than
another: the arena measures decisions, not reflexes.

## How the network picks

The network outputs one logit per act. Illegal acts are masked out, and the act is **sampled** from the softmax of
the rest with the bot's own seeded dice (`net::sample`). A bot that wants to always play its best act trains its
logits sharp.

## The weights file

A player submits one file, the format `train/export.py` writes and `pixfx_mind::policy::Net::load` reads:

```
magic        8 bytes   "PXNET1\0\0"
count        u32 LE    number of tensors
per tensor:
  name_len   u32 LE
  name       UTF-8
  rank       u32 LE
  dims       rank x u32 LE
  data       f32 LE, row-major
```

The network is fixed for a season (`net.rs`): an input layer `relu(in · x)` to width 128; two residual blocks
`h + down(relu(up(ln(h))))` with 256 inside; a final LayerNorm; a simplicial embedding (16 softmaxes of 8); the policy
head `pi` to 30 logits. A file with a missing tensor or a tensor of the wrong shape is refused, naming the tensor.
The value head the trainer uses is not part of the file. The shipped file is 794,182 bytes.

The network runs in plain `f32` in one fixed order on every target (each row summed in 8 partial sums joined in a
fixed tree, no FMA), so the same file gives the same bits natively and in wasm; `train/net.py` mirrors it in PyTorch
within 1e-4.
