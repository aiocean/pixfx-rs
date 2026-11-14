import type { BlinkLike, SquashLike } from "./engine";

/** What a sprite with feel looks like this frame, over its own scale and alpha. */
export interface Look {
  scaleX: number;
  scaleY: number;
  alpha: number;
}

/** A velocity in pixels a second, y down. */
export interface Velocity {
  x: number;
  y: number;
}

/**
 * One sprite's squash, stretch and blink, with no Phaser calls: each step it hands the engine where the sprite is
 * and its velocity when it has one (the engine works it out from movement otherwise) and says how to draw it.
 */
export class SpriteFeelCore {
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
   * Let `dt` seconds of play pass for a sprite now at `(x, y)`, moving at `velocity` when the game knows it (a
   * physics body), and return how to draw it. A frame with no play time (a hit-stop) holds its shape.
   */
  step(dt: number, x: number, y: number, velocity: Velocity | null): Look {
    const [scaleX, scaleY] = this.squash.step(dt, x, y, velocity?.x, velocity?.y);
    this.blinking.advance(dt);
    return { scaleX, scaleY, alpha: this.blinking.alpha() };
  }

  free(): void {
    this.squash.free();
    this.blinking.free();
  }
}

/** What `scene.pixfx.squash(sprite)` drives: a Phaser sprite or image (any game object with these). */
export interface FeelSprite {
  x: number;
  y: number;
  scaleX: number;
  scaleY: number;
  alpha: number;
  active: boolean;
  /** An arcade body: its velocity is read from it when there is one. */
  body?: unknown;
  setScale(x: number, y: number): unknown;
  setAlpha(alpha: number): unknown;
}

function velocityOf(body: unknown): Velocity | null {
  if (typeof body !== "object" || body === null || !("velocity" in body)) return null;
  const v: unknown = body.velocity;
  if (typeof v !== "object" || v === null || !("x" in v) || !("y" in v)) return null;
  const { x, y } = v;
  return typeof x === "number" && typeof y === "number" && Number.isFinite(x) && Number.isFinite(y)
    ? { x, y }
    : null;
}

/**
 * One sprite's squash, stretch and blink (`scene.pixfx.squash(sprite)`). Each scene update it stretches the
 * sprite along its velocity when it moves fast (upright: tall for a rise or fall, wide for a dash, since a Phaser
 * sprite cannot skew), flattens it as it lands (`land`) and eases back, and blinks it while the game
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
   * @internal Let `dt` seconds of play pass and draw the sprite for them (the engine holds its shape when none
   * pass: a hit-stop). A sprite gone inactive (destroyed) ends it.
   */
  step(dt: number): void {
    if (this.gone) return;
    if (!this.sprite.active) {
      this.destroy();
      return;
    }
    const look = this.core.step(dt, this.sprite.x, this.sprite.y, velocityOf(this.sprite.body));
    this.sprite.setScale(this.base.scaleX * look.scaleX, this.base.scaleY * look.scaleY);
    this.sprite.setAlpha(this.base.alpha * look.alpha);
  }
}
