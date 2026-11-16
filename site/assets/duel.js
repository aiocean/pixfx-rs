// The landing duel, whose rules are games/arena/crates/pixfx-duel, run through the engine's wasm (`Duel`). This file only hands
// the page what the rules decided: a duel whose `fighters`, `time`, `round`, `wins`, `winner`, `left`, `right` and
// `ground` are refreshed from the rules' view after each `step`, and the rules' tables, read from the engine once it
// has loaded (`useEngine`). It decides nothing.

/** The engine's duel bindings, set by `useEngine`. */
let engine = null;
/** Each corner's trained policy, `[left, right]`, set by `usePolicies` once their files have arrived; until then duels
 *  seat the brain. */
let policies = null;

/** The engine, or a clear error when the page forgot `useEngine`. */
function loaded() {
  if (!engine) throw new Error("duel.js: call useEngine(pkg) after loadPixfx() first");
  return engine;
}

/** Every spell by name, each school's fighter stats, what each trait does: the rules' own tables. */
export let SPELLS = {};
export let STATS = {};
export let TRAITS = {};
/** The rage an ultimate needs, the seconds a bar skill waits unused before it leaves, the decor effects and the
 *  tiers, lowest first. */
export let RAGE = 0;
export let STALE = 0;
export let DECOR = {};
export let TIERS = [];

/**
 * Read the rules' tables from the engine, once it has loaded (`await loadPixfx()`): `pkg` holds `Duel`, `effects`,
 * `duelSpells`, `duelStats`, `duelTraits`, `duelConstants`, `damageOf` and `orbOffset`.
 */
export function useEngine(pkg) {
  engine = pkg;
  SPELLS = pkg.duelSpells();
  STATS = pkg.duelStats();
  TRAITS = pkg.duelTraits();
  ({ RAGE, STALE, DECOR, TIERS } = pkg.duelConstants());
}

/**
 * Seat a trained policy in each corner of every duel created from now on: `[left, right]` are the bytes of weights files
 * (`games/arena/crates/pixfx-duel/weights/*.bin`), which the page fetches beside the wasm. The same bytes in both corners are read
 * once. Throws when they are not a policy of this duel's shape.
 */
export function usePolicies([left, right]) {
  const { Policy } = loaded();
  const l = new Policy(left);
  const next = [l, right === left ? l : new Policy(right)];
  if (policies) for (const p of new Set(policies)) p.free();
  policies = next;
}

/**
 * A new duel in the arena from `left` to `right` (game pixels), standing on `ground`, flying no higher than `sky`.
 * `has(id)` says whether an effect is in the registry. `seats`, for tests, seats another mind in a corner:
 * `["policy" | "brain" | "first", ...]` (each corner's trained policy when `usePolicies` has run, else the brain, when left out).
 */
export function createDuel({ seed = 1, left, right, ground, has, sky, seats }) {
  const pkg = loaded();
  const effects = pkg.effects().filter((id) => has(id));
  const config = { seed, left, right, ground, sky, effects };
  const rs = policies ? pkg.Duel.withPolicies(config, seats, ...policies) : new pkg.Duel(config, seats);
  return { rs, seed, ...rs.view() };
}

/** Advance the duel by `dt` seconds; returns the effect commands and events of this step. */
export function step(duel, dt) {
  const out = duel.rs.step(dt);
  Object.assign(duel, duel.rs.view());
  return out;
}

/** Let go of a duel the page no longer shows. */
export function freeDuel(duel) {
  duel.rs.free();
}

/** Fighter f's skill bar: three bar slots, then the ultimate. */
export const skillBar = (f) => f.skillBar;

/** What the HUD shows under fighter f's bars, good first, then bad. */
export const buffsOf = (f) => f.buffs;

/** The damage `by` (`{stats, fury}`) deals `f` (`{stats, defDown}`) with a blow of `power` and `type`. */
export function damageOf(by, f, power, type, crit, roll) {
  return loaded().damageOf({ stats: by.stats, fury: by.fury }, { stats: f.stats, defDown: f.defDown }, power, type, crit, roll);
}

/** Where an orb bobs `t` seconds after it was cast, from its line: `[dx, dy]`. */
export const orbOffset = (t) => loaded().orbOffset(t);
