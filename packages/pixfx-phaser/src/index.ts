import { PixfxEffect as WebEffect } from "pixfx-web";

export { PixfxPlugin } from "./plugin";
export { PixfxSpriteFeel } from "./sprite-feel";
export type { FeelSprite } from "./sprite-feel";
export { defaultWorkers, loadBaked, loadPixfx, PixfxFeel } from "pixfx-web";
export type { Counts, EffectMeta, Engine, FollowTarget, LoadOptions, WorldCue } from "pixfx-web";

/** One effect in a Phaser scene: its images are placed by its scroll factor. */
export const PixfxEffect = WebEffect;
export type PixfxEffect = WebEffect<number>;
