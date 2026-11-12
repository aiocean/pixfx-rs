import Phaser from "phaser";
import type { Counts, PixfxEffect } from "./effect";
import { EffectManager } from "./effects";
import { getEngine } from "./engine";
import type { LayerConfig, PixfxLayer } from "./layer";
import type { Gfx } from "./gfx";

/** Phaser-backed textures (canvas, nearest-filtered) and images (top-left origin) for one scene. */
function phaserGfx(scene: Phaser.Scene): Gfx {
  return {
    texture(key, width, height) {
      const tex = scene.textures.createCanvas(key, width, height);
      if (!tex) throw new Error(`pixfx: texture key "${key}" is already in use`);
      tex.setFilter(Phaser.Textures.FilterMode.NEAREST);
      const data = tex.imageData;
      return {
        pixels: data.data,
        upload() {
          tex.putData(data, 0, 0);
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
 * `plugins.scene`, then `this.pixfx.play(id, x, y, opts)`. Effects advance on scene time and are freed
 * when the scene shuts down or is destroyed.
 */
export class PixfxPlugin extends Phaser.Plugins.ScenePlugin {
  private readonly host: Phaser.Scene;
  private readonly effects: EffectManager;
  private destroyed = false;

  constructor(scene: Phaser.Scene, pluginManager: Phaser.Plugins.PluginManager, pluginKey: string) {
    super(scene, pluginManager, pluginKey);
    this.host = scene;
    this.effects = new EffectManager(phaserGfx(scene), getEngine, `pixfx:${scene.sys.settings.key}:`);
  }

  override boot(): void {
    const events = this.host.sys.events;
    events.on(Phaser.Scenes.Events.UPDATE, this.onUpdate);
    events.on(Phaser.Scenes.Events.SHUTDOWN, this.onShutdown);
    events.once(Phaser.Scenes.Events.DESTROY, this.onDestroy);
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
    const events = this.host.sys.events;
    events.off(Phaser.Scenes.Events.UPDATE, this.onUpdate);
    events.off(Phaser.Scenes.Events.SHUTDOWN, this.onShutdown);
    events.off(Phaser.Scenes.Events.DESTROY, this.onDestroy);
    super.destroy();
  }

  private readonly onUpdate = (_time: number, delta: number): void => {
    this.effects.update(delta, this.host.time.timeScale);
  };

  private readonly onShutdown = (): void => {
    this.effects.destroyAll();
  };

  private readonly onDestroy = (): void => {
    this.destroy();
  };
}
