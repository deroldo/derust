//! Server-driven UI (SDUI) support.
//!
//! Each supported engine lives in its own submodule, enabled by its own feature.

#[cfg(feature = "sdui_stac_1")]
pub mod stacx;
