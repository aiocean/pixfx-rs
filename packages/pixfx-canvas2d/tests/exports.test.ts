import { expect, test } from "bun:test";
import * as pkg from "../src/index";
import "./wasm";

// the arena duel's page kit (projects/arena/site/embed.sh), which the pixfx site embeds, reaches the engine only
// through this package: every binding it constructs or calls
test("the package hands the arena duel's page what it uses", () => {
  for (const name of ["createPixfx", "loadPixfx", "effects", "FightFeel", "Lighting", "Sprites", "lookColors", "putOutLight", "stepLights"] as const) {
    expect(typeof pkg[name]).toBe("function");
  }
});
