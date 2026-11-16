// pixfx in a Pixi 8 application: the Rust scene plays, times and places every effect, workers paint their frames
// (`pixfx-web`), and this shows them with Pixi sprites, moves a camera container by the feel's shake and flashes
// the screen.
import type { Application, Container, Renderer, Sprite } from "pixi.js";
import { Container as PixiContainer, Graphics, UPDATE_PRIORITY } from "pixi.js";
import { z } from "zod";
import type { ActorSource, Counts, Game, PixfxEffect } from "pixfx-web";
import { loadedGame, PixfxFeel, splitPlayOptions, Stage } from "pixfx-web";
import { spriteActor } from "./actor";
import type { PixiTextures } from "./pixi-gfx";
import { pixiImages, pixiTextures } from "./pixi-gfx";

/** The shell's own play options: the depths (`zIndex`) of the back and front sprites. Every other field is the engine's. */
const OWN = { depth: z.object({ back: z.number(), front: z.number() }).optional() };

/** Ticker priority of the step: after the game's own updates (`NORMAL`), before the render (`LOW`). */
const STEP_PRIORITY = (UPDATE_PRIORITY.NORMAL + UPDATE_PRIORITY.LOW) / 2;

export interface PixfxOptions {
  /** Where the effects' sprites go; it sorts its children by `zIndex`. Default: a new container on `app.stage`. */
  container?: Container;
  /** What the shake moves and tilts. Default: `app.stage`. */
  camera?: Container;
}

/** The surfaces' textures of the one renderer a game shows pixfx with. */
const texturesOf = new WeakMap<Game, { renderer: Renderer; textures: PixiTextures }>();

function texturesFor(game: Game, renderer: Renderer): PixiTextures {
  const held = texturesOf.get(game);
  if (held) {
    if (held.renderer !== renderer) throw new Error("pixfx: a game shows pixfx through one Pixi renderer");
    return held.textures;
  }
  const textures = pixiTextures(renderer);
  game.useTextures(textures.textures);
  texturesOf.set(game, { renderer, textures });
  return textures;
}

/**
 * pixfx for one Pixi application: `play(id, x, y, opts)` places an effect's origin at `(x, y)` in `container`. Effects
 * advance on the app's ticker (`ticker.speed` scales them) and are freed by `destroy()`. Make it after
 * `await loadPixfx()`.
 */
export class Pixfx {
  /** The answer to impacts: the camera shake, hit-stop, slow motion, flash, and every effect's cues. */
  readonly feel: PixfxFeel;
  readonly container: Container;
  private readonly camera: Container;
  private readonly stage: Stage<null>;
  private readonly ownContainer: boolean;
  private flashRect: Graphics | null = null;
  /** The camera's place and tilt before this frame's shake, put back after the render. */
  private moved: { x: number; y: number; rotation: number } | null = null;
  private destroyed = false;

  constructor(
    private readonly app: Application,
    options: PixfxOptions = {},
  ) {
    const game = loadedGame();
    if (!game) throw new Error("pixfx: call await loadPixfx() before createPixfx()");
    const { textureOf } = texturesFor(game, app.renderer);
    this.ownContainer = options.container === undefined;
    this.container = options.container ?? app.stage.addChild(new PixiContainer());
    this.container.sortableChildren = true;
    this.camera = options.camera ?? app.stage;
    let stage: Stage<null> | null = null;
    this.feel = new PixfxFeel(() => stage?.live ?? null);
    stage = new Stage<null>(game, pixiImages(this.container, textureOf), (cue, effect) => this.feel.take(cue, effect), null);
    this.stage = stage;
    app.ticker.add(this.step, undefined, STEP_PRIORITY);
    app.ticker.add(this.after, undefined, UPDATE_PRIORITY.UTILITY);
  }

  /** Play effect `id` with its origin at `(x, y)`. Throws on an unknown id or invalid options. */
  play(id: string, x: number, y: number, opts?: unknown): PixfxEffect<null> {
    if (this.destroyed) throw new Error("pixfx: play() after destroy()");
    const { own, engine } = splitPlayOptions(OWN, opts);
    return this.stage.play(id, x, y, engine, null, own.depth);
  }

  /**
   * `sprite` as the character an effect plays on: `effect.wrap(pixfx.actorOf(sprite))`. An effect that wraps its
   * actor (as `feedback/power-up` does) grips the sprite's outline and follows each texture it shows. Its rotation
   * is not read.
   */
  actorOf(sprite: Sprite): ActorSource {
    return spriteActor(sprite, this.container);
  }

  /**
   * Draw at most `layers` effect layers per frame, so many effects reaching a new frame at once cannot make one
   * frame slow; the ones left over show their last frame and are drawn in a later frame. 0 (the default) draws every
   * one that changed. Throws unless `layers` is a whole number, 0 or more.
   */
  setDrawBudget(layers: number): void {
    this.stage.setDrawBudget(layers);
  }

  /**
   * Draw only the effects whose frame reaches into this rectangle of the world, in game pixels: what the game's
   * camera shows. The rest play on undrawn. Throws if a value is not finite.
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

  /** Effects playing, textures the game holds, and sprites. */
  stats(): Counts {
    return this.stage.counts();
  }

  /** End every effect (each emits `"destroy"`), free the sprites, and stop following the ticker. */
  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.app.ticker.remove(this.step);
    this.app.ticker.remove(this.after);
    this.restoreCamera();
    this.stage.clear();
    this.feel.free();
    this.flashRect?.destroy();
    this.flashRect = null;
    if (this.ownContainer) this.container.destroy({ children: true });
  }

  private readonly step = (): void => {
    const dt = this.app.ticker.deltaMS / 1000;
    const played = this.stage.update(Number.isFinite(dt) && dt > 0 ? dt : 0);
    this.feel.played(dt, played);
    this.stage.draw();
    this.showFeel();
  };

  /** After the render: the camera goes back where the game put it; the game's frame ends. */
  private readonly after = (): void => {
    this.restoreCamera();
    loadedGame()?.endFrame();
  };

  /** Move the camera by the shake (game pixels × its scale), tilt it about the screen's centre, flash the screen. */
  private showFeel(): void {
    const c = this.camera;
    const [dx, dy] = this.feel.shake;
    const roll = this.feel.roll;
    if (dx !== 0 || dy !== 0 || roll !== 0) {
      this.moved = { x: c.x, y: c.y, rotation: c.rotation };
      const cx = this.app.screen.width / 2;
      const cy = this.app.screen.height / 2;
      const cos = Math.cos(roll);
      const sin = Math.sin(roll);
      const rx = c.x - cx;
      const ry = c.y - cy;
      c.position.set(cx + rx * cos - ry * sin + dx * c.scale.x, cy + rx * sin + ry * cos + dy * c.scale.y);
      c.rotation += roll;
    }
    const flash = this.feel.flash;
    if (flash <= 0 && !this.flashRect) return;
    this.flashRect ??= new Graphics().rect(0, 0, 1, 1).fill(0xffffff);
    const r = this.flashRect;
    r.visible = flash > 0;
    if (!r.visible) return;
    // on top of everything, and half a screen bigger each way, so it covers the screen however the stage shakes
    this.app.stage.addChild(r);
    const { width, height } = this.app.screen;
    r.position.set(-width / 4, -height / 4);
    r.setSize(width * 1.5, height * 1.5);
    r.alpha = flash;
  }

  private restoreCamera(): void {
    const m = this.moved;
    if (!m) return;
    this.camera.position.set(m.x, m.y);
    this.camera.rotation = m.rotation;
    this.moved = null;
  }
}

/** pixfx for `app`; see `Pixfx`. Call after `await loadPixfx()`. */
export function createPixfx(app: Application, options?: PixfxOptions): Pixfx {
  return new Pixfx(app, options);
}
