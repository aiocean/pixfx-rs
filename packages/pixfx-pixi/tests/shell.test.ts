import { expect, test } from "bun:test";
import { Container, Sprite } from "pixi.js";
import { Game, Stage } from "pixfx-web";
import { pixiBufferTextures, pixiImages } from "../src/pixi-gfx";
import { memory, wasm } from "./wasm";

/** A stage drawing into a Pixi container through the shell's textures and sprites, painting on the spot. */
function setup() {
  const game = new Game(wasm, memory, []);
  const { textures, textureOf } = pixiBufferTextures();
  game.useTextures(textures);
  const container = new Container();
  const stage = new Stage<null>(game, pixiImages(container, textureOf), () => {}, null);
  const frame = (dt = 1 / 30): void => {
    stage.update(dt);
    stage.draw();
    game.endFrame();
  };
  return { game, container, stage, frame };
}

function shown(container: Container): Sprite[] {
  return container.children.filter((c): c is Sprite => c instanceof Sprite && c.visible);
}

test("an effect shows as sprites near where it plays, its colour normal and its light added, each its frame's size", () => {
  const { container, stage, frame } = setup();
  stage.play("explosions/body-burst", 30, 40, {}, null);
  frame();
  const sprites = shown(container);
  expect(sprites.length).toBeGreaterThan(0);
  expect(new Set(sprites.map((s) => s.blendMode))).toEqual(new Set(["normal", "add"]));
  for (const s of sprites) {
    // the body about to burst stands on its origin
    expect(s.x).toBeLessThanOrEqual(30);
    expect(s.x + s.width).toBeGreaterThan(30);
    expect(s.y).toBeLessThan(40);
    expect([s.width * 2, s.height * 2]).toEqual([s.texture.frame.width, s.texture.frame.height]);
    expect(s.texture.source.scaleMode).toBe("nearest");
  }
});

test("the back and front sprites take the play's depths as zIndex", () => {
  const { container, stage, frame } = setup();
  // a power-up's aura draws behind and in front of the body it grips, once it has come in
  stage.play("feedback/power-up", 30, 40, {}, null, { back: 3, front: 9 });
  for (let i = 0; i < 16; i++) frame();
  expect(new Set(shown(container).map((s) => s.zIndex))).toEqual(new Set([3, 9]));
});

test("a frame's pixels land in the shell's texture, colour beside light", () => {
  const { container, stage, frame } = setup();
  stage.play("explosions/body-burst", 30, 40, {}, null);
  frame();
  for (const s of shown(container)) {
    const src = s.texture.source;
    const bytes: unknown = src.resource;
    if (!(bytes instanceof Uint8Array)) throw new Error("not a buffer texture");
    const { x, y, width, height } = s.texture.frame;
    let lit = 0;
    for (let row = y; row < y + height; row++) {
      for (let col = x; col < x + width; col++) if ((bytes[(row * src.width + col) * 4 + 3] ?? 0) !== 0) lit++;
    }
    expect(lit).toBeGreaterThan(0);
  }
});

test("destroying the stage leaves the container empty and every effect destroyed", () => {
  const { game, container, stage, frame } = setup();
  const ended: string[] = [];
  for (let i = 0; i < 3; i++) stage.play("feedback/power-up", 20 * i, 40, { seed: i }, null).on("destroy", () => ended.push("destroy"));
  frame();
  frame();
  expect(container.children.length).toBeGreaterThan(0);
  stage.clear();
  expect(container.children.length).toBe(0);
  expect(ended).toEqual(["destroy", "destroy", "destroy"]);
  game.clear();
  expect(game.counts()).toEqual({ surfaces: 0, textures: 0 });
});
