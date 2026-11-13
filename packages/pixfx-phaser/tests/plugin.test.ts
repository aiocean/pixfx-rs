import { expect, mock, test } from "bun:test";
import EventEmitter from "eventemitter3";
import type { Backend } from "../src/backend";
import type { EngineOptions, FxLike } from "../src/engine";
import { InlineBackend } from "../src/inline";
import { FakeEngine } from "./fake-engine";
import { FakeGfx } from "./fake-gfx";

// Phaser's runtime needs a DOM. Phaser.Events.EventEmitter is a thin wrapper over eventemitter3,
// the only Phaser value the Phaser-free effect code uses, so the test supplies exactly that.
void mock.module("phaser", () => ({ default: { Events: { EventEmitter } } }));
const { EffectManager } = await import("../src/effects");
const { PixfxFeel } = await import("../src/feel");

/** Records the options handed to the engine. */
class SpyEngine extends FakeEngine {
  readonly opts: EngineOptions[] = [];
  override create(id: string, opts: EngineOptions): FxLike {
    const fx = super.create(id, opts);
    this.opts.push(opts);
    return fx;
  }
}

function setup() {
  const engine = new SpyEngine();
  const gfx = new FakeGfx();
  const backend = new InlineBackend(engine);
  const fx = new EffectManager(gfx, (): Backend => backend, "pixfx:test:", new PixfxFeel(() => engine));
  return { engine, gfx, fx };
}

const EMPTY = { engines: 0, textures: 0, images: 0 };

test("light images blend additively, colour images normally, light drawn after colour", () => {
  const { gfx, fx } = setup();
  fx.play("fake/spark", 0, 0);
  expect(gfx.images.map((i) => i.blend)).toEqual(["normal", "add", "normal", "add"]);
  expect(gfx.images.map((i) => i.key)).toEqual(gfx.textures.map((t) => t.key));
});

test("scene shutdown with three effects playing frees everything the scene holds", () => {
  const { engine, gfx, fx } = setup();
  for (let i = 0; i < 3; i++) fx.play("fake/spark", i, i);
  // three identical effects show one shared frame: four textures for the game, none of the scene's own
  expect(fx.stats()).toEqual({ engines: 3, textures: 0, images: 12 });
  expect(gfx.textures.length).toBe(4);
  fx.destroyAll();
  expect(fx.stats()).toEqual(EMPTY);
  expect(engine.live).toBe(0);
  expect(gfx.images.every((i) => i.destroyed)).toBe(true);
  // A restarted scene plays again and shows the frame the game still holds.
  const again = fx.play("fake/spark", 0, 0);
  expect(gfx.textures.length).toBe(4);
  expect(gfx.images.slice(12).map((i) => i.key)).toEqual(gfx.textures.map((t) => t.key));
  again.destroy();
});
