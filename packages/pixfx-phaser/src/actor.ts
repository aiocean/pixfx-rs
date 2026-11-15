// A Phaser sprite as the character an effect plays on: its current frame's pixels and where Phaser draws them.
import type Phaser from "phaser";
import type { ActorSource, Cut } from "pixfx-web";
import { cutPixels } from "pixfx-web";

/** What placing a frame reads of a sprite (Phaser's `Sprite` and `Image` both have it). */
export interface ActorSprite {
  x: number;
  y: number;
  scaleX: number;
  scaleY: number;
  flipX: boolean;
  displayOriginX: number;
  displayOriginY: number;
}

/** What placing a frame reads of the frame: where it is cut from its texture and its trim. */
export interface ActorFrameCut {
  cutX: number;
  cutY: number;
  cutWidth: number;
  cutHeight: number;
  /** The trimmed pixels' offset inside the frame's untrimmed box. */
  x: number;
  y: number;
  realWidth: number;
}

/**
 * Where Phaser draws `frame` of `sprite` in the world, as `scene.setActor` takes it, and the region of its texture
 * it cuts: Phaser's own placement (`TransformerImage`), flipped about its origin with `flipX`, without rotation.
 */
export function actorPlace(sprite: ActorSprite, frame: ActorFrameCut): { at: { x: number; y: number; w: number; h: number; shownW: number; shownH: number; flipX: boolean }; cut: Cut } {
  const [sx, sy] = [Math.abs(sprite.scaleX), Math.abs(sprite.scaleY)];
  const left = sprite.flipX
    ? frame.realWidth - sprite.displayOriginX - frame.x - frame.cutWidth
    : frame.x - sprite.displayOriginX;
  const top = frame.y - sprite.displayOriginY;
  return {
    at: {
      x: Math.round(sprite.x + left * sx),
      y: Math.round(sprite.y + top * sy),
      w: Math.max(1, Math.round(frame.cutWidth)),
      h: Math.max(1, Math.round(frame.cutHeight)),
      shownW: Math.max(1, Math.round(frame.cutWidth * sx)),
      shownH: Math.max(1, Math.round(frame.cutHeight * sy)),
      flipX: sprite.flipX,
    },
    cut: { x: frame.cutX, y: frame.cutY, w: frame.cutWidth, h: frame.cutHeight },
  };
}

/**
 * `sprite` as the character an effect plays on: `effect.wrap(spriteActor(sprite))`. Each update gives its current
 * frame (each frame of its texture is read once), where it shows, at its scale and `flipX`; none while it is
 * inactive, hidden, or its texture has no image to read (a render texture). Its rotation and a parent container
 * are not read.
 */
export function spriteActor(sprite: Phaser.GameObjects.Sprite | Phaser.GameObjects.Image): ActorSource {
  return () => {
    if (!sprite.active || !sprite.visible) return null;
    const frame = sprite.frame;
    const image = frame.source.image;
    if (!(image instanceof HTMLImageElement || image instanceof HTMLCanvasElement)) return null;
    const { at, cut } = actorPlace(sprite, frame);
    return { at, rgba: cutPixels(image, cut) };
  };
}
