//! Shadow: ports of pixelfx `src/fx/shadow`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "shadow-blood-shield")]
mod blood_shield;
#[cfg(feature = "shadow-void-orb")]
mod void_orb;

#[cfg(feature = "shadow-blood-shield")]
pub use blood_shield::BLOOD_SHIELD;
#[cfg(feature = "shadow-void-orb")]
pub use void_orb::VOID_ORB;
