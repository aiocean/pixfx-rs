//! Impacts: ports of pixelfx `src/fx/impacts`. [`lib`] is the shared `_lib.js`.

pub mod lib;

#[cfg(feature = "impacts-parry")]
mod parry;

#[cfg(feature = "impacts-parry")]
pub use parry::PARRY;

#[cfg(feature = "impacts-star-converge")]
mod star_converge;

#[cfg(feature = "impacts-star-converge")]
pub use star_converge::STAR_CONVERGE;

#[cfg(any(
    feature = "impacts-star-converge",
    feature = "impacts-twin-converge",
    feature = "projectiles-drop-dart"
))]
pub(crate) mod drops;

#[cfg(feature = "impacts-twin-converge")]
mod twin_converge;

#[cfg(feature = "impacts-twin-converge")]
pub use twin_converge::TWIN_CONVERGE;
