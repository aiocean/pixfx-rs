/** Effect step in seconds for one scene update; anything not finite or not positive yields 0. */
export function stepSeconds(deltaMs: number, timeScale: number): number {
  const s = (deltaMs / 1000) * timeScale;
  return Number.isFinite(s) && s > 0 ? s : 0;
}
