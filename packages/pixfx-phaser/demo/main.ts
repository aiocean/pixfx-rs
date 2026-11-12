import Phaser from "phaser";
import type { Counts, Engine, PixfxEffect } from "../src";
import { PixfxPlugin, loadPixfx } from "../src";

const WIDTH = 400;
const HEIGHT = 200;
const GROUND_Y = 120;

/** What the browser checks read: the live scene, the plugin's counts and the effect event log. */
interface DemoHandle {
  scene: DemoScene;
  game: Phaser.Game;
  engine: Engine;
  stats: () => Counts;
  events: string[];
}

declare global {
  interface Window {
    pixfxDemo?: DemoHandle;
  }
}

const events: string[] = [];

/** Log `complete` and `destroy` of `effect` as `"<event>:<name>"`. */
function logged(effect: PixfxEffect, name: string): PixfxEffect {
  effect.on("complete", () => events.push(`complete:${name}`));
  effect.on("destroy", () => events.push(`destroy:${name}`));
  return effect;
}

class DemoScene extends Phaser.Scene {
  declare pixfx: PixfxPlugin;
  campfire: PixfxEffect | null = null;
  walker: Phaser.GameObjects.Rectangle | null = null;
  character: Phaser.GameObjects.Rectangle | null = null;

  constructor() {
    super("demo");
  }

  create(): void {
    this.add.rectangle(0, 0, WIDTH, HEIGHT, 0x161620).setOrigin(0, 0).setDepth(-10);
    this.add.rectangle(0, GROUND_Y, WIDTH, HEIGHT - GROUND_Y, 0x24242e).setOrigin(0, 0).setDepth(-9);

    logged(this.pixfx.play("fire/candle", 24, GROUND_Y + 40, { seed: 1 }), "candle");
    this.campfire = logged(this.pixfx.play("fire/campfire", 150, GROUND_Y + 50, { seed: 3 }), "campfire");
    logged(this.pixfx.play("fire/brazier", 235, GROUND_Y + 60, { seed: 8 }), "brazier");

    // The character walks through the campfire, drawn between its back and front (depths 0 and 1).
    this.character = this.add.rectangle(110, GROUND_Y + 50, 10, 20, 0xff00ff).setOrigin(0.5, 1).setDepth(0.5);
    this.tweens.add({ targets: this.character, x: 190, duration: 4000, yoyo: true, repeat: -1, ease: "Linear" });

    // A candle rides on a second walker; destroying the walker (D) leaves the candle to fade where it stood.
    this.walker = this.add.rectangle(40, 60, 8, 8, 0x40c0ff).setDepth(2);
    this.tweens.add({ targets: this.walker, x: 100, duration: 3000, yoyo: true, repeat: -1, ease: "Sine.easeInOut" });
    logged(this.pixfx.play("fire/candle", 40, 56, { seed: 5, depth: { back: 1.5, front: 3 } }), "follower").follow(this.walker, 0, -4);

    // Two braziers in one scene-wide layer at the right: four textures between them.
    const layer = this.pixfx.createLayer({ bounds: { x: 290, y: 0, width: 110, height: HEIGHT } });
    logged(layer.play("fire/brazier", 315, GROUND_Y + 60, { seed: 11 }), "layer-brazier-1");
    logged(layer.play("fire/brazier", 370, GROUND_Y + 40, { seed: 12 }), "layer-brazier-2");

    events.push("create");
  }
}

function rendererType(): number {
  return new URLSearchParams(location.search).get("renderer") === "canvas" ? Phaser.CANVAS : Phaser.WEBGL;
}

/** Keys live on the window, not the scene: a paused scene does not process its own input. */
function bindKeys(game: Phaser.Game): void {
  window.addEventListener("keydown", (e) => {
    const scene = game.scene.getScene("demo");
    if (!(scene instanceof DemoScene)) return;
    switch (e.key.toLowerCase()) {
      case "p":
        if (scene.scene.isPaused()) scene.scene.resume();
        else scene.scene.pause();
        break;
      case "s":
        scene.time.timeScale = scene.time.timeScale === 1 ? 0.25 : 1;
        break;
      case "x":
        scene.campfire?.stop();
        break;
      case "d":
        scene.walker?.destroy();
        break;
      case "r":
        scene.scene.restart();
        break;
    }
  });
}

async function main(): Promise<void> {
  const engine = await loadPixfx();
  const scene = new DemoScene();
  const game = new Phaser.Game({
    type: rendererType(),
    width: WIDTH,
    height: HEIGHT,
    parent: "game",
    zoom: 4,
    pixelArt: true,
    backgroundColor: "#000000",
    scene,
    plugins: { scene: [{ key: "pixfx", plugin: PixfxPlugin, mapping: "pixfx" }] },
  });
  bindKeys(game);
  window.pixfxDemo = {
    get scene() {
      const s = game.scene.getScene("demo");
      if (!(s instanceof DemoScene)) throw new Error("demo scene missing");
      return s;
    },
    game,
    engine,
    stats: () => scene.pixfx.stats(),
    events,
  };
}

void main();
