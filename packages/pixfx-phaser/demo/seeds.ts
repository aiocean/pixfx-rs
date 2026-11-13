// How the stress page seeds its effects: `unique` gives each effect its own seed, so no two show the same frame
// and nothing is shared; `shared` gives every effect of one id the same seed, as a game placing many torches
// without a seed does, so identical effects can share frames.
export type Seeds = "unique" | "shared";

export function seedsParam(v: string | null): Seeds {
  return v === "shared" ? "shared" : "unique";
}

/** The seed of the effect spawned `spawned`-th (from 0), whose id is `IDS[idIndex]`. */
export function stressSeed(seeds: Seeds, spawned: number, idIndex: number): number {
  return seeds === "shared" ? 1 + idIndex : 1 + spawned;
}
