# Notices

pixfx-rs builds on three MIT-licensed projects:

- **pixelfx**, Copyright (c) 2026 Patch, Copyright (c) 2026 Hugo Duprez, MIT License
  (https://github.com/patch-gg/pixelfx — commit 044e12a). pixfx-rs ports its engine and effects to Rust. Its own
  notices follow verbatim.
- **Sprite Fusion pixel art VFX**, Copyright (c) 2026 Hugo Duprez, MIT License
  (https://www.spritefusion.com/pixel-art-vfx). pixfx-rs ports the effects pixelfx does not have to Rust; their
  sources are vendored verbatim under `tools/spritefusion/`, each keeping its own license header.
- **beanfx**, Copyright (c) 2026 bien, MIT License (https://github.com/baenv/beanfx). pixfx-rs sprite sheets originally used
  the beanfx layout and retain its rule for compositing light over transparency.

## pixelfx notices (verbatim)

**Hugo Duprez.** `src/core/sim.js` and the effects that say so at the top of their file adapt Sprite Fusion's
open-source pixel art VFX (https://www.spritefusion.com/pixel-art-vfx), Copyright (c) 2026 Hugo Duprez, MIT License.

**Hollow Crown.** `src/core/act.js` adapts how Hollow Crown plays its skills (poses on 12 fps steps, hit-stops, the springy
straw target), from
https://github.com/rehan-remade/hollow-crown, Copyright (c) 2026 rehan-remade, MIT License.

**Patch Pixel.** The font in `gallery/fonts` is Patch's own and is not covered by this repository's MIT license. See
`gallery/fonts/NOTICE.md`.
