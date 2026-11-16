//! Shadow: ports of pixelfx `src/fx/shadow`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "shadow-blood-gush")]
mod blood_gush;
#[cfg(feature = "shadow-blood-shield")]
mod blood_shield;
#[cfg(feature = "shadow-blood-shield")]
mod blood_shield_veins;
#[cfg(feature = "shadow-void-orb")]
mod void_orb;
#[cfg(feature = "shadow-winter-ordeal")]
mod winter_ordeal;
#[cfg(feature = "shadow-winter-ordeal")]
mod winter_ordeal_sky;

#[cfg(feature = "shadow-blood-gush")]
pub use blood_gush::BLOOD_GUSH;
#[cfg(feature = "shadow-blood-shield")]
pub use blood_shield::BLOOD_SHIELD;
#[cfg(feature = "shadow-void-orb")]
pub use void_orb::VOID_ORB;
#[cfg(feature = "shadow-winter-ordeal")]
pub use winter_ordeal::WINTER_ORDEAL;
