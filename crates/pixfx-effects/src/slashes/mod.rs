//! Procedural blade arcs and motion trails.

#[path = "lib.rs"]
mod painting;

#[cfg(feature = "slashes-slash-1")]
mod slash_1;
#[cfg(feature = "slashes-slash-1")]
pub use slash_1::SLASH_1;

#[cfg(feature = "slashes-slash-2")]
mod slash_2;
#[cfg(feature = "slashes-slash-2")]
pub use slash_2::SLASH_2;

#[cfg(feature = "slashes-slash-3")]
mod slash_3;
#[cfg(feature = "slashes-slash-3")]
pub use slash_3::SLASH_3;
