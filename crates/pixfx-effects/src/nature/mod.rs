//! Nature: ports of pixelfx `src/fx/nature`.

#[cfg(feature = "nature-poison-cloud")]
mod poison_cloud;

#[cfg(feature = "nature-poison-cloud")]
pub use poison_cloud::POISON_CLOUD;
