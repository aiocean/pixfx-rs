//! Melee: `sword-slash`, the smear a blade leaves through a cut.

#[cfg(feature = "melee-sword-slash")]
mod sword_slash;

#[cfg(feature = "melee-sword-slash")]
pub use sword_slash::SWORD_SLASH;
