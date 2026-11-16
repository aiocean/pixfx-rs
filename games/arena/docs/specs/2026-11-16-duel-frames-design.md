# Duel frames: design

Status: shipped 2026-11-16 (main `629f9d1`, the page deployed with the policy retrained on these rules). Every number
is in `pixfx_duel::data::FRAMES` and the spells' `recover`; the rules' tests are `games/arena/crates/pixfx-duel/tests/frames.rs`.

Built differently from the text below:

- An ultimate gets no extra recovery when blocked: it falls as a zone, which no guard stops, so it is never blocked.
  `FRAMES.unsafe_on_block` is dive, step, palm, burst and lotus.
- The rules carry no hit-stop numbers. A blow says what kind it was (`Out::Hit.how`), and the feel decides how long it
  stops (`pixfx_fighter::feel::Stop`: counter 2, punish 3 extra ticks) and floats the word (`NumberKind::Counter` /
  `Punish`). `crates/pixfx-wasm/tests/hit_words.rs` keeps the duel's spelling of the kind and the feel's the same.

## Intent

Each skill has a visible start (windup), a moment it lands, and a recovery, and each has its weak spots, the way
Street Fighter's frame data does: hit a fighter while it gathers a cast and it is a **COUNTER**; hit it while it
recovers and it is a **PUNISH**; a heavy blow that is guarded leaves its striker open; an ultimate's opening cannot be
hit. The windows are sized for a viewer and for the policy, not to the frame: a viewer cannot see a 1-frame
difference, and the policy decides every 0.096-0.144 s and sees 0.16 s late. Every window is ≥ 0.1 s (6 ticks), and
"safe" and "unsafe" moves differ by ≥ 8 ticks.

## What exists today

- A cast runs Windup -> `release` -> Recover -> done (`games/arena/crates/pixfx-duel/src/act.rs:240-264`), with fixed `windup`
  and `recover` per spell (`src/data.rs:545-546`). Pose shows `Doing::Windup` / `Doing::Recover` (`act.rs:106-114`),
  so the phases are already visible.
- Everything is paid at `begin`: qi (`act.rs:133`), rage for an ultimate (`act.rs:136`), cooldown (`act.rs:145-146`),
  the bar slot (`act.rs:161`). A hit breaks the cast (`act.rs:858`, `break_act` `act.rs:934-942`), so a fighter hit
  in Windup already loses the whole cast: a counter-hit's penalty without a bonus.
- Melee and blasts check once, at release (`act.rs:315`, `act.rs:437`); a dash checks the line it cuts once
  (`act.rs:504`). Shots (bolt, orb, zone, field) hit later from `shot.rs` (`shot.rs:129,152,257,322`), all through
  `hurt` (`act.rs:733`). A slash meeting a slash late in its windup clashes (`act.rs:319-328`).
- A hit in Recover tells the struck fighter's mind `Outcome::Punished` (`act.rs:846-856`); there is no extra damage or
  stun. The utility mind scores it -1 (`crates/pixfx-mind/src/mind.rs:207`).
- Stagger and launch (`crates/pixfx-fighter/src/body.rs`): power ≥ 12 launches (`body.rs:124`, `heavy`
  `body.rs:338`); a lighter blow staggers `0.24 + 0.012 × power` s (`body.rs:129-130`, `hit` `body.rs:551`); no
  invincibility after a blow (`body.rs:147`); one juggle, and the floor is untouchable (`body.rs:696-702`).
- Hit-stop (`crates/pixfx-fighter/src/feel.rs:101-108`, `hit_stop` `feel.rs:347-360`): light 6 frames, medium
  (power ≥ 9) 8, launch 11; crit +3, ranged −2; capped at 13 (`crates/pixfx-core/src/feel.rs:52`).
- Guard: `Choice::Guard { hold }` sets a timer (`duel.rs:326`; the policy holds 0.4 s, `policy/action.rs:22`) that
  decays (`duel.rs:236`). `blocks` (`act.rs:624-649`) needs facing, not reeling, and 8 qi (`data.rs:435`); the chance
  to turn a blow aside is `min(1, 0.5r, 0.5√r)`, `r = 20/power` (`data.rs:439-449`): slash 1.0, bolt 0.62, palm
  0.50, burst 0.42; chip 0.2 (`data.rs:437`). A blocked melee pushes both apart, a blocked blast or dash pushes only
  the defender (`act.rs:324,442,506`, `KNOCKBACK` 130 `data.rs:451`, about 22 px of slide). The defender has no
  blockstun (`duel/src/sense.rs:209-216`).
- **Bug:** `begin` never clears `guard` and `blocks` never checks `act`, so a fighter that guards, then casts, still
  blocks all through its windup while it looks like it is casting (`act.rs:106`, `act.rs:127-236`, `act.rs:624-627`).
- Golden bell: a close Melee, Blast or Dash striking it takes `Counter { damage 9, stun 0.6 }` (`data.rs:959-973`,
  `act.rs:665-692`). Its 0.06 s windup is already the duel's "reversal".
- What the policy sees (`crates/pixfx-mind/src/policy/observe.rs`): the foe's phase flags, its skill kind, and how
  far into its phase as a fraction (`observe.rs:146-153`): no seconds left, no span, no foe ultimate flag. Of itself,
  only `can_act` (`observe.rs:112`). `OBS` = 15 + 35 + 48 + 147 + 5 = 250 (`observe.rs:30`). It sees the foe 0.16 s
  late (`data.rs:430`), thinks every 0.12 s × U(0.8, 1.2) (`policy/mind.rs:95-98`) and acts a tick later.
- Events: `Out::Hit` (`src/out.rs:61-72`) becomes `Cue::Hit` (`fighter/src/feel.rs:167-177`), which drives hit-stop,
  jolt, trauma and the floating number (`feel.rs:417-450`, `NumberKind` `feel.rs:253-260`). The page only calls
  `view.feel.react(commands)` (`site/assets/landing.js:176`) and draws numbers with the look table `NUMBER`
  (`site/assets/duel-art.js:136-143`).

### Frames today (seconds; × 60 = ticks)

| Kind | Skills (school) | Windup | Recover |
|---|---|---|---|
| Melee | slash (both) | 0.20 | 0.32 |
| Dive | dive (both) | 0.22 | 0.40 |
| Dash | step (T) | 0.26 | 0.32 |
| Blast, close | palm (T) .32, burst (T) .36, lotus (F) .30 | 0.30-0.36 | 0.30-0.40 |
| Blast, mid | jet (F) | 0.30 | 0.30 |
| Bolt | bolt (T) .28, spark (T) .18, shard (F) .18 | 0.18-0.28 | 0.18-0.22 |
| Seeker | orb (T) | 0.60 | 0.30 |
| Area | seal, quake (T), shatter, icicle .42, collapse .50 (F) | 0.30-0.50 | 0.28-0.30 |
| Field | frostfire, flurry .40, void .90 (F) | 0.40-0.90 | 0.30-0.45 |
| Heal / Ward | heal .85 (T), ward .50 (F), hex .45 (T) | 0.45-0.85 | 0.25-0.30 |
| Barrier / Blink | bell (T), mirror (F), blink (both) | 0.06 | 0.10-0.12 |
| Ultimate | tribulation (T), prison (F) | 0.70 | 0.50 |

What the policy can answer: it reacts in about 0.16 + U(0, 0.144) + 0.017 s (mean ≈ 0.24 s), and its fastest blows
take 0.18-0.20 s. So it can **counter on reaction** only windups of about 0.45 s or more (orb, collapse, ward, heal,
void, ultimates, icicle at the edge), and **punish on reaction** only recoveries of about 0.45 s or more (today only
void and the ultimates). Everything else is caught by a read. That is right for a viewer: slow, big moves are the ones
seen to be caught.

## Prior art: what we take and what we leave

| Street Fighter idea | Taken? | Why |
|---|---|---|
| Startup / recovery | Already there | `windup` / `recover` exist and show as poses. |
| Counter hit (hit in startup) | Yes, a small bonus | The victim already loses the whole cast; the bonus is mostly the pop and the feel. |
| Punish counter (hit in recovery) | Yes, a bigger bonus | Being hit in Recover costs nothing extra today: the missing risk. |
| Safe / unsafe moves | Yes, two tiers ≥ 8 ticks apart | A viewer and a 7-tick decider can tell 0.32 s from 0.5 s, not 0.30 from 0.33. |
| Frame advantage on block | Partly: extra recovery for heavy close moves | No per-move blockstun; the defender is already free at once. |
| Invincible reversals (DP) | No new ones | Bell and mirror (0.06 s windup, the bell's counter) do that job. |
| Invincible super startup | Yes, ultimates only, first 0.3 s | Losing 4 blows of rage to a poke in a 0.7 s windup is the worst feel in the duel. |
| Active frames | No | See rule 5. |
| Plus/minus frames, links, cancels, meaties | No | Nothing decides at 1/60 s; viewers cannot read them. |

## Proposed rules

All numbers live in one new table `FRAMES` in `data.rs`, so the balance loop can tune them.

### 1. COUNTER: struck during Windup (HIGH)

Power × **1.2** (the power `hurt` uses for damage and launch, applied after the payoff, `act.rs:769-784`), stagger
**+0.10 s** when the blow staggers, hit-stop **+2 frames** (cap 13 kept), `Out::Hit.how = "counter"`. It applies to
every blow through `hurt` (melee, dive, dash, blast, bolt, seeker, area, and the bell's answer, which lands while the
striker is still in Windup, `act.rs:245-257`), but **not** to field ticks (frostfire, flurry, void, `shot.rs:322`): a
lingering field must not auto-counter. It is self-limiting: the first hit breaks the act (`act.rs:858`).

### 2. PUNISH: struck during Recover (HIGH for the rule, MEDIUM for the numbers)

Power × **1.2**, stagger **+0.20 s**, hit-stop **+3 frames**, `how = "punish"`; field ticks excluded. A punished
slash staggers 0.30 + 0.20 = 0.50 s, long enough for a visible follow-up (slash windup 0.20). No chain: the punish
breaks the act, so the follow-up hits a fighter no longer in Recover.

### 3. Unsafe recoveries, by kind (MEDIUM; exact values go to the balance loop)

Safe: recover ≤ 0.32 s, a punish needs a read. Unsafe: recover ≥ 0.45 s, about the policy's mean reaction plus its
fastest blow.

| Kind | Skills | Recover today -> proposed | Extra recover when blocked |
|---|---|---|---|
| Melee | slash | 0.32 (safe, unchanged) | 0 (the clash pushes both apart) |
| Dive | dive | 0.40 -> **0.50** | +0.15 |
| Dash | step | 0.32 -> **0.45** | +0.15 |
| Blast, close | palm, burst, lotus | .35/.40/.30 -> **.50/.50/.45** | +0.15 |
| Blast, mid | jet | 0.30 -> 0.35 | 0 |
| Bolt, Seeker, Area, Field | all | unchanged | n/a (shots resolve after the caster is free) |
| Heal | heal | 0.30 -> **0.45** | n/a |
| Ward, Barrier, Blink | ward, hex, bell, mirror, blink | unchanged | n/a |
| Ultimate | tribulation, prison | 0.50 -> **0.80** | +0.15 |

Projectiles and zones stay safe: that is their price for range (Street Fighter's fireball rule). Close heavies, dive,
dash, heal and ultimates become the moves that get punished.

### 4. Guard (HIGH for the fix, MEDIUM for the extra recovery)

- `begin` clears `guard`: a fighter casting is not guarding, which is what the pose already shows. Without this,
  counters against guard-then-cast are silently blocked.
- On block, Dive, Dash, close Blast and Ultimate add **+0.15 s** to the striker's Recover (`BLOCKED_RECOVER`). Its
  Recover is set at `advance` (`act.rs:249-253`), so `blocks` returning true marks the act.
- The defender gets no blockstun; it is already free at once. No new dice: `blocks` already rolls once (`act.rs:635`).

### 5. Active frames: not added (HIGH)

Melee and blast resolve on the release tick. Every lasting hitbox the duel needs already exists as a shot (bolt
flight, zone sigil and impact, field life, jet spray). A 3-6 tick active window is shorter than one policy decision
(6-9 ticks), so it would never be used on purpose; what it would add is wake-up "meaties" (which the floor's
untouchability already removes) and accidental trades, at the cost of a second hit-test path per kind.

### 6. Ultimate invincibility (MEDIUM)

During an ultimate's first **0.30 s** of windup (of 0.70), `hurt` returns `false` at the `untouchable` check
(`act.rs:741`): no dice are drawn for that blow, and an `Out::Dodge` at the caster shows it passed through. Bounds:
it needs full rage (4 landed blows, `data.rs:20`); its last 0.40 s of windup can be hit (a COUNTER that wipes the
rage); its recovery becomes 0.80 s (+0.15 when blocked). Blocks are unaffected: the caster is not guarding (rule 4).

## What the policy must see (HIGH)

| Block | Add | Why |
|---|---|---|
| foe (35 -> 38) | its phase's seconds left (`span - t`), its span (s), its ultimate flag | A fraction cannot tell dive's 0.50 recovery from slash's 0.32; the flag shows the invincible start. |
| me (15 -> 18) | own Windup flag, own Recover flag, own seconds left | To plan the follow-up after a punish, and to know it is exposed. |
| slots | unchanged size | Each slot already carries windup and recover (`observe.rs:216-217`); only the values change. |

`Sense.me` gains `act: Option<(phase, t, windup, recover)>`; `doing_of` (`duel/src/sense.rs:98-121`) already holds
the foe's. Both are serde `deny_unknown_fields` (`mind/src/sense.rs`), so the wasm `mind` types and
`site/tests/mind.test.js` fixtures update.

`OBS` grows (250 -> 256, plus the status fields of auto-balance step 2), and `Net`'s input tensor changes shape, so
the old `policy.bin` is refused (`NetError::Shape`). One input change carries both the frame and the status fields,
and one full retrain makes the auto-balance spec's base model. A cheaper first look: widen the input layer with zero
columns (same outputs, since `Net` is `relu(in · x)` with no input normalisation, `policy/net.rs:1-4`) and fine-tune a
few hundred updates, to see whether the rules play well (LOW: a fine-tune may not find the punish game).

No reward is added for counters or punishes: winning and the decaying damage term already pay for them, and a direct
bonus would be farmed.

## The page and its events (HIGH)

`Out::Hit` gains `how: "plain" | "counter" | "punish"` (one field; the hash changes once). In `pixfx-fighter`'s feel,
`Cue::Hit` gains `#[serde(default)] how`, `Stop` gains `counter: 2.0` and `punish: 3.0`, `NumberKind` gains `Counter`
and `Punish`, and `react` floats **COUNTER!** / **PUNISH!** above the damage number with crit-sized trauma. Rust
decides all of it; the page only gains two look entries in `NUMBER` (`duel-art.js:136-143`), for example counter
`{ size: 11, color: "#ffd84a" }` and punish `{ size: 12, color: "#ff3d6a" }`. The ultimate's invincible start reuses
`Out::Dodge`, so it needs no page change.

## Measurement

`ladder::Record` (`ladder.rs:27-42`) gains, per school and per seat (dealt and taken), counters, punishes and hits,
counted from `Out::Hit.how`, plus blows that passed through an ultimate's start. The ladder example prints them.

| Check (150 seeds both ways, policy vs itself) | Target |
|---|---|
| Punishes / landed blows | 10-25% |
| Counters / landed blows | 5-15% |
| Mean round length, time-outs | still 15-45 s, 0 (spec "Done when") |
| Attack share | no more than 10% below the base model before frames |
| Guard among choices | under 25% of decisions |
| Thunder in the mirror | reported; fixed by the balance loop, not here |

## Risks and how each is tested

1. **Stalling, everyone waiting to punish.** Safe pokes and projectiles keep paying; `STALL` and `QUIET` already
   charge long quiet stretches (`train.rs`). Test: the ladder's length, time-outs and attack share.
2. **Turtling behind guard, now that blocks hurt the striker.** Guard still costs 8 qi a block and heavy blows break
   it about half the time. Test: guard's share of choices.
3. **Ultimate too strong.** Unit tests in `tests/act.rs` (a blow at 0.29 s passes through; at 0.31 s it lands as a
   COUNTER); the ladder's ultimate hit rate before and after.
4. **Dice order.** COUNTER, PUNISH, the recover changes and the block extra are pure functions of state: no new rolls.
   The ultimate's early return skips the dodge, crit and damage rolls for that blow, on purpose. Test: a COUNTER
   draws exactly the dice a plain hit draws (equal rng state after both); `site/tests/policy.test.js` still matches
   native and wasm.
5. **Goldens.** `event_hashes.json`, `first_mind.json` and `policy_fight.json` re-pin on purpose; `arenas.json` (pools
   and opening bars) must not change.
6. **The shipped policy plays the old game until retrained.** The rules and the new `policy.bin` land in one change.
7. **Doc drift.** `games/arena/crates/pixfx-duel/CLAUDE.md` still names a `duel.js` parity gate and a dump tool that no longer
   exist (`site/assets/duel.js` is a 74-line wrapper); fix it with this work.

## Decided (user, 2026-11-16)

1. **The safe/unsafe split as above.** Close heavies, dive, dash, heal and ultimates become punishable; projectiles and
   zones stay safe. Thunder holds more of the unsafe moves, so the balance pass gives it something back.
2. **One full retrain from scratch** for the frame fields and the status fields together; it is the base model of the
   balance work.
3. **An ultimate's first 0.30 s is invincible**: blows pass through, shown as a dodge.

## Build order

1. `FRAMES` table; `begin` clears guard; COUNTER and PUNISH in `hurt`; recover changes; block extra; ultimate start.
   Unit tests in `tests/act.rs`; re-pin goldens; `cargo test` and clippy for `pixfx-duel`.
2. `Out::Hit.how`; `Cue`, `Stop` and `NumberKind` in `pixfx-fighter`'s feel; two `NUMBER` entries. Watch it on the
   page with the current policy.
3. Ladder counts; run the current policy for a before/after table (no retrain yet).
4. `Sense` and observation changes with the status fields; `OBS` grows; one base retrain; export; re-pin
   `policy_fight.json`.
5. The user watches rounds; then the auto-balance loop takes `FRAMES` into its allowed list.
