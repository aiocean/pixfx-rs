// One host scene's effects, played, timed and drawn by a Rust scene drawing from the game's frames. Rust decides
// what shows where; this follows its draw list with the shell's images.
import type * as Wasm from "../wasm/pixfx_wasm.js";
import type { Counts, EffectHost, FollowTarget } from "./effect";
import { PixfxEffect } from "./effect";
import type { Delivering, Game } from "./game";
import type { GfxImage, GfxImages } from "./gfx";
import type { Shape } from "./protocol";
import { OP_SHOW, OP_STRIDE } from "./surfaces";

/** A side of a surface holds pixels (as `pixfx-wasm` marks them). */
const SIDE_COLOR = 1;
const SIDE_LIGHT = 2;

const DEFAULT_DEPTHS = { back: 0, front: 1 };

/** A cue an effect fired, placed in the world: a ripple's `x`/`y` and a sound's `x` are world positions. */
export interface WorldCue {
  t: number;
  pan: number | null;
  cue: Wasm.Cue;
}

/** Where a scene's cues go: the scene's feel, which emits them. */
export type CueSink<P> = (cue: WorldCue, effect: PixfxEffect<P> | undefined) => void;

interface Follow {
  target: FollowTarget;
  dx: number;
  dy: number;
}

/** The colour image of a surface and the light image drawn over it. */
interface Pair<P> {
  color: GfxImage<P>;
  light: GfxImage<P>;
}

/** `P`: how the shell places an effect's images beyond their depth (see `GfxImage`). */
export class Stage<P> implements EffectHost<P>, Delivering {
  readonly scene: Wasm.Scene;
  readonly id: number;
  private readonly effects = new Map<number, PixfxEffect<P>>();
  private readonly follows = new Map<number, Follow>();
  private readonly pairs: Pair<P>[] = [];
  private cleared = false;

  /** @param place how images of an effect the stage no longer knows are placed (never in practice). */
  constructor(
    private readonly game: Game,
    private readonly images: GfxImages<P>,
    private readonly sink: CueSink<P>,
    private readonly place: P,
  ) {
    this.scene = game.frames.scene(1);
    this.id = this.scene.id();
    game.add(this);
  }

  /** The Rust scene while the stage runs, null once `clear()` let it go: what a feel reads (`new PixfxFeel(() => stage.live)`). */
  get live(): Wasm.Scene | null {
    return this.cleared ? null : this.scene;
  }

  /** Play effect `id` at world (`x`, `y`); throws on an unknown id or options the engine refuses. */
  play(id: string, x: number, y: number, engine: Record<string, unknown>, place: P, depth = DEFAULT_DEPTHS): PixfxEffect<P> {
    const handle = this.scene.play(this.game.frames, this.game.effectId(id), x, y, engine);
    const effect = new PixfxEffect<P>(this, handle, depth, place);
    this.effects.set(handle, effect);
    return effect;
  }

  stop(handle: number): void {
    this.scene.stop(handle);
  }

  remove(effect: PixfxEffect<P>): void {
    this.effects.delete(effect.handle);
    this.follows.delete(effect.handle);
    this.scene.remove(effect.handle);
  }

  follow(handle: number, target: FollowTarget | null, dx: number, dy: number): void {
    if (target) this.follows.set(handle, { target, dx, dy });
    else this.follows.delete(handle);
  }

  moveTo(handle: number, x: number, y: number): void {
    this.scene.moveTo(handle, x, y);
  }

  /**
   * Let `dt` seconds of scene time pass: followed effects move to their targets (an effect whose target went
   * inactive stops), then the scene plays. Emits the events. Returns the seconds the effects played (0 in a
   * hit-stop).
   */
  update(dt: number): number {
    for (const [handle, f] of this.follows) {
      if (f.target.active) {
        this.scene.moveTo(handle, f.target.x + f.dx, f.target.y + f.dy);
      } else {
        this.follows.delete(handle);
        this.scene.stop(handle);
      }
    }
    const played = this.scene.update(dt);
    for (const e of this.scene.events()) {
      const effect = this.effects.get(e.handle);
      if (e.type === "end") {
        this.effects.delete(e.handle);
        this.follows.delete(e.handle);
        effect?.complete();
      } else {
        const cue: WorldCue = { t: e.t, pan: e.pan, cue: e.cue };
        effect?.emit("cue", cue, effect);
        this.sink(cue, effect);
      }
    }
    return played;
  }

  /** Draw: follow the scene's draw list, showing each surface with a pair of images, and hide the rest. */
  draw(): void {
    const ops = this.scene.draw(this.game.frames);
    this.game.drawn(ops);
    const surfaces = this.game.surfaces;
    let used = 0;
    for (let i = 0; i + OP_STRIDE <= ops.length; i += OP_STRIDE) {
      if (ops[i] !== OP_SHOW) continue;
      const held = surfaces.get(ops[i + 1] ?? -1);
      if (!held?.pooled) continue;
      const [x = 0, y = 0, w = 0, h = 0, handle = -1, layer = 0] = ops.subarray(i + 2, i + 8);
      const effect = this.effects.get(handle);
      const depth = effect?.depthOf(layer) ?? layer;
      const place = effect ? effect.place : this.place;
      const pair = this.pair(used++);
      const t = held.pooled.texture;
      if (held.sides & SIDE_COLOR) pair.color.show(t.key, t.color, x, y, w, h, depth, place);
      else pair.color.hide();
      if (held.sides & SIDE_LIGHT) pair.light.show(t.key, t.light, x, y, w, h, depth, place);
      else pair.light.hide();
    }
    for (let i = used; i < this.pairs.length; i++) {
      this.pairs[i]?.color.hide();
      this.pairs[i]?.light.hide();
    }
  }

  deliver(job: number, shape: Shape): Int32Array {
    return this.scene.deliver(this.game.frames, job, shape);
  }

  counts(): Counts {
    return { engines: this.effects.size, textures: this.game.counts().textures, images: this.pairs.length * 2 };
  }

  /** End every effect (each emits `"destroy"`), free the scene's surfaces and images, and let the scene go. */
  clear(): void {
    this.game.apply(this.scene.clear(this.game.frames));
    for (const e of [...this.effects.values()]) e.end();
    this.effects.clear();
    this.follows.clear();
    for (const p of this.pairs) {
      p.color.destroy();
      p.light.destroy();
    }
    this.pairs.length = 0;
    this.game.remove(this);
    this.cleared = true;
    this.scene.free();
  }

  /** The `i`-th pair of images, made the first time: made in show order, so equal depths draw in show order. */
  private pair(i: number): Pair<P> {
    let p = this.pairs[i];
    if (!p) {
      p = { color: this.images("normal"), light: this.images("add") };
      this.pairs.push(p);
    }
    return p;
  }
}
