import { expect, test } from "bun:test";
import { Game, Stage } from "pixfx-web";
import type { DrawContext, Holder } from "../src/canvas-gfx";
import { canvasImages, canvasTextures } from "../src/canvas-gfx";
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
