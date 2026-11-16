// pixfx on a Canvas 2D context: the Rust scene plays, times and places every effect, workers paint their frames
// (`pixfx-web`), and this draws them layer by layer onto the host's canvas, among its own drawing.
import type { Counts, Game, PixfxEffect } from "pixfx-web";
import { loadedGame, PixfxFeel, splitPlayOptions, Stage } from "pixfx-web";
import type { CanvasImages, CanvasPictures, HolderOf, MakeHolder, PictureLike } from "./canvas-gfx";
import { canvasHolder, canvasImages, canvasPictures, canvasTextures, owned } from "./canvas-gfx";

/** An effect's back layer (under what it plays around) or its front layer (over it). */
export type Layer = "back" | "front";

/** The depth each layer's images are shown at. */
const DEPTH: Record<Layer, number> = { back: 0, front: 1 };

export interface PixfxOptions {
  /** Canvas pixels per game pixel, a whole number. Default 1. */
  scale?: number;
  /** Where a surface's pixels are held. Default: an `OffscreenCanvas` each. Tests pass their own. */
  holder?: MakeHolder;
}

/** The holders of the one kind a game holds its surfaces in. */
const holdersOf = new WeakMap<Game, { make: MakeHolder; holderOf: HolderOf }>();

function holdersFor(game: Game, make: MakeHolder): HolderOf {
  const held = holdersOf.get(game);
  if (held) {
    if (held.make !== make) throw new Error("pixfx: a game holds its surfaces in one kind of holder");
    return held.holderOf;
  }
  const { textures, holderOf } = canvasTextures(make);
  game.useTextures(textures);
  holdersOf.set(game, { make, holderOf });
  return holderOf;
}

/**
 * pixfx on one canvas: `play(id, x, y, opts)` places an effect's origin at (`x`, `y`) in game pixels. The host runs
 * the loop: each frame `update(dt)`, then `drawLayer("back")` and `drawLayer("front")` where they go among its own
 * drawing (or `draw()` for both), in the context's current transform. Make it after `await loadPixfx()`.
 */
export class Pixfx {
  /** The answer to impacts: the shake, hit-stop, slow motion, flash, and every effect's cues. */
  readonly feel: PixfxFeel;
  readonly scale: number;
  private readonly stage: Stage<null>;
  private readonly images: CanvasImages;
  private readonly pictures: CanvasPictures;
  private destroyed = false;

  constructor(
    private readonly ctx: CanvasRenderingContext2D,
    options: PixfxOptions = {},
  ) {
    const game = loadedGame();
    if (!game) throw new Error("pixfx: call await loadPixfx() before createPixfx()");
    const scale = options.scale ?? 1;
    if (!Number.isInteger(scale) || scale < 1) throw new Error(`pixfx: scale must be a whole number, 1 or more (got ${scale})`);
    this.scale = scale;
    this.images = canvasImages(holdersFor(game, options.holder ?? canvasHolder));
    this.pictures = canvasPictures(options.holder ?? canvasHolder);
    let stage: Stage<null> | null = null;
    this.feel = new PixfxFeel(() => stage?.live ?? null);
    stage = new Stage<null>(game, this.images.images, (cue, effect) => this.feel.take(cue, effect), null);
    this.stage = stage;
  }

  /** Play effect `id` with its origin at (`x`, `y`). Throws on an unknown id or invalid options. */
  play(id: string, x: number, y: number, opts?: unknown): PixfxEffect<null> {
    if (this.destroyed) throw new Error("pixfx: play() after destroy()");
    const { engine } = splitPlayOptions({}, opts);
    return this.stage.play(id, x, y, engine, null, DEPTH);
  }

  /**
   * Let `dt` seconds pass: the effects play (frozen through a hit-stop the feel answers), what they show is made
   * ready to draw, and the frames still to paint go to the workers. Returns the seconds played.
   */
  update(dt: number): number {
    if (this.destroyed) return 0;
    const t = Number.isFinite(dt) && dt > 0 ? dt : 0;
    const played = this.stage.update(t);
    this.feel.played(t, played);
    this.stage.draw();
    this.pictures.frame();
    loadedGame()?.endFrame();
    return played;
  }

  /**
   * Draw only the effects whose frame reaches into this rectangle of the world, in game pixels: what the host's
   * transform shows. The rest play on undrawn. Throws if a value is not finite.
   */
  setView(x: number, y: number, w: number, h: number): void {
    this.stage.scene.setView(x, y, w, h, 0, 0);
  }

  /** Draw every effect again, wherever it is (the default). */
  clearView(): void {
    this.stage.scene.clearView();
  }

  /**
   * Body `id` (a fighter, a wall) is the box (`x`, `y`, `w`, `h`) in game pixels: the pieces of a volley an effect
   * was ordered to launch (`effect.attack`) strike it when they meet it. Set it as the thing moves.
   */
  setBody(id: number, x: number, y: number, w: number, h: number): void {
    this.stage.setBody(id, x, y, w, h);
  }

  /** Body `id` is gone: nothing strikes it any more. */
  clearBody(id: number): void {
    this.stage.clearBody(id);
  }

  /** Draw the effects' `layer` onto the canvas, in the context's current transform. */
  drawLayer(layer: Layer): void {
    this.images.draw(this.ctx, this.scale, DEPTH[layer]);
  }

  /** Draw both layers, back then front. */
  draw(): void {
    this.images.draw(this.ctx, this.scale, null);
  }

  /**
   * Draw a picture Rust painted outside the effects (`Sprites.picture()`, `Lighting.paint()`), at its place in game
   * pixels, in the context's current transform. Nothing when `picture` is undefined (nothing showed).
   */
  drawPicture(picture: PictureLike | undefined): void {
    if (picture) this.pictures.draw(this.ctx, this.scale, picture);
  }

  /**
   * Bend what the canvas shows now by the screen ripples alive (a heavy blow's ring): call it after drawing
   * everything the ripples should move. Nothing while none is alive, the feel is off, or the shake strength is 0.
   */
  warp(): void {
    const { width, height } = this.ctx.canvas;
    if (width === 0 || height === 0) return;
    const shown = this.ctx.getImageData(0, 0, width, height);
    const bent = this.feel.bend(shown.data, width, height, this.scale);
    if (bent) this.ctx.putImageData(new ImageData(owned(bent), width, height), 0, 0);
  }

  /**
   * Draw at most `layers` effect layers per frame, so many effects reaching a new frame at once cannot make one
   * frame slow; the ones left over show their last frame and are drawn in a later frame. 0 (the default) draws every
   * one that changed. Throws unless `layers` is a whole number, 0 or more.
   */
  setDrawBudget(layers: number): void {
    this.stage.setDrawBudget(layers);
  }

  /** Effects playing, surfaces the game holds, and images. */
  stats(): Counts {
    return this.stage.counts();
  }

  /** End every effect (each emits `"destroy"`) and free what they hold. */
  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.stage.clear();
    this.feel.free();
  }
}

/** pixfx on `ctx`; see `Pixfx`. Call after `await loadPixfx()`. */
export function createPixfx(ctx: CanvasRenderingContext2D, options?: PixfxOptions): Pixfx {
  return new Pixfx(ctx, options);
}
