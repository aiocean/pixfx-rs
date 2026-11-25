---
name: pixfx-demo
description: Design, build, or update playable effect demos on the pixfx website. Use when showing how an existing or new effect works in a game or interface, including several use cases on one effect page.
---

# Demo a pixfx effect

Read the effect's Rust implementation and metadata, then inspect `site/effects/effect.html`,
`site/assets/effect.js` and `site/assets/sprite-show.js` for the current effect page and scene helpers.
When working from `pixfx-new-effect`, start with the storyboard's candidate uses.

## Pick scenarios

Look at what the effect reads and how it can play. Check whether following a moving object, reading a sprite
or trace, changing options, or playing alongside another effect gives it a distinct role. For each plausible
use, describe the trigger, target or anchor, what the player should read, and the options or composition it needs.

Start with the clearest use. Show each additional distinct use as its own playable example on the same effect
page. Keep one example when it covers the effect's role. Use presets or sliders for variations within a use case.

For **Comet Weave**, two scenarios can use the same effect:

- **Projectile**. Fire a projectile with the weave attached. Show the launch, travel and arrival.
- **Moving object**. Move a sprite along a path while the weave orbits it. Show that the weave stays anchored
  as the sprite changes direction.

## Build and verify

- Give each scenario a short label and its own trigger or replay control. Show enough of the surrounding
  action or interface to make the trigger and result clear. Check that playing one demo leaves the others usable.
- Reuse the effect's existing ID, runtime metadata and options. The demo scenario supplies its game action,
  such as launching a projectile or moving a target. Use the normal scene and shell APIs for placement and
  follow. Rust owns the weave's geometry, timing and pixels.
- Build and serve locally with `site/build.sh --serve`. Open the effect page, trigger and replay each example,
  and verify the intended motion and cleanup. Check that the demos and controls fit at a narrow viewport.
  When handing back the result, link the effect page and name the scenarios you verified.
