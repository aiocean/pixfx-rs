# pixfx-mind

Layer: **Engine**. Read the root `AGENTS.md` ("The boundary" and "Layers and where code goes") before changing
anything here; this file only says what this crate holds. The design is
`docs/superpowers/specs/2026-11-14-pixfx-mind-design.md`.

## Job

A fighter's brain. Each tick the game hands it a `Sense` (itself, the foe, the arena, the threats in flight, which
of its skills are ready) and it answers, when it is time to think, with a `Thought`: a `Choice` and the `Posture` it
is in. Inside: the foe seen `reaction` seconds late (`view`), a `Reading` of the moment (`reading`), the foe's habits
and yomi levels (`habits`, `yomi`), postures chosen by scored considerations with commitment and interrupt bands
(`curve`, `posture`), candidate actions per skill kind and a rank-then-weight pick (`candidates`, `pick`), and the
`Mind` that holds it all for a round (`mind`).

A trained policy's runtime too (`policy`): what it sees (`observe`, a frame where the foe is ahead), the acts it can
take and which are legal now (`Act`, `legal`, `choice`), the network in plain `f32` with its weights file (`Net`,
`sample`), and `PolicyMind`, which sees as late and thinks as often as `Mind` and acts on the tick after it decides.
The trainer that makes the weights is `train/`; the duel's weights live in `pixfx-duel`.

The policy sees frame data (`docs/superpowers/specs/2026-11-16-duel-frames-design.md`): its own act (`Me.act`:
winding up or recovering, and the seconds left), the foe's phase with the seconds left in it, its span and whether
it is an ultimate (whose opening cannot be hit), and each slot's windup, recover and what it inflicts in sizes
(`Inflicts`), so a slow recovery reads as an opening to punish. `observe.rs`'s header lists every float (`OBS` = 354);
a change to it changes the weights file's input size and needs a retrain.

## Belongs here

- How a fighter decides: postures, scoring, reading and learning the foe, reaction time, tempo.

## Does not belong here

| If the code... | It goes to |
|---|---|
| is the rules of a fight: damage, cooldowns, shots, statuses | the game (the landing duel's: `pixfx-duel`) |
| names a game's skill or school | the game, as `Skill` and `Profile` data |
| moves a body or poses it | `pixfx-fighter` |
| is a JS binding of these types | `pixfx-wasm` (`mind` feature) |

## Dependencies

- Required: `libm`.
- Optional, behind `serde` / `ts`: `serde`, `tsify`, `wasm-bindgen`.
- Never `pixfx-fighter`, `pixfx-scene`, `pixfx-wasm` or a shell.

## Traps

- Pure: no clock (time comes in the `Sense`), no I/O, randomness only from the seed `Mind::new` takes.
- `dt = 0` (hit-stop) must hold every clock of the mind still (`PolicyMind` holds its queued act too).
- `policy::Net` sums each row in 8 partial sums joined in a fixed tree: change that order and every target's bits,
  the pinned hash in `tests/policy_net.rs`, and `train/net.py`'s match (within 1e-4) must all be checked.
- A skill is known by its `SkillKind`, never its name: the name is only handed back in `Choice::Cast`.

## Before you finish

```
cargo test -p pixfx-mind --all-features
cargo clippy -p pixfx-mind --all-targets --all-features -- -D warnings
```
