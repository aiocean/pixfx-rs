# pixfx-web

The JS runtime under every pixfx JS shell (`pixfx-pixi`, and `pixfx-phaser` next). It loads the engine (Rust compiled
to WebAssembly), starts the workers that paint, holds the frames every scene of a game shares, and follows the draw
list the Rust scene hands it each frame. A shell gives it two things: room for a surface (`GfxTextures`) and images
that show one (`GfxImages`), and calls a `Stage` from its loop.

Games use a shell package, not this one, except for `loadPixfx()` and `loadBaked()`, which every shell re-exports.

```sh
bun install
bun run build:wasm   # wasm-pack → wasm/ (git-ignored)
bun run check        # tsc, eslint, bun test
```
