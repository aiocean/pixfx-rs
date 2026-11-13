import Phaser from "phaser";
import type { Counts, PixfxEffect } from "./effect";
import { EffectManager } from "./effects";
import { PixfxFeel } from "./feel";
import { flushPixfx, getBackend, loadedEngine } from "./load";
import type { LayerConfig, PixfxLayer } from "./layer";
import type { Gfx } from "./gfx";

/** Phaser-backed textures (canvas, nearest-filtered) and images (top-left origin) for one scene. */
function phaserGfx(scene: Phaser.Scene): Gfx {
  return {
    texture(key, width, height) {
      const tex = scene.textures.createCanvas(key, width, height);
      if (!tex) throw new Error(`pixfx: texture key "${key}" is already in use`);
      tex.setFilter(Phaser.Textures.FilterMode.NEAREST);
      return {
        upload(rgba) {
          tex.putData(new ImageData(rgba, width, height), 0, 0);
          tex.refresh();
        },
        destroy() {
          scene.textures.remove(tex);
        },
      };
    },
    image(textureKey, blend, scrollFactor) {
      const img = scene.add.image(0, 0, textureKey).setOrigin(0, 0).setScrollFactor(scrollFactor);
      if (blend === "add") img.setBlendMode(Phaser.BlendModes.ADD);
      return {
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
 * reads `feel.timeScale` to freeze its own world), the shake moves the main camera's view for the render only
 * (its scroll and follow are untouched), and the flash lifts the view toward white.
 */
export class PixfxPlugin extends Phaser.Plugins.ScenePlugin {
  private readonly host: Phaser.Scene;
  private readonly effects: EffectManager;
  private destroyed = false;
  /** The scene's answer to impacts: camera shake, hit-stop, slow motion, flash, and every effect's cues. */
  readonly feel: PixfxFeel;
  /** The main camera's place before this frame's shake, restored after the render. */
  private moved: { camera: Phaser.Cameras.Scene2D.Camera; x: number; y: number } | null = null;
  private flashRect: Phaser.GameObjects.Rectangle | null = null;

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

  /** Live engine instances, textures and images held by this scene's effects and layers. */
  stats(): Counts {
    return this.effects.stats();
  }

  override destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.effects.destroyAll();
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
    this.effects.update(delta, this.host.time.timeScale);
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
    m.camera.setPosition(m.x, m.y);
    this.moved = null;
  };

  /** Move the main camera's view by the shake (in screen pixels: game pixels × zoom) and show the flash over it. */
  private showFeel(): void {
    const camera = this.host.cameras.main;
    const [dx, dy] = this.feel.shake;
    if (dx !== 0 || dy !== 0) {
      this.moved = { camera, x: camera.x, y: camera.y };
      camera.setPosition(camera.x + dx * camera.zoom, camera.y + dy * camera.zoom);
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

  private readonly onShutdown = (): void => {
    this.effects.destroyAll();
    this.flashRect = null;
  };

  private readonly onDestroy = (): void => {
    this.destroy();
  };
}
