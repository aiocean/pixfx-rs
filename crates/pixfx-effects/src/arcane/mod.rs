//! Arcane: ports of pixelfx `src/fx/arcane`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(any(
    feature = "arcane-bagua-mirror",
    feature = "arcane-bagua-mirror-burst",
    feature = "arcane-bagua-mirror-flying",
    feature = "arcane-bagua-mirror-strike"
))]
mod bagua;

#[cfg(feature = "arcane-bagua-mirror")]
mod bagua_mirror;

#[cfg(feature = "arcane-bagua-mirror")]
pub use bagua_mirror::BAGUA_MIRROR;

// the strike's volley flies these, so they come with it
#[cfg(any(
    feature = "arcane-bagua-mirror-burst",
    feature = "arcane-bagua-mirror-strike"
))]
mod bagua_mirror_burst;

#[cfg(any(
    feature = "arcane-bagua-mirror-burst",
    feature = "arcane-bagua-mirror-strike"
))]
pub use bagua_mirror_burst::BAGUA_MIRROR_BURST;

#[cfg(any(
    feature = "arcane-bagua-mirror-flying",
    feature = "arcane-bagua-mirror-strike"
))]
mod bagua_mirror_flying;

#[cfg(any(
    feature = "arcane-bagua-mirror-flying",
    feature = "arcane-bagua-mirror-strike"
))]
pub use bagua_mirror_flying::BAGUA_MIRROR_FLYING;

#[cfg(feature = "arcane-bagua-mirror-strike")]
mod bagua_mirror_strike;

#[cfg(feature = "arcane-bagua-mirror-strike")]
pub use bagua_mirror_strike::BAGUA_MIRROR_STRIKE;

#[cfg(feature = "arcane-hex-shield")]
mod hex_shield;

#[cfg(feature = "arcane-restriction")]
mod restriction;

#[cfg(feature = "arcane-restriction")]
pub use restriction::RESTRICTION;

#[cfg(feature = "arcane-rune-circle")]
mod rune_circle;

#[cfg(feature = "arcane-rune-circle")]
pub use rune_circle::RUNE_CIRCLE;

#[cfg(feature = "arcane-hex-shield")]
pub use hex_shield::HEX_SHIELD;

#[cfg(any(
    feature = "arcane-flying-sword",
    feature = "arcane-held-sword",
    feature = "arcane-sword-formation",
    feature = "arcane-sword-slash"
))]
mod sword;

#[cfg(feature = "arcane-sword-slash")]
mod sword_slash;

#[cfg(feature = "arcane-sword-slash")]
pub use sword_slash::SWORD_SLASH;

#[cfg(feature = "arcane-sword-formation")]
mod sword_formation;

// the formation's volley flies these, so they come with it
#[cfg(any(feature = "arcane-flying-sword", feature = "arcane-sword-formation"))]
mod flying_sword;
#[cfg(any(feature = "arcane-held-sword", feature = "arcane-sword-formation"))]
mod held_sword;

#[cfg(any(feature = "arcane-flying-sword", feature = "arcane-sword-formation"))]
pub use flying_sword::FLYING_SWORD;
#[cfg(any(feature = "arcane-held-sword", feature = "arcane-sword-formation"))]
pub use held_sword::HELD_SWORD;

#[cfg(any(feature = "arcane-sword-strike", feature = "arcane-sword-formation"))]
mod sword_strike;

#[cfg(any(feature = "arcane-sword-strike", feature = "arcane-sword-formation"))]
pub use sword_strike::SWORD_STRIKE;

#[cfg(feature = "arcane-sword-formation")]
pub use sword_formation::SWORD_FORMATION;
