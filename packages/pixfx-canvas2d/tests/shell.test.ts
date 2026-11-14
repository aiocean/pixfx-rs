import { expect, test } from "bun:test";
import { Game, Stage } from "pixfx-web";
import type { DrawContext, Holder, PictureLike } from "../src/canvas-gfx";
import { canvasImages, canvasPictures, canvasTextures } from "../src/canvas-gfx";
import type { Appearance } from "pixfx-web/wasm";
import { memory, wasm } from "./wasm";

/** A holder keeping its bytes, so a test reads what reached it. */
interface FakeHolder extends Holder {
  readonly bytes: Uint8ClampedArray;
}

function fakeHolder(width: number, height: number): FakeHolder {
  const bytes = new Uint8ClampedArray(width * height * 4);
  // what drawImage is handed: shaped as an ImageBitmap, which a canvas draws from
  const source: ImageBitmap = { width, height, close() {} };
  return {
    source,
    width,
    bytes,
    put(src, x, w, h) {
      for (let row = 0; row < h; row++) bytes.set(src.subarray(row * w * 4, (row + 1) * w * 4), (row * width + x) * 4);
    },
  };
}

interface Drawn {
  holder: FakeHolder;
  op: string;
  sx: number;
  sw: number;
  dx: number;
  dy: number;
  dw: number;
  dh: number;
}

/** A context recording each draw and the compositing it was drawn with. */
function fakeContext(holders: Map<CanvasImageSource, FakeHolder>): { ctx: DrawContext; drawn: Drawn[] } {
  const drawn: Drawn[] = [];
  const ctx: DrawContext = {
    globalCompositeOperation: "source-over",
    imageSmoothingEnabled: true,
    save() {},
    restore() {
      ctx.globalCompositeOperation = "source-over";
    },
    drawImage(source: CanvasImageSource, ...n: number[]) {
      const holder = holders.get(source);
      if (!holder) throw new Error("drew something that is no holder");
      const [sx = 0, , sw = 0, , dx = 0, dy = 0, dw = 0, dh = 0] = n;
      drawn.push({ holder, op: ctx.globalCompositeOperation, sx, sw, dx, dy, dw, dh });
    },
  };
  return { ctx, drawn };
}

/** A stage drawing through the shell's holders and images, painting on the spot. */
function setup(scale = 3) {
  const game = new Game(wasm, memory, []);
  const holders = new Map<CanvasImageSource, FakeHolder>();
  const { textures, holderOf } = canvasTextures((w, h) => {
    const holder = fakeHolder(w, h);
    holders.set(holder.source, holder);
    return holder;
  });
  game.useTextures(textures);
  const images = canvasImages(holderOf);
  const stage = new Stage<null>(game, images.images, () => {}, null);
  const { ctx, drawn } = fakeContext(holders);
  const frame = (dt = 1 / 30): void => {
    stage.update(dt);
    stage.draw();
    game.endFrame();
  };
  const draw = (depth: number | null): Drawn[] => {
    drawn.length = 0;
    images.draw(ctx, scale, depth);
    return [...drawn];
  };
  return { stage, frame, draw, ctx };
}

const DEPTHS = { back: 0, front: 1 };

test("an effect draws near where it plays, scaled, its colour normally then its light added", () => {
  const { stage, frame, draw } = setup(3);
  stage.play("fire/candle", 30, 40, {}, null, DEPTHS);
  frame();
  const drawn = draw(null);
  expect(drawn.length).toBeGreaterThan(0);
  expect(new Set(drawn.map((d) => d.op))).toEqual(new Set(["source-over", "lighter"]));
  for (const d of drawn) {
    // a candle's flame sits on its origin, three canvas pixels to a game pixel
    expect(d.dx).toBeLessThanOrEqual(30 * 3);
    expect(d.dx + d.dw).toBeGreaterThan(30 * 3);
    expect(d.dy).toBeLessThan(40 * 3);
    expect(d.dw).toBe(d.sw * 3);
    // colour from the holder's left half, light from its right
    expect(d.sx).toBe(d.op === "lighter" ? d.holder.width / 2 : 0);
  }
});

test("a layer draws only its own depth; both draw back first", () => {
  const { stage, frame, draw } = setup();
  stage.play("arcane/rune-circle", 30, 40, {}, null, DEPTHS);
  frame();
  const back = draw(0).length;
  const front = draw(1).length;
  expect(back).toBeGreaterThan(0);
  expect(front).toBeGreaterThan(0);
  expect(draw(null).length).toBe(back + front);
});

test("the pixels Rust painted reach the holder, colour on the left and light on the right", () => {
  const { stage, frame, draw } = setup();
  stage.play("fire/candle", 30, 40, {}, null, DEPTHS);
  frame();
  const lit = (d: Drawn): number => {
    let n = 0;
    const { bytes, width } = d.holder;
    for (let row = 0; row * width * 4 < bytes.length; row++)
      for (let x = d.sx; x < d.sx + d.sw; x++) if ((bytes[(row * width + x) * 4 + 3] ?? 0) > 0) n++;
    return n;
  };
  const drawn = draw(null);
  expect(drawn.some((d) => d.op === "source-over" && lit(d) > 0)).toBe(true);
  expect(drawn.some((d) => d.op === "lighter" && lit(d) > 0)).toBe(true);
});

test("an effect removed draws nothing more", () => {
  const { stage, frame, draw } = setup();
  const fx = stage.play("fire/candle", 30, 40, {}, null, DEPTHS);
  frame();
  expect(draw(null).length).toBeGreaterThan(0);
  fx.destroy();
  frame();
  expect(draw(null)).toEqual([]);
});

/** Draws pictures through fake holders and a fake context. */
function pictures() {
  const holders = new Map<CanvasImageSource, FakeHolder>();
  const drawer = canvasPictures((w, h) => {
    const holder = fakeHolder(w, h);
    holders.set(holder.source, holder);
    return holder;
  });
  const { ctx, drawn } = fakeContext(holders);
  return { draw: (c: DrawContext, scale: number, p: PictureLike) => drawer.draw(c, scale, p), ctx, drawn };
}

test("fighters Rust painted draw at their place, scaled, colour normally then light added", () => {
  const sprites = new wasm.Sprites(120, 80);
  sprites.begin();
  const body = new wasm.Body(60);
  const a: Appearance = { x: body.x, air: 0, pose: "idle", poseT: 0, face: 1, vx: 0, vy: 0, out: false, flash: false, untouchable: false,
    frozen: false, shocked: false, hitT: 0, hitPower: 0, dodged: 0, jolt: [0, 0], act: null };
  sprites.fighter("thunder", a, 70, null);
  const picture = sprites.picture();
  if (!picture) throw new Error("a fighter painted nothing");
  const { draw, ctx, drawn } = pictures();
  draw(ctx, 2, picture);
  expect(drawn.map((d) => d.op)).toEqual(["source-over", "lighter"]);
  for (const d of drawn) {
    expect([d.dx, d.dy, d.dw, d.dh]).toEqual([picture.x * 2, picture.y * 2, picture.w * 2, picture.h * 2]);
  }
  // the fighter stands on its feet at x 60, row 70
  expect(picture.x).toBeLessThan(60);
  expect(picture.x + picture.w).toBeGreaterThan(60);
  // its feet (and the outline and shadow under them) end on the ground row, its head well above
  expect(Math.abs(picture.y + picture.h - 70)).toBeLessThanOrEqual(3);
  expect(picture.y).toBeLessThan(70 - 30);
  const color = drawn[0];
  if (!color) throw new Error("no colour drawn");
  const { bytes, width } = color.holder;
  let opaque = 0;
  for (let row = 0; row < picture.h; row++) for (let x = 0; x < picture.w; x++) if ((bytes[(row * width + x) * 4 + 3] ?? 0) > 0) opaque++;
  expect(opaque).toBeGreaterThan(100);
});

test("scene light draws added only, and nothing when no light burns", () => {
  const lighting = new wasm.Lighting(100, 60);
  const { draw, ctx, drawn } = pictures();
  expect(lighting.paint([], [])).toBeUndefined();
  const lights = wasm.stepLights([{ x: 50, y: 30, reach: 12, color: "#ff8040", strength: 0.8 }], 0.5);
  const picture = lighting.paint(lights, []);
  if (!picture) throw new Error("a burning light painted nothing");
  expect(picture.color).toBeUndefined();
  draw(ctx, 3, picture);
  expect(drawn.map((d) => d.op)).toEqual(["lighter"]);
  // gone once put out and aged past it
  expect(wasm.stepLights(lights.map((l) => wasm.putOutLight(l, 0.1)), 0.2)).toEqual([]);
});
