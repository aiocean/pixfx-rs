// The JS runtime every JS shell (Phaser, Pixi, ...) shares: the engine in wasm, the painting workers, the game-wide
// frames and their surfaces, and a stage per host scene that follows the Rust scene's draw list. A shell supplies
// only its host objects (`GfxTextures`, `GfxImages`) and its loop.
export { Game } from "./game";
export type { Delivering, WorkerPort } from "./game";
export { drawBudget, Stage } from "./stage";
export type { CueSink, WorldCue } from "./stage";
export { cutPixels, imageActor } from "./actor";
export type { ActorFrame, ActorSource, Cut, DrawnImage } from "./actor";
export { PixfxEffect } from "./effect";
export type { Aim, Counts, EffectHost, FollowTarget, Strike } from "./effect";
export { PixfxFeel } from "./feel";
export { splitPlayOptions } from "./options";
export type { Play } from "./options";
export { defaultWorkers, loadBaked, loadedEngine, loadedGame, loadPixfx, openWorkers, workerPort } from "./load";
export type { LoadOptions } from "./load";
export { wasmEngine } from "./engine";
export type { BlinkLike, EffectMeta, Engine, SquashLike } from "./engine";
export type { GfxBlend, GfxImage, GfxImages, GfxTexture, GfxTextures } from "./gfx";
