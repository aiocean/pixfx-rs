//! Disappear: effects that play when a character, item or object exits the scene.
//!
//! A disappear effect is always a one-shot. It plays at the object's last position and shows the
//! object dissolving. When it ends, the object is gone from the scene.
//!
//! # Two kinds
//!
//! ## Standalone (`Effect::wraps_actor` = false)
//!
//! The effect plays a visual flourish — a puff of smoke, a scatter of sparks — without reading the
//! object's sprite. The game controls when the sprite is hidden.
//!
//! Game contract:
//! 1. Hide the object's own sprite at the moment the effect's `desc` documents.
//! 2. Play the effect at the object's last position.
//!
//! ## Sprite-reading (`Effect::wraps_actor` = true)
//!
//! The effect wraps the object's sprite and shows it dissolving (pixels scattering, fading out,
//! a shimmer wipe). The game hides the object's sprite when the effect starts.
//!
//! Game contract:
//! 1. Hide the object's own sprite.
//! 2. Play the effect with `Scene::set_actor` giving it the object's sprite.
//! 3. When the effect ends, remove or keep the object hidden.
//!
//! A disappear effect never sets `Effect::replaces_actor`: that flag is only for effects that
//! permanently destroy the character (see `explosions/body-burst`). A disappear effect is
//! non-destructive — the game may bring the same object back later.
//!
//! # Tags
//!
//! Every disappear effect carries `"disappear"` and `"one-shot"`. Style tags describe the look:
//! `"pixel"`, `"dissolve"`, `"scatter"`, `"shimmer"`, `"teleport"`, `"magic"`, `"cartoon"`, `"melt"`, etc.
//!
//! # Differs from
//!
//! | Category | Description |
//! |---|---|
//! | `spawn/` | A cloud or burst after a death/despawn; not wrapping the sprite. |
//! | `appear/` | The entrance counterpart: plays as the object arrives. |
//! | `explosions/body-burst` | Permanent violent destruction (`replaces_actor = true`). |
//! | `transitions/` | Full-screen (16:9) scene wipes; not tied to one object. |

#[cfg(feature = "disappear-fade")]
mod fade;

#[cfg(feature = "disappear-fade")]
pub use fade::FADE;
