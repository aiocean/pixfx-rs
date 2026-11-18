//! Appear: effects that play when a character, item or object enters the scene.
//!
//! An appear effect is always a one-shot. It plays at the object's position and shows the object
//! materialising. When it ends, the object is fully present and its own sprite takes over.
//!
//! It always wraps the object's sprite (`Effect::wraps_actor` = true) and reads it: a flourish that does not
//! read the sprite (a sparkle ring, a puff of smoke) is sprite art the game plays, not an effect here.
//!
//! It draws the sprite materialising pixel by pixel (a dissolve-in,
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
//! | `disappear/` | The exit counterpart: plays as the object leaves. |
//! | `feedback/` | Effects that stay on a live character (outline, power-up). |

#[cfg(feature = "appear-teleport-beam")]
mod teleport_beam;

#[cfg(feature = "appear-teleport-beam")]
pub use teleport_beam::TELEPORT_BEAM;
