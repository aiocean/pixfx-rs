import type { BlinkLike, SquashLike } from "./engine";

/** What a sprite with feel looks like this frame, over its own scale and alpha. */
export interface Look {
  scaleX: number;
  scaleY: number;
  alpha: number;
}

/**
 * One sprite's squash, stretch and blink, with no Phaser calls: each step it reads how fast the sprite moved up or
 * down (from its velocity when it has one, else from how far it moved) and says how to draw it.
 */
export class SpriteFeelCore {
  private lastY: number | null = null;

  constructor(
    private readonly squash: SquashLike,
    private readonly blinking: BlinkLike,
  ) {}

  /** It landed with `strength` (0 to 1). */
  land(strength: number): void {
    this.squash.land(strength);
  }

  /** Blink for `secs` seconds (while the game keeps it from harm). */
  blink(secs: number): void {
    this.blinking.start(secs);
  }

  get isBlinking(): boolean {
    return this.blinking.active();
  }

  /**
   * Let `dt` seconds of play pass for a sprite now at height `y`, moving up or down at `vy` px/s when the game
   * knows it (a physics body), and return how to draw it. A frame with no play time (a hit-stop) holds its shape.
   */
  step(dt: number, y: number, vy: number | null): Look {
    const moved = this.lastY === null || dt <= 0 ? 0 : (y - this.lastY) / dt;
    this.lastY = y;
    this.squash.advance(dt);
    this.blinking.advance(dt);
    const [scaleX, scaleY] = this.squash.scale(vy ?? moved);
    return { scaleX, scaleY, alpha: this.blinking.alpha() };
  }

  free(): void {
    this.squash.free();
    this.blinking.free();
  }
}

/** What `scene.pixfx.squash(sprite)` drives: a Phaser sprite or image (any game object with these). */
export interface FeelSprite {
  y: number;
  scaleX: number;
  scaleY: number;
  alpha: number;
  active: boolean;
  /** An arcade body: its vertical speed is read from it when there is one. */
  body?: unknown;
  setScale(x: number, y: number): unknown;
  setAlpha(alpha: number): unknown;
}

function velocityY(body: unknown): number | null {
  if (typeof body !== "object" || body === null || !("velocity" in body)) return null;
  const v: unknown = body.velocity;
  if (typeof v !== "object" || v === null || !("y" in v)) return null;
  return typeof v.y === "number" && Number.isFinite(v.y) ? v.y : null;
}

/**
 * One sprite's squash, stretch and blink (`scene.pixfx.squash(sprite)`). Each scene update it stretches the
 * sprite along a fast rise or fall, flattens it as it lands (`land`) and eases back, and blinks it while the game
 * keeps it from harm (`blink`). It sets the sprite's scale and alpha over a base (`setBase`, by default what they
 * were when it was made), so give the sprite its origin at its feet (`setOrigin(0.5, 1)`) to keep them on the
 * floor. Its body for collisions keeps its size. In a hit-stop it holds its shape.
 */
export class PixfxSpriteFeel {
  private base: { scaleX: number; scaleY: number; alpha: number };
  private gone = false;

  /** @internal Made by the plugin's `squash()`; the sprite's scale and alpha now are the base. */
  constructor(
    private readonly sprite: FeelSprite,
    private readonly core: SpriteFeelCore,
    private readonly onGone: (s: PixfxSpriteFeel) => void,
  ) {
    this.base = { scaleX: sprite.scaleX, scaleY: sprite.scaleY, alpha: sprite.alpha };
  }

  get isBlinking(): boolean {
    return !this.gone && this.core.isBlinking;
  }

  /** It landed with `strength` (0 to 1: how hard, its fall's speed over the fastest you expect). */
  land(strength: number): this {
    this.core.land(strength);
    return this;
  }

  /** Blink for `secs` seconds; the protection itself is the game's rule. */
  blink(secs: number): this {
    this.core.blink(secs);
    return this;
  }

  /** The scale and alpha the feel works over, when the game changes them. */
  setBase(scaleX: number, scaleY: number, alpha = 1): this {
    this.base = { scaleX, scaleY, alpha };
    return this;
  }

  /** Stop driving the sprite; it keeps its base scale and alpha. Does nothing twice. */
  destroy(): void {
    if (this.gone) return;
    this.gone = true;
    if (this.sprite.active) {
      this.sprite.setScale(this.base.scaleX, this.base.scaleY);
      this.sprite.setAlpha(this.base.alpha);
    }
    this.core.free();
    this.onGone(this);
  }

  /**
   * @internal Let `dt` seconds of play pass and draw the sprite for them; with none (a hit-stop) it keeps its
   * shape. A sprite gone inactive (destroyed) ends it.
   */
  step(dt: number): void {
    if (this.gone) return;
    if (!this.sprite.active) {
      this.destroy();
      return;
    }
    if (!(dt > 0)) return;
    const look = this.core.step(dt, this.sprite.y, velocityY(this.sprite.body));
    this.sprite.setScale(this.base.scaleX * look.scaleX, this.base.scaleY * look.scaleY);
    this.sprite.setAlpha(this.base.alpha * look.alpha);
  }
}
