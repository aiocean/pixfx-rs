// A Pixi sprite as the character an effect plays on: its texture's pixels and where Pixi draws them.
import type { Container, Sprite } from "pixi.js";
import type { ActorSource, Cut } from "pixfx-web";
import { cutPixels } from "pixfx-web";

/** What placing a texture reads of a sprite: its position (in the effects' container), anchor and scale. */
export interface ActorSprite {
  x: number;
  y: number;
  anchor: { x: number; y: number };
  scale: { x: number; y: number };
}

/** What placing a texture reads of it: where it is cut from its source, its untrimmed size and its trim. */
export interface ActorTexture {
  frame: { x: number; y: number; width: number; height: number };
  orig: { width: number; height: number };
  trim?: { x: number; y: number } | null;
}

/**
 * Where Pixi draws `texture` on `sprite`, as `scene.setActor` takes it, and the region of its source it cuts: the
 * trimmed pixels placed about the anchor, at the sprite's scale; a negative x scale shows it mirrored. Without
 * rotation.
 */
export function actorPlace(sprite: ActorSprite, texture: ActorTexture): { at: { x: number; y: number; w: number; h: number; shownW: number; shownH: number; flipX: boolean }; cut: Cut } {
  const { frame, orig } = texture;
  const left = (texture.trim?.x ?? 0) - sprite.anchor.x * orig.width;
  const top = (texture.trim?.y ?? 0) - sprite.anchor.y * orig.height;
  const [sx, sy] = [sprite.scale.x, Math.abs(sprite.scale.y)];
  // a negative scale mirrors the drawn box about the anchor
  const x = sx < 0 ? sprite.x + sx * (left + frame.width) : sprite.x + sx * left;
  return {
    at: {
      x: Math.round(x),
      y: Math.round(sprite.y + top * sy),
      w: Math.max(1, Math.round(frame.width)),
      h: Math.max(1, Math.round(frame.height)),
      shownW: Math.max(1, Math.round(frame.width * Math.abs(sx))),
      shownH: Math.max(1, Math.round(frame.height * sy)),
      flipX: sx < 0,
    },
    cut: { x: frame.x, y: frame.y, w: frame.width, h: frame.height },
  };
}

/**
 * `sprite` as the character an effect plays on: `effect.wrap(pixfx.actorOf(sprite))`. Each update gives its texture
 * (each region of a source is read once), where it shows in `container` (the effects' container), at its scale and
 * mirrored by a negative x scale; none while it is hidden, destroyed, or its source has no image to read. Its
 * rotation and the scale of containers between it and `container` are not read.
 */
export function spriteActor(sprite: Sprite, container: Container): ActorSource {
  return () => {
    if (sprite.destroyed || !sprite.visible || !sprite.parent) return null;
    const texture = sprite.texture;
    const image: unknown = texture.source.resource;
    if (!(image instanceof HTMLImageElement || image instanceof HTMLCanvasElement || image instanceof ImageBitmap)) return null;
    const at = container.toLocal(sprite.position, sprite.parent);
    const place = actorPlace({ x: at.x, y: at.y, anchor: sprite.anchor, scale: sprite.scale }, texture);
    return { at: place.at, rgba: cutPixels(image, place.cut) };
  };
}
