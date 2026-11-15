import Phaser from "phaser";
import type { ActorSource, Counts, Game, PixfxEffect } from "pixfx-web";
import { drawBudget, loadedEngine, loadedGame, PixfxFeel, Stage } from "pixfx-web";
import { spriteActor } from "./actor";
import { parsePlayOptions } from "./options";
import { phaserImages, phaserTextures } from "./phaser-gfx";
import { type FeelSprite, PixfxSpriteFeel, SpriteFeelCore } from "./sprite-feel";

/** Effect seconds for one scene update; anything not finite or not positive gives 0. */
function stepSeconds(deltaMs: number, timeScale: number): number {
  const s = (deltaMs / 1000) * timeScale;
  return Number.isFinite(s) && s > 0 ? s : 0;
}

/**
 * What `camera` sees this frame in world pixels (`x, y, w, h`), tilted by `roll` more than its own rotation, and how
 * far it scrolled: Phaser's `worldView`, worked out from the scroll the game set, since the camera's `preRender` fills
 * `worldView` only after pixfx draws. A tilted view is the box around the tilted rectangle.
 */
function cameraView(camera: Phaser.Cameras.Scene2D.Camera, roll: number): [number, number, number, number, number, number] {
  const sx = camera.useBounds ? camera.clampX(camera.scrollX) : camera.scrollX;
  const sy = camera.useBounds ? camera.clampY(camera.scrollY) : camera.scrollY;
  let w = camera.width / camera.zoomX;
  let h = camera.height / camera.zoomY;
  // the game's own tilt: Phaser's types keep `rotation` private and give it through `toJSON`
  const tilt = camera.toJSON().rotation + roll;
  if (tilt !== 0) {
    const [c, s] = [Math.abs(Math.cos(tilt)), Math.abs(Math.sin(tilt))];
    [w, h] = [w * c + h * s, w * s + h * c];
  }
  const [mx, my] = [sx + camera.width / 2, sy + camera.height / 2];
  return [mx - w / 2, my - h / 2, w, h, sx, sy];
}

/**
 * Scene plugin: install with `{ key: "pixfx", plugin: PixfxPlugin, mapping: "pixfx" }` in the game's
 * `plugins.scene`, then `this.pixfx.play(id, x, y, opts)`. The Rust scene plays, times and places every effect;
 * workers paint their frames, and this plugin shows them with Phaser images. Effects advance on scene time (after
 * the scene's `update()` and physics, so `follow` sees final positions) and are freed when the scene shuts down or
 * is destroyed.
 *
 * Only effects the main camera sees are drawn (its view, at each effect's scroll factor); the rest play on unseen.
 *
 * Game feel: `this.pixfx.feel` answers the cues effects fire. A hit-stop freezes the scene's effects (the game
 * reads `feel.timeScale` to freeze its own world), the shake moves and tilts the main camera's view for the render
 * only (its scroll, follow and rotation are untouched), and the flash lifts the view toward white.
 * `this.pixfx.squash(sprite)` gives a sprite squash, stretch and a blink on the same clock.
 */
export class PixfxPlugin extends Phaser.Plugins.ScenePlugin {
  private readonly host: Phaser.Scene;
  private stageHeld: Stage<number> | null = null;
  /** Layers drawn at most per frame, for the stage made after `loadPixfx()`; 0 draws every changed one. */
  private drawBudget = 0;
  private destroyed = false;
  /** The scene's answer to impacts: camera shake, hit-stop, slow motion, flash, and every effect's cues. */
  readonly feel: PixfxFeel;
  /** The main camera's place and tilt before this frame's shake, restored after the render. */
  private moved: { camera: Phaser.Cameras.Scene2D.Camera; x: number; y: number; rotation: number } | null = null;
  private flashRect: Phaser.GameObjects.Rectangle | null = null;
  private readonly sprites = new Set<PixfxSpriteFeel>();

  constructor(scene: Phaser.Scene, pluginManager: Phaser.Plugins.PluginManager, pluginKey: string) {
    super(scene, pluginManager, pluginKey);
    this.host = scene;
    this.feel = new PixfxFeel(() => this.stage()?.live ?? null);
  }

  override boot(): void {
    const events = this.host.sys.events;
    events.on(Phaser.Scenes.Events.POST_UPDATE, this.onUpdate);
    events.on(Phaser.Scenes.Events.PRE_RENDER, this.onPreRender);
    events.on(Phaser.Scenes.Events.RENDER, this.onRender);
    events.on(Phaser.Scenes.Events.SHUTDOWN, this.onShutdown);
    events.once(Phaser.Scenes.Events.DESTROY, this.onDestroy);
    this.game.events.on(Phaser.Core.Events.POST_RENDER, this.onPostRender);
  }

  /** Play effect `id` with its origin at world (x, y). Throws on an unknown id, invalid options, or before `loadPixfx()`. */
  play(id: string, x: number, y: number, opts?: unknown): PixfxEffect<number> {
    const stage = this.stage();
    if (!stage) throw new Error("pixfx: call await loadPixfx() before play()");
    const p = parsePlayOptions(opts);
    const effect = stage.play(id, x, y, p.engine, p.scrollFactor, p.depth);
    if (p.scrollFactor !== 1) stage.scene.setScrollFactor(effect.handle, p.scrollFactor);
    return effect;
  }

  /**
   * `sprite` as the character an effect plays on: `effect.wrap(this.pixfx.actorOf(sprite))`. An effect that wraps
   * its actor (as `feedback/power-up` does) grips the sprite's outline and follows each frame it shows. Its rotation
   * and a parent container are not read.
   */
  actorOf(sprite: Phaser.GameObjects.Sprite | Phaser.GameObjects.Image): ActorSource {
    return spriteActor(sprite);
  }

  /**
   * Give `sprite` squash, stretch and a blink: it stretches along its velocity when it moves fast, upright (read
   * from its arcade body's velocity, else from how far it moved), flattens when the game calls `land`, and blinks
   * on `blink`. Set its origin at its feet (`setOrigin(0.5, 1)`). Ends when the sprite is destroyed, `destroy()`
   * is called, or the scene shuts down. Throws before `loadPixfx()`.
   */
  squash(sprite: FeelSprite): PixfxSpriteFeel {
    const engine = loadedEngine();
    if (!engine) throw new Error("pixfx: call await loadPixfx() before squash()");
    const s = new PixfxSpriteFeel(sprite, new SpriteFeelCore(engine.squash(), engine.blink()), (gone) =>
      this.sprites.delete(gone),
    );
    this.sprites.add(s);
    return s;
  }

  /**
   * Draw at most `layers` effect layers per frame in this scene, so many effects reaching a new frame at once cannot
   * make one frame slow; the ones left over show their last frame and are drawn in a later frame. 0 (the default)
   * draws every one that changed. Set before `loadPixfx()` resolves, it applies once it does. Throws unless `layers`
   * is a whole number, 0 or more.
   */
  setDrawBudget(layers: number): void {
    this.drawBudget = drawBudget(layers);
    this.stageHeld?.setDrawBudget(this.drawBudget);
  }

  /** Effects playing in this scene, textures the game holds for every scene, and this scene's images. */
  stats(): Counts {
    return this.stageHeld?.counts() ?? { engines: 0, textures: loadedGame()?.counts().textures ?? 0, images: 0 };
  }

  override destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.end();
    this.feel.free();
    const events = this.host.sys.events;
    events.off(Phaser.Scenes.Events.POST_UPDATE, this.onUpdate);
    events.off(Phaser.Scenes.Events.PRE_RENDER, this.onPreRender);
    events.off(Phaser.Scenes.Events.RENDER, this.onRender);
    events.off(Phaser.Scenes.Events.SHUTDOWN, this.onShutdown);
    events.off(Phaser.Scenes.Events.DESTROY, this.onDestroy);
    this.game.events.off(Phaser.Core.Events.POST_RENDER, this.onPostRender);
    super.destroy();
  }

  /** The scene's stage, made the first time it is needed after `loadPixfx()` resolves. */
  private stage(): Stage<number> | null {
    if (this.stageHeld) return this.stageHeld;
    const game: Game | null = loadedGame();
    if (!game || this.destroyed) return null;
    game.useTextures(phaserTextures(this.host));
    this.stageHeld = new Stage(game, phaserImages(this.host), (cue, effect) => this.feel.take(cue, effect), 1);
    this.stageHeld.scene.setFeel(this.feel.enabled);
    this.stageHeld.setDrawBudget(this.drawBudget);
    return this.stageHeld;
  }

  private readonly onUpdate = (_time: number, delta: number): void => {
    const stage = this.stage();
    if (!stage) return;
    const dt = stepSeconds(delta, this.host.time.timeScale);
    const played = stage.update(dt);
    this.feel.played(dt, played);
    for (const s of [...this.sprites]) s.step(played);
  };

  private readonly onPreRender = (): void => {
    const stage = this.stageHeld;
    if (stage) {
      stage.scene.setView(...cameraView(this.host.cameras.main, this.feel.roll));
      stage.draw();
    }
    this.showFeel();
  };

  /** After every scene rendered: end the game's frame (once, however many scenes call it) and send work to workers. */
  private readonly onPostRender = (): void => {
    loadedGame()?.endFrame();
  };

  /** After the render: the camera goes back where the game put it. */
  private readonly onRender = (): void => {
    const m = this.moved;
    if (!m) return;
    m.camera.setPosition(m.x, m.y).setRotation(m.rotation);
    this.moved = null;
  };

  /**
   * Move the main camera's view by the shake (in screen pixels: game pixels × zoom), tilt it by the roll, and show
   * the flash over it.
   */
  private showFeel(): void {
    const camera = this.host.cameras.main;
    const [dx, dy] = this.feel.shake;
    const roll = this.feel.roll;
    if (dx !== 0 || dy !== 0 || roll !== 0) {
      // the game's own tilt: Phaser's types keep `rotation` private and give it through `toJSON`
      const rotation = camera.toJSON().rotation;
      this.moved = { camera, x: camera.x, y: camera.y, rotation };
      camera.setPosition(camera.x + dx * camera.zoom, camera.y + dy * camera.zoom).setRotation(rotation + roll);
    }
    const flash = this.feel.flash;
    if (flash <= 0 && !this.flashRect) return;
    // in the camera's space, centred and big enough to cover the view at any zoom
    const cover = 1 / Math.min(1, camera.zoom);
    this.flashRect ??= this.host.add.rectangle(0, 0, 1, 1, 0xffffff).setScrollFactor(0).setDepth(Number.MAX_SAFE_INTEGER);
    this.flashRect
      .setPosition(camera.width / 2, camera.height / 2)
      .setSize(camera.width * cover, camera.height * cover)
      .setAlpha(flash)
      .setVisible(flash > 0);
  }

  /** Free the scene's effects, images and sprites' feel. */
  private end(): void {
    this.stageHeld?.clear();
    this.stageHeld = null;
    for (const s of [...this.sprites]) s.destroy();
  }

  private readonly onShutdown = (): void => {
    this.end();
    this.flashRect = null;
  };

  private readonly onDestroy = (): void => {
    this.destroy();
  };
}
