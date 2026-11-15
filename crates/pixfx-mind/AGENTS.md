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
- `dt = 0` (hit-stop) must hold every clock of the mind still.
- A skill is known by its `SkillKind`, never its name: the name is only handed back in `Choice::Cast`.

## Before you finish

```
cargo test -p pixfx-mind --all-features
cargo clippy -p pixfx-mind --all-targets --all-features -- -D warnings
```
