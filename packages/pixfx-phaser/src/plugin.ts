import Phaser from "phaser";
import type { Counts, PixfxEffect } from "./effect";
import { EffectManager } from "./effects";
import { PixfxFeel } from "./feel";
import { flushPixfx, getBackend, loadedEngine } from "./load";
import { type FeelSprite, PixfxSpriteFeel, SpriteFeelCore } from "./sprite-feel";
import type { LayerConfig, PixfxLayer } from "./layer";
import type { Gfx } from "./gfx";

/**
 * Phaser-backed textures (canvas, nearest-filtered) and images (top-left origin) for one scene. Textures live in
 * the game's texture manager, so a texture made through one scene shows in every scene.
 */
function phaserGfx(scene: Phaser.Scene): Gfx {
  const textures = scene.textures;
  return {
    texture(key, width, height) {
      const tex = textures.createCanvas(key, width, height);
      if (!tex) throw new Error(`pixfx: texture key "${key}" is already in use`);
      tex.setFilter(Phaser.Textures.FilterMode.NEAREST);
      return {
        upload(rgba) {
          tex.putData(new ImageData(rgba, width, height), 0, 0);
          tex.refresh();
        },
        destroy() {
          textures.remove(tex);
        },
      };
    },
    image(textureKey, blend, scrollFactor) {
      const img = scene.add
        .image(0, 0, textureKey ?? "__DEFAULT")
        .setOrigin(0, 0)
        .setScrollFactor(scrollFactor)
        .setVisible(textureKey !== null);
      if (blend === "add") img.setBlendMode(Phaser.BlendModes.ADD);
      return {
        setTexture(key) {
          // `setTexture` resizes the image to the texture; the top-left origin stays
          if (img.texture.key !== key) img.setTexture(key);
        },
        setVisible(visible) {
          img.setVisible(visible);
        },
        setPosition(x, y) {
          img.setPosition(x, y);
        },
        setDepth(depth) {
          img.setDepth(depth);
        },
        destroy() {
          img.destroy();
        },
      };
    },
  };
}

/**
 * Scene plugin: install with `{ key: "pixfx", plugin: PixfxPlugin, mapping: "pixfx" }` in the game's
 * `plugins.scene`, then `this.pixfx.play(id, x, y, opts)`. Effects advance on scene time (after the scene's `update()` and physics, so `follow` sees final positions) and are freed
 * when the scene shuts down or is destroyed. The work every scene queued is sent to the effect workers once per
 * game step, after all scenes updated.
 *
 * Game feel: `this.pixfx.feel` answers the cues effects fire. A hit-stop freezes the scene's effects (the game
 * reads `feel.timeScale` to freeze its own world), the shake moves and tilts the main camera's view for the render
 * only (its scroll, follow and rotation are untouched), and the flash lifts the view toward white. `this.pixfx.squash(sprite)`
 * gives a sprite squash, stretch and a blink on the same clock.
 */
export class PixfxPlugin extends Phaser.Plugins.ScenePlugin {
  private readonly host: Phaser.Scene;
  private readonly effects: EffectManager;
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
    this.feel = new PixfxFeel(loadedEngine);
    this.effects = new EffectManager(phaserGfx(scene), getBackend, `pixfx:${scene.sys.settings.key}:`, this.feel);
  }

  override boot(): void {
    const events = this.host.sys.events;
    events.on(Phaser.Scenes.Events.POST_UPDATE, this.onUpdate);
    events.on(Phaser.Scenes.Events.PRE_RENDER, this.onPreRender);
    events.on(Phaser.Scenes.Events.RENDER, this.onRender);
    events.on(Phaser.Scenes.Events.SHUTDOWN, this.onShutdown);
    events.once(Phaser.Scenes.Events.DESTROY, this.onDestroy);
    this.game.events.on(Phaser.Core.Events.POST_STEP, this.onPostStep);
  }

  /** Play effect `id` with its origin at world (x, y). Throws on an unknown id, invalid options, or before `loadPixfx()`. */
  play(id: string, x: number, y: number, opts?: unknown): PixfxEffect {
    return this.effects.play(id, x, y, opts);
  }

  /** Create a scene-wide layer: effects played into it share four textures covering `cfg.bounds`. Throws on an invalid config. */
  createLayer(cfg: LayerConfig): PixfxLayer {
    return this.effects.createLayer(cfg);
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

  /** Live engine instances, textures and images held by this scene's effects and layers. */
  stats(): Counts {
    return this.effects.stats();
  }

  override destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.effects.destroyAll();
    this.endSprites();
    this.feel.free();
    const events = this.host.sys.events;
    events.off(Phaser.Scenes.Events.POST_UPDATE, this.onUpdate);
    events.off(Phaser.Scenes.Events.PRE_RENDER, this.onPreRender);
    events.off(Phaser.Scenes.Events.RENDER, this.onRender);
    events.off(Phaser.Scenes.Events.SHUTDOWN, this.onShutdown);
    events.off(Phaser.Scenes.Events.DESTROY, this.onDestroy);
    this.game.events.off(Phaser.Core.Events.POST_STEP, this.onPostStep);
    super.destroy();
  }

  private readonly onUpdate = (_time: number, delta: number): void => {
    const dt = this.effects.update(delta, this.host.time.timeScale);
    for (const s of [...this.sprites]) s.step(dt);
  };

  /** Its own function per plugin: `off` removes every listener sharing a function, and each scene has a plugin. */
  private readonly onPostStep = (): void => {
    flushPixfx();
  };

  private readonly onPreRender = (): void => {
    this.effects.flushLayers();
    this.showFeel();
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

  private endSprites(): void {
    for (const s of [...this.sprites]) s.destroy();
  }

  private readonly onShutdown = (): void => {
    this.effects.destroyAll();
    this.endSprites();
    this.flashRect = null;
  };

  private readonly onDestroy = (): void => {
    this.destroy();
  };
}
