//! Appear: effects that play when a character, item or object enters the scene.
//!
//! An appear effect is always a one-shot. It plays at the object's position and shows the object
//! materialising. When it ends, the object is fully present and its own sprite takes over.
//!
//! # Two kinds
//!
//! ## Standalone (`Effect::wraps_actor` = false)
//!
//! The effect plays a visual flourish — a sparkle ring, a flash, a swirl — without reading the
//! object's sprite. The game controls when the sprite becomes visible.
//!
//! Game contract:
//! 1. Play the effect at the object's position.
//! 2. Show the object's own sprite at the moment the effect's `desc` documents (the effect's
//!    midpoint, its end, or immediately).
//!
//! ## Sprite-reading (`Effect::wraps_actor` = true)
//!
//! The effect wraps the object's sprite and draws it materialising pixel by pixel (a dissolve-in,
//! a shimmer, a crystal-build). The game hides the object's sprite while the effect plays so it
//! does not show twice.
//!
//! Game contract:
//! 1. Hide the object's own sprite.
//! 2. Play the effect with `Scene::set_actor` giving it the object's sprite.
//! 3. When the effect ends, show the object's sprite again.
//!
//! An appear effect never sets `Effect::replaces_actor`: the object is not destroyed, it is arriving.
//!
//! # Tags
//!
//! Every appear effect carries `"appear"` and `"one-shot"`. Style tags describe the look:
//! `"pixel"`, `"dissolve"`, `"materialize"`, `"shimmer"`, `"teleport"`, `"magic"`, `"cartoon"`, etc.
//!
//! # Differs from
//!
//! | Category | Description |
//! |---|---|
//! | `spawn/` | A cloud or burst that plays *after* a death/despawn; the character is already gone. |
//! | `disappear/` | The exit counterpart: plays as the object leaves. |
//! | `transitions/` | Full-screen (16:9) scene wipes; not tied to one object. |
//! | `feedback/` | Looping effects that persist on a live character (outline, power-up). |