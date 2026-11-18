//! Disappear: effects that play when a character, item or object exits the scene.
//!
//! A disappear effect is always a one-shot. It plays at the object's last position and shows the
//! object dissolving. When it ends, the object is gone from the scene.
//!
//! It always wraps the object's sprite (`Effect::wraps_actor` = true) and reads it: a flourish that does not
//! read the sprite (a sparkle ring, a puff of smoke) is sprite art the game plays, not an effect here.
//!
//! It shows the sprite dissolving (pixels scattering, fading out,
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
//! | `appear/` | The entrance counterpart: plays as the object arrives. |
//! | `explosions/body-burst` | Permanent violent destruction (`replaces_actor = true`). |
#[cfg(feature = "disappear-blink-out")]
mod blink_out;
#[cfg(feature = "disappear-dither")]
mod dither;
#[cfg(feature = "disappear-fade")]
mod fade;
#[cfg(feature = "disappear-incinerate")]
mod incinerate;

#[cfg(feature = "disappear-blink-out")]
pub use blink_out::BLINK_OUT;
#[cfg(feature = "disappear-dither")]
pub use dither::DITHER;
#[cfg(feature = "disappear-fade")]
pub use fade::FADE;
#[cfg(feature = "disappear-incinerate")]
pub use incinerate::INCINERATE;
