export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Intersection of `effect` with `bounds`, or null when they share no area. */
export function clipRect(bounds: Rect, effect: Rect): Rect | null {
  const x = Math.max(bounds.x, effect.x);
  const y = Math.max(bounds.y, effect.y);
  const right = Math.min(bounds.x + bounds.w, effect.x + effect.w);
  const bottom = Math.min(bounds.y + bounds.h, effect.y + effect.h);
  if (right <= x || bottom <= y) return null;
  return { x, y, w: right - x, h: bottom - y };
}
