import { expect, test } from "bun:test";
import * as pkg from "../src/index";
import "./wasm";

// the site's duel (site/assets/duel.js) reaches the duel only through this package: every binding it constructs or calls
test("the package hands the site's duel what it uses", () => {
  for (const name of ["Duel", "Policy", "duelConstants", "duelSpells", "duelStats", "duelTraits", "damageOf", "orbOffset", "effects"] as const) {
    expect(typeof pkg[name]).toBe("function");
  }
});
