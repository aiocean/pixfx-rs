export { createPixfx, Pixfx } from "./pixfx";
export type { Layer, PixfxOptions } from "./pixfx";
export { canvasHolder } from "./canvas-gfx";
export type { DrawContext, Holder, MakeHolder, PictureLike } from "./canvas-gfx";
export { defaultWorkers, imageActor, loadBaked, loadPixfx, PixfxEffect, PixfxFeel } from "pixfx-web";
export type { ActorFrame, ActorSource, Counts, Cut, DrawnImage, EffectMeta, Engine, FollowTarget, LoadOptions, WorldCue } from "pixfx-web";
// the engine itself, usable once `loadPixfx()` resolved: its registry (`effects`, `meta`), an effect on its own
// (`Fx`) and the colour it lights its surroundings in (`lightColor`), and fighters and scene light for games that
// draw them (`Sprites`, `Lighting` give pictures to `drawPicture`)
export { Body, effects, FightFeel, Fx, Lighting, lightColor, lookColors, meta, Mind, putOutLight, Sprites, stepLights } from "pixfx-web/wasm";
