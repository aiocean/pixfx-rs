# The bot

What a bot sees, what it can do, when it decides, and the file a player submits. The code is the source of truth:
`crates/pixfx-mind/src/policy/` (`observe.rs`, `action.rs`, `mind.rs`, `net.rs`) and
`games/arena/crates/pixfx-duel/src/seat.rs`.

## What it sees: 354 numbers

Each decision the bot gets `OBS` = 354 floats, in a frame where the foe is always ahead (positive x), so the same bot
plays from either corner. Every value is clamped to [-4, 4], anything not finite reads 0, and times are in seconds
unless a scale is given.

**Seeing late.** A blow the foe winds up reaches the bot 0.16 s after it starts (`TEMPERAMENT.reaction`), and a
threat in flight appears once it is 0.16 s old: that is the time to react to a danger. Everything else about the foe
(where it is, a walk, a cast it gathers that is not a blow, a recovery) is current, so an opening can be taken the
moment it shows (`pixfx-mind/src/view.rs`).

| Range | Count | What |
|---|---|---|
| 0..18 | 18 | **Me.** Distance to the wall behind and the wall ahead (over the arena's width), height (over the ceiling), facing the foe, hp / 100 (hp is a percentage of full life, so this reads 0..1), qi / 100, rage (over a full ultimate), flying, hover (over the ceiling), speed toward the foe / 200, then flags: can act, silenced, sealed, barrier up, ward up (indexes 0..15). Winding up, recovering, seconds left in that phase (15..18). |
| 18..56 | 38 | **Foe.** Distance (over the width), height, flying, hp / 100, qi / 100 (18..23). What it does: idle, moving toward me, moving away, guarding, meditating, flying (23..29). Winding up, recovering (29..31). The kind of skill it uses, one-hot over 12 kinds (31..43). How far into its phase as a share (43), seconds left (44), the phase's span (45), the skill's reach / 100 (46), damage / 50 (47), whether it is an ultimate (48); 29..49 are zero when it uses no skill. Out for, open in, frozen, shocked, barrier, the share of its skills ready, my seeker out (49..56). |
| 56..104 | 4 x 12 | **Threats** in flight, nearest first, zero when absent: present, shape (projectile, area, seeker, field, barrage), distance ahead / 100, height / 50, speed toward me / 200, lands in, size / 50, damage / 50. |
| 104..349 | 7 x 35 | **Slots**, zero when empty: the three basics, the three bar slots, the ultimate. Each: present, ready, kind (12), cost / 50, damage x power / 50, reach / 100, windup, recover, radius / 50, ultimate (+0..+21); then what it inflicts (+21..+35): freeze s, stun s, silence s / 2, defence-down s / 5, defence lost (share), burn s / 3, burn's share of attack a second x 5, chill stacks a hit adds now over the most it holds, slow per chill stack x 4, shock stacks a hit adds now over the most it holds, hold per shock twitch x 4, share of attack per shock twitch x 10, pull / 100, rage stolen over a full ultimate. |
| 349..354 | 5 | **Tag.** Its school (Thunder, Frost) and its style (aggressive, balanced, defensive), one-hot. |

The 12 skill kinds, in order: melee, dive, dash, blast, bolt, seeker, area, field, heal, ward, barrier, blink.

A skill is known by its kind and its numbers, never its name, so a bot reads a skill the bar has just drawn the same
way as one it has seen a thousand times.

What it does not see: a skill's cooldown left (only whether it is ready), its tier, traits and whether it evolved
(only their sum, in `damage x power`), and a cheap skill's lower cost (the slot shows the base cost). The two corners
see the same kinds of numbers, so neither side knows more.

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
| 9 + 3 x slot + aim | cast the skill in `slot` (0..7) with `aim` 0, 1 or 2 |

Where an aim lands:

| Skill kind | aim 0 | aim 1 | aim 2 |
|---|---|---|---|
| area | on the foe | 30 px past the foe | 30 px short of the foe |
| blink | 20 px on this side of the foe | 20 px past the foe | a full-reach blink backwards, away from the foe |
| any other | at the foe | (illegal) | (illegal) |

**Legal acts.** Each decision also comes with a mask of the acts that can happen now (`legal`):

- wait is always legal;
- while the fighter cannot act (in its own cast, or held by a status), nothing else is;
- jump and fly high whenever not flying (a jump in mid-air does nothing), land only when flying;
- a cast only from a filled slot whose skill is ready: cooled down, qi enough to pay, full rage for the ultimate;
  a silenced fighter casts only its blade (slash and dive), a sealed one cannot blink;
- aiming ahead or short only for areas and blinks.

An illegal act is played as a wait. An act that was legal when chosen but is no longer legal on the next tick (when
it is played) is dropped, and the fighter keeps doing what it was doing.

## When it decides

- The bot thinks every 0.12 s, jittered between 0.8x and 1.2x on its own seeded dice (`TEMPERAMENT.think`): at most
  about eight decisions a second.
- A decision comes only while the fighter can act: never during its own cast, a stagger, a freeze or shock, the
  crouch before a jump, or the 0.4 s of a duck. After a round starts, the first decision comes 0.4-0.7 s in.
- What it decides on one tick is played on the next.
- An act lasts until the next one: walking and meditating carry on, and `wait` stops both.
- Hit-stop on the page pauses the whole duel, so it changes nothing for a bot.

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
head `pi` to 30 logits. LayerNorms use eps 1e-5. The tensors (`net.rs`, `layout`):

| Name | Shape |
|---|---|
| `in.w`, `in.b` | [128, 354], [128] |
| `block{0,1}.ln.g`, `block{0,1}.ln.b` | [128], [128] |
| `block{0,1}.up.w`, `block{0,1}.up.b` | [256, 128], [256] |
| `block{0,1}.down.w`, `block{0,1}.down.b` | [128, 256], [128] |
| `out.ln.g`, `out.ln.b` | [128], [128] |
| `sem.w`, `sem.b` | [128, 128], [128] |
| `pi.w`, `pi.b` | [30, 128], [30] |

A file with a missing tensor or a tensor of the wrong shape is refused, naming the tensor; extra tensors are ignored.
The value head the trainer uses is not part of the file. The shipped file is 794,182 bytes.

The network runs in plain `f32` in one fixed order on every target (each row summed in 8 partial sums joined in a
fixed tree, no FMA), so the same file gives the same bits natively and in wasm; `train/net.py` mirrors it in PyTorch
within 1e-4.
