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
  outline: PixfxEffect | null = null;
  walker: Phaser.GameObjects.Image | null = null;
  hero: Phaser.GameObjects.Image | null = null;

  constructor() {
    super("demo");
  }

  /** An image of one `w` x `h` block of `color`, standing on its bottom middle. */
  private block(key: string, w: number, h: number, color: number): Phaser.GameObjects.Image {
    if (!this.textures.exists(key)) {
      const g = this.make.graphics({}, false);
      g.fillStyle(color).fillRect(0, 0, w, h);
      g.generateTexture(key, w, h);
      g.destroy();
    }
    return this.add.image(0, 0, key).setOrigin(0.5, 1);
  }

  create(): void {
    this.add.rectangle(0, 0, WIDTH, HEIGHT, 0x161620).setOrigin(0, 0).setDepth(-10);
    this.add.rectangle(0, GROUND_Y, WIDTH, HEIGHT - GROUND_Y, 0x24242e).setOrigin(0, 0).setDepth(-9);

    // A walker the outline rings and the trail smears, drawn between the effects' back and front (depths 0 and 1);
    // destroying it (D) leaves them to fade where it stood.
    const walker = this.block("walker", 8, 18, 0x4fd1c5).setPosition(90, GROUND_Y + 40).setDepth(0.5);
    this.walker = walker;
    this.tweens.add({ targets: walker, x: 170, duration: 3000, yoyo: true, repeat: -1, ease: "Sine.easeInOut" });
    logged(this.pixfx.play("projectiles/trail", 0, 0, { seed: 3 }), "trail").follow(walker, 0, -9).wrap(this.pixfx.actorOf(walker));
    this.outline = logged(this.pixfx.play("feedback/outline", 0, 0, { seed: 4 }), "outline")
      .follow(walker, 0, -9)
      .wrap(this.pixfx.actorOf(walker));

    // A hero powering up, again and again.
    const hero = this.block("hero", 12, 22, 0xd94f5c).setPosition(280, GROUND_Y + 50).setDepth(0.5);
    this.hero = hero;
    const powerUp = (): void => {
      if (!hero.active) return;
      logged(this.pixfx.play("feedback/power-up", hero.x, hero.y, { seed: 5 }), "power-up")
        .wrap(this.pixfx.actorOf(hero))
        .on("complete", powerUp);
    };
    powerUp();

    // A body bursts where you click: no character given, it blows apart a stand-in body standing there.
    this.input.on("pointerdown", (p: Phaser.Input.Pointer) => {
      logged(this.pixfx.play("explosions/body-burst", Math.round(p.worldX), Math.round(p.worldY)), "body-burst");
    });

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
        scene.outline?.stop();
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

main().catch((err: unknown) => {
  const help = document.getElementById("help");
  if (help) help.textContent = `pixfx demo failed to start: ${err instanceof Error ? err.message : String(err)} (run \`bun run build:wasm\` first)`;
});
